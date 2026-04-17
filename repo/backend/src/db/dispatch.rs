use sqlx::{MySqlPool, Row};
use chrono::{NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StationZone {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub zone_type: String,
    pub max_concurrent_tasks: i32,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShiftWindow {
    pub id: i64,
    pub user_id: i64,
    pub zone_id: i64,
    pub shift_date: NaiveDate,
    pub start_time: String,
    pub end_time: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaffReputation {
    pub user_id: i64,
    pub total_tasks_completed: i32,
    pub avg_completion_time_secs: i32,
    pub quality_score: f64,
    pub reliability_score: f64,
    pub composite_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskAssignment {
    pub id: i64,
    pub order_id: i64,
    pub assigned_to: Option<i64>,
    pub zone_id: Option<i64>,
    pub dispatch_mode: String,
    pub status: String,
    pub priority: i32,
    pub offered_at: Option<NaiveDateTime>,
    pub accepted_at: Option<NaiveDateTime>,
    pub started_at: Option<NaiveDateTime>,
    pub completed_at: Option<NaiveDateTime>,
    pub offer_expires_at: Option<NaiveDateTime>,
    pub notes: Option<String>,
    pub created_at: NaiveDateTime,
}

// ---------------------------------------------------------------------------
// Zone operations
// ---------------------------------------------------------------------------

pub async fn list_zones(pool: &MySqlPool) -> Vec<StationZone> {
    sqlx::query("SELECT id, name, description, zone_type, max_concurrent_tasks, is_active FROM station_zones ORDER BY name")
        .fetch_all(pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|r| StationZone {
            id: r.get("id"),
            name: r.get("name"),
            description: r.get("description"),
            zone_type: r.get("zone_type"),
            max_concurrent_tasks: r.get("max_concurrent_tasks"),
            is_active: r.get("is_active"),
        })
        .collect()
}

pub async fn get_zone(pool: &MySqlPool, id: i64) -> Option<StationZone> {
    sqlx::query("SELECT id, name, description, zone_type, max_concurrent_tasks, is_active FROM station_zones WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .ok()?
        .map(|r| StationZone {
            id: r.get("id"),
            name: r.get("name"),
            description: r.get("description"),
            zone_type: r.get("zone_type"),
            max_concurrent_tasks: r.get("max_concurrent_tasks"),
            is_active: r.get("is_active"),
        })
}

// ---------------------------------------------------------------------------
// Shift operations
// ---------------------------------------------------------------------------

pub async fn create_shift(
    pool: &MySqlPool,
    user_id: i64,
    zone_id: i64,
    shift_date: NaiveDate,
    start_time: &str,
    end_time: &str,
) -> Result<i64, sqlx::Error> {
    let r = sqlx::query(
        "INSERT INTO shift_windows (user_id, zone_id, shift_date, start_time, end_time) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(user_id)
    .bind(zone_id)
    .bind(shift_date)
    .bind(start_time)
    .bind(end_time)
    .execute(pool)
    .await?;
    Ok(r.last_insert_id() as i64)
}

pub async fn get_staff_shifts(pool: &MySqlPool, user_id: i64, date: NaiveDate) -> Vec<ShiftWindow> {
    sqlx::query("SELECT id, user_id, zone_id, shift_date, start_time, end_time FROM shift_windows WHERE user_id = ? AND shift_date = ? AND is_active = 1")
        .bind(user_id)
        .bind(date)
        .fetch_all(pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|r| ShiftWindow {
            id: r.get("id"),
            user_id: r.get("user_id"),
            zone_id: r.get("zone_id"),
            shift_date: r.get("shift_date"),
            start_time: r.get("start_time"),
            end_time: r.get("end_time"),
        })
        .collect()
}

pub async fn get_zone_shifts(pool: &MySqlPool, zone_id: i64, date: NaiveDate) -> Vec<ShiftWindow> {
    sqlx::query("SELECT id, user_id, zone_id, shift_date, start_time, end_time FROM shift_windows WHERE zone_id = ? AND shift_date = ? AND is_active = 1")
        .bind(zone_id)
        .bind(date)
        .fetch_all(pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|r| ShiftWindow {
            id: r.get("id"),
            user_id: r.get("user_id"),
            zone_id: r.get("zone_id"),
            shift_date: r.get("shift_date"),
            start_time: r.get("start_time"),
            end_time: r.get("end_time"),
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Reputation
// ---------------------------------------------------------------------------

pub async fn get_reputation(pool: &MySqlPool, user_id: i64) -> Option<StaffReputation> {
    sqlx::query("SELECT user_id, total_tasks_completed, avg_completion_time_secs, quality_score, reliability_score, composite_score FROM staff_reputation WHERE user_id = ?")
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .ok()?
        .map(|r| StaffReputation {
            user_id: r.get("user_id"),
            total_tasks_completed: r.get("total_tasks_completed"),
            avg_completion_time_secs: r.get("avg_completion_time_secs"),
            quality_score: r.get("quality_score"),
            reliability_score: r.get("reliability_score"),
            composite_score: r.get("composite_score"),
        })
}

pub async fn update_reputation(
    pool: &MySqlPool,
    user_id: i64,
    completed_time_secs: i32,
    quality: f64,
) -> Result<(), sqlx::Error> {
    // Upsert reputation with running average
    sqlx::query(
        "INSERT INTO staff_reputation (user_id, total_tasks_completed, avg_completion_time_secs, quality_score, composite_score, last_updated)
         VALUES (?, 1, ?, ?, ?, NOW())
         ON DUPLICATE KEY UPDATE
            total_tasks_completed = total_tasks_completed + 1,
            avg_completion_time_secs = (avg_completion_time_secs * (total_tasks_completed - 1) + ?) / total_tasks_completed,
            quality_score = (quality_score * 0.8 + ? * 0.2),
            composite_score = (quality_score * 40 + reliability_score * 30 + (300.0 / GREATEST(avg_completion_time_secs, 1)) * 30),
            last_updated = NOW()"
    )
    .bind(user_id)
    .bind(completed_time_secs)
    .bind(quality)
    .bind(quality * 40.0 + 5.0 * 30.0 + 30.0) // initial composite
    .bind(completed_time_secs)
    .bind(quality)
    .execute(pool)
    .await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Task assignment operations
// ---------------------------------------------------------------------------

fn row_to_task(r: sqlx::mysql::MySqlRow) -> TaskAssignment {
    TaskAssignment {
        id: r.get("id"),
        order_id: r.get("order_id"),
        assigned_to: r.get("assigned_to"),
        zone_id: r.get("zone_id"),
        dispatch_mode: r.get("dispatch_mode"),
        status: r.get("status"),
        priority: r.get("priority"),
        offered_at: r.get("offered_at"),
        accepted_at: r.get("accepted_at"),
        started_at: r.get("started_at"),
        completed_at: r.get("completed_at"),
        offer_expires_at: r.get("offer_expires_at"),
        notes: r.get("notes"),
        created_at: r.get("created_at"),
    }
}

const TASK_COLS: &str = "id, order_id, assigned_to, zone_id, dispatch_mode, status, priority, offered_at, accepted_at, started_at, completed_at, offer_expires_at, notes, created_at";

pub async fn create_task_assignment(
    pool: &MySqlPool,
    order_id: i64,
    zone_id: Option<i64>,
    mode: &str,
    priority: i32,
) -> Result<i64, sqlx::Error> {
    let r = sqlx::query(
        "INSERT INTO task_assignments (order_id, zone_id, dispatch_mode, priority) VALUES (?, ?, ?, ?)",
    )
    .bind(order_id)
    .bind(zone_id)
    .bind(mode)
    .bind(priority)
    .execute(pool)
    .await?;
    Ok(r.last_insert_id() as i64)
}

pub async fn get_task(pool: &MySqlPool, task_id: i64) -> Option<TaskAssignment> {
    let q = format!("SELECT {} FROM task_assignments WHERE id = ?", TASK_COLS);
    sqlx::query(&q)
        .bind(task_id)
        .fetch_optional(pool)
        .await
        .ok()?
        .map(row_to_task)
}

pub async fn get_queued_tasks(pool: &MySqlPool, zone_id: Option<i64>, limit: i32) -> Vec<TaskAssignment> {
    let (q, needs_zone) = if zone_id.is_some() {
        (format!("SELECT {} FROM task_assignments WHERE status = 'Queued' AND (zone_id = ? OR zone_id IS NULL) ORDER BY priority DESC, created_at ASC LIMIT ?", TASK_COLS), true)
    } else {
        (format!("SELECT {} FROM task_assignments WHERE status = 'Queued' ORDER BY priority DESC, created_at ASC LIMIT ?", TASK_COLS), false)
    };

    let mut query = sqlx::query(&q);
    if needs_zone {
        query = query.bind(zone_id.unwrap());
    }
    query
        .bind(limit)
        .fetch_all(pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(row_to_task)
        .collect()
}

pub async fn get_staff_active_tasks(pool: &MySqlPool, user_id: i64) -> Vec<TaskAssignment> {
    let q = format!("SELECT {} FROM task_assignments WHERE assigned_to = ? AND status IN ('Accepted', 'InProgress') ORDER BY priority DESC", TASK_COLS);
    sqlx::query(&q)
        .bind(user_id)
        .fetch_all(pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(row_to_task)
        .collect()
}

pub async fn offer_task(pool: &MySqlPool, task_id: i64, user_id: i64, expires_secs: i64) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE task_assignments SET status = 'Offered', assigned_to = ?, offered_at = NOW(), offer_expires_at = DATE_ADD(NOW(), INTERVAL ? SECOND) WHERE id = ? AND status = 'Queued'",
    )
    .bind(user_id)
    .bind(expires_secs)
    .bind(task_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Accept a task that was explicitly offered to this user.
///
/// The WHERE clause includes `assigned_to = ?` so that the update is atomic:
/// even if two concurrent requests pass the pre-check in the service layer,
/// only the correct recipient can flip the row.  Returns `Err` if 0 rows were
/// updated (lost race or wrong user).
pub async fn accept_offered_task(pool: &MySqlPool, task_id: i64, user_id: i64) -> Result<(), sqlx::Error> {
    let res = sqlx::query(
        "UPDATE task_assignments
         SET status = 'Accepted', assigned_to = ?, accepted_at = NOW()
         WHERE id = ? AND status = 'Offered' AND assigned_to = ?",
    )
    .bind(user_id)
    .bind(task_id)
    .bind(user_id)
    .execute(pool)
    .await?;

    if res.rows_affected() == 0 {
        return Err(sqlx::Error::RowNotFound);
    }
    Ok(())
}

/// Grab a queued (grab-mode) task.
///
/// The WHERE clause restricts to `status = 'Queued'` so that exactly one
/// concurrent grab wins; the loser gets 0 rows_affected and an error.
pub async fn grab_queued_task(pool: &MySqlPool, task_id: i64, user_id: i64) -> Result<(), sqlx::Error> {
    let res = sqlx::query(
        "UPDATE task_assignments
         SET status = 'Accepted', assigned_to = ?, accepted_at = NOW()
         WHERE id = ? AND status = 'Queued' AND dispatch_mode = 'Grab'",
    )
    .bind(user_id)
    .bind(task_id)
    .execute(pool)
    .await?;

    if res.rows_affected() == 0 {
        return Err(sqlx::Error::RowNotFound);
    }
    Ok(())
}

pub async fn start_task(pool: &MySqlPool, task_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE task_assignments SET status = 'InProgress', started_at = NOW() WHERE id = ? AND status = 'Accepted'")
        .bind(task_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn complete_task(pool: &MySqlPool, task_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE task_assignments SET status = 'Completed', completed_at = NOW() WHERE id = ? AND status = 'InProgress'")
        .bind(task_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn reject_task(pool: &MySqlPool, task_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE task_assignments SET status = 'Rejected', assigned_to = NULL WHERE id = ? AND status = 'Offered'")
        .bind(task_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn requeue_task(pool: &MySqlPool, task_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE task_assignments SET status = 'Queued', assigned_to = NULL, offered_at = NULL, offer_expires_at = NULL WHERE id = ?")
        .bind(task_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn expire_stale_offers(pool: &MySqlPool) -> Result<u64, sqlx::Error> {
    let r = sqlx::query(
        "UPDATE task_assignments SET status = 'Queued', assigned_to = NULL WHERE status = 'Offered' AND offer_expires_at < NOW()",
    )
    .execute(pool)
    .await?;
    Ok(r.rows_affected())
}

pub async fn get_zone_workload(pool: &MySqlPool, zone_id: i64) -> i64 {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM task_assignments WHERE zone_id = ? AND status IN ('Accepted', 'InProgress')")
        .bind(zone_id)
        .fetch_one(pool)
        .await
        .unwrap_or(0)
}

pub async fn get_staff_workload(pool: &MySqlPool, user_id: i64) -> i64 {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM task_assignments WHERE assigned_to = ? AND status IN ('Accepted', 'InProgress')")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .unwrap_or(0)
}

pub async fn get_dispatch_config(pool: &MySqlPool, key: &str) -> Option<String> {
    sqlx::query_scalar::<_, String>("SELECT config_value FROM dispatch_config WHERE config_key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await
        .ok()?
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

    // ── StationZone ──────────────────────────────────────────────────────

    #[test]
    fn station_zone_construction_and_fields() {
        let zone = StationZone {
            id: 1,
            name: "Barista Station".to_string(),
            description: Some("Main coffee station".to_string()),
            zone_type: "preparation".to_string(),
            max_concurrent_tasks: 5,
            is_active: true,
        };
        assert_eq!(zone.id, 1);
        assert_eq!(zone.name, "Barista Station");
        assert!(zone.is_active);
        assert_eq!(zone.max_concurrent_tasks, 5);
    }

    #[test]
    fn station_zone_serde_round_trip() {
        let zone = StationZone {
            id: 2,
            name: "Pickup Counter".to_string(),
            description: None,
            zone_type: "pickup".to_string(),
            max_concurrent_tasks: 10,
            is_active: false,
        };
        let json = serde_json::to_string(&zone).unwrap();
        let back: StationZone = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, 2);
        assert_eq!(back.zone_type, "pickup");
        assert!(!back.is_active);
        assert!(back.description.is_none());
    }

    // ── ShiftWindow ──────────────────────────────────────────────────────

    #[test]
    fn shift_window_construction() {
        let shift = ShiftWindow {
            id: 1,
            user_id: 42,
            zone_id: 3,
            shift_date: NaiveDate::from_ymd_opt(2026, 4, 15).unwrap(),
            start_time: "08:00".to_string(),
            end_time: "16:00".to_string(),
        };
        assert_eq!(shift.user_id, 42);
        assert_eq!(shift.start_time, "08:00");
        assert_eq!(shift.end_time, "16:00");
    }

    #[test]
    fn shift_window_serde_round_trip() {
        let shift = ShiftWindow {
            id: 5,
            user_id: 10,
            zone_id: 2,
            shift_date: NaiveDate::from_ymd_opt(2026, 5, 1).unwrap(),
            start_time: "14:00".to_string(),
            end_time: "22:00".to_string(),
        };
        let json = serde_json::to_string(&shift).unwrap();
        let back: ShiftWindow = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, 5);
        assert_eq!(back.shift_date, NaiveDate::from_ymd_opt(2026, 5, 1).unwrap());
    }

    // ── StaffReputation ──────────────────────────────────────────────────

    #[test]
    fn staff_reputation_construction() {
        let rep = StaffReputation {
            user_id: 7,
            total_tasks_completed: 100,
            avg_completion_time_secs: 300,
            quality_score: 88.5,
            reliability_score: 92.0,
            composite_score: 90.0,
        };
        assert_eq!(rep.user_id, 7);
        assert_eq!(rep.total_tasks_completed, 100);
    }

    #[test]
    fn staff_reputation_composite_score_in_range() {
        let rep = StaffReputation {
            user_id: 1,
            total_tasks_completed: 50,
            avg_completion_time_secs: 200,
            quality_score: 85.0,
            reliability_score: 90.0,
            composite_score: 87.0,
        };
        assert!(rep.composite_score >= 0.0 && rep.composite_score <= 100.0);
    }

    #[test]
    fn staff_reputation_serde_round_trip() {
        let rep = StaffReputation {
            user_id: 3,
            total_tasks_completed: 25,
            avg_completion_time_secs: 180,
            quality_score: 75.0,
            reliability_score: 80.0,
            composite_score: 77.5,
        };
        let json = serde_json::to_string(&rep).unwrap();
        let back: StaffReputation = serde_json::from_str(&json).unwrap();
        assert_eq!(back.user_id, 3);
        assert!((back.composite_score - 77.5).abs() < 1e-9);
    }

    // ── TaskAssignment ───────────────────────────────────────────────────

    #[test]
    fn task_assignment_construction() {
        let task = TaskAssignment {
            id: 1,
            order_id: 100,
            assigned_to: Some(42),
            zone_id: Some(2),
            dispatch_mode: "Offer".to_string(),
            status: "Queued".to_string(),
            priority: 5,
            offered_at: None,
            accepted_at: None,
            started_at: None,
            completed_at: None,
            offer_expires_at: None,
            notes: Some("rush order".to_string()),
            created_at: sample_dt(),
        };
        assert_eq!(task.id, 1);
        assert_eq!(task.order_id, 100);
        assert_eq!(task.assigned_to, Some(42));
        assert_eq!(task.dispatch_mode, "Offer");
        assert_eq!(task.status, "Queued");
    }

    #[test]
    fn task_assignment_serde_round_trip() {
        let task = TaskAssignment {
            id: 10,
            order_id: 50,
            assigned_to: None,
            zone_id: None,
            dispatch_mode: "Grab".to_string(),
            status: "Completed".to_string(),
            priority: 1,
            offered_at: Some(sample_dt()),
            accepted_at: Some(sample_dt()),
            started_at: Some(sample_dt()),
            completed_at: Some(sample_dt()),
            offer_expires_at: None,
            notes: None,
            created_at: sample_dt(),
        };
        let json = serde_json::to_string(&task).unwrap();
        let back: TaskAssignment = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, 10);
        assert_eq!(back.dispatch_mode, "Grab");
        assert_eq!(back.status, "Completed");
        assert!(back.assigned_to.is_none());
    }

    #[test]
    fn task_assignment_status_values() {
        for status in &["Queued", "Offered", "Accepted", "InProgress", "Completed", "Rejected"] {
            let task = TaskAssignment {
                id: 1,
                order_id: 1,
                assigned_to: None,
                zone_id: None,
                dispatch_mode: "Offer".to_string(),
                status: status.to_string(),
                priority: 0,
                offered_at: None,
                accepted_at: None,
                started_at: None,
                completed_at: None,
                offer_expires_at: None,
                notes: None,
                created_at: sample_dt(),
            };
            assert_eq!(task.status, *status);
        }
    }

    #[test]
    fn task_assignment_dispatch_modes() {
        for mode in &["Offer", "Grab"] {
            let task = TaskAssignment {
                id: 1,
                order_id: 1,
                assigned_to: None,
                zone_id: None,
                dispatch_mode: mode.to_string(),
                status: "Queued".to_string(),
                priority: 0,
                offered_at: None,
                accepted_at: None,
                started_at: None,
                completed_at: None,
                offer_expires_at: None,
                notes: None,
                created_at: sample_dt(),
            };
            assert_eq!(task.dispatch_mode, *mode);
        }
    }
}
