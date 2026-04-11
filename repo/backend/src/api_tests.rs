/// Route-level integration tests using Rocket's `local::blocking::Client`.
///
/// # Test tiers
///
/// ## No-DB tests (always run)
/// A minimal test Rocket mounts:
/// - `/health/live`  — no state needed → 200
/// - `/test/*`       — tiny test-only routes that only use auth guards (no pool)
///
/// These verify auth-guard 401 behaviour and health probe reachability without
/// a live database.
///
/// ## DB-dependent tests (require `TEST_DATABASE_URL`)
/// Full-stack tests that exercise real login, role enforcement, business rules.
/// Seed the DB with `database/test_users.sql` before running:
/// ```
/// TEST_DATABASE_URL=mysql://... cargo test --package backend
/// ```
/// When `TEST_DATABASE_URL` is absent these tests pass immediately (CI must
/// always provide the variable so the assertions are exercised).
#[cfg(test)]
mod tests {
    use rocket::http::{ContentType, Status};
    use rocket::local::blocking::Client;
    use rocket::{get, routes};

    use crate::middleware::auth_guard::{AdminGuard, AuthenticatedUser, StaffGuard};

    // ── Test-only stub routes ─────────────────────────────────────────────────
    // These routes use only the auth guards (no DB pool), so we can mount them
    // in a Rocket that has no MySqlPool managed.

    #[get("/auth-required")]
    fn stub_auth(_user: AuthenticatedUser) -> &'static str { "ok" }

    #[get("/staff-required")]
    fn stub_staff(_staff: StaffGuard) -> &'static str { "ok" }

    #[get("/admin-required")]
    fn stub_admin(_admin: AdminGuard) -> &'static str { "ok" }

    // ── Test Rocket builders ──────────────────────────────────────────────────

    /// Minimal Rocket — no pool, only the routes/mounts we want to test without DB.
    fn no_db_rocket() -> rocket::Rocket<rocket::Build> {
        let session_config = crate::services::session::SessionConfig {
            cookie_secret: [0xABu8; 32],
            idle_timeout_secs: 1800,
            rotation_interval_secs: 300,
        };

        rocket::build()
            .manage(session_config)
            // Test-only stubs that only use auth guards (no pool needed)
            .mount(
                "/test",
                routes![stub_auth, stub_staff, stub_admin],
            )
            // /health/live has zero state dependencies
            .mount("/health", routes![crate::routes::health::live])
    }

    /// Full Rocket connected to `TEST_DATABASE_URL`.
    /// Returns `None` when the env-var is absent (local dev without DB).
    async fn db_rocket() -> Option<rocket::Rocket<rocket::Build>> {
        let url = std::env::var("TEST_DATABASE_URL")
            .or_else(|_| std::env::var("DATABASE_URL"))
            .ok()?;
        let pool = sqlx::MySqlPool::connect(&url).await.ok()?;

        let session_config = crate::services::session::SessionConfig::from_env();
        let crypto_config = crate::services::crypto::CryptoConfig::from_env();
        let lock_mgr = crate::services::reservation_lock::ReservationLockManager::new();
        let degradation =
            std::sync::Arc::new(crate::services::resilience::DegradationManager::new());
        let job_mgr = std::sync::Arc::new(
            crate::services::resilience::BackgroundJobManager::new(degradation.clone()),
        );
        let start_time =
            crate::routes::health::AppStartTime(chrono::Utc::now().naive_utc());

        Some(
            rocket::build()
                .manage(pool)
                .manage(session_config)
                .manage(crypto_config)
                .manage(lock_mgr)
                .manage(degradation)
                .manage(job_mgr)
                .manage(start_time)
                .mount("/api/auth", crate::routes::auth::routes())
                .mount("/api/cart", crate::routes::cart::routes())
                .mount("/api/orders", crate::routes::orders::routes())
                .mount("/api/staff", crate::routes::staff::routes())
                .mount("/api/training", crate::routes::training::routes())
                .mount("/api/exam", crate::routes::exam::routes())
                .mount("/api/admin", crate::routes::admin::routes())
                .mount("/health", crate::routes::health::routes()),
        )
    }

    // ── Helper: login and return session cookie ───────────────────────────────

