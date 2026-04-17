use std::collections::HashMap;
use std::sync::Arc;

use chrono::NaiveDateTime;
use sqlx::{MySqlPool, Row};
use tokio::sync::Mutex;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// In-memory lock entry mirroring the DB row.
#[derive(Debug, Clone)]
pub struct LockEntry {
    pub lock_key: String,
    pub user_id: i64,
    pub expires_at: NaiveDateTime,
    pub reservation_id: Option<i64>,
}

/// Handle returned on successful acquisition.
#[derive(Debug, Clone)]
pub struct LockHandle {
    pub lock_key: String,
    pub acquired_at: NaiveDateTime,
    pub expires_at: NaiveDateTime,
}

/// Errors that can occur during lock operations.
#[derive(Debug)]
pub enum LockError {
    AlreadyLocked {
        holder_id: i64,
        expires_at: String,
    },
    StockExhausted,
    DatabaseError(String),
}

impl std::fmt::Display for LockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LockError::AlreadyLocked { holder_id, expires_at } => {
                write!(
                    f,
                    "Lock already held by user {} until {}",
                    holder_id, expires_at
                )
            }
            LockError::StockExhausted => write!(f, "Stock exhausted for this SKU/option combination"),
            LockError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
        }
    }
}

// ---------------------------------------------------------------------------
// ReservationLockManager
// ---------------------------------------------------------------------------

/// Manages per-SKU-option pessimistic locks backed by MySQL.
#[derive(Clone)]
pub struct ReservationLockManager {
    inner: Arc<Mutex<HashMap<String, LockEntry>>>,
}

impl ReservationLockManager {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

// ---------------------------------------------------------------------------
// Key generation
// ---------------------------------------------------------------------------

/// Build a deterministic lock key for a given SKU + sorted option-value IDs.
/// Example: `sku:123:opts:1,2,5`
pub fn lock_key_for_sku_options(sku_id: i64, option_value_ids: &[i64]) -> String {
    let mut sorted = option_value_ids.to_vec();
    sorted.sort_unstable();
    let opts: Vec<String> = sorted.iter().map(|id| id.to_string()).collect();
    format!("sku:{}:opts:{}", sku_id, opts.join(","))
}

// ---------------------------------------------------------------------------
// Core operations
// ---------------------------------------------------------------------------

/// Try to acquire a concurrency lock for the given key.
///
/// Uses `SELECT ... FOR UPDATE` on the `reservation_locks` table so that only
/// one transaction can inspect/modify a row at a time.  If the lock is already
/// held by another user and has not expired, returns `AlreadyLocked`.  If it
/// exists but is expired, the previous holder's inventory is released first.
///
/// On success the SKU stock is decremented and a `LockHandle` is returned.
pub async fn try_acquire(
    pool: &MySqlPool,
    manager: &ReservationLockManager,
    key: &str,
    sku_id: i64,
    user_id: i64,
    quantity: i32,
    hold_duration_secs: u64,
) -> Result<LockHandle, LockError> {
    let now = chrono::Utc::now().naive_utc();
    let expires_at = now
        + chrono::Duration::seconds(hold_duration_secs as i64);

    // -- transactional work against MySQL --------------------------------
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| LockError::DatabaseError(e.to_string()))?;

