mod alert_routes;
mod auth_middleware;
mod auth_routes;
mod grpc;
mod health_routes;
mod plugin_routes;
mod poller;
mod report_routes;
mod routes;
mod topology_routes;
mod ws;
#[cfg(windows)]
mod win_service;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::http::{HeaderValue, Method};
use axum::routing::{get, post};
use axum::Router;
use collector_core::InterfaceCollector;
use store::{AuthStore, RingBufferStore, SqliteAlertStore, SqliteAuthStore, SqliteHistoryStore};
use tokio::sync::{broadcast, Mutex as AsyncMutex};
use tower_http::cors::{AllowHeaders, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::{info, warn};

use routes::AppState;

#[cfg(windows)]
fn build_collector() -> Arc<dyn InterfaceCollector> {
    Arc::new(collector_windows::WindowsCollector::new())
}

#[cfg(unix)]
fn build_collector() -> Arc<dyn InterfaceCollector> {
    Arc::new(collector_linux::LinuxCollector::new())
}

/// Grafana/Kibana-style bootstrap: if no user exists yet, create a default
/// admin with a random password logged once. Avoids a manual seeding step
/// while never storing/printing a fixed default credential.
fn bootstrap_admin(auth_store: &dyn AuthStore) -> anyhow::Result<()> {
    if auth_store.user_count()? > 0 {
        return Ok(());
    }
    use rand::Rng;
    let password: String = rand::thread_rng()
        .sample_iter(rand::distributions::Alphanumeric)
        .take(16)
        .map(char::from)
        .collect();
    auth_store.create_user("admin", &password)?;
    warn!("bootstrapped default admin user — username=admin password={password} (change this; see docs/roadmap.md Phase H)");
    Ok(())
}

fn init_tracing() {
    // Avoid double-initialization when running under the Windows service
    // dispatcher, which calls into this module from a different thread.
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .try_init();
}

/// Core server body: shared by the normal console entrypoint (`main`) and the
/// Windows Service entrypoint (`win_service::run`, Phase I). Runs until the
/// HTTP or gRPC listener fails — the Windows Service wrapper terminates the
/// process itself in response to SCM stop control, rather than this function
/// returning cleanly (see docs/roadmap.md Phase I for the tradeoff).
pub async fn run_server() -> anyhow::Result<()> {
    let collector = build_collector();
    let ring_buffer = Arc::new(RingBufferStore::new());
    let history: Arc<dyn store::HistoryStore> =
        Arc::new(SqliteHistoryStore::open("network-observatory.db")?);
    let alert_store: Arc<dyn store::AlertStore> =
        Arc::new(SqliteAlertStore::open("network-observatory.db")?);
    let auth_store: Arc<dyn store::AuthStore> =
        Arc::new(SqliteAuthStore::open("network-observatory.db")?);
    bootstrap_admin(auth_store.as_ref())?;

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
        auth_store,
        tx,
        started_at: Instant::now(),
        poll_interval_ms: poll_interval.as_millis() as u64,
    });

    let state_for_grpc = state.clone();

    let web_origin = std::env::var("NETOBS_WEB_ORIGIN").unwrap_or_else(|_| "http://localhost:3000".to_string());
    let cors = CorsLayer::new()
        .allow_origin(web_origin.parse::<HeaderValue>().expect("valid NETOBS_WEB_ORIGIN"))
        .allow_methods([Method::GET, Method::POST])
        .allow_headers(AllowHeaders::list([axum::http::header::CONTENT_TYPE]))
        .allow_credentials(true);

    let auth_required = std::env::var("NETOBS_AUTH_REQUIRED").map(|v| v == "true").unwrap_or(false);
    info!("auth enforcement on protected REST routes: {auth_required}");

    let public_routes = Router::new()
        .route("/api/v1/auth/login", post(auth_routes::login))
        .route("/api/v1/auth/logout", post(auth_routes::logout))
        .route("/api/v1/auth/me", get(auth_routes::me))
        .route("/api/v1/version", get(routes::get_version));

    let mut protected_routes = Router::new()
        .route("/api/v1/status", get(routes::get_status))
        .route("/api/v1/interfaces", get(routes::get_interfaces))
        .route("/api/v1/interfaces/:index", get(routes::get_interface))
        .route(
            "/api/v1/interfaces/:index/history",
            get(routes::get_interface_history),
        )
        .route("/api/v1/ws/interfaces", get(ws::ws_interfaces))
        .route("/api/v1/health", get(health_routes::get_health_all))
        .route("/api/v1/health/:index", get(health_routes::get_health_one))
        .route("/api/v1/alerts", get(alert_routes::get_active_alerts))
        .route("/api/v1/alerts/recent", get(alert_routes::get_recent_alerts))
        .route("/api/v1/topology", get(topology_routes::get_topology))
        .route("/api/v1/reports", get(report_routes::get_report))
        .route("/api/v1/plugins", get(plugin_routes::list_plugins))
        .route("/api/v1/plugins/:name/run", get(plugin_routes::run_plugin));

    if auth_required {
        protected_routes = protected_routes.route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth_middleware::require_session,
        ));
    }

    let app = public_routes
        .merge(protected_routes)
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

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    #[cfg(windows)]
    if args.iter().any(|a| a == "--service") {
        // Handed off to the Service Control Manager dispatcher; does not
        // return until the service stops. Tracing is initialized inside
        // win_service::run once the service thread is running.
        return win_service::run();
    }

    let _ = &args;
    init_tracing();
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(run_server())
}
