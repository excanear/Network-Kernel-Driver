use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use chrono::Utc;
use reports::{ReportFormat, ReportInput};
use serde::Deserialize;

use crate::routes::AppState;

#[derive(Deserialize)]
pub struct ReportQuery {
    pub format: Option<String>,
}

pub async fn get_report(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ReportQuery>,
) -> Result<Response, (StatusCode, String)> {
    let format_str = query.format.unwrap_or_else(|| "html".to_string());
    let format = ReportFormat::from_str(&format_str)
        .ok_or((StatusCode::BAD_REQUEST, format!("unknown report format '{format_str}'")))?;

    let interfaces = state.ring_buffer.latest_all();
    let health = interfaces
        .iter()
        .map(|i| health::compute_health(i.index, &state.ring_buffer.recent(i.index)))
        .collect();
    let alerts = state
        .alert_store
        .list_recent(200)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let input = ReportInput {
        generated_at: Utc::now(),
        interfaces,
        health,
        alerts,
    };

    let bytes = reports::generate(format, &input)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let filename = format!("network-observatory-report.{}", format.extension());
    Ok((
        [
            (header::CONTENT_TYPE, format.content_type().to_string()),
            (header::CONTENT_DISPOSITION, format!("attachment; filename=\"{filename}\"")),
        ],
        bytes,
    )
        .into_response())
}
