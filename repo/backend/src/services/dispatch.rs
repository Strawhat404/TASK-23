use sqlx::MySqlPool;
use std::fmt;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum DispatchError {
    TaskNotFound,
    AlreadyAssigned,
    MaxWorkloadReached,
    NoEligibleStaff,
    InvalidState(String),
    DatabaseError(String),
}

impl fmt::Display for DispatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TaskNotFound => write!(f, "Task not found"),
            Self::AlreadyAssigned => write!(f, "Task already assigned"),
            Self::MaxWorkloadReached => write!(f, "Staff member has reached max concurrent tasks"),
            Self::NoEligibleStaff => write!(f, "No eligible staff available"),
            Self::InvalidState(s) => write!(f, "Invalid task state: {}", s),
            Self::DatabaseError(s) => write!(f, "Database error: {}", s),
        }
    }
}

// ---------------------------------------------------------------------------
// Staff scoring
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct StaffScore {
    pub user_id: i64,
    pub zone_match: f64,
    pub shift_match: f64,
    pub workload_score: f64,
    pub reputation_score: f64,
    pub total_score: f64,
}

/// Recommend staff for a given order, scored by zone affinity, shift fit,
/// current workload, and reputation.
///
/// Only staff whose shift covers the current time are considered — shifts that
/// have already ended (or not yet started) are excluded.
pub async fn recommend_staff(
    pool: &MySqlPool,
    _order_id: i64,
    zone_id: Option<i64>,
) -> Vec<StaffScore> {
    let now = chrono::Utc::now();
    let today = now.date_naive();
    let current_time = now.time();

    // Gather shifts for all zones (or specific zone) today
    let all_shifts = if let Some(zid) = zone_id {
        crate::db::dispatch::get_zone_shifts(pool, zid, today).await
    } else {
        // Get shifts across all active zones
        let zones = crate::db::dispatch::list_zones(pool).await;
        let mut all = Vec::new();
        for z in &zones {
            all.extend(crate::db::dispatch::get_zone_shifts(pool, z.id, today).await);
        }
        all
    };

    // Filter to shifts whose time window covers the current time.
    let shifts: Vec<_> = all_shifts
        .into_iter()
        .filter(|s| {
            let start = chrono::NaiveTime::parse_from_str(&s.start_time, "%H:%M:%S")
                .or_else(|_| chrono::NaiveTime::parse_from_str(&s.start_time, "%H:%M"));
            let end = chrono::NaiveTime::parse_from_str(&s.end_time, "%H:%M:%S")
                .or_else(|_| chrono::NaiveTime::parse_from_str(&s.end_time, "%H:%M"));
            match (start, end) {
                (Ok(s), Ok(e)) => current_time >= s && current_time <= e,
                _ => false, // unparseable shift times are excluded
            }
        })
        .collect();

    let max_concurrent: i64 = crate::db::dispatch::get_dispatch_config(pool, "max_concurrent_per_staff")
        .await
        .and_then(|v| v.parse().ok())
        .unwrap_or(3);

    let mut scores = Vec::new();
    let mut seen_users = std::collections::HashSet::new();

    for shift in &shifts {
        if !seen_users.insert(shift.user_id) {
            continue; // already scored this user
        }

        let zone_match = if zone_id.map(|z| z == shift.zone_id).unwrap_or(false) {
            1.0
        } else {
            0.5
        };

        // Shift-match score: proportion of shift time remaining.
        // Staff near shift-end are penalised so tasks go to staff with more runway.
        let shift_match = {
            let start = chrono::NaiveTime::parse_from_str(&shift.start_time, "%H:%M:%S")
                .or_else(|_| chrono::NaiveTime::parse_from_str(&shift.start_time, "%H:%M"));
            let end = chrono::NaiveTime::parse_from_str(&shift.end_time, "%H:%M:%S")
                .or_else(|_| chrono::NaiveTime::parse_from_str(&shift.end_time, "%H:%M"));
            match (start, end) {
                (Ok(s), Ok(e)) => {
                    let total = (e - s).num_seconds().max(1) as f64;
                    let remaining = (e - current_time).num_seconds().max(0) as f64;
                    remaining / total // 1.0 at shift start → 0.0 at shift end
                }
                _ => 0.5, // fallback if unparseable (shouldn't happen after filter)
            }
        };

        let workload = crate::db::dispatch::get_staff_workload(pool, shift.user_id).await;
        let workload_score = if workload >= max_concurrent {
            0.0
        } else {
            (max_concurrent - workload) as f64 / max_concurrent as f64
        };

        let rep = crate::db::dispatch::get_reputation(pool, shift.user_id).await;
        let reputation_score = rep.map(|r| r.composite_score / 100.0).unwrap_or(0.5);

        let total = 0.25 * zone_match + 0.15 * shift_match + 0.35 * workload_score + 0.25 * reputation_score;

        scores.push(StaffScore {
            user_id: shift.user_id,
            zone_match,
            shift_match,
            workload_score,
            reputation_score,
            total_score: total,
        });
    }

    scores.sort_by(|a, b| b.total_score.partial_cmp(&a.total_score).unwrap_or(std::cmp::Ordering::Equal));
    scores
}

