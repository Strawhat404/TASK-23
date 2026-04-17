use chrono::NaiveDateTime;
use sqlx::{MySqlPool, Row};

/// A row from the `sessions` table.
#[derive(Debug, Clone)]
pub struct SessionRow {
    pub session_id: String,
    pub user_id: i64,
    pub last_activity: NaiveDateTime,
    pub created_at: NaiveDateTime,
    pub rotated_at: NaiveDateTime,
}

/// Insert a new session.
pub async fn create_session(
    pool: &MySqlPool,
    session_id: &str,
    user_id: i64,
    user_agent: Option<&str>,
    ip_addr: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO sessions (session_id, user_id, user_agent, ip_address, last_activity, created_at, rotated_at)
         VALUES (?, ?, ?, ?, NOW(), NOW(), NOW())",
    )
    .bind(session_id)
    .bind(user_id)
    .bind(user_agent)
    .bind(ip_addr)
    .execute(pool)
    .await?;

    Ok(())
}

/// Look up a session by its ID.
pub async fn get_session(pool: &MySqlPool, session_id: &str) -> Option<SessionRow> {
    let row = sqlx::query(
        "SELECT session_id, user_id, last_activity, created_at, rotated_at
         FROM sessions WHERE session_id = ?",
    )
    .bind(session_id)
    .fetch_optional(pool)
    .await
    .ok()?;

    row.map(|r| SessionRow {
        session_id: r.get("session_id"),
        user_id: r.get("user_id"),
        last_activity: r.get("last_activity"),
        created_at: r.get("created_at"),
        rotated_at: r.get("rotated_at"),
    })
}

/// Update `last_activity` to NOW() for the given session.
pub async fn touch_session(pool: &MySqlPool, session_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE sessions SET last_activity = NOW() WHERE session_id = ?")
        .bind(session_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Rotate a session: change its ID and update `rotated_at`.
pub async fn rotate_session(
    pool: &MySqlPool,
    old_session_id: &str,
    new_session_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE sessions SET session_id = ?, rotated_at = NOW() WHERE session_id = ?",
    )
    .bind(new_session_id)
    .bind(old_session_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Delete a single session.
pub async fn delete_session(pool: &MySqlPool, session_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM sessions WHERE session_id = ?")
        .bind(session_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Remove all sessions whose `last_activity` is older than `idle_timeout_secs`
/// seconds ago.  Returns the number of deleted rows.
pub async fn cleanup_expired_sessions(
    pool: &MySqlPool,
    idle_timeout_secs: u64,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "DELETE FROM sessions WHERE last_activity < DATE_SUB(NOW(), INTERVAL ? SECOND)",
    )
    .bind(idle_timeout_secs as i64)
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn sample_dt() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 4, 15)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap()
    }

    #[test]
    fn session_row_construction() {
        let row = SessionRow {
            session_id: "sess-abc-123".to_string(),
            user_id: 42,
            last_activity: sample_dt(),
            created_at: sample_dt(),
            rotated_at: sample_dt(),
        };
        assert_eq!(row.session_id, "sess-abc-123");
        assert_eq!(row.user_id, 42);
    }

    #[test]
    fn session_row_field_types_are_correct() {
        let row = SessionRow {
            session_id: String::new(),
            user_id: 0,
            last_activity: sample_dt(),
            created_at: sample_dt(),
            rotated_at: sample_dt(),
        };
        // session_id is a String, user_id is i64, timestamps are NaiveDateTime
        let _s: &str = &row.session_id;
        let _id: i64 = row.user_id;
        let _ts: NaiveDateTime = row.last_activity;
        let _ts2: NaiveDateTime = row.created_at;
        let _ts3: NaiveDateTime = row.rotated_at;
    }

    #[test]
    fn session_row_clone() {
        let row = SessionRow {
            session_id: "sess-1".to_string(),
            user_id: 7,
            last_activity: sample_dt(),
            created_at: sample_dt(),
            rotated_at: sample_dt(),
        };
        let cloned = row.clone();
        assert_eq!(cloned.session_id, row.session_id);
        assert_eq!(cloned.user_id, row.user_id);
    }

    #[test]
    fn session_row_debug_format() {
        let row = SessionRow {
            session_id: "s1".to_string(),
            user_id: 1,
            last_activity: sample_dt(),
            created_at: sample_dt(),
            rotated_at: sample_dt(),
        };
        let debug = format!("{:?}", row);
        assert!(debug.contains("SessionRow"));
        assert!(debug.contains("s1"));
    }

    #[test]
    fn session_row_empty_session_id() {
        let row = SessionRow {
            session_id: String::new(),
            user_id: 0,
            last_activity: sample_dt(),
            created_at: sample_dt(),
            rotated_at: sample_dt(),
        };
        assert!(row.session_id.is_empty());
    }

    #[test]
    fn session_row_large_user_id() {
        let row = SessionRow {
            session_id: "s".to_string(),
            user_id: i64::MAX,
            last_activity: sample_dt(),
            created_at: sample_dt(),
            rotated_at: sample_dt(),
        };
        assert_eq!(row.user_id, i64::MAX);
    }
}
