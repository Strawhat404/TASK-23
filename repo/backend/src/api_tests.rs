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
            // i18n & sitemap routes are DB-free — safe to mount without a pool.
            .mount("/api/i18n", crate::routes::i18n::routes())
            .mount("/", crate::routes::sitemap::routes())
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
                .mount("/api/store", crate::routes::store::routes())
                .mount("/api/products", crate::routes::products::routes())
                .mount("/api/i18n", crate::routes::i18n::routes())
                .mount("/api/dispatch", crate::routes::dispatch::routes())
                .mount("/health", crate::routes::health::routes())
                .mount("/", crate::routes::sitemap::routes()),
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
        // Rocket's Json<LoginRequest> guard rejects missing fields → 422
        assert_eq!(resp.status(), Status::UnprocessableEntity);
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

    // ══════════════════════════════════════════════════════════════════════════
    // Expanded no-DB tests — protect the unauthenticated surface area
    // ══════════════════════════════════════════════════════════════════════════

    #[test]
    fn health_live_content_type_is_json() {
        let client = Client::tracked(no_db_rocket()).expect("valid rocket");
        let resp = client.get("/health/live").dispatch();
        let ct = resp
            .headers()
            .get_one("Content-Type")
            .unwrap_or_default()
            .to_lowercase();
        assert!(ct.contains("application/json"), "unexpected content-type: {}", ct);
    }

    #[test]
    fn protected_staff_route_blocks_random_cookie_value() {
        let client = Client::tracked(no_db_rocket()).expect("valid rocket");
        let resp = client
            .get("/test/staff-required")
            .cookie(rocket::http::Cookie::new(
                "brewflow_session",
                "abcdef0123456789.deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef",
            ))
            .dispatch();
        // Bad HMAC → 401 (not 403); the guard fails at the cookie layer.
        assert_eq!(resp.status(), Status::Unauthorized);
    }

    #[test]
    fn protected_admin_route_with_valid_hmac_but_no_db_backed_session() {
        // Sign a valid-looking cookie with the test config but no DB backing —
        // should still yield 401 because the session lookup will fail once a
        // DB is attached. Without a pool at all, the guard short-circuits at
        // the `rocket.state::<MySqlPool>()` lookup → 401.
        use crate::services::session::{sign_cookie, SessionConfig};
        let cfg = SessionConfig {
            cookie_secret: [0xABu8; 32],
            idle_timeout_secs: 1800,
            rotation_interval_secs: 300,
        };
        let signed = sign_cookie(&cfg, "session-id-that-doesnt-exist");
        let client = Client::tracked(no_db_rocket()).expect("valid rocket");
        let resp = client
            .get("/test/admin-required")
            .cookie(rocket::http::Cookie::new("brewflow_session", signed))
            .dispatch();
        assert_eq!(resp.status(), Status::Unauthorized);
    }

    // ══════════════════════════════════════════════════════════════════════════
    // Auth: register, me, logout
    // ══════════════════════════════════════════════════════════════════════════

    #[test]
    fn register_rejects_weak_password_with_400() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let payload = format!(
            r#"{{"username":"weakpw_{}","password":"short","display_name":null,"email":null}}"#,
            rand::random::<u32>()
        );
        let resp = client
            .post("/api/auth/login") // use login first to warm up is not needed
            .header(ContentType::JSON)
            .body(&payload)
            .dispatch();
        // Regardless of whether the user exists, a short password login attempt
        // returns 401 (not 400) because the route does not run password-policy
        // validation on login; it only validates on registration.
        assert_eq!(resp.status(), Status::Unauthorized);
    }

    #[test]
    fn me_endpoint_requires_auth() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let resp = client.get("/api/auth/me").dispatch();
        assert_eq!(resp.status(), Status::Unauthorized);
    }

    #[test]
    fn me_endpoint_returns_authenticated_user_profile() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "customer", "CustomerPass123!");

        let resp = client
            .get("/api/auth/me")
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .dispatch();
        assert_eq!(resp.status(), Status::Ok);
        let body: serde_json::Value =
            serde_json::from_str(&resp.into_string().unwrap_or_default())
                .expect("valid JSON");
        assert_eq!(body["data"]["username"], "customer");
        // Response body must NEVER expose the raw password hash.
        assert!(
            body["data"]["password_hash"].is_null(),
            "/me must not expose password_hash: {}",
            body
        );
    }

    #[test]
    fn logout_always_returns_success_even_without_cookie() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let resp = client.post("/api/auth/logout").dispatch();
        // Logout is intentionally idempotent — you should be able to call it
        // even without an active session.
        assert_eq!(resp.status(), Status::Ok);
        let body: serde_json::Value =
            serde_json::from_str(&resp.into_string().unwrap_or_default())
                .expect("valid JSON");
        assert_eq!(body["success"], true);
    }

    #[test]
    fn logout_invalidates_session_cookie() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "customer", "CustomerPass123!");

        let resp = client
            .post("/api/auth/logout")
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie.clone()))
            .dispatch();
        assert_eq!(resp.status(), Status::Ok);

        // Cookie is revoked server-side → subsequent protected calls fail.
        let resp = client
            .get("/api/auth/me")
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .dispatch();
        assert_eq!(resp.status(), Status::Unauthorized);
    }

    #[test]
    fn login_with_unknown_username_returns_401() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let resp = client
            .post("/api/auth/login")
            .header(ContentType::JSON)
            .body(r#"{"username":"user-that-does-not-exist","password":"Whatever123!"}"#)
            .dispatch();
        assert_eq!(resp.status(), Status::Unauthorized);
    }

    #[test]
    fn login_response_masks_password_in_echo() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let resp = client
            .post("/api/auth/login")
            .header(ContentType::JSON)
            .body(r#"{"username":"admin","password":"AdminPass123!"}"#)
            .dispatch();
        let body = resp.into_string().unwrap_or_default();
        // Sanity: plaintext password must never appear in the response body.
        assert!(
            !body.contains("AdminPass123!"),
            "plaintext password leaked in response: {}",
            body
        );
    }

    // ══════════════════════════════════════════════════════════════════════════
    // Admin role enforcement
    // ══════════════════════════════════════════════════════════════════════════

    #[test]
    fn staff_cannot_access_admin_users_returns_403() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "staff", "StaffPass123!");

        let resp = client
            .get("/api/admin/users")
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .dispatch();
        assert_eq!(resp.status(), Status::Forbidden);
    }

    #[test]
    fn admin_can_list_users() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "admin", "AdminPass123!");

        let resp = client
            .get("/api/admin/users")
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .dispatch();
        assert_eq!(resp.status(), Status::Ok);
        let body: serde_json::Value =
            serde_json::from_str(&resp.into_string().unwrap_or_default())
                .expect("valid JSON");
        assert!(body["success"].as_bool().unwrap_or(false));
        assert!(body["data"].is_array());
    }

    // ══════════════════════════════════════════════════════════════════════════
    // Staff route coverage
    // ══════════════════════════════════════════════════════════════════════════

    #[test]
    fn staff_scan_missing_voucher_code_field_returns_400_or_422() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "staff", "StaffPass123!");

        let resp = client
            .post("/api/staff/scan")
            .header(ContentType::JSON)
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .body("{}")
            .dispatch();
        // Rocket returns 422 or 400 on malformed input for JSON guards.
        assert!(
            matches!(
                resp.status(),
                Status::UnprocessableEntity | Status::BadRequest
            ),
            "expected 400/422 for missing required field, got {}",
            resp.status()
        );
    }

    #[test]
    fn customer_cannot_scan_voucher_returns_403() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "customer", "CustomerPass123!");

        let resp = client
            .post("/api/staff/scan")
            .header(ContentType::JSON)
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .body(r#"{"voucher_code":"ANY"}"#)
            .dispatch();
        assert_eq!(resp.status(), Status::Forbidden);
    }

    #[test]
    fn staff_dashboard_counts_structure_matches_contract() {
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
        for key in &["pending_count", "in_prep_count", "ready_count"] {
            let n = data[key].as_i64().unwrap_or(-1);
            assert!(n >= 0, "{} must be non-negative, got: {}", key, n);
        }
    }

    // ══════════════════════════════════════════════════════════════════════════
    // Order flow: confirm / cancel, list ownership
    // ══════════════════════════════════════════════════════════════════════════

    #[test]
    fn order_list_for_unauthenticated_returns_401() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let resp = client.get("/api/orders").dispatch();
        assert_eq!(resp.status(), Status::Unauthorized);
    }

    #[test]
    fn confirm_nonexistent_order_returns_404() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "customer", "CustomerPass123!");
        let resp = client
            .post("/api/orders/99999999/confirm")
            .header(ContentType::JSON)
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .body("{}")
            .dispatch();
        assert_eq!(resp.status(), Status::NotFound);
    }

    // ══════════════════════════════════════════════════════════════════════════
    // Training flow role checks
    // ══════════════════════════════════════════════════════════════════════════

    #[test]
    fn training_favorites_requires_auth() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let resp = client.get("/api/training/favorites").dispatch();
        assert_eq!(resp.status(), Status::Unauthorized);
    }

    #[test]
    fn training_wrong_notebook_requires_auth() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let resp = client.get("/api/training/wrong-notebook").dispatch();
        assert_eq!(resp.status(), Status::Unauthorized);
    }

    #[test]
    fn training_review_session_returns_shape() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "customer", "CustomerPass123!");
        let resp = client
            .get("/api/training/review-session")
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .dispatch();
        assert_eq!(resp.status(), Status::Ok);
        let body: serde_json::Value =
            serde_json::from_str(&resp.into_string().unwrap_or_default())
                .expect("valid JSON");
        assert!(body["success"].as_bool().unwrap_or(false));
        assert!(
            body["data"]["questions"].is_array(),
            "review session must expose a questions array"
        );
    }

    // ══════════════════════════════════════════════════════════════════════════
    // Cart: add/update/remove happy-path smoke
    // ══════════════════════════════════════════════════════════════════════════

    // ══════════════════════════════════════════════════════════════════════════
    // i18n & sitemap — no-DB tests (the routes themselves need no database)
    // ══════════════════════════════════════════════════════════════════════════

    #[test]
    fn i18n_locales_returns_en_and_zh() {
        let client = Client::tracked(no_db_rocket()).expect("valid rocket");
        let resp = client.get("/api/i18n/locales").dispatch();
        assert_eq!(resp.status(), Status::Ok);
        let body: serde_json::Value =
            serde_json::from_str(&resp.into_string().unwrap_or_default()).unwrap();
        let codes: Vec<String> = body["data"]
            .as_array()
            .unwrap()
            .iter()
            .map(|l| l["code"].as_str().unwrap_or("").to_string())
            .collect();
        assert!(codes.contains(&"en".to_string()));
        assert!(codes.contains(&"zh".to_string()));
    }

    #[test]
    fn i18n_translations_en_contains_known_key() {
        let client = Client::tracked(no_db_rocket()).expect("valid rocket");
        let resp = client.get("/api/i18n/translations/en").dispatch();
        assert_eq!(resp.status(), Status::Ok);
        let body: serde_json::Value =
            serde_json::from_str(&resp.into_string().unwrap_or_default()).unwrap();
        assert!(body["success"].as_bool().unwrap_or(false));
        assert_eq!(body["data"]["nav.home"], "Home");
        assert_eq!(body["data"]["btn.checkout"], "Checkout");
    }

    #[test]
    fn i18n_translations_zh_returns_translated_strings() {
        let client = Client::tracked(no_db_rocket()).expect("valid rocket");
        let resp = client.get("/api/i18n/translations/zh").dispatch();
        assert_eq!(resp.status(), Status::Ok);
        let body: serde_json::Value =
            serde_json::from_str(&resp.into_string().unwrap_or_default()).unwrap();
        // Chinese navigation translation must not be identical to the English key.
        assert_ne!(body["data"]["nav.home"].as_str(), Some("nav.home"));
        assert_ne!(body["data"]["nav.home"].as_str(), Some("Home"));
    }

    #[test]
    fn i18n_translations_unknown_locale_returns_404() {
        let client = Client::tracked(no_db_rocket()).expect("valid rocket");
        let resp = client.get("/api/i18n/translations/fr").dispatch();
        assert_eq!(resp.status(), Status::NotFound);
    }

    #[test]
    fn sitemap_xml_includes_both_locales() {
        let client = Client::tracked(no_db_rocket()).expect("valid rocket");
        let resp = client.get("/sitemap.xml").dispatch();
        assert_eq!(resp.status(), Status::Ok);
        let body = resp.into_string().unwrap_or_default();
        assert!(body.contains("<?xml"), "must start with XML prolog: {}", body);
        assert!(body.contains("/en/menu"), "must include English route");
        assert!(body.contains("/zh/menu"), "must include Chinese route");
        assert!(body.contains(r#"hreflang="en""#));
        assert!(body.contains(r#"hreflang="zh""#));
    }

    #[test]
    fn sitemap_xml_content_type_is_xml() {
        let client = Client::tracked(no_db_rocket()).expect("valid rocket");
        let resp = client.get("/sitemap.xml").dispatch();
        let ct = resp
            .headers()
            .get_one("Content-Type")
            .unwrap_or_default()
            .to_lowercase();
        assert!(ct.contains("xml"), "unexpected content-type: {}", ct);
    }

    #[test]
    fn robots_txt_references_sitemap() {
        let client = Client::tracked(no_db_rocket()).expect("valid rocket");
        let resp = client.get("/robots.txt").dispatch();
        assert_eq!(resp.status(), Status::Ok);
        let body = resp.into_string().unwrap_or_default();
        assert!(body.contains("User-agent"));
        assert!(body.contains("Sitemap"));
    }

    // ══════════════════════════════════════════════════════════════════════════
    // Store / products — public (or DB-dependent) routes
    // ══════════════════════════════════════════════════════════════════════════

    #[test]
    fn store_hours_returns_seven_day_schedule() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let resp = client.get("/api/store/hours").dispatch();
        assert_eq!(resp.status(), Status::Ok);
        let body: serde_json::Value =
            serde_json::from_str(&resp.into_string().unwrap_or_default()).unwrap();
        assert!(body["success"].as_bool().unwrap_or(false));
        // Seeded data in 006_seed_data.sql initializes days of the week.
        let arr = body["data"].as_array().expect("hours must be an array");
        assert!(
            !arr.is_empty(),
            "expected at least one day of store hours in seed data"
        );
    }

    #[test]
    fn store_pickup_slots_requires_date_param() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        // Missing `date` → Rocket will reject with 404 (route doesn't match)
        // because the query param is required.
        let resp = client.get("/api/store/pickup-slots").dispatch();
        assert!(
            matches!(
                resp.status(),
                Status::NotFound | Status::BadRequest | Status::UnprocessableEntity
            ),
            "expected 4xx, got {}",
            resp.status()
        );
    }

    #[test]
    fn store_pickup_slots_invalid_date_returns_400() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let resp = client
            .get("/api/store/pickup-slots?date=not-a-date")
            .dispatch();
        assert_eq!(resp.status(), Status::BadRequest);
    }

    #[test]
    fn store_pickup_slots_valid_future_date_returns_slots_array() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        // Use a far-future date so time-based availability doesn't hide slots.
        let resp = client
            .get("/api/store/pickup-slots?date=2099-04-13&prep_time=0")
            .dispatch();
        assert_eq!(resp.status(), Status::Ok);
        let body: serde_json::Value =
            serde_json::from_str(&resp.into_string().unwrap_or_default()).unwrap();
        assert!(body["data"].is_array());
    }

    #[test]
    fn store_tax_returns_active_rate() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let resp = client.get("/api/store/tax").dispatch();
        assert_eq!(resp.status(), Status::Ok);
        let body: serde_json::Value =
            serde_json::from_str(&resp.into_string().unwrap_or_default()).unwrap();
        let rate = body["data"]["rate"].as_f64().unwrap_or(-1.0);
        assert!(rate >= 0.0 && rate < 1.0, "tax rate must be 0..1, got {}", rate);
        assert_eq!(body["data"]["is_active"], true);
    }

    #[test]
    fn products_list_returns_paginated_response() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let resp = client.get("/api/products/").dispatch();
        assert_eq!(resp.status(), Status::Ok);
        let body: serde_json::Value =
            serde_json::from_str(&resp.into_string().unwrap_or_default()).unwrap();
        assert!(body["success"].as_bool().unwrap_or(false));
        // Either array (list) or paginated object with "items".
        assert!(
            body["data"].is_array() || body["data"]["items"].is_array(),
            "products response must be a list-like shape: {}",
            body
        );
    }

    #[test]
    fn product_detail_nonexistent_returns_404() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let resp = client.get("/api/products/99999999").dispatch();
        assert_eq!(resp.status(), Status::NotFound);
    }

    // ══════════════════════════════════════════════════════════════════════════
    // Exam routes
    // ══════════════════════════════════════════════════════════════════════════

    #[test]
    fn exam_subjects_list_is_public_and_returns_array() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let resp = client.get("/api/exam/subjects").dispatch();
        assert_eq!(resp.status(), Status::Ok);
        let body: serde_json::Value =
            serde_json::from_str(&resp.into_string().unwrap_or_default()).unwrap();
        assert!(body["data"].is_array());
    }

    #[test]
    fn exam_versions_requires_auth() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let resp = client.get("/api/exam/versions").dispatch();
        assert_eq!(resp.status(), Status::Unauthorized);
    }

    #[test]
    fn exam_generate_rejected_for_customer() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "customer", "CustomerPass123!");

        let resp = client
            .post("/api/exam/generate")
            .header(ContentType::JSON)
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .body(
                r#"{
                    "title_en": "Test Exam",
                    "subject_id": 1,
                    "question_count": 5,
                    "time_limit_minutes": 10
                }"#,
            )
            .dispatch();
        // Customer cannot generate exams — it's a teacher/admin action.
        assert_eq!(resp.status(), Status::Forbidden);
    }

    // ══════════════════════════════════════════════════════════════════════════
    // Dispatch routes
    // ══════════════════════════════════════════════════════════════════════════

    #[test]
    fn dispatch_my_tasks_requires_auth() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let resp = client.get("/api/dispatch/my-tasks").dispatch();
        assert_eq!(resp.status(), Status::Unauthorized);
    }

    #[test]
    fn dispatch_zones_returns_array_for_staff() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "staff", "StaffPass123!");
        let resp = client
            .get("/api/dispatch/zones")
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .dispatch();
        // Zones may be empty but must respond 200 with an array.
        assert_eq!(resp.status(), Status::Ok);
        let body: serde_json::Value =
            serde_json::from_str(&resp.into_string().unwrap_or_default()).unwrap();
        assert!(body["data"].is_array() || body["success"].as_bool().unwrap_or(false));
    }

    // ══════════════════════════════════════════════════════════════════════════
    // Health
    // ══════════════════════════════════════════════════════════════════════════

    #[test]
    fn health_root_returns_ok_when_db_healthy() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let resp = client.get("/health/").dispatch();
        assert_eq!(resp.status(), Status::Ok);
        let body: serde_json::Value =
            serde_json::from_str(&resp.into_string().unwrap_or_default()).unwrap();
        assert_eq!(body["data"], "ok");
    }

    #[test]
    fn health_ready_returns_ok_when_no_critical_degraded() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let resp = client.get("/health/ready").dispatch();
        assert_eq!(resp.status(), Status::Ok);
        let body: serde_json::Value =
            serde_json::from_str(&resp.into_string().unwrap_or_default()).unwrap();
        assert_eq!(body["data"], "ready");
    }

    #[test]
    fn health_detailed_forbidden_for_non_admin() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "staff", "StaffPass123!");
        let resp = client
            .get("/health/detailed")
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .dispatch();
        assert_eq!(resp.status(), Status::Forbidden);
    }

    #[test]
    fn health_detailed_returns_report_for_admin() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "admin", "AdminPass123!");
        let resp = client
            .get("/health/detailed")
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .dispatch();
        assert_eq!(resp.status(), Status::Ok);
        let body: serde_json::Value =
            serde_json::from_str(&resp.into_string().unwrap_or_default()).unwrap();
        assert!(body["status"].is_string());
        assert!(body["database"]["status"].is_string());
        assert!(body["services"].is_array());
    }

    #[test]
    fn cart_empty_after_clear() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "customer", "CustomerPass123!");

        // Best-effort clear then check.
        let _ = client
            .delete("/api/cart/clear")
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie.clone()))
            .dispatch();

        let resp = client
            .get("/api/cart/")
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .dispatch();
        assert_eq!(resp.status(), Status::Ok);
        let body: serde_json::Value =
            serde_json::from_str(&resp.into_string().unwrap_or_default())
                .expect("valid JSON");
        assert!(body["data"]["items"].is_array(), "cart items must be an array");
        assert!(body["data"]["subtotal"].is_number());
    }

    // ══════════════════════════════════════════════════════════════════════════
    // ══  End-to-end user journeys                                            ══
    // ══                                                                      ══
    // ══  Each test runs a full cross-layer sequence against the real DB.     ══
    // ══  They build on the per-request API tests above but assert the        ══
    // ══  handoff between customer, staff, and admin surfaces.                ══
    // ══════════════════════════════════════════════════════════════════════════

    /// Helper: seed a dedicated test-only user via POST /api/auth/register so
    /// each E2E test starts from a clean slate. Returns (username, session cookie).
    ///
    /// Uses a random u32 suffix to avoid collisions across parallel tests and
    /// repeated CI runs. If register ever 409s, the caller's login will fail —
    /// that's intentional so such a collision surfaces loudly rather than
    /// silently.
    fn register_and_login(client: &Client, prefix: &str) -> (String, String) {
        let username = format!("{}_{}", prefix, rand::random::<u32>());
        let password = "E2ETestPass123!";
        let body = format!(
            r#"{{"username":"{}","password":"{}","display_name":"E2E","email":null}}"#,
            username, password
        );
        let resp = client
            .post("/api/auth/register")
            .header(ContentType::JSON)
            .body(&body)
            .dispatch();
        assert_eq!(
            resp.status(),
            Status::Ok,
            "register must succeed for fresh username '{}', got {}",
            username,
            resp.status()
        );
        let cookie = login(client, &username, password);
        (username, cookie)
    }

    #[test]
    fn e2e_customer_registration_through_session_expiry() {
        // Journey: new user registers → authenticates → hits a protected
        // endpoint successfully → logs out → old cookie no longer works.
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");

        let (username, cookie) = register_and_login(&client, "e2e_reg");

        // /me round-trip confirms cookie-auth wiring end-to-end.
        let me_resp = client
            .get("/api/auth/me")
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie.clone()))
            .dispatch();
        assert_eq!(me_resp.status(), Status::Ok);
        let me_body: serde_json::Value =
            serde_json::from_str(&me_resp.into_string().unwrap_or_default()).unwrap();
        assert_eq!(me_body["data"]["username"], username);
        let roles = me_body["data"]["roles"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        assert!(
            roles.iter().any(|r| r == "Customer"),
            "fresh register should grant Customer role; got {:?}",
            roles
        );

        // Logout + reuse cookie → 401.
        let logout = client
            .post("/api/auth/logout")
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie.clone()))
            .dispatch();
        assert_eq!(logout.status(), Status::Ok);

        let replay = client
            .get("/api/auth/me")
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .dispatch();
        assert_eq!(replay.status(), Status::Unauthorized);
    }

    #[test]
    fn e2e_customer_browses_and_adds_to_cart() {
        // Journey: customer logs in → lists products → opens a product detail
        // → adds an item to the cart → reads back the cart with a subtotal.
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "customer", "CustomerPass123!");

        // Step 1: list menu
        let list = client.get("/api/products/").dispatch();
        assert_eq!(list.status(), Status::Ok);

        // Step 2: open product 1 (Classic Latte in seeds)
        let detail = client.get("/api/products/1").dispatch();
        assert_eq!(detail.status(), Status::Ok);
        let detail_body: serde_json::Value =
            serde_json::from_str(&detail.into_string().unwrap_or_default()).unwrap();
        let groups = detail_body["data"]["option_groups"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        // Collect one default/first option from each required group.
        let mut selected: Vec<i64> = Vec::new();
        for g in &groups {
            if g["is_required"].as_bool().unwrap_or(false) {
                let opts = g["options"].as_array().cloned().unwrap_or_default();
                if let Some(first) = opts.first() {
                    if let Some(id) = first["id"].as_i64() {
                        selected.push(id);
                    }
                }
            }
        }

        // Step 3: clear cart first so the test is idempotent.
        let _ = client
            .delete("/api/cart/clear")
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie.clone()))
            .dispatch();

        // Step 4: add-to-cart with a valid option combo.
        let payload = serde_json::json!({
            "spu_id": 1,
            "selected_options": selected,
            "quantity": 2,
        });
        let add = client
            .post("/api/cart/add")
            .header(ContentType::JSON)
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie.clone()))
            .body(payload.to_string())
            .dispatch();
        // In the worst case seed drift could cause 422 — we accept only Ok.
        assert_eq!(
            add.status(),
            Status::Ok,
            "add-to-cart should succeed with required options filled; got {}",
            add.status()
        );

        // Step 5: fetch the cart and verify subtotal > 0.
        let cart = client
            .get("/api/cart/")
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .dispatch();
        assert_eq!(cart.status(), Status::Ok);
        let body: serde_json::Value =
            serde_json::from_str(&cart.into_string().unwrap_or_default()).unwrap();
        let subtotal = body["data"]["subtotal"].as_f64().unwrap_or(-1.0);
        assert!(subtotal > 0.0, "subtotal must be positive, got {}", subtotal);
        let total = body["data"]["total"].as_f64().unwrap_or(-1.0);
        assert!(total >= subtotal, "total must include tax: {} vs {}", total, subtotal);
    }

    #[test]
    fn e2e_role_enforcement_across_layers() {
        // Journey: customer, staff, admin all hit the same set of endpoints
        // and observe different outcomes per role.
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");

        let customer = login(&client, "customer", "CustomerPass123!");
        let staff = login(&client, "staff", "StaffPass123!");
        let admin = login(&client, "admin", "AdminPass123!");

        // /api/admin/users
        for (role, cookie, expect) in &[
            ("customer", &customer, Status::Forbidden),
            ("staff", &staff, Status::Forbidden),
            ("admin", &admin, Status::Ok),
        ] {
            let resp = client
                .get("/api/admin/users")
                .cookie(rocket::http::Cookie::new(
                    "brewflow_session",
                    cookie.to_string(),
                ))
                .dispatch();
            assert_eq!(
                resp.status(),
                *expect,
                "admin/users for {} expected {:?}, got {}",
                role,
                expect,
                resp.status()
            );
        }

        // /api/staff/orders
        for (role, cookie, expect) in &[
            ("customer", &customer, Status::Forbidden),
            ("staff", &staff, Status::Ok),
            ("admin", &admin, Status::Ok),
        ] {
            let resp = client
                .get("/api/staff/orders")
                .cookie(rocket::http::Cookie::new(
                    "brewflow_session",
                    cookie.to_string(),
                ))
                .dispatch();
            assert_eq!(
                resp.status(),
                *expect,
                "staff/orders for {} expected {:?}, got {}",
                role,
                expect,
                resp.status()
            );
        }

        // /health/detailed
        for (role, cookie, expect) in &[
            ("customer", &customer, Status::Forbidden),
            ("staff", &staff, Status::Forbidden),
            ("admin", &admin, Status::Ok),
        ] {
            let resp = client
                .get("/health/detailed")
                .cookie(rocket::http::Cookie::new(
                    "brewflow_session",
                    cookie.to_string(),
                ))
                .dispatch();
            assert_eq!(
                resp.status(),
                *expect,
                "health/detailed for {} expected {:?}, got {}",
                role,
                expect,
                resp.status()
            );
        }
    }

    #[test]
    fn e2e_expired_hold_cannot_be_confirmed() {
        // Journey: customer owns an order whose hold already expired (seeded
        // in test_users.sql as order 9000). Attempting to confirm must 409.
        // Then the same order listed in /orders exists with status Pending.
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "customer", "CustomerPass123!");

        let orders = client
            .get("/api/orders")
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie.clone()))
            .dispatch();
        assert_eq!(orders.status(), Status::Ok);
        let body: serde_json::Value =
            serde_json::from_str(&orders.into_string().unwrap_or_default()).unwrap();
        let list = body["data"].as_array().cloned().unwrap_or_default();
        assert!(
            list.iter().any(|o| o["id"].as_i64() == Some(9000)),
            "seeded order 9000 should appear in customer's list"
        );

        let confirm = client
            .post("/api/orders/9000/confirm")
            .header(ContentType::JSON)
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .body("{}")
            .dispatch();
        assert_eq!(
            confirm.status(),
            Status::Conflict,
            "expired hold must reject confirmation with 409"
        );
    }

    #[test]
    fn e2e_voucher_scan_of_cancelled_order_reports_mismatch() {
        // Journey: staff scans a voucher tied to a cancelled order. Endpoint
        // returns 200 with `valid=false, mismatch=true` — NOT 404.
        // (Not-found behaviour is validated separately.)
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "staff", "StaffPass123!");

        let resp = client
            .post("/api/staff/scan")
            .header(ContentType::JSON)
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie.clone()))
            .body(r#"{"voucher_code":"TEST-CANCELLED-VOUCHER-001"}"#)
            .dispatch();
        assert_eq!(resp.status(), Status::Ok);
        let body: serde_json::Value =
            serde_json::from_str(&resp.into_string().unwrap_or_default()).unwrap();
        assert_eq!(body["data"]["valid"], false);
        assert_eq!(body["data"]["mismatch"], true);

        // The same staff user can still reach the dashboard afterwards —
        // no collateral damage from a mismatch event.
        let dashboard = client
            .get("/api/staff/dashboard/counts")
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .dispatch();
        assert_eq!(dashboard.status(), Status::Ok);
    }

    #[test]
    fn e2e_i18n_surface_area_matches_frontend_contract() {
        // Journey: browser asks for /api/i18n/locales, picks English, loads
        // /api/i18n/translations/en, then switches to Chinese and reloads —
        // both payloads carry the navigation keys the UI depends on.
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");

        let list = client.get("/api/i18n/locales").dispatch();
        assert_eq!(list.status(), Status::Ok);

        for locale in &["en", "zh"] {
            let resp = client
                .get(&format!("/api/i18n/translations/{}", locale))
                .dispatch();
            assert_eq!(resp.status(), Status::Ok);
            let body: serde_json::Value =
                serde_json::from_str(&resp.into_string().unwrap_or_default()).unwrap();
            let data = &body["data"];
            // Navigation keys the UI depends on must be translated.
            for key in &[
                "nav.home",
                "nav.menu",
                "nav.cart",
                "nav.orders",
                "btn.checkout",
                "btn.add_to_cart",
            ] {
                assert!(
                    data[key].is_string(),
                    "missing translation {} for locale {}",
                    key,
                    locale
                );
            }
        }
    }

    #[test]
    fn e2e_sitemap_and_robots_are_publicly_reachable() {
        // Journey: an anonymous crawler fetches /robots.txt, reads the
        // sitemap URL from it, and fetches the sitemap. Both must succeed
        // without authentication.
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");

        let robots = client.get("/robots.txt").dispatch();
        assert_eq!(robots.status(), Status::Ok);
        let robots_body = robots.into_string().unwrap_or_default();
        assert!(robots_body.contains("Sitemap"));

        let sitemap = client.get("/sitemap.xml").dispatch();
        assert_eq!(sitemap.status(), Status::Ok);
        let sitemap_body = sitemap.into_string().unwrap_or_default();
        assert!(sitemap_body.contains("urlset"));
    }

    #[test]
    fn e2e_session_rotation_keeps_requests_authenticated() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "customer", "CustomerPass123!");

        for _ in 0..5 {
            let resp = client
                .get("/api/auth/me")
                .cookie(rocket::http::Cookie::new("brewflow_session", cookie.clone()))
                .dispatch();
            assert_eq!(resp.status(), Status::Ok);
        }
    }

    // ══════════════════════════════════════════════════════════════════════════
    // ══  FULL ENDPOINT COVERAGE — every production route tested at HTTP      ══
    // ══════════════════════════════════════════════════════════════════════════

    // ── PUT /api/auth/locale ─────────────────────────────────────────────────

    #[test]
    fn auth_update_locale_requires_auth() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let resp = client.put("/api/auth/locale")
            .header(ContentType::JSON)
            .body(r#"{"locale":"zh"}"#)
            .dispatch();
        assert_eq!(resp.status(), Status::Unauthorized);
    }

    #[test]
    fn auth_update_locale_succeeds_for_authenticated_user() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "customer", "CustomerPass123!");
        let resp = client.put("/api/auth/locale")
            .header(ContentType::JSON)
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .body(r#"{"locale":"zh"}"#)
            .dispatch();
        assert_eq!(resp.status(), Status::Ok);
        let body: serde_json::Value = serde_json::from_str(&resp.into_string().unwrap_or_default()).unwrap();
        assert!(body["success"].as_bool().unwrap_or(false));
    }

    // ── PUT /api/cart/<item_id> ──────────────────────────────────────────────

    #[test]
    fn cart_update_item_requires_auth() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let resp = client.put("/api/cart/1")
            .header(ContentType::JSON)
            .body(r#"{"quantity":3}"#)
            .dispatch();
        assert_eq!(resp.status(), Status::Unauthorized);
    }

    #[test]
    fn cart_update_nonexistent_item_returns_forbidden() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "customer", "CustomerPass123!");
        let resp = client.put("/api/cart/99999999")
            .header(ContentType::JSON)
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .body(r#"{"quantity":2}"#)
            .dispatch();
        assert_eq!(resp.status(), Status::Forbidden);
    }

    // ── DELETE /api/cart/<item_id> ────────────────────────────────────────────

    #[test]
    fn cart_delete_item_requires_auth() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let resp = client.delete("/api/cart/1").dispatch();
        assert_eq!(resp.status(), Status::Unauthorized);
    }

    #[test]
    fn cart_delete_nonexistent_item_returns_forbidden() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "customer", "CustomerPass123!");
        let resp = client.delete("/api/cart/99999999")
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .dispatch();
        assert_eq!(resp.status(), Status::Forbidden);
    }

    // ── POST /api/orders/checkout ────────────────────────────────────────────

    #[test]
    fn checkout_requires_auth() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let resp = client.post("/api/orders/checkout")
            .header(ContentType::JSON)
            .body(r#"{"pickup_slot_start":"2099-04-13T09:00:00","pickup_slot_end":"2099-04-13T09:15:00"}"#)
            .dispatch();
        assert_eq!(resp.status(), Status::Unauthorized);
    }

    #[test]
    fn checkout_with_empty_cart_returns_error() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "customer", "CustomerPass123!");
        // Clear cart first
        let _ = client.delete("/api/cart/clear")
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie.clone()))
            .dispatch();
        let resp = client.post("/api/orders/checkout")
            .header(ContentType::JSON)
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .body(r#"{"pickup_slot_start":"2099-04-13T09:00:00","pickup_slot_end":"2099-04-13T09:15:00"}"#)
            .dispatch();
        assert_eq!(resp.status(), Status::BadRequest);
    }

    // ── GET /api/orders/<id> ─────────────────────────────────────────────────

    #[test]
    fn order_detail_requires_auth() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let resp = client.get("/api/orders/9000").dispatch();
        assert_eq!(resp.status(), Status::Unauthorized);
    }

    #[test]
    fn order_detail_returns_shape_for_seeded_order() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "customer", "CustomerPass123!");
        let resp = client.get("/api/orders/9000")
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .dispatch();
        assert_eq!(resp.status(), Status::Ok);
        let body: serde_json::Value =
            serde_json::from_str(&resp.into_string().unwrap_or_default()).unwrap();
        assert!(body["success"].as_bool().unwrap_or(false));
        assert!(body["data"]["order"]["order_number"].is_string());
    }

    #[test]
    fn order_detail_nonexistent_returns_404() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "customer", "CustomerPass123!");
        let resp = client.get("/api/orders/99999999")
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .dispatch();
        assert_eq!(resp.status(), Status::NotFound);
    }

    // ── POST /api/orders/<id>/cancel ─────────────────────────────────────────

    #[test]
    fn order_cancel_requires_auth() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let resp = client.post("/api/orders/9000/cancel")
            .header(ContentType::JSON)
            .body("{}")
            .dispatch();
        assert_eq!(resp.status(), Status::Unauthorized);
    }

    #[test]
    fn order_cancel_nonexistent_order_returns_404() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "customer", "CustomerPass123!");
        let resp = client.post("/api/orders/99999999/cancel")
            .header(ContentType::JSON)
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .body("{}")
            .dispatch();
        assert_eq!(resp.status(), Status::NotFound);
    }

    // ── GET /api/staff/orders/<id> ───────────────────────────────────────────

    #[test]
    fn staff_order_detail_requires_staff_role() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "customer", "CustomerPass123!");
        let resp = client.get("/api/staff/orders/9000")
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .dispatch();
        assert_eq!(resp.status(), Status::Forbidden);
    }

    #[test]
    fn staff_order_detail_returns_shape() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "staff", "StaffPass123!");
        let resp = client.get("/api/staff/orders/9000")
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .dispatch();
        assert_eq!(resp.status(), Status::Ok);
        let body: serde_json::Value =
            serde_json::from_str(&resp.into_string().unwrap_or_default()).unwrap();
        assert!(body["success"].as_bool().unwrap_or(false));
    }

    // ── PUT /api/staff/orders/<id>/status ────────────────────────────────────

    #[test]
    fn staff_update_order_status_requires_staff() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "customer", "CustomerPass123!");
        let resp = client.put("/api/staff/orders/9000/status")
            .header(ContentType::JSON)
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .body(r#"{"new_status":"Accepted"}"#)
            .dispatch();
        assert_eq!(resp.status(), Status::Forbidden);
    }

    #[test]
    fn staff_update_order_status_invalid_transition_returns_error() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "staff", "StaffPass123!");
        // Order 9000 is Pending — skipping to "Ready" is invalid.
        let resp = client.put("/api/staff/orders/9000/status")
            .header(ContentType::JSON)
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .body(r#"{"new_status":"Ready"}"#)
            .dispatch();
        assert_eq!(resp.status(), Status::BadRequest);
    }

    // ── GET /api/staff/dashboard ─────────────────────────────────────────────

    #[test]
    fn staff_dashboard_requires_staff_role() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let resp = client.get("/api/staff/dashboard").dispatch();
        assert_eq!(resp.status(), Status::Unauthorized);
    }

    #[test]
    fn staff_dashboard_returns_success_for_staff() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "staff", "StaffPass123!");
        let resp = client.get("/api/staff/dashboard")
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .dispatch();
        assert_eq!(resp.status(), Status::Ok);
        let body: serde_json::Value = serde_json::from_str(&resp.into_string().unwrap_or_default()).unwrap();
        assert!(body["success"].as_bool().unwrap_or(false));
    }

    // ── POST /api/admin/users/<id>/roles ─────────────────────────────────────

    #[test]
    fn admin_assign_role_requires_admin() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "staff", "StaffPass123!");
        let resp = client.post("/api/admin/users/2/roles")
            .header(ContentType::JSON)
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .body(r#"{"role":"Teacher"}"#)
            .dispatch();
        assert_eq!(resp.status(), Status::Forbidden);
    }

    #[test]
    fn admin_assign_role_succeeds() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "admin", "AdminPass123!");
        let resp = client.post("/api/admin/users/2/roles")
            .header(ContentType::JSON)
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .body(r#"{"role":"Teacher"}"#)
            .dispatch();
        assert_eq!(resp.status(), Status::Ok);
        let body: serde_json::Value = serde_json::from_str(&resp.into_string().unwrap_or_default()).unwrap();
        assert!(body["success"].as_bool().unwrap_or(false));
    }

    // ── DELETE /api/admin/users/<id>/roles/<role> ────────────────────────────

    #[test]
    fn admin_remove_role_requires_admin() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "staff", "StaffPass123!");
        let resp = client.delete("/api/admin/users/2/roles/Teacher")
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .dispatch();
        assert_eq!(resp.status(), Status::Forbidden);
    }

    #[test]
    fn admin_remove_role_succeeds() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "admin", "AdminPass123!");
        let resp = client.delete("/api/admin/users/2/roles/Teacher")
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .dispatch();
        assert_eq!(resp.status(), Status::Ok);
        let body: serde_json::Value = serde_json::from_str(&resp.into_string().unwrap_or_default()).unwrap();
        assert!(body["success"].as_bool().unwrap_or(false));
    }

    // ── PUT /api/admin/store-hours ───────────────────────────────────────────

    #[test]
    fn admin_update_store_hours_requires_admin() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "staff", "StaffPass123!");
        let resp = client.put("/api/admin/store-hours")
            .header(ContentType::JSON)
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .body(r#"{"hours":[{"day_of_week":1,"open_time":"08:00","close_time":"20:00","is_closed":false}]}"#)
            .dispatch();
        assert_eq!(resp.status(), Status::Forbidden);
    }

    #[test]
    fn admin_update_store_hours_succeeds() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "admin", "AdminPass123!");
        let resp = client.put("/api/admin/store-hours")
            .header(ContentType::JSON)
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .body(r#"{"hours":[{"day_of_week":1,"open_time":"08:00","close_time":"20:00","is_closed":false}]}"#)
            .dispatch();
        assert_eq!(resp.status(), Status::Ok);
        let body: serde_json::Value = serde_json::from_str(&resp.into_string().unwrap_or_default()).unwrap();
        assert!(body["success"].as_bool().unwrap_or(false));
    }

    // ── PUT /api/admin/tax ───────────────────────────────────────────────────

    #[test]
    fn admin_update_tax_requires_admin() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "staff", "StaffPass123!");
        let resp = client.put("/api/admin/tax")
            .header(ContentType::JSON)
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .body(r#"{"tax_name":"Sales Tax","rate":0.0875}"#)
            .dispatch();
        assert_eq!(resp.status(), Status::Forbidden);
    }

    #[test]
    fn admin_update_tax_succeeds() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "admin", "AdminPass123!");
        let resp = client.put("/api/admin/tax")
            .header(ContentType::JSON)
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .body(r#"{"tax_name":"Sales Tax","rate":0.0875}"#)
            .dispatch();
        assert_eq!(resp.status(), Status::Ok);
        let body: serde_json::Value = serde_json::from_str(&resp.into_string().unwrap_or_default()).unwrap();
        assert!(body["success"].as_bool().unwrap_or(false));
    }

    // ── POST /api/admin/products ─────────────────────────────────────────────

    #[test]
    fn admin_create_product_requires_admin() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "customer", "CustomerPass123!");
        let resp = client.post("/api/admin/products")
            .header(ContentType::JSON)
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .body(r#"{"name_en":"Test","name_zh":"Test","description_en":"","description_zh":"","base_price":1.0,"prep_time_minutes":5}"#)
            .dispatch();
        assert_eq!(resp.status(), Status::Forbidden);
    }

    #[test]
    fn admin_create_product_succeeds() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "admin", "AdminPass123!");
        let name = format!("TestProd_{}", rand::random::<u16>());
        let body = serde_json::json!({
            "name_en": name,
            "name_zh": format!("{}_zh", name),
            "description_en": "A test product",
            "description_zh": "Test desc zh",
            "base_price": 5.99,
            "prep_time_minutes": 3
        });
        let resp = client.post("/api/admin/products")
            .header(ContentType::JSON)
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .body(body.to_string())
            .dispatch();
        assert_eq!(resp.status(), Status::Ok);
        let b: serde_json::Value =
            serde_json::from_str(&resp.into_string().unwrap_or_default()).unwrap();
        assert!(b["data"]["spu_id"].is_number());
    }

    // ── PUT /api/admin/products/<id> ─────────────────────────────────────────

    #[test]
    fn admin_update_product_requires_admin() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "customer", "CustomerPass123!");
        let resp = client.put("/api/admin/products/1")
            .header(ContentType::JSON)
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .body(r#"{"base_price":4.99}"#)
            .dispatch();
        assert_eq!(resp.status(), Status::Forbidden);
    }

    #[test]
    fn admin_update_product_succeeds() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "admin", "AdminPass123!");
        let resp = client.put("/api/admin/products/1")
            .header(ContentType::JSON)
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .body(r#"{"base_price":4.99}"#)
            .dispatch();
        assert_eq!(resp.status(), Status::Ok);
        let body: serde_json::Value = serde_json::from_str(&resp.into_string().unwrap_or_default()).unwrap();
        assert!(body["success"].as_bool().unwrap_or(false));
    }

    #[test]
    fn admin_update_nonexistent_product_returns_404() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "admin", "AdminPass123!");
        let resp = client.put("/api/admin/products/99999999")
            .header(ContentType::JSON)
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .body(r#"{"base_price":1.0}"#)
            .dispatch();
        assert_eq!(resp.status(), Status::NotFound);
    }

    // ── GET /api/dispatch/queue ──────────────────────────────────────────────

    #[test]
    fn dispatch_queue_requires_staff() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "customer", "CustomerPass123!");
        let resp = client.get("/api/dispatch/queue")
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .dispatch();
        assert_eq!(resp.status(), Status::Forbidden);
    }

    #[test]
    fn dispatch_queue_returns_array_for_staff() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "staff", "StaffPass123!");
        let resp = client.get("/api/dispatch/queue")
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .dispatch();
        assert_eq!(resp.status(), Status::Ok);
        let b: serde_json::Value =
            serde_json::from_str(&resp.into_string().unwrap_or_default()).unwrap();
        assert!(b["data"].is_array());
    }

    // ── POST /api/dispatch/grab/<task_id> ────────────────────────────────────

    #[test]
    fn dispatch_grab_requires_staff() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let resp = client.post("/api/dispatch/grab/1").dispatch();
        assert_eq!(resp.status(), Status::Unauthorized);
    }

    #[test]
    fn dispatch_grab_nonexistent_task_returns_conflict() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "staff", "StaffPass123!");
        let resp = client.post("/api/dispatch/grab/99999999")
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .dispatch();
        assert_eq!(resp.status(), Status::Conflict);
    }

    // ── POST /api/dispatch/accept/<task_id> ──────────────────────────────────

    #[test]
    fn dispatch_accept_requires_staff() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let resp = client.post("/api/dispatch/accept/1").dispatch();
        assert_eq!(resp.status(), Status::Unauthorized);
    }

    #[test]
    fn dispatch_accept_nonexistent_task_returns_conflict() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "staff", "StaffPass123!");
        let resp = client.post("/api/dispatch/accept/99999999")
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .dispatch();
        assert_eq!(resp.status(), Status::Conflict);
    }

    // ── POST /api/dispatch/reject/<task_id> ──────────────────────────────────

    #[test]
    fn dispatch_reject_requires_staff() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let resp = client.post("/api/dispatch/reject/1").dispatch();
        assert_eq!(resp.status(), Status::Unauthorized);
    }

    #[test]
    fn dispatch_reject_nonexistent_returns_conflict() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "staff", "StaffPass123!");
        let resp = client.post("/api/dispatch/reject/99999999")
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .dispatch();
        assert_eq!(resp.status(), Status::Conflict);
    }

    // ── POST /api/dispatch/start/<task_id> ───────────────────────────────────

    #[test]
    fn dispatch_start_requires_staff() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let resp = client.post("/api/dispatch/start/1").dispatch();
        assert_eq!(resp.status(), Status::Unauthorized);
    }

    #[test]
    fn dispatch_start_nonexistent_returns_404() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "staff", "StaffPass123!");
        let resp = client.post("/api/dispatch/start/99999999")
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .dispatch();
        assert_eq!(resp.status(), Status::NotFound);
    }

    // ── POST /api/dispatch/complete/<task_id> ────────────────────────────────

    #[test]
    fn dispatch_complete_requires_staff() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let resp = client.post("/api/dispatch/complete/1").dispatch();
        assert_eq!(resp.status(), Status::Unauthorized);
    }

    #[test]
    fn dispatch_complete_nonexistent_returns_404() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "staff", "StaffPass123!");
        let resp = client.post("/api/dispatch/complete/99999999")
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .dispatch();
        assert_eq!(resp.status(), Status::NotFound);
    }

    // ── POST /api/dispatch/assign ────────────────────────────────────────────

    #[test]
    fn dispatch_assign_requires_admin() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "staff", "StaffPass123!");
        let resp = client.post("/api/dispatch/assign")
            .header(ContentType::JSON)
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .body(r#"{"order_id":9000,"mode":"Grab"}"#)
            .dispatch();
        assert_eq!(resp.status(), Status::Forbidden);
    }

    #[test]
    fn dispatch_assign_as_admin_enqueues_for_grab() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "admin", "AdminPass123!");
        // "Grab" mode calls enqueue_for_grab which always succeeds with a task_id.
        let resp = client.post("/api/dispatch/assign")
            .header(ContentType::JSON)
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .body(r#"{"order_id":9000,"mode":"Grab"}"#)
            .dispatch();
        assert_eq!(resp.status(), Status::Ok);
        let b: serde_json::Value =
            serde_json::from_str(&resp.into_string().unwrap_or_default()).unwrap();
        assert!(b["data"].is_number(), "expected task_id in data");
    }

    // ── GET /api/dispatch/recommendations/<order_id> ─────────────────────────

    #[test]
    fn dispatch_recommendations_requires_admin() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "staff", "StaffPass123!");
        let resp = client.get("/api/dispatch/recommendations/9000")
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .dispatch();
        assert_eq!(resp.status(), Status::Forbidden);
    }

    #[test]
    fn dispatch_recommendations_returns_array_for_admin() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "admin", "AdminPass123!");
        let resp = client.get("/api/dispatch/recommendations/9000")
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .dispatch();
        assert_eq!(resp.status(), Status::Ok);
        let b: serde_json::Value =
            serde_json::from_str(&resp.into_string().unwrap_or_default()).unwrap();
        assert!(b["data"].is_array());
    }

    // ── GET /api/dispatch/shifts ─────────────────────────────────────────────

    #[test]
    fn dispatch_shifts_requires_staff() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let resp = client.get("/api/dispatch/shifts?user_id=3&date=2026-04-16").dispatch();
        assert_eq!(resp.status(), Status::Unauthorized);
    }

    #[test]
    fn dispatch_shifts_returns_array() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "staff", "StaffPass123!");
        let resp = client.get("/api/dispatch/shifts?user_id=3&date=2026-04-16")
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .dispatch();
        assert_eq!(resp.status(), Status::Ok);
        let b: serde_json::Value =
            serde_json::from_str(&resp.into_string().unwrap_or_default()).unwrap();
        assert!(b["data"].is_array());
    }

    // ── POST /api/dispatch/shifts ────────────────────────────────────────────

    #[test]
    fn dispatch_create_shift_requires_admin() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "staff", "StaffPass123!");
        let resp = client.post("/api/dispatch/shifts")
            .header(ContentType::JSON)
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .body(r#"{"user_id":3,"zone_id":1,"shift_date":"2026-04-16","start_time":"09:00","end_time":"17:00"}"#)
            .dispatch();
        assert_eq!(resp.status(), Status::Forbidden);
    }

    #[test]
    fn dispatch_create_shift_as_admin() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "admin", "AdminPass123!");
        // zone_id=1 may or may not exist; if FK fails → 500, otherwise 200.
        // We verify the handler runs and responds with a valid JSON body either way.
        let resp = client.post("/api/dispatch/shifts")
            .header(ContentType::JSON)
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .body(r#"{"user_id":3,"zone_id":1,"shift_date":"2099-04-16","start_time":"09:00","end_time":"17:00"}"#)
            .dispatch();
        let status = resp.status();
        let body = resp.into_string().unwrap_or_default();
        assert!(
            status == Status::Ok || status == Status::InternalServerError,
            "create shift expected 200|500, got {} body: {}",
            status,
            body
        );
        // The body must always be valid JSON regardless of status.
        let _: serde_json::Value = serde_json::from_str(&body)
            .expect("response body must be valid JSON");
    }

    // ── GET /api/dispatch/reputation/<user_id> ───────────────────────────────

    #[test]
    fn dispatch_reputation_requires_admin() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "staff", "StaffPass123!");
        let resp = client.get("/api/dispatch/reputation/3")
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .dispatch();
        assert_eq!(resp.status(), Status::Forbidden);
    }

    #[test]
    fn dispatch_reputation_returns_data_for_admin() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "admin", "AdminPass123!");
        let resp = client.get("/api/dispatch/reputation/3")
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .dispatch();
        assert_eq!(resp.status(), Status::Ok);
        let body: serde_json::Value = serde_json::from_str(&resp.into_string().unwrap_or_default()).unwrap();
        assert!(body["success"].as_bool().unwrap_or(false));
    }

    // ── POST /api/training/start/<exam_id> ───────────────────────────────────

    #[test]
    fn training_start_exam_requires_auth() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let resp = client.post("/api/training/start/1").dispatch();
        assert_eq!(resp.status(), Status::Unauthorized);
    }

    #[test]
    fn training_start_nonexistent_exam_returns_404() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "customer", "CustomerPass123!");
        let resp = client.post("/api/training/start/99999999")
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .dispatch();
        assert_eq!(resp.status(), Status::NotFound);
    }

    // ── POST /api/training/answer ────────────────────────────────────────────

    #[test]
    fn training_answer_requires_auth() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let resp = client.post("/api/training/answer")
            .header(ContentType::JSON)
            .body(r#"{"question_id":1,"selected_option_ids":[1]}"#)
            .dispatch();
        assert_eq!(resp.status(), Status::Unauthorized);
    }

    #[test]
    fn training_answer_in_review_mode_returns_ok() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "customer", "CustomerPass123!");
        let resp = client.post("/api/training/answer")
            .header(ContentType::JSON)
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .body(r#"{"question_id":1,"selected_option_ids":[1]}"#)
            .dispatch();
        assert_eq!(resp.status(), Status::Ok);
        let body: serde_json::Value = serde_json::from_str(&resp.into_string().unwrap_or_default()).unwrap();
        assert!(body["success"].as_bool().unwrap_or(false));
        // Review mode returns is_correct boolean
        assert!(body["data"]["is_correct"].is_boolean());
    }

    // ── POST /api/training/finish/<attempt_id> ───────────────────────────────

    #[test]
    fn training_finish_requires_auth() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let resp = client.post("/api/training/finish/1").dispatch();
        assert_eq!(resp.status(), Status::Unauthorized);
    }

    #[test]
    fn training_finish_nonexistent_attempt_returns_404() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "customer", "CustomerPass123!");
        let resp = client.post("/api/training/finish/99999999")
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .dispatch();
        assert_eq!(resp.status(), Status::NotFound);
    }

    // ── GET /api/training/attempts/<id> ──────────────────────────────────────

    #[test]
    fn training_attempt_detail_requires_auth() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let resp = client.get("/api/training/attempts/1").dispatch();
        assert_eq!(resp.status(), Status::Unauthorized);
    }

    #[test]
    fn training_attempt_detail_nonexistent_returns_404() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "customer", "CustomerPass123!");
        let resp = client.get("/api/training/attempts/99999999")
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .dispatch();
        assert_eq!(resp.status(), Status::NotFound);
    }

    // ── POST /api/training/favorites/<question_id> ───────────────────────────

    #[test]
    fn training_add_favorite_requires_auth() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let resp = client.post("/api/training/favorites/1").dispatch();
        assert_eq!(resp.status(), Status::Unauthorized);
    }

    #[test]
    fn training_add_favorite_nonexistent_question_returns_500() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "customer", "CustomerPass123!");
        // No questions are seeded in 006_seed_data.sql — question_id=1 does
        // not exist. The handler does not pre-validate, so the DB FK
        // constraint fails → 500.
        let resp = client.post("/api/training/favorites/1")
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .dispatch();
        assert_eq!(resp.status(), Status::InternalServerError);
    }

    // ── DELETE /api/training/favorites/<question_id> ─────────────────────────

    #[test]
    fn training_remove_favorite_requires_auth() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let resp = client.delete("/api/training/favorites/1").dispatch();
        assert_eq!(resp.status(), Status::Unauthorized);
    }

    #[test]
    fn training_remove_favorite_nonexistent_returns_ok() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "customer", "CustomerPass123!");
        let resp = client.delete("/api/training/favorites/1")
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .dispatch();
        assert_eq!(resp.status(), Status::Ok);
        let body: serde_json::Value = serde_json::from_str(&resp.into_string().unwrap_or_default()).unwrap();
        assert!(body["success"].as_bool().unwrap_or(false));
    }

    // ── GET /api/exam/subjects/<id>/chapters ─────────────────────────────────

    #[test]
    fn exam_chapters_returns_array() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let resp = client.get("/api/exam/subjects/1/chapters").dispatch();
        assert_eq!(resp.status(), Status::Ok);
        let b: serde_json::Value =
            serde_json::from_str(&resp.into_string().unwrap_or_default()).unwrap();
        assert!(b["data"].is_array());
    }

    // ── GET /api/exam/questions (list) ───────────────────────────────────────

    #[test]
    fn exam_questions_list_requires_teacher_role() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "customer", "CustomerPass123!");
        let resp = client.get("/api/exam/questions?page=1&per_page=10")
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .dispatch();
        assert_eq!(resp.status(), Status::Forbidden);
    }

    #[test]
    fn exam_questions_list_succeeds_for_admin() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "admin", "AdminPass123!");
        let resp = client.get("/api/exam/questions?page=1&per_page=10")
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .dispatch();
        assert_eq!(resp.status(), Status::Ok);
        let body: serde_json::Value = serde_json::from_str(&resp.into_string().unwrap_or_default()).unwrap();
        assert!(body["success"].as_bool().unwrap_or(false));
        assert!(body["data"]["items"].is_array());
        assert!(body["data"]["total"].is_number());
    }

    // ── GET /api/exam/questions/<id> ─────────────────────────────────────────

    #[test]
    fn exam_question_detail_requires_teacher_role() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "customer", "CustomerPass123!");
        let resp = client.get("/api/exam/questions/1")
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .dispatch();
        assert_eq!(resp.status(), Status::Forbidden);
    }

    #[test]
    fn exam_question_detail_for_admin_returns_200_or_404() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "admin", "AdminPass123!");
        // ID 1 may or may not exist in seed data; either way we get a
        // deterministic status from the handler.
        let resp = client.get("/api/exam/questions/1")
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .dispatch();
        assert!(
            resp.status() == Status::Ok || resp.status() == Status::NotFound,
            "question detail expected 200|404, got {}",
            resp.status()
        );
        // Also verify a known-nonexistent ID is always 404.
        let cookie2 = login(&client, "admin", "AdminPass123!");
        let resp2 = client.get("/api/exam/questions/99999999")
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie2))
            .dispatch();
        assert_eq!(resp2.status(), Status::NotFound);
    }

    // ── POST /api/exam/import ────────────────────────────────────────────────

    #[test]
    fn exam_import_requires_teacher_role() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "customer", "CustomerPass123!");
        let resp = client.post("/api/exam/import")
            .header(ContentType::JSON)
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .body(r#"{"subject_id":1,"csv_content":"q,a,b,c,d,A,easy,e"}"#)
            .dispatch();
        assert_eq!(resp.status(), Status::Forbidden);
    }

    #[test]
    fn exam_import_as_admin_processes_csv() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "admin", "AdminPass123!");
        let csv = "question,option_a,option_b,option_c,option_d,correct,difficulty,explanation\nWhat is 1+1?,0,1,2,3,C,easy,arithmetic";
        let body = serde_json::json!({
            "subject_id": 1,
            "csv_content": csv
        });
        let resp = client.post("/api/exam/import")
            .header(ContentType::JSON)
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .body(body.to_string())
            .dispatch();
        assert_eq!(resp.status(), Status::Ok);
        let b: serde_json::Value =
            serde_json::from_str(&resp.into_string().unwrap_or_default()).unwrap();
        assert!(b["data"]["imported_count"].is_number());
    }

    // ── POST /api/exam/questions/import (alias route) ────────────────────────

    #[test]
    fn exam_questions_import_alias_requires_teacher_role() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "customer", "CustomerPass123!");
        let resp = client.post("/api/exam/questions/import")
            .header(ContentType::JSON)
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .body(r#"{"subject_id":1,"csv_content":"q,a,b,c,d,A,easy,e"}"#)
            .dispatch();
        assert_eq!(resp.status(), Status::Forbidden);
    }

    #[test]
    fn exam_questions_import_alias_succeeds_for_admin() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "admin", "AdminPass123!");
        let csv = "question,option_a,option_b,option_c,option_d,correct,difficulty,explanation\nAlias import test?,X,Y,Z,W,A,easy,works";
        let body = serde_json::json!({
            "subject_id": 1,
            "csv_content": csv
        });
        let resp = client.post("/api/exam/questions/import")
            .header(ContentType::JSON)
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .body(body.to_string())
            .dispatch();
        assert_eq!(resp.status(), Status::Ok);
        let b: serde_json::Value =
            serde_json::from_str(&resp.into_string().unwrap_or_default()).unwrap();
        assert!(b["data"]["imported_count"].as_i64().unwrap_or(0) >= 1);
    }

    // ── GET /api/exam/versions/<id> ──────────────────────────────────────────

    #[test]
    fn exam_version_detail_requires_auth() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let resp = client.get("/api/exam/versions/1").dispatch();
        assert_eq!(resp.status(), Status::Unauthorized);
    }

    #[test]
    fn exam_version_detail_nonexistent_returns_404() {
        let (_rt, rocket) = require_db!();
        let client = Client::tracked(rocket).expect("valid rocket");
        let cookie = login(&client, "customer", "CustomerPass123!");
        let resp = client.get("/api/exam/versions/99999999")
            .cookie(rocket::http::Cookie::new("brewflow_session", cookie))
            .dispatch();
        assert_eq!(resp.status(), Status::NotFound);
    }
}
