mod alert_routes;
mod grpc;
mod health_routes;
mod poller;
mod routes;
mod topology_routes;
mod ws;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::routing::get;
use axum::Router;
use collector_core::InterfaceCollector;
use store::{RingBufferStore, SqliteAlertStore, SqliteHistoryStore};
use tokio::sync::{broadcast, Mutex as AsyncMutex};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::info;

use routes::AppState;

#[cfg(windows)]
fn build_collector() -> Arc<dyn InterfaceCollector> {
    Arc::new(collector_windows::WindowsCollector::new())
}

#[cfg(unix)]
fn build_collector() -> Arc<dyn InterfaceCollector> {
    Arc::new(collector_linux::LinuxCollector::new())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let collector = build_collector();
    let ring_buffer = Arc::new(RingBufferStore::new());
    let history: Arc<dyn store::HistoryStore> =
        Arc::new(SqliteHistoryStore::open("network-observatory.db")?);
    let alert_store: Arc<dyn store::AlertStore> =
        Arc::new(SqliteAlertStore::open("network-observatory.db")?);
    let alert_engine = Arc::new(AsyncMutex::new(alerts::AlertEngine::new(
        alerts::AlertRuleConfig::default(),
    )));
    let (tx, _rx) = broadcast::channel(64);

    let poll_interval = Duration::from_secs(1);
    let persist_every_n_ticks = 10; // ~10s downsample, see docs/phase1-slice-design.md

    poller::Poller {
        collector: collector.clone(),
        ring_buffer: ring_buffer.clone(),
        history: history.clone(),
        alert_store: alert_store.clone(),
        alert_engine: alert_engine.clone(),
        tx: tx.clone(),
        poll_interval,
        persist_every_n_ticks,
    }
    .spawn();

    let state = Arc::new(AppState {
        collector,
        ring_buffer,
        history,
        alert_store,
        tx,
        started_at: Instant::now(),
        poll_interval_ms: poll_interval.as_millis() as u64,
    });

    let state_for_grpc = state.clone();

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/api/v1/status", get(routes::get_status))
        .route("/api/v1/interfaces", get(routes::get_interfaces))
        .route("/api/v1/interfaces/:index", get(routes::get_interface))
        .route(
            "/api/v1/interfaces/:index/history",
            get(routes::get_interface_history),
        )
        .route("/api/v1/version", get(routes::get_version))
        .route("/api/v1/ws/interfaces", get(ws::ws_interfaces))
        .route("/api/v1/health", get(health_routes::get_health_all))
        .route("/api/v1/health/:index", get(health_routes::get_health_one))
        .route("/api/v1/alerts", get(alert_routes::get_active_alerts))
        .route("/api/v1/alerts/recent", get(alert_routes::get_recent_alerts))
        .route("/api/v1/topology", get(topology_routes::get_topology))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 7878));
    let grpc_addr = SocketAddr::from(([127, 0, 0, 1], 50051));

    info!("network-observatoryd listening on http://{addr} (REST/WS) and grpc://{grpc_addr}");

    let http_server = async {
        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;
        anyhow::Ok(())
    };

    let grpc_server = async {
        tonic::transport::Server::builder()
            .add_service(grpc::build_server(state_for_grpc))
            .serve(grpc_addr)
            .await?;
        anyhow::Ok(())
    };

    tokio::try_join!(http_server, grpc_server)?;

    Ok(())
}
