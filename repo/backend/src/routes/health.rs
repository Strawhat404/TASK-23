use rocket::{get, routes};
use rocket::serde::json::Json;
use rocket::http::Status;
use rocket::State;
use sqlx::MySqlPool;

use crate::middleware::auth_guard::AdminGuard;
use crate::services::resilience::{BackgroundJobManager, DegradationManager};
use shared::dto::ApiResponse;

/// App start time, stored as managed state for uptime calculation.
pub struct AppStartTime(pub chrono::NaiveDateTime);

// ---------------------------------------------------------------------------
// Routes
// ---------------------------------------------------------------------------

/// Basic health check – returns 200 if the DB is reachable, 503 otherwise.
#[get("/")]
pub async fn health(pool: &State<MySqlPool>) -> Result<Json<ApiResponse<String>>, (Status, Json<ApiResponse<()>>)> {
    let db = crate::services::health::check_database(pool.inner()).await;
    if db.status == "healthy" {
        Ok(Json(ApiResponse {
            success: true,
            data: Some("ok".into()),
            error: None,
        }))
    } else {
        Err((
            Status::ServiceUnavailable,
            Json(ApiResponse {
                success: false,
                data: None,
                error: Some(db.details.unwrap_or_else(|| "Database unreachable".into())),
            }),
        ))
    }
}

/// Detailed health report (admin only).
#[get("/detailed")]
pub async fn detailed(
    pool: &State<MySqlPool>,
    degradation: &State<std::sync::Arc<DegradationManager>>,
    job_mgr: &State<std::sync::Arc<BackgroundJobManager>>,
    start: &State<AppStartTime>,
    _admin: AdminGuard,
) -> Json<crate::services::health::HealthReport> {
    let jobs = job_mgr.get_job_statuses().await;
    let report = crate::services::health::full_health_check(
        pool.inner(),
        degradation.inner(),
        start.0,
        jobs,
    )
    .await;
    Json(report)
}

/// Readiness probe – 200 when all critical services are operational.
#[get("/ready")]
pub async fn ready(
    pool: &State<MySqlPool>,
    degradation: &State<std::sync::Arc<DegradationManager>>,
) -> Result<Json<ApiResponse<String>>, (Status, Json<ApiResponse<()>>)> {
    let db = crate::services::health::check_database(pool.inner()).await;
    if db.status != "healthy" {
        return Err((
            Status::ServiceUnavailable,
            Json(ApiResponse { success: false, data: None, error: Some("Database unavailable".into()) }),
        ));
    }

    let statuses = degradation.get_status().await;
    let critical_degraded: Vec<_> = statuses
        .iter()
        .filter(|(_, v)| v.is_critical && v.is_degraded)
        .map(|(k, _)| k.clone())
        .collect();

    if critical_degraded.is_empty() {
        Ok(Json(ApiResponse { success: true, data: Some("ready".into()), error: None }))
    } else {
        Err((
            Status::ServiceUnavailable,
            Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Critical services degraded: {}", critical_degraded.join(", "))),
            }),
        ))
    }
}

/// Liveness probe – always 200 if the process is alive.
#[get("/live")]
pub async fn live() -> Json<ApiResponse<String>> {
    Json(ApiResponse { success: true, data: Some("alive".into()), error: None })
}

pub fn routes() -> Vec<rocket::Route> {
    routes![health, detailed, ready, live]
}