    // Attempt to lock the row (or discover it doesn't exist yet).
    let existing = sqlx::query(
        "SELECT lock_key, user_id, expires_at, released
         FROM reservation_locks
         WHERE lock_key = ?
         FOR UPDATE",
    )
    .bind(key)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| LockError::DatabaseError(e.to_string()))?;

    if let Some(row) = existing {
        let released: bool = row.get("released");
        let holder_id: i64 = row.get("user_id");
        let row_expires: NaiveDateTime = row.get("expires_at");

        if !released && holder_id != user_id && row_expires > now {
            // Still held by someone else.
            tx.rollback()
                .await
                .map_err(|e| LockError::DatabaseError(e.to_string()))?;
            return Err(LockError::AlreadyLocked {
                holder_id,
                expires_at: row_expires.format("%Y-%m-%dT%H:%M:%S").to_string(),
            });
        }

        // Expired or same user or already released -- reclaim.
        // If not yet released, restore stock first.
        if !released {
            let qty: i32 = sqlx::query(
                "SELECT quantity FROM reservation_locks WHERE lock_key = ? FOR UPDATE",
            )
            .bind(key)
            .fetch_one(&mut *tx)
            .await
            .map(|r| r.get("quantity"))
            .unwrap_or(1);

            sqlx::query("UPDATE sku SET stock_quantity = stock_quantity + ? WHERE id = ?")
                .bind(qty)
                .bind(sku_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| LockError::DatabaseError(e.to_string()))?;
        }

        // Check stock before decrementing
        let stock: i32 =
            sqlx::query("SELECT stock_quantity FROM sku WHERE id = ?")
                .bind(sku_id)
                .fetch_one(&mut *tx)
                .await
                .map(|r| r.get("stock_quantity"))
                .map_err(|e| LockError::DatabaseError(e.to_string()))?;

        if stock < quantity {
            tx.rollback()
                .await
                .map_err(|e| LockError::DatabaseError(e.to_string()))?;
            return Err(LockError::StockExhausted);
        }

        // Decrement stock by the requested quantity
        sqlx::query("UPDATE sku SET stock_quantity = stock_quantity - ? WHERE id = ?")
            .bind(quantity)
            .bind(sku_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| LockError::DatabaseError(e.to_string()))?;

        // Update the lock row
        sqlx::query(
            "UPDATE reservation_locks
             SET user_id = ?, acquired_at = ?, expires_at = ?, released = FALSE, quantity = ?
             WHERE lock_key = ?",
        )
        .bind(user_id)
        .bind(now)
        .bind(expires_at)
        .bind(quantity)
        .bind(key)
        .execute(&mut *tx)
        .await
        .map_err(|e| LockError::DatabaseError(e.to_string()))?;
    } else {
        // No existing row -- check stock and insert.
        let stock: i32 =
            sqlx::query("SELECT stock_quantity FROM sku WHERE id = ?")
                .bind(sku_id)
                .fetch_one(&mut *tx)
                .await
                .map(|r| r.get("stock_quantity"))
                .map_err(|e| LockError::DatabaseError(e.to_string()))?;

        if stock < quantity {
            tx.rollback()
                .await
                .map_err(|e| LockError::DatabaseError(e.to_string()))?;
            return Err(LockError::StockExhausted);
        }

        sqlx::query("UPDATE sku SET stock_quantity = stock_quantity - ? WHERE id = ?")
            .bind(quantity)
            .bind(sku_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| LockError::DatabaseError(e.to_string()))?;

        sqlx::query(
            "INSERT INTO reservation_locks (lock_key, user_id, sku_id, quantity, acquired_at, expires_at, released)
             VALUES (?, ?, ?, ?, ?, ?, FALSE)",
        )
        .bind(key)
        .bind(user_id)
        .bind(sku_id)
        .bind(quantity)
        .bind(now)
        .bind(expires_at)
        .execute(&mut *tx)
        .await
        .map_err(|e| LockError::DatabaseError(e.to_string()))?;
    }

    tx.commit()
        .await
        .map_err(|e| LockError::DatabaseError(e.to_string()))?;

    // -- update in-memory map --------------------------------------------
    {
        let mut map = manager.inner.lock().await;
        map.insert(
            key.to_string(),
            LockEntry {
                lock_key: key.to_string(),
                user_id,
                expires_at,
                reservation_id: None,
            },
        );
    }

    Ok(LockHandle {
        lock_key: key.to_string(),
        acquired_at: now,
        expires_at,
    })
}

/// Release a previously acquired lock, restoring inventory.
pub async fn release(
    pool: &MySqlPool,
    manager: &ReservationLockManager,
    key: &str,
) -> Result<(), LockError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| LockError::DatabaseError(e.to_string()))?;

    let row = sqlx::query(
        "SELECT sku_id, quantity, released FROM reservation_locks WHERE lock_key = ? FOR UPDATE",
    )
    .bind(key)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| LockError::DatabaseError(e.to_string()))?;

    if let Some(row) = row {
        let released: bool = row.get("released");
        if !released {
            let sku_id: i64 = row.get("sku_id");
            let qty: i32 = row.get("quantity");

            sqlx::query("UPDATE sku SET stock_quantity = stock_quantity + ? WHERE id = ?")
                .bind(qty)
                .bind(sku_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| LockError::DatabaseError(e.to_string()))?;

            sqlx::query("UPDATE reservation_locks SET released = TRUE WHERE lock_key = ?")
                .bind(key)
                .execute(&mut *tx)
                .await
                .map_err(|e| LockError::DatabaseError(e.to_string()))?;
        }
    }

    tx.commit()
        .await
        .map_err(|e| LockError::DatabaseError(e.to_string()))?;

    // Remove from in-memory map
    {
        let mut map = manager.inner.lock().await;
        map.remove(key);
    }

    Ok(())
}

/// Associate a confirmed reservation with all locks the checkout acquired.
///
/// Called after the reservation row is created so that `consume_by_reservation_id`
/// can later mark those locks consumed without restoring inventory.
pub async fn associate_reservation(
    pool: &MySqlPool,
    key: &str,
    reservation_id: i64,
) -> Result<(), LockError> {
    sqlx::query(
        "UPDATE reservation_locks SET reservation_id = ? WHERE lock_key = ?",
    )
    .bind(reservation_id)
    .bind(key)
    .execute(pool)
    .await
    .map_err(|e| LockError::DatabaseError(e.to_string()))?;

    Ok(())
}

