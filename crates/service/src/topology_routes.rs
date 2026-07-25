use std::sync::Arc;

use axum::extract::State;
use axum::Json;
use topology::{GatewayInfo, TopologyGraph};

use crate::routes::AppState;

#[cfg(windows)]
fn discover_network_config() -> (Vec<(u32, String)>, Vec<String>) {
    collector_windows::discover_network_config()
}

#[cfg(unix)]
fn discover_network_config() -> (Vec<(u32, String)>, Vec<String>) {
    collector_linux::discover_network_config()
}

pub async fn get_topology(State(state): State<Arc<AppState>>) -> Json<TopologyGraph> {
    let interfaces = state.ring_buffer.latest_all();
    let (gateway_pairs, dns_servers) = discover_network_config();
    let gateways: Vec<GatewayInfo> = gateway_pairs
        .into_iter()
        .map(|(if_index, address)| GatewayInfo { if_index, address })
        .collect();

    Json(topology::build_topology(&interfaces, &gateways, &dns_servers))
}
