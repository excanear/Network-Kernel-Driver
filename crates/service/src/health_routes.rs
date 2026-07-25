use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use health::HealthScore;

use crate::routes::AppState;

pub async fn get_health_all(State(state): State<Arc<AppState>>) -> Json<Vec<HealthScore>> {
    let interfaces = state.ring_buffer.latest_all();
    let scores = interfaces
        .into_iter()
        .map(|iface| {
            let history = state.ring_buffer.recent(iface.index);
            health::compute_health(iface.index, &history)
        })
        .collect();
    Json(scores)
}

pub async fn get_health_one(
    State(state): State<Arc<AppState>>,
    Path(index): Path<u32>,
) -> Result<Json<HealthScore>, (StatusCode, String)> {
    let history = state.ring_buffer.recent(index);
    if history.is_empty() {
        return Err((StatusCode::NOT_FOUND, format!("no samples for interface {index}")));
    }
    Ok(Json(health::compute_health(index, &history)))
}
