use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::Response;
use axum_extra::extract::cookie::CookieJar;

use crate::auth_routes::SESSION_COOKIE;
use crate::routes::AppState;

/// Gate applied to protected routes when `NETOBS_AUTH_REQUIRED=true`. Off by
/// default so the CLI/gRPC/existing REST consumers built in earlier phases
/// keep working without retrofitting session handling everywhere; operators
/// who want the multi-user web login enforced end-to-end opt in via env var.
/// See docs/roadmap.md (Phase H) for the rationale.
pub async fn require_session(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let token = jar.get(SESSION_COOKIE).map(|c| c.value().to_string()).ok_or(StatusCode::UNAUTHORIZED)?;
    state
        .auth_store
        .validate_session(&token)
        .ok()
        .flatten()
        .ok_or(StatusCode::UNAUTHORIZED)?;
    Ok(next.run(request).await)
}
