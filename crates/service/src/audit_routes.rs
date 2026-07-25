use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use store::AuditEvent;

use crate::routes::AppState;

#[derive(Deserialize)]
pub struct AuditQuery {
    pub limit: Option<u32>,
}

pub async fn get_audit_log(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AuditQuery>,
) -> Result<Json<Vec<AuditEvent>>, (StatusCode, String)> {
    state
        .audit_store
        .list_recent(query.limit.unwrap_or(200))
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}
