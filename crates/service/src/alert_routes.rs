use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use ::alerts::Alert;
use serde::Deserialize;

use crate::routes::AppState;

#[derive(Deserialize)]
pub struct RecentQuery {
    pub limit: Option<u32>,
}

pub async fn get_active_alerts(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<Alert>>, (StatusCode, String)> {
    state
        .alert_store
        .list_active()
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

pub async fn get_recent_alerts(
    State(state): State<Arc<AppState>>,
    Query(query): Query<RecentQuery>,
) -> Result<Json<Vec<Alert>>, (StatusCode, String)> {
    state
        .alert_store
        .list_recent(query.limit.unwrap_or(100))
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}