    fn login(client: &Client, username: &str, password: &str) -> String {
        let resp = client
            .post("/api/auth/login")
            .header(ContentType::JSON)
            .body(format!(r#"{{"username":"{}","password":"{}"}}"#, username, password))
            .dispatch();
        assert_eq!(resp.status(), Status::Ok, "{} login must succeed", username);
        let body: serde_json::Value =
            serde_json::from_str(&resp.into_string().unwrap_or_default())
                .expect("login response must be valid JSON");
        let cookie = body["data"]["session_cookie"]
            .as_str()
            .expect("session_cookie must be present")
            .to_string();
        assert!(!cookie.is_empty(), "session_cookie must not be empty");
        cookie
    }

    // ── Macro: require DB — panics if TEST_DATABASE_URL is absent ───────────

    /// Builds a `db_rocket()` or panics.
    ///
    /// DB-dependent tests MUST fail loudly when `TEST_DATABASE_URL` is not set.
    /// Use `cargo test` filters or `run_tests.sh` to exclude them locally.
    /// Returns `(runtime, rocket)`.  The runtime **must** stay alive for the
    /// entire test because the sqlx pool spawns background tasks on it.
    /// Bind it as `_rt` so the drop runs at the end of the test function.
    macro_rules! require_db {
        () => {{
            let rt = tokio::runtime::Runtime::new().unwrap();
            let rocket = rt.block_on(db_rocket()).expect(
                "TEST_DATABASE_URL (or DATABASE_URL) must be set and connectable to run DB tests.\n\
                 To skip DB tests locally: cargo test --package backend -- --skip login --skip customer --skip staff --skip add_to_cart --skip confirm_order --skip scan_voucher --skip nonexistent --skip training"
            );
            (rt, rocket)
        }};
    }

    // ── Health endpoints (no DB required) ─────────────────────────────────────

    #[test]
    fn health_live_returns_200() {
        let client = Client::tracked(no_db_rocket()).expect("valid rocket");
        let resp = client.get("/health/live").dispatch();
        assert_eq!(resp.status(), Status::Ok);
        let body = resp.into_string().unwrap_or_default();
        assert!(body.contains("alive"), "expected 'alive' in body, got: {}", body);
    }

    // ── Auth guard: unauthenticated requests → 401 (no DB required) ──────────

    #[test]
    fn protected_endpoint_returns_401_without_cookie() {
        let client = Client::tracked(no_db_rocket()).expect("valid rocket");
        let resp = client.get("/test/auth-required").dispatch();
        assert_eq!(resp.status(), Status::Unauthorized);
    }

    #[test]
    fn staff_endpoint_returns_401_without_cookie() {
        let client = Client::tracked(no_db_rocket()).expect("valid rocket");
        let resp = client.get("/test/staff-required").dispatch();
        assert_eq!(resp.status(), Status::Unauthorized);
    }

    #[test]
    fn admin_endpoint_returns_401_without_cookie() {
        let client = Client::tracked(no_db_rocket()).expect("valid rocket");
        let resp = client.get("/test/admin-required").dispatch();
        assert_eq!(resp.status(), Status::Unauthorized);
    }

    // ── Auth guard: tampered cookie rejected (no DB required) ─────────────────

    #[test]
    fn endpoint_rejects_tampered_session_cookie() {
        let client = Client::tracked(no_db_rocket()).expect("valid rocket");
        let resp = client
            .get("/test/auth-required")
            .cookie(rocket::http::Cookie::new(
                "brewflow_session",
                "fakesession.badsignature",
            ))
            .dispatch();
        assert_eq!(resp.status(), Status::Unauthorized);
    }

    // ══════════════════════════════════════════════════════════════════════════
    // DB-dependent integration tests
    //
    // These run when TEST_DATABASE_URL is set (always in CI).
    // Locally without a DB they pass trivially via `require_db!()`.
    // ══════════════════════════════════════════════════════════════════════════

    #[test]
    fn login_with_valid_credentials_returns_session_cookie() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let resp = client
            .post("/api/auth/login")
            .header(ContentType::JSON)
            .body(r#"{"username":"admin","password":"AdminPass123!"}"#)
            .dispatch();
        assert_eq!(resp.status(), Status::Ok);
        let body: serde_json::Value =
            serde_json::from_str(&resp.into_string().unwrap_or_default())
                .expect("response must be valid JSON");
        assert!(body["success"].as_bool().unwrap_or(false), "success must be true");
        assert!(
            body["data"]["session_cookie"].as_str().map_or(false, |s| !s.is_empty()),
            "session_cookie must be a non-empty string, got: {}",
            body
        );
    }

    #[test]
    fn login_with_wrong_password_returns_401() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let resp = client
            .post("/api/auth/login")
            .header(ContentType::JSON)
            .body(r#"{"username":"admin","password":"WrongPassword!"}"#)
            .dispatch();
        assert_eq!(resp.status(), Status::Unauthorized);
    }

    #[test]
    fn customer_cannot_access_staff_orders_returns_403() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "customer", "CustomerPass123!");

        let resp = client
            .get("/api/staff/orders")
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .dispatch();
        assert_eq!(resp.status(), Status::Forbidden);
    }

