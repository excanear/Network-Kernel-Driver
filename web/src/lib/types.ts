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
