use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use serde::{Deserialize, Serialize};

use crate::routes::AppState;

pub const SESSION_COOKIE: &str = "netobs_session";

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct UserResponse {
    pub username: String,
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    Json(req): Json<LoginRequest>,
) -> Result<(CookieJar, Json<UserResponse>), (StatusCode, String)> {
    let user = state
        .auth_store
        .verify_login(&req.username, &req.password)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::UNAUTHORIZED, "invalid username or password".to_string()))?;

    let token = state
        .auth_store
        .create_session(user.id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let cookie = Cookie::build((SESSION_COOKIE, token))
        .http_only(true)
        .same_site(SameSite::Lax)
        .path("/")
        .build();

    Ok((jar.add(cookie), Json(UserResponse { username: user.username })))
}

pub async fn logout(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
) -> (CookieJar, StatusCode) {
    if let Some(cookie) = jar.get(SESSION_COOKIE) {
        let _ = state.auth_store.delete_session(cookie.value());
    }
    (jar.remove(Cookie::from(SESSION_COOKIE)), StatusCode::NO_CONTENT)
}

pub async fn me(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
) -> Result<Json<UserResponse>, StatusCode> {
    let token = jar.get(SESSION_COOKIE).map(|c| c.value().to_string()).ok_or(StatusCode::UNAUTHORIZED)?;
    let user = state
        .auth_store
        .validate_session(&token)
        .ok()
        .flatten()
        .ok_or(StatusCode::UNAUTHORIZED)?;
    Ok(Json(UserResponse { username: user.username }))
}