/// Auto-assign an order to the best-scoring staff member.
pub async fn auto_assign(
    pool: &MySqlPool,
    order_id: i64,
    zone_id: Option<i64>,
) -> Result<i64, DispatchError> {
    let recs = recommend_staff(pool, order_id, zone_id).await;
    let best = recs.first().ok_or(DispatchError::NoEligibleStaff)?;

    if best.workload_score <= 0.0 {
        return Err(DispatchError::MaxWorkloadReached);
    }

    let offer_timeout: i64 = crate::db::dispatch::get_dispatch_config(pool, "offer_timeout_secs")
        .await
        .and_then(|v| v.parse().ok())
        .unwrap_or(30);

    let task_id = crate::db::dispatch::create_task_assignment(pool, order_id, zone_id, "Assigned", 50)
        .await
        .map_err(|e| DispatchError::DatabaseError(e.to_string()))?;

    crate::db::dispatch::offer_task(pool, task_id, best.user_id, offer_timeout)
        .await
        .map_err(|e| DispatchError::DatabaseError(e.to_string()))?;

    Ok(task_id)
}

/// Enqueue an order for grab-mode.
pub async fn enqueue_for_grab(
    pool: &MySqlPool,
    order_id: i64,
    zone_id: Option<i64>,
    priority: i32,
) -> Result<i64, DispatchError> {
    crate::db::dispatch::create_task_assignment(pool, order_id, zone_id, "Grab", priority)
        .await
        .map_err(|e| DispatchError::DatabaseError(e.to_string()))
}

/// Staff member grabs a queued task.
///
/// Workload is checked before the attempt.  The atomic `grab_queued_task` DB
/// call provides the final concurrency safety net: if two grabs race, only
/// one will update a row and the other will receive `AlreadyAssigned`.
pub async fn grab_task(pool: &MySqlPool, task_id: i64, user_id: i64) -> Result<(), DispatchError> {
    let task = crate::db::dispatch::get_task(pool, task_id)
        .await
        .ok_or(DispatchError::TaskNotFound)?;

    if task.status != "Queued" || task.dispatch_mode != "Grab" {
        return Err(DispatchError::InvalidState(format!(
            "Task is {} in {} mode",
            task.status, task.dispatch_mode
        )));
    }

    let max: i64 = crate::db::dispatch::get_dispatch_config(pool, "max_concurrent_per_staff")
        .await
        .and_then(|v| v.parse().ok())
        .unwrap_or(3);

    let current = crate::db::dispatch::get_staff_workload(pool, user_id).await;
    if current >= max {
        return Err(DispatchError::MaxWorkloadReached);
    }

    // Atomic grab: only one concurrent caller wins; the rest get RowNotFound.
    crate::db::dispatch::grab_queued_task(pool, task_id, user_id)
        .await
        .map_err(|_| DispatchError::AlreadyAssigned)
}