    #[test]
    fn customer_cannot_access_admin_endpoint_returns_403() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "customer", "CustomerPass123!");

        let resp = client
            .get("/api/admin/users")
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .dispatch();
        assert_eq!(resp.status(), Status::Forbidden);
    }

    #[test]
    fn add_to_cart_without_required_option_group_returns_422() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "customer", "CustomerPass123!");

        // spu_id=1 (Classic Latte) has required option groups (Size, Milk, Sweetness)
        // in seed data — omitting selections MUST yield 422.
        let resp = client
            .post("/api/cart/add")
            .header(ContentType::JSON)
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .body(r#"{"spu_id":1,"selected_options":[],"quantity":1}"#)
            .dispatch();
        assert_eq!(
            resp.status(),
            Status::UnprocessableEntity,
            "missing required option group must return 422, got: {}",
            resp.status()
        );
    }

    #[test]
    fn confirm_order_with_expired_hold_returns_409() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "customer", "CustomerPass123!");

        // Order 9000 (from test_users.sql fixtures) has reservation 9000
        // with hold_expires_at in the past → must return 409 Conflict.
        let resp = client
            .post("/api/orders/9000/confirm")
            .header(ContentType::JSON)
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .body("{}")
            .dispatch();
        assert_eq!(
            resp.status(),
            Status::Conflict,
            "expired reservation hold must return 409, got: {}",
            resp.status()
        );
    }

    #[test]
    fn scan_voucher_for_cancelled_order_returns_valid_false() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "staff", "StaffPass123!");

        // Voucher TEST-CANCELLED-VOUCHER-001 (from test_users.sql fixtures)
        // is linked to order 9001 which is in 'Canceled' status.
        // Scan must return 200 with valid=false.
        let resp = client
            .post("/api/staff/scan")
            .header(ContentType::JSON)
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .body(r#"{"voucher_code":"TEST-CANCELLED-VOUCHER-001"}"#)
            .dispatch();
        assert_eq!(
            resp.status(),
            Status::Ok,
            "scan endpoint must return 200, got: {}",
            resp.status()
        );
        let body: serde_json::Value =
            serde_json::from_str(&resp.into_string().unwrap_or_default())
                .expect("scan response must be valid JSON");
        assert_eq!(
            body["data"]["valid"],
            serde_json::Value::Bool(false),
            "cancelled-order voucher must have valid=false, got: {}",
            body
        );
        assert_eq!(
            body["data"]["mismatch"],
            serde_json::Value::Bool(true),
            "cancelled-order voucher must flag mismatch, got: {}",
            body
        );
    }

    // ══════════════════════════════════════════════════════════════════════════
    // Additional no-DB tests
    // ══════════════════════════════════════════════════════════════════════════

    #[test]
    fn health_live_body_is_valid_json() {
        let client = Client::tracked(no_db_rocket()).expect("valid rocket");
        let resp = client.get("/health/live").dispatch();
        let body = resp.into_string().unwrap_or_default();
        let parsed: serde_json::Value = serde_json::from_str(&body)
            .expect("health response must be valid JSON");
        assert!(parsed["success"].as_bool().unwrap_or(false), "expected success=true");
        assert_eq!(parsed["data"].as_str(), Some("alive"));
    }

    #[test]
    fn empty_cookie_value_returns_401() {
        let client = Client::tracked(no_db_rocket()).expect("valid rocket");
        let resp = client
            .get("/test/auth-required")
            .cookie(rocket::http::Cookie::new("brewflow_session", ""))
            .dispatch();
        assert_eq!(resp.status(), Status::Unauthorized);
    }

    #[test]
    fn missing_signature_separator_returns_401() {
        let client = Client::tracked(no_db_rocket()).expect("valid rocket");
        let resp = client
            .get("/test/admin-required")
            .cookie(rocket::http::Cookie::new(
                "brewflow_session",
                "noseparatorhere",
            ))
            .dispatch();
        assert_eq!(resp.status(), Status::Unauthorized);
    }

    // ══════════════════════════════════════════════════════════════════════════
    // Additional DB-dependent tests
    // ══════════════════════════════════════════════════════════════════════════

    #[test]
    fn login_response_contains_user_info() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let resp = client
            .post("/api/auth/login")
            .header(ContentType::JSON)
            .body(r#"{"username":"admin","password":"AdminPass123!"}"#)
            .dispatch();
        assert_eq!(resp.status(), Status::Ok);
        let body: serde_json::Value =
            serde_json::from_str(&resp.into_string().unwrap_or_default())
                .expect("valid JSON");
        let user = &body["data"]["user"];
        assert_eq!(user["username"].as_str(), Some("admin"));
        assert!(user["roles"].as_array().map_or(false, |r| !r.is_empty()));
    }

    #[test]
    fn login_with_empty_body_returns_error() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let resp = client
            .post("/api/auth/login")
            .header(ContentType::JSON)
            .body("{}")
            .dispatch();
        // Missing username/password should not return 200
        assert_ne!(resp.status(), Status::Ok);
    }

    #[test]
    fn staff_can_access_staff_orders() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "staff", "StaffPass123!");

        let resp = client
            .get("/api/staff/orders")
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .dispatch();
        assert_eq!(resp.status(), Status::Ok);
        let body: serde_json::Value =
            serde_json::from_str(&resp.into_string().unwrap_or_default())
                .expect("valid JSON");
        assert!(body["success"].as_bool().unwrap_or(false));
    }

    #[test]
    fn staff_dashboard_returns_counts() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "staff", "StaffPass123!");

        let resp = client
            .get("/api/staff/dashboard/counts")
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .dispatch();
        assert_eq!(resp.status(), Status::Ok);
        let body: serde_json::Value =
            serde_json::from_str(&resp.into_string().unwrap_or_default())
                .expect("valid JSON");
        let data = &body["data"];
        assert!(data["pending_count"].is_number());
        assert!(data["in_prep_count"].is_number());
        assert!(data["ready_count"].is_number());
    }

    #[test]
    fn customer_can_list_own_orders() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "customer", "CustomerPass123!");

        let resp = client
            .get("/api/orders")
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .dispatch();
        assert_eq!(resp.status(), Status::Ok);
        let body: serde_json::Value =
            serde_json::from_str(&resp.into_string().unwrap_or_default())
                .expect("valid JSON");
        assert!(body["success"].as_bool().unwrap_or(false));
        assert!(body["data"].is_array());
    }

    #[test]
    fn nonexistent_voucher_scan_returns_404() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "staff", "StaffPass123!");

        let resp = client
            .post("/api/staff/scan")
            .header(ContentType::JSON)
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .body(r#"{"voucher_code":"NONEXISTENT-CODE-999"}"#)
            .dispatch();
        assert_eq!(resp.status(), Status::NotFound);
    }

    #[test]
    fn training_attempts_requires_auth() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");

        let resp = client.get("/api/training/attempts").dispatch();
        assert_eq!(resp.status(), Status::Unauthorized);
    }

    #[test]
    fn training_analytics_returns_data_for_customer() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "customer", "CustomerPass123!");

        let resp = client
            .get("/api/training/analytics")
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .dispatch();
        assert_eq!(resp.status(), Status::Ok);
        let body: serde_json::Value =
            serde_json::from_str(&resp.into_string().unwrap_or_default())
                .expect("valid JSON");
        assert!(body["success"].as_bool().unwrap_or(false));
        assert!(body["data"]["overall_score"].is_number());
    }
}
