mod db;
mod middleware;
mod routes;
mod services;
#[cfg(test)]
mod api_tests;

use std::sync::Arc;
use rocket_cors::{AllowedOrigins, CorsOptions};

#[rocket::launch]
async fn rocket() -> _ {
    // Initialise tracing subscriber for structured logging.
    tracing_subscriber::fmt::init();

    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "mysql://root:root@localhost/brewflow".into());

    let pool = sqlx::MySqlPool::connect(&database_url)
        .await
        .expect("Failed to connect to MySQL");

    // CORS: restrict to origins configured via ALLOWED_ORIGINS (comma-separated).
    // Falls back to localhost:8080 in development.  Set ALLOWED_ORIGINS= (empty)
    // to disallow all cross-origin requests in strict intranet deployments.
    let allowed_origins = {
        let raw = std::env::var("ALLOWED_ORIGINS")
            .unwrap_or_else(|_| "http://localhost:8080".into());
        if raw.is_empty() {
            AllowedOrigins::some_exact(&[] as &[&str])
        } else {
            let origins: Vec<String> = raw.split(',').map(|s| s.trim().to_owned()).collect();
            AllowedOrigins::some_exact(&origins)
        }
    };
    let cors = CorsOptions {
        allowed_origins,
        allow_credentials: true,
        ..Default::default()
    }
    .to_cors()
    .expect("CORS config error");

    // Session config
    let session_config = services::session::SessionConfig::from_env();
    let idle_timeout = session_config.idle_timeout_secs;

    // Crypto config
    let crypto_config = services::crypto::CryptoConfig::from_env();

    // Reservation lock manager
    let lock_manager = services::reservation_lock::ReservationLockManager::new();

    // Degradation manager
    let degradation = Arc::new(services::resilience::DegradationManager::new());

    // Background job manager
    let job_mgr = Arc::new(services::resilience::BackgroundJobManager::new(degradation.clone()));

    // Register background jobs
    {
        let jm = job_mgr.clone();
        tokio::spawn(async move {
            jm.register_job("session_cleanup", 300, true, 10).await;
            jm.register_job("reservation_expiry", 60, true, 10).await;
            jm.register_job("offer_expiry", 15, false, 20).await;
            jm.register_job("analytics_snapshot", 3600, false, 5).await;
            jm.register_job("lock_cleanup", 60, true, 10).await;
        });
    }

    // App start time for uptime tracking
    let start_time = routes::health::AppStartTime(chrono::Utc::now().naive_utc());

    // -----------------------------------------------------------------------
    // Background master loop
    // -----------------------------------------------------------------------
    {
        let bg_pool = pool.clone();
        let bg_lock_mgr = lock_manager.clone();
        let bg_job_mgr = job_mgr.clone();
        let bg_degradation = degradation.clone();
        let bg_idle_timeout = idle_timeout;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));
            loop {
                interval.tick().await;

                let candidates = bg_job_mgr.get_due_jobs().await;
                let mut due = Vec::new();
                for name in candidates {
                    if bg_job_mgr.should_run(&name).await {
                        due.push(name);
                    }
                }
                for job_name in due {
                    match job_name.as_str() {
                        "session_cleanup" => {
                            match db::sessions::cleanup_expired_sessions(&bg_pool, bg_idle_timeout).await {
                                Ok(n) => {
                                    if n > 0 { tracing::info!(deleted = n, "cleaned up expired sessions"); }
                                    bg_job_mgr.record_job_success("session_cleanup").await;
                                    bg_degradation.record_success("sessions").await;
                                }
                                Err(e) => {
                                    bg_job_mgr.record_job_failure("session_cleanup", &e.to_string()).await;
                                }
                            }
                        }
                        "reservation_expiry" => {
                            match db::store::expire_stale_reservations(&bg_pool).await {
                                Ok(n) => {
                                    if n > 0 { tracing::info!(expired = n, "expired stale reservations"); }
                                    bg_job_mgr.record_job_success("reservation_expiry").await;
                                    bg_degradation.record_success("reservations").await;
                                }
                                Err(e) => {
                                    bg_job_mgr.record_job_failure("reservation_expiry", &e.to_string()).await;
                                }
                            }
                        }
                        "offer_expiry" => {
                            match db::dispatch::expire_stale_offers(&bg_pool).await {
                                Ok(n) => {
                                    if n > 0 { tracing::info!(expired = n, "expired stale task offers"); }
                                    bg_job_mgr.record_job_success("offer_expiry").await;
                                }
                                Err(e) => {
                                    bg_job_mgr.record_job_failure("offer_expiry", &e.to_string()).await;
                                }
                            }
                        }
                        "lock_cleanup" => {
                            let released = services::reservation_lock::release_expired(
                                &bg_pool,
                                &bg_lock_mgr,
                            )
                            .await;
                            if !released.is_empty() {
                                tracing::info!(count = released.len(), "released expired reservation locks");
                            }
                            bg_job_mgr.record_job_success("lock_cleanup").await;
                            bg_degradation.record_success("reservations").await;
                        }
                        "analytics_snapshot" => {
                            // Placeholder - analytics snapshot job
                            bg_job_mgr.record_job_success("analytics_snapshot").await;
                            bg_degradation.record_success("analytics").await;
                        }
                        _ => {}
                    }
                }
            }
        });
    }

    rocket::build()
        .manage(pool)
        .manage(job_mgr)
        .manage(session_config)
        .manage(crypto_config)
        .manage(lock_manager)
        .manage(degradation)
        .manage(start_time)
        .attach(cors)
        .attach(middleware::log_mask::LogMaskFairing)
        .mount("/api/auth", routes::auth::routes())
        .mount("/api/products", routes::products::routes())
        .mount("/api/cart", routes::cart::routes())
        .mount("/api/orders", routes::orders::routes())
        .mount("/api/staff", routes::staff::routes())
        .mount("/api/store", routes::store::routes())
        .mount("/api/exam", routes::exam::routes())
        .mount("/api/training", routes::training::routes())
        .mount("/api/admin", routes::admin::routes())
        .mount("/api/i18n", routes::i18n::routes())
        .mount("/api/dispatch", routes::dispatch::routes())
        .mount("/health", routes::health::routes())
        .mount("/", routes::sitemap::routes())
}
