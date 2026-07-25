export type OperStatus =
  | "Up"
  | "Down"
  | "Testing"
  | "Unknown"
  | "Dormant"
  | "NotPresent"
  | "LowerLayerDown";

export type Duplex = "Half" | "Full";

export type CollectorSource = "WindowsIpHelper" | "LinuxProcSys" | "KernelDriver";

export interface InterfaceStats {
  index: number;
  name: string;
  description: string;
  mac_address: string;
  mtu: number;
  oper_status: OperStatus;
  link_speed_bps: number | null;
  duplex: Duplex | null;
  if_type: string;
  ipv4_addresses: string[];
  ipv6_addresses: string[];

  rx_bytes: number;
  tx_bytes: number;
  rx_packets: number;
  tx_packets: number;
  rx_errors: number;
  tx_errors: number;
  rx_drops: number;
  tx_drops: number;
  rx_broadcast_packets: number | null;
  rx_multicast_packets: number | null;

  timestamp: string;
  collector_source: CollectorSource;
}

export interface Snapshot {
  interfaces: InterfaceStats[];
  taken_at: string;
}

export interface StatusResponse {
  uptime_seconds: number;
  collector_backend: string;
  poll_interval_ms: number;
  interface_count: number;
}

export type NodeType =
  | "Host"
  | "Nic"
  | "Gateway"
  | "Dns"
  | "Vpn"
  | "HyperV"
  | "Docker"
  | "Wsl"
  | "VMware"
  | "VirtualBox"
  | "VirtualSwitch";

export interface TopologyNode {
  id: string;
  label: string;
  node_type: NodeType;
  if_index: number | null;
}

export interface TopologyEdge {
  from: string;
  to: string;
}

export interface TopologyGraph {
  nodes: TopologyNode[];
  edges: TopologyEdge[];
}

export interface HealthScore {
  if_index: number;
  score: number;
  availability_pct: number;
  stability_score: number;
  packet_loss_pct: number;
  error_rate_pct: number;
  link_state_changes: number;
  sample_count: number;
  computed_at: string;
}

export type AlertSeverity = "Info" | "Warning" | "Critical";

export interface Alert {
  id: string;
  if_index: number;
  if_name: string;
  kind: string;
  severity: AlertSeverity;
  message: string;
  triggered_at: string;
  resolved_at: string | null;
}
