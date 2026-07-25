use std::sync::Arc;
use std::time::Instant;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::{DateTime, TimeZone, Utc};
use collector_core::{InterfaceCollector, InterfaceStats};
use serde::{Deserialize, Serialize};
use store::{HistoryStore, RingBufferStore};
use tokio::sync::broadcast;

use collector_core::Snapshot;

pub struct AppState {
    pub collector: Arc<dyn InterfaceCollector>,
    pub ring_buffer: Arc<RingBufferStore>,
    pub history: Arc<dyn HistoryStore>,
    pub tx: broadcast::Sender<Snapshot>,
    pub started_at: Instant,
    pub poll_interval_ms: u64,
}

#[derive(Serialize)]
pub struct StatusResponse {
    pub uptime_seconds: u64,
    pub collector_backend: &'static str,
    pub poll_interval_ms: u64,
    pub interface_count: usize,
}

pub async fn get_status(State(state): State<Arc<AppState>>) -> Json<StatusResponse> {
    let interfaces = state.ring_buffer.latest_all();
    Json(StatusResponse {
        uptime_seconds: state.started_at.elapsed().as_secs(),
        collector_backend: state.collector.platform_name(),
        poll_interval_ms: state.poll_interval_ms,
        interface_count: interfaces.len(),
    })
}

pub async fn get_interfaces(State(state): State<Arc<AppState>>) -> Json<Snapshot> {
    Json(Snapshot {
        interfaces: state.ring_buffer.latest_all(),
        taken_at: Utc::now(),
    })
}

pub async fn get_interface(
    State(state): State<Arc<AppState>>,
    Path(index): Path<u32>,
) -> Result<Json<InterfaceStats>, (StatusCode, String)> {
    state
        .ring_buffer
        .latest_all()
        .into_iter()
        .find(|i| i.index == index)
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, format!("interface {index} not found")))
}

#[derive(Deserialize)]
pub struct HistoryQuery {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub limit: Option<u32>,
}

pub async fn get_interface_history(
    State(state): State<Arc<AppState>>,
    Path(index): Path<u32>,
    Query(query): Query<HistoryQuery>,
) -> Result<Json<Vec<InterfaceStats>>, (StatusCode, String)> {
    let from = query.from.unwrap_or_else(|| {
        Utc.timestamp_opt(0, 0).single().unwrap_or_else(Utc::now)
    });
    let to = query.to.unwrap_or_else(Utc::now);
    let limit = query.limit.unwrap_or(500);

    state
        .history
        .query_range(index, from, to, limit)
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

#[derive(Serialize)]
pub struct VersionResponse {
    pub name: &'static str,
    pub version: &'static str,
}

pub async fn get_version() -> Json<VersionResponse> {
    Json(VersionResponse {
        name: "network-observatoryd",
        version: env!("CARGO_PKG_VERSION"),
    })
}