/// Mark all locks for `reservation_id` as consumed (released = TRUE) **without**
/// restoring inventory.
///
/// This is called when an order is confirmed so that the background expiry job
/// does not incorrectly restore stock for items that have been sold.
pub async fn consume_by_reservation_id(
    pool: &MySqlPool,
    manager: &ReservationLockManager,
    reservation_id: i64,
) -> Result<(), LockError> {
    // Fetch the lock_keys so we can remove them from the in-memory map.
    let keys: Vec<String> = sqlx::query_scalar(
        "SELECT lock_key FROM reservation_locks WHERE reservation_id = ? AND released = FALSE",
    )
    .bind(reservation_id)
    .fetch_all(pool)
    .await
    .map_err(|e| LockError::DatabaseError(e.to_string()))?;

    if keys.is_empty() {
        return Ok(());
    }

    // Mark released in DB — no stock restore.
    sqlx::query(
        "UPDATE reservation_locks SET released = TRUE WHERE reservation_id = ? AND released = FALSE",
    )
    .bind(reservation_id)
    .execute(pool)
    .await
    .map_err(|e| LockError::DatabaseError(e.to_string()))?;

    // Remove from in-memory map.
    {
        let mut map = manager.inner.lock().await;
        for key in &keys {
            map.remove(key);
        }
    }

    Ok(())
}

/// Scan all locks and release any that have expired. Returns the keys that
/// were released so the caller can log them.
///
/// Uses the database as the sole source of truth — the in-memory map is only
/// updated as a side-effect of calling `release`, which keeps it consistent.
pub async fn release_expired(
    pool: &MySqlPool,
    manager: &ReservationLockManager,
) -> Vec<String> {
    // Query DB for expired, unreleased locks
    let expired_keys: Vec<String> = sqlx::query_scalar(
        "SELECT lock_key FROM reservation_locks WHERE released = FALSE AND expires_at < NOW()"
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let mut released = Vec::new();
    for key in expired_keys {
        if let Ok(()) = release(pool, manager, &key).await {
            released.push(key);
        }
    }
    released
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── lock_key_for_sku_options ────────────────────────────────────────────

    #[test]
    fn lock_key_is_deterministic_regardless_of_order() {
        let a = lock_key_for_sku_options(42, &[3, 1, 2]);
        let b = lock_key_for_sku_options(42, &[1, 2, 3]);
        let c = lock_key_for_sku_options(42, &[2, 3, 1]);
        assert_eq!(a, b);
        assert_eq!(b, c);
        assert_eq!(a, "sku:42:opts:1,2,3");
    }

    #[test]
    fn lock_key_different_skus_differ() {
        let a = lock_key_for_sku_options(1, &[1, 2]);
        let b = lock_key_for_sku_options(2, &[1, 2]);
        assert_ne!(a, b);
    }

    #[test]
    fn lock_key_different_options_differ() {
        let a = lock_key_for_sku_options(1, &[1, 2]);
        let b = lock_key_for_sku_options(1, &[1, 3]);
        assert_ne!(a, b);
    }

    #[test]
    fn lock_key_empty_options() {
        assert_eq!(lock_key_for_sku_options(7, &[]), "sku:7:opts:");
    }

    #[test]
    fn lock_key_single_option() {
        assert_eq!(lock_key_for_sku_options(9, &[5]), "sku:9:opts:5");
    }

    // ── LockError Display ───────────────────────────────────────────────────

    #[test]
    fn lock_error_display_formats_holder_and_expires() {
        let e = LockError::AlreadyLocked {
            holder_id: 77,
            expires_at: "2026-01-01T00:00:00".into(),
        };
        let s = format!("{}", e);
        assert!(s.contains("77"));
        assert!(s.contains("2026-01-01"));
    }

    #[test]
    fn lock_error_display_stock_exhausted() {
        let s = format!("{}", LockError::StockExhausted);
        assert!(s.to_lowercase().contains("stock"));
    }

    #[test]
    fn lock_error_display_db_error_includes_message() {
        let s = format!("{}", LockError::DatabaseError("connection refused".into()));
        assert!(s.contains("connection refused"));
    }

    // ── ReservationLockManager ──────────────────────────────────────────────

    #[tokio::test]
    async fn manager_starts_empty() {
        let mgr = ReservationLockManager::new();
        let inner = mgr.inner.lock().await;
        assert!(inner.is_empty());
    }

    #[tokio::test]
    async fn manager_is_clonable_and_shares_state() {
        let mgr = ReservationLockManager::new();
        let clone = mgr.clone();
        // Inserting into the original map should be visible through the clone.
        {
            let mut m = mgr.inner.lock().await;
            m.insert(
                "k".into(),
                LockEntry {
                    lock_key: "k".into(),
                    user_id: 1,
                    expires_at: chrono::Utc::now().naive_utc(),
                    reservation_id: None,
                },
            );
        }
        let cloned = clone.inner.lock().await;
        assert!(cloned.contains_key("k"));
    }
}