/// Accept an offered task.
///
/// The DB call is atomic (`WHERE status = 'Offered' AND assigned_to = ?`) so
/// duplicate accept requests or a concurrent reassignment cannot both succeed.
pub async fn handle_accept(pool: &MySqlPool, task_id: i64, user_id: i64) -> Result<(), DispatchError> {
    // Pre-flight check so we can return a descriptive error rather than a
    // generic "0 rows updated" message.
    let task = crate::db::dispatch::get_task(pool, task_id)
        .await
        .ok_or(DispatchError::TaskNotFound)?;

    if task.status != "Offered" {
        return Err(DispatchError::InvalidState(format!("Task is {}", task.status)));
    }

    if task.assigned_to != Some(user_id) {
        return Err(DispatchError::InvalidState("Task not offered to you".into()));
    }

    // Atomic accept: verifies status = 'Offered' AND assigned_to = user_id at
    // the DB level, eliminating the TOCTOU gap between the pre-flight check
    // above and the actual state change.
    crate::db::dispatch::accept_offered_task(pool, task_id, user_id)
        .await
        .map_err(|_| DispatchError::AlreadyAssigned)
}

/// Reject an offered task and re-queue it.
pub async fn handle_reject(pool: &MySqlPool, task_id: i64, user_id: i64) -> Result<(), DispatchError> {
    let task = crate::db::dispatch::get_task(pool, task_id)
        .await
        .ok_or(DispatchError::TaskNotFound)?;

    if task.status != "Offered" || task.assigned_to != Some(user_id) {
        return Err(DispatchError::InvalidState("Task not offered to you".into()));
    }

    crate::db::dispatch::reject_task(pool, task_id)
        .await
        .map_err(|e| DispatchError::DatabaseError(e.to_string()))?;

    // Re-queue
    crate::db::dispatch::requeue_task(pool, task_id)
        .await
        .map_err(|e| DispatchError::DatabaseError(e.to_string()))
}

/// Complete a task and update reputation.
pub async fn complete_and_score(pool: &MySqlPool, task_id: i64) -> Result<(), DispatchError> {
    let task = crate::db::dispatch::get_task(pool, task_id)
        .await
        .ok_or(DispatchError::TaskNotFound)?;

    if task.status != "InProgress" {
        return Err(DispatchError::InvalidState(format!("Task is {}", task.status)));
    }

    crate::db::dispatch::complete_task(pool, task_id)
        .await
        .map_err(|e| DispatchError::DatabaseError(e.to_string()))?;

    // Calculate completion time and update reputation
    if let (Some(started), Some(user_id)) = (task.started_at, task.assigned_to) {
        let now = chrono::Utc::now().naive_utc();
        let duration_secs = now.signed_duration_since(started).num_seconds() as i32;
        // Default quality of 4.5/5.0 for completion
        let _ = crate::db::dispatch::update_reputation(pool, user_id, duration_secs, 4.5).await;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatch_error_display_task_not_found() {
        assert_eq!(format!("{}", DispatchError::TaskNotFound), "Task not found");
    }

    #[test]
    fn dispatch_error_display_already_assigned() {
        assert_eq!(
            format!("{}", DispatchError::AlreadyAssigned),
            "Task already assigned"
        );
    }

    #[test]
    fn dispatch_error_display_max_workload() {
        assert!(format!("{}", DispatchError::MaxWorkloadReached).contains("max concurrent"));
    }

    #[test]
    fn dispatch_error_display_no_eligible_staff() {
        assert!(format!("{}", DispatchError::NoEligibleStaff).contains("No eligible"));
    }

    #[test]
    fn dispatch_error_display_invalid_state_includes_reason() {
        let e = DispatchError::InvalidState("Queued".into());
        assert!(format!("{}", e).contains("Queued"));
    }

    #[test]
    fn dispatch_error_display_db_error_includes_reason() {
        let e = DispatchError::DatabaseError("timeout".into());
        assert!(format!("{}", e).contains("timeout"));
    }

    #[test]
    fn staff_score_serializes_all_fields() {
        let score = StaffScore {
            user_id: 7,
            zone_match: 1.0,
            shift_match: 0.75,
            workload_score: 0.5,
            reputation_score: 0.9,
            total_score: 0.82,
        };
        let json = serde_json::to_value(&score).unwrap();
        assert_eq!(json["user_id"], 7);
        assert!(json["zone_match"].is_number());
        assert!(json["total_score"].is_number());
    }
}
