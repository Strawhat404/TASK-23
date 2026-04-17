use sqlx::{MySqlPool, Row};
use chrono::NaiveDateTime;
use shared::models::{StoreHours, Reservation, SalesTaxConfig, Voucher};

pub async fn get_store_hours(pool: &MySqlPool) -> Vec<StoreHours> {
    let rows = sqlx::query(
        "SELECT id, day_of_week, open_time, close_time, is_closed
         FROM store_hours ORDER BY day_of_week"
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    rows.into_iter()
        .map(|r| StoreHours {
            id: r.get("id"),
            day_of_week: r.get::<u8, _>("day_of_week"),
            open_time: r.get("open_time"),
            close_time: r.get("close_time"),
            is_closed: r.get("is_closed"),
        })
        .collect()
}

pub async fn get_tax_config(pool: &MySqlPool) -> Option<SalesTaxConfig> {
    let row = sqlx::query(
        "SELECT id, tax_name, CAST(rate AS DOUBLE) AS rate, is_active FROM sales_tax_config WHERE is_active = 1 LIMIT 1"
    )
    .fetch_optional(pool)
    .await
    .ok()?;

    row.map(|r| SalesTaxConfig {
        id: r.get("id"),
        tax_name: r.get("tax_name"),
        rate: r.get("rate"),
        is_active: r.get("is_active"),
    })
}

pub async fn create_reservation(
    pool: &MySqlPool,
    user_id: i64,
    slot_start: NaiveDateTime,
    slot_end: NaiveDateTime,
    voucher_code: &str,
    hold_expires_at: NaiveDateTime,
) -> Result<i64, sqlx::Error> {
    // Store the SHA-256 hash, not the plaintext, to protect the code at rest.
    let hashed = sha256_hex(voucher_code);
    let result = sqlx::query(
        "INSERT INTO reservations (user_id, pickup_slot_start, pickup_slot_end, voucher_code, hold_expires_at, status)
         VALUES (?, ?, ?, ?, ?, 'Held')"
    )
    .bind(user_id)
    .bind(slot_start)
    .bind(slot_end)
    .bind(&hashed)
    .bind(hold_expires_at)
    .execute(pool)
    .await?;

    Ok(result.last_insert_id() as i64)
}

fn row_to_reservation(r: sqlx::mysql::MySqlRow) -> Reservation {
    Reservation {
        id: r.get("id"),
        user_id: r.get("user_id"),
        pickup_slot_start: r.get("pickup_slot_start"),
        pickup_slot_end: r.get("pickup_slot_end"),
        voucher_code: r.get("voucher_code"),
        hold_expires_at: r.get("hold_expires_at"),
        status: r.get("status"),
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
    }
}

pub async fn get_reservation(pool: &MySqlPool, id: i64) -> Option<Reservation> {
    let row = sqlx::query(
        "SELECT id, user_id, pickup_slot_start, pickup_slot_end, voucher_code,
                hold_expires_at, status, created_at, updated_at
         FROM reservations WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .ok()?;

    row.map(row_to_reservation)
}

pub async fn get_reservation_by_voucher(pool: &MySqlPool, code: &str) -> Option<Reservation> {
    let hashed = sha256_hex(code);
    let row = sqlx::query(
        "SELECT id, user_id, pickup_slot_start, pickup_slot_end, voucher_code,
                hold_expires_at, status, created_at, updated_at
         FROM reservations WHERE voucher_code = ?"
    )
    .bind(&hashed)
    .fetch_optional(pool)
    .await
    .ok()?;

    row.map(row_to_reservation)
}

pub async fn update_reservation_status(
    pool: &MySqlPool,
    id: i64,
    status: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE reservations SET status = ?, updated_at = NOW() WHERE id = ?")
        .bind(status)
        .bind(id)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn expire_stale_reservations(pool: &MySqlPool) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE reservations SET status = 'Expired', updated_at = NOW()
         WHERE status = 'Held' AND hold_expires_at < NOW()"
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}

pub fn sha256_hex(input: &str) -> String {
    use sha2::{Sha256, Digest};
    let hash = Sha256::digest(input.as_bytes());
    hex::encode(hash)
}

pub async fn create_voucher(
    pool: &MySqlPool,
    reservation_id: i64,
    order_id: i64,
    code: &str,
    encrypted_code: &str,
) -> Result<i64, sqlx::Error> {
    let hashed = sha256_hex(code);
    let result = sqlx::query(
        "INSERT INTO vouchers (reservation_id, order_id, code, encrypted_code) VALUES (?, ?, ?, ?)"
    )
    .bind(reservation_id)
    .bind(order_id)
    .bind(&hashed)
    .bind(encrypted_code)
    .execute(pool)
    .await?;

    Ok(result.last_insert_id() as i64)
}

pub async fn get_voucher_by_code(pool: &MySqlPool, code: &str) -> Option<Voucher> {
    let hashed = sha256_hex(code);
    let row = sqlx::query(
        "SELECT id, reservation_id, order_id, code, scanned_at, scanned_by_user_id,
                mismatch_flag, mismatch_reason
         FROM vouchers WHERE code = ?"
    )
    .bind(&hashed)
    .fetch_optional(pool)
    .await
    .ok()?;

    row.map(|r| Voucher {
        id: r.get("id"),
        reservation_id: r.get("reservation_id"),
        order_id: r.get("order_id"),
        code: r.get("code"),
        scanned_at: r.get("scanned_at"),
        scanned_by_user_id: r.get("scanned_by_user_id"),
        mismatch_flag: r.get("mismatch_flag"),
        mismatch_reason: r.get("mismatch_reason"),
    })
}

pub async fn mark_voucher_scanned(
    pool: &MySqlPool,
    voucher_id: i64,
    scanned_by: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE vouchers SET scanned_at = NOW(), scanned_by_user_id = ? WHERE id = ?"
    )
    .bind(scanned_by)
    .bind(voucher_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn set_voucher_mismatch(
    pool: &MySqlPool,
    voucher_id: i64,
    reason: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE vouchers SET mismatch_flag = 1, mismatch_reason = ? WHERE id = ?"
    )
    .bind(reason)
    .bind(voucher_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn update_store_hours(pool: &MySqlPool, day: u8, open_time: &str, close_time: &str, is_closed: bool) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE store_hours SET open_time = ?, close_time = ?, is_closed = ? WHERE day_of_week = ?")
        .bind(open_time)
        .bind(close_time)
        .bind(is_closed)
        .bind(day)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_tax_config(pool: &MySqlPool, tax_name: &str, rate: f64) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE sales_tax_config SET tax_name = ?, rate = ? WHERE is_active = 1")
        .bind(tax_name)
        .bind(rate)
        .execute(pool)
        .await?;
    Ok(())
}

/// Return the AES-GCM `encrypted_code` stored in `vouchers` for a given reservation.
/// Used to recover the display voucher code without storing plaintext in `reservations`.
pub async fn get_encrypted_code_by_reservation_id(
    pool: &MySqlPool,
    reservation_id: i64,
) -> Option<String> {
    let row = sqlx::query("SELECT encrypted_code FROM vouchers WHERE reservation_id = ? LIMIT 1")
        .bind(reservation_id)
        .fetch_optional(pool)
        .await
        .ok()??;
    let enc: Option<String> = row.get("encrypted_code");
    enc
}

pub async fn get_reservations_for_date(pool: &MySqlPool, date: chrono::NaiveDate) -> Vec<Reservation> {
    let start = date.and_hms_opt(0, 0, 0).unwrap();
    let end = date.and_hms_opt(23, 59, 59).unwrap();
    sqlx::query("SELECT id, user_id, pickup_slot_start, pickup_slot_end, voucher_code, hold_expires_at, status, created_at, updated_at FROM reservations WHERE pickup_slot_start BETWEEN ? AND ? AND status IN ('Held', 'Confirmed')")
        .bind(start)
        .bind(end)
        .fetch_all(pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(row_to_reservation)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_hex_known_input() {
        // SHA-256 of "hello" is well-known
        let result = sha256_hex("hello");
        assert_eq!(
            result,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[test]
    fn sha256_hex_empty_string() {
        let result = sha256_hex("");
        assert_eq!(
            result,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn sha256_hex_unicode() {
        let hash1 = sha256_hex("\u{4f60}\u{597d}");
        // Must be a valid 64-char lowercase hex string
        assert_eq!(hash1.len(), 64);
        assert!(hash1.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn sha256_hex_deterministic() {
        let a = sha256_hex("BF-ABC123");
        let b = sha256_hex("BF-ABC123");
        assert_eq!(a, b);
    }

    #[test]
    fn sha256_hex_different_inputs_differ() {
        let a = sha256_hex("abc");
        let b = sha256_hex("def");
        assert_ne!(a, b);
    }

    #[test]
    fn sha256_hex_output_length_is_64() {
        let result = sha256_hex("anything");
        assert_eq!(result.len(), 64);
    }
}
