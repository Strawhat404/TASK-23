use rocket::{get, post, put, routes};
use rocket::http::{Cookie, CookieJar, SameSite, Status};
use rocket::serde::json::Json;
use rocket::State;
use sqlx::MySqlPool;

use shared::dto::{ApiResponse, LoginRequest, LoginResponse, UserInfo};
use crate::middleware::auth_guard::AuthenticatedUser;
use crate::services::session::SessionConfig;

// ---------------------------------------------------------------------------
// Request / response helpers
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    pub display_name: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct UpdateLocaleRequest {
    pub locale: String,
}

// ---------------------------------------------------------------------------
// Routes
// ---------------------------------------------------------------------------

#[post("/login", data = "<body>")]
pub async fn login(
    pool: &State<MySqlPool>,
    session_config: &State<SessionConfig>,
    cookies: &CookieJar<'_>,
    body: Json<LoginRequest>,
) -> Result<Json<ApiResponse<LoginResponse>>, (Status, Json<ApiResponse<()>>)> {
    let user = crate::db::users::find_by_username(pool.inner(), &body.username)
        .await
        .ok_or_else(|| {
            (
                Status::Unauthorized,
                Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid username or password".into()),
                }),
            )
        })?;

    let valid = bcrypt::verify(&body.password, &user.password_hash).unwrap_or(false);
    if !valid {
        return Err((
            Status::Unauthorized,
            Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Invalid username or password".into()),
            }),
        ));
    }

    let roles = crate::db::users::get_user_roles(pool.inner(), user.id).await;

    // Create session in DB
    let session_id = crate::services::session::create_session_id();
    let _ = crate::db::sessions::create_session(pool.inner(), &session_id, user.id, None, None)
        .await;

    // Sign the session cookie and set it on the response (for browsers).
    let signed_cookie_value = crate::services::session::sign_cookie(session_config.inner(), &session_id);
    let cookie = Cookie::build(("brewflow_session", signed_cookie_value.clone()))
        .http_only(true)
        .same_site(SameSite::Strict)
        .secure(true)
        .path("/")
        .max_age(rocket::time::Duration::seconds(
            session_config.idle_timeout_secs as i64,
        ));
    cookies.add(cookie);

    let info = UserInfo {
        id: user.id,
        username: user.username,
        display_name: user.display_name,
        roles,
        preferred_locale: user.preferred_locale,
    };

    // Return the signed cookie value in the body so that WASM/API clients that
    // cannot rely on the browser cookie jar can attach it manually.
    Ok(Json(ApiResponse {
        success: true,
        data: Some(LoginResponse { session_cookie: signed_cookie_value, user: info }),
        error: None,
    }))
}

#[post("/register", data = "<body>")]
pub async fn register(
    pool: &State<MySqlPool>,
    body: Json<RegisterRequest>,
) -> Result<Json<ApiResponse<UserInfo>>, (Status, Json<ApiResponse<()>>)> {
    // Validate password policy
    if let Err(violations) = crate::services::auth::validate_password(&body.password) {
        return Err((
            Status::BadRequest,
            Json(ApiResponse {
                success: false,
                data: None,
                error: Some(violations.join("; ")),
            }),
        ));
    }

    // Check if username already exists
    if crate::db::users::find_by_username(pool.inner(), &body.username)
        .await
        .is_some()
    {
        return Err((
            Status::Conflict,
            Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Username already exists".into()),
            }),
        ));
    }

    let hash = bcrypt::hash(&body.password, bcrypt::DEFAULT_COST).map_err(|_| {
        (
            Status::InternalServerError,
            Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Failed to hash password".into()),
            }),
        )
    })?;

    let user_id = crate::db::users::create_user(
        pool.inner(),
        &body.username,
        &hash,
        body.display_name.as_deref(),
        body.email.as_deref(),
        "en",
    )
    .await
    .map_err(|_| {
        (
            Status::InternalServerError,
            Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Failed to create user".into()),
            }),
        )
    })?;

    // Public registration always assigns Customer role — privileged roles
    // must be granted by an admin through a separate workflow.
    let _ = crate::db::users::assign_role(pool.inner(), user_id, "Customer").await;
    let roles = crate::db::users::get_user_roles(pool.inner(), user_id).await;

    Ok(Json(ApiResponse {
        success: true,
        data: Some(UserInfo {
            id: user_id,
            username: body.username.clone(),
            display_name: body.display_name.clone(),
            roles,
            preferred_locale: "en".into(),
        }),
        error: None,
    }))
}

#[post("/logout")]
pub async fn logout(
    pool: &State<MySqlPool>,
    session_config: &State<SessionConfig>,
    cookies: &CookieJar<'_>,
) -> Json<ApiResponse<()>> {
    if let Some(cookie) = cookies.get("brewflow_session") {
        if let Some(session_id) =
            crate::services::session::verify_cookie(session_config.inner(), cookie.value())
        {
            let _ = crate::db::sessions::delete_session(pool.inner(), &session_id).await;
        }
    }

    cookies.remove(Cookie::build("brewflow_session").path("/"));

    Json(ApiResponse {
        success: true,
        data: None,
        error: None,
    })
}

#[get("/me")]
pub async fn me(
    pool: &State<MySqlPool>,
    user: AuthenticatedUser,
) -> Result<Json<ApiResponse<UserInfo>>, (Status, Json<ApiResponse<()>>)> {
    let db_user = crate::db::users::find_by_id(pool.inner(), user.claims.sub)
        .await
        .ok_or_else(|| {
            (
                Status::NotFound,
                Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("User not found".into()),
                }),
            )
        })?;

    let roles = crate::db::users::get_user_roles(pool.inner(), db_user.id).await;

    Ok(Json(ApiResponse {
        success: true,
        data: Some(UserInfo {
            id: db_user.id,
            username: db_user.username,
            display_name: db_user.display_name,
            roles,
            preferred_locale: db_user.preferred_locale,
        }),
        error: None,
    }))
}

#[put("/locale", data = "<body>")]
pub async fn update_locale(
    pool: &State<MySqlPool>,
    user: AuthenticatedUser,
    body: Json<UpdateLocaleRequest>,
) -> Result<Json<ApiResponse<()>>, (Status, Json<ApiResponse<()>>)> {
    crate::db::users::update_locale(pool.inner(), user.claims.sub, &body.locale)
        .await
        .map_err(|_| {
            (
                Status::InternalServerError,
                Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Failed to update locale".into()),
                }),
            )
        })?;

    Ok(Json(ApiResponse {
        success: true,
        data: None,
        error: None,
    }))
}

pub fn routes() -> Vec<rocket::Route> {
    routes![login, register, logout, me, update_locale]
}
