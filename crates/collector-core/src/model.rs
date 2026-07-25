use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperStatus {
    Up,
    Down,
    Testing,
    Unknown,
    Dormant,
    NotPresent,
    LowerLayerDown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Duplex {
    Half,
    Full,
}

/// Where a given `InterfaceStats` sample was produced from. `KernelDriver` is
/// reserved for Phase 2 (see docs/phase2-kernel-driver-design.md) so the data
/// model does not need to change when the real kernel-mode backend lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CollectorSource {
    WindowsIpHelper,
    LinuxProcSys,
    KernelDriver,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InterfaceStats {
    pub index: u32,
    pub name: String,
    pub description: String,
    pub mac_address: String,
    pub mtu: u32,
    pub oper_status: OperStatus,
    pub link_speed_bps: Option<u64>,
    pub duplex: Option<Duplex>,
    pub if_type: String,
    pub ipv4_addresses: Vec<String>,
    pub ipv6_addresses: Vec<String>,

    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_packets: u64,
    pub tx_packets: u64,
    pub rx_errors: u64,
    pub tx_errors: u64,
    pub rx_drops: u64,
    pub tx_drops: u64,
    pub rx_broadcast_packets: Option<u64>,
    pub rx_multicast_packets: Option<u64>,

    pub timestamp: DateTime<Utc>,
    pub collector_source: CollectorSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub interfaces: Vec<InterfaceStats>,
    pub taken_at: DateTime<Utc>,
}
