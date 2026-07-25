use std::pin::Pin;
use std::sync::Arc;

use chrono::{TimeZone, Utc};
use collector_core::{CollectorSource, Duplex, InterfaceStats, OperStatus, Snapshot};
use futures::Stream;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use tonic::{Request, Response, Status};

use crate::routes::AppState;

pub mod pb {
    tonic::include_proto!("network_observatory.v1");
}

use pb::interface_service_server::{InterfaceService, InterfaceServiceServer};
use pb::{HistoryRequest, HistoryResponse, Snapshot as PbSnapshot, SnapshotRequest};

fn oper_status_str(s: OperStatus) -> String {
    match s {
        OperStatus::Up => "Up",
        OperStatus::Down => "Down",
        OperStatus::Testing => "Testing",
        OperStatus::Unknown => "Unknown",
        OperStatus::Dormant => "Dormant",
        OperStatus::NotPresent => "NotPresent",
        OperStatus::LowerLayerDown => "LowerLayerDown",
    }
    .to_string()
}

fn duplex_str(d: Option<Duplex>) -> Option<String> {
    d.map(|d| match d {
        Duplex::Full => "Full".to_string(),
        Duplex::Half => "Half".to_string(),
    })
}

fn collector_source_str(s: CollectorSource) -> String {
    match s {
        CollectorSource::WindowsIpHelper => "WindowsIpHelper",
        CollectorSource::LinuxProcSys => "LinuxProcSys",
        CollectorSource::KernelDriver => "KernelDriver",
    }
    .to_string()
}

fn to_pb_stats(s: &InterfaceStats) -> pb::InterfaceStats {
    pb::InterfaceStats {
        index: s.index,
        name: s.name.clone(),
        description: s.description.clone(),
        mac_address: s.mac_address.clone(),
        mtu: s.mtu,
        oper_status: oper_status_str(s.oper_status),
        link_speed_bps: s.link_speed_bps,
        duplex: duplex_str(s.duplex),
        if_type: s.if_type.clone(),
        ipv4_addresses: s.ipv4_addresses.clone(),
        ipv6_addresses: s.ipv6_addresses.clone(),
        rx_bytes: s.rx_bytes,
        tx_bytes: s.tx_bytes,
        rx_packets: s.rx_packets,
        tx_packets: s.tx_packets,
        rx_errors: s.rx_errors,
        tx_errors: s.tx_errors,
        rx_drops: s.rx_drops,
        tx_drops: s.tx_drops,
        rx_broadcast_packets: s.rx_broadcast_packets,
        rx_multicast_packets: s.rx_multicast_packets,
        timestamp_unix_ms: s.timestamp.timestamp_millis(),
        collector_source: collector_source_str(s.collector_source),
    }
}

fn to_pb_snapshot(s: &Snapshot) -> PbSnapshot {
    PbSnapshot {
        interfaces: s.interfaces.iter().map(to_pb_stats).collect(),
        taken_at_unix_ms: s.taken_at.timestamp_millis(),
    }
}

pub struct InterfaceGrpcService {
    state: Arc<AppState>,
}

#[tonic::async_trait]
impl InterfaceService for InterfaceGrpcService {
    async fn get_snapshot(
        &self,
        _request: Request<SnapshotRequest>,
    ) -> Result<Response<PbSnapshot>, Status> {
        let snapshot = Snapshot {
            interfaces: self.state.ring_buffer.latest_all(),
            taken_at: Utc::now(),
        };
        Ok(Response::new(to_pb_snapshot(&snapshot)))
    }

    type StreamSnapshotsStream =
        Pin<Box<dyn Stream<Item = Result<PbSnapshot, Status>> + Send + 'static>>;

    async fn stream_snapshots(
        &self,
        _request: Request<SnapshotRequest>,
    ) -> Result<Response<Self::StreamSnapshotsStream>, Status> {
        let rx = self.state.tx.subscribe();
        let stream = BroadcastStream::new(rx).filter_map(|item| match item {
            Ok(snapshot) => Some(Ok(to_pb_snapshot(&snapshot))),
            Err(_lagged) => None,
        });
        Ok(Response::new(Box::pin(stream)))
    }

    async fn get_history(
        &self,
        request: Request<HistoryRequest>,
    ) -> Result<Response<HistoryResponse>, Status> {
        let req = request.into_inner();
        let from = Utc
            .timestamp_millis_opt(req.from_unix_ms)
            .single()
            .unwrap_or_else(|| Utc.timestamp_opt(0, 0).single().unwrap());
        let to = Utc
            .timestamp_millis_opt(req.to_unix_ms)
            .single()
            .unwrap_or_else(Utc::now);
        let limit = if req.limit == 0 { 100 } else { req.limit };

        let samples = self
            .state
            .history
            .query_range(req.if_index, from, to, limit)
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(HistoryResponse {
            samples: samples.iter().map(to_pb_stats).collect(),
        }))
    }
}

pub fn build_server(state: Arc<AppState>) -> InterfaceServiceServer<InterfaceGrpcService> {
    InterfaceServiceServer::new(InterfaceGrpcService { state })
}
