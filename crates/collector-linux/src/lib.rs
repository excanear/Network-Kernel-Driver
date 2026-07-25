#![cfg(unix)]

use std::collections::HashMap;
use std::fs;

use chrono::Utc;
use collector_core::{
    CollectorError, CollectorSource, Duplex, InterfaceCollector, InterfaceStats, OperStatus,
    Snapshot,
};

/// Default gateway per interface index (from `/proc/net/route`) plus DNS
/// servers configured in `/etc/resolv.conf` — used by `crates/topology` to
/// build the network graph without requiring the real kernel module (Phase 2).
pub fn discover_network_config() -> (Vec<(u32, String)>, Vec<String>) {
    let gateways = parse_proc_net_route().unwrap_or_default();
    let dns_servers = parse_resolv_conf().unwrap_or_default();
    (gateways, dns_servers)
}

fn if_index_for(name: &str) -> Option<u32> {
    sys_read_trimmed(name, "ifindex").and_then(|s| s.parse().ok())
}

fn parse_proc_net_route() -> Result<Vec<(u32, String)>, CollectorError> {
    let content = fs::read_to_string("/proc/net/route")?;
    let mut gateways = Vec::new();
    for line in content.lines().skip(1) {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 3 {
            continue;
        }
        let (iface, destination, gateway_hex) = (fields[0], fields[1], fields[2]);
        if destination != "00000000" {
            continue; // only the default route
        }
        if let Ok(raw) = u32::from_str_radix(gateway_hex, 16) {
            if raw == 0 {
                continue;
            }
            let bytes = raw.to_le_bytes();
            let ip = std::net::Ipv4Addr::from(bytes).to_string();
            if let Some(index) = if_index_for(iface) {
                gateways.push((index, ip));
            }
        }
    }
    Ok(gateways)
}

fn parse_resolv_conf() -> Result<Vec<String>, CollectorError> {
    let content = fs::read_to_string("/etc/resolv.conf")?;
    let mut servers = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("nameserver") {
            let ip = rest.trim();
            if !ip.is_empty() {
                servers.push(ip.to_string());
            }
        }
    }
    Ok(servers)
}

pub struct LinuxCollector;

impl LinuxCollector {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LinuxCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Default)]
struct ProcCounters {
    rx_bytes: u64,
    rx_packets: u64,
    rx_errors: u64,
    rx_drops: u64,
    rx_multicast_packets: u64,
    tx_bytes: u64,
    tx_packets: u64,
    tx_errors: u64,
    tx_drops: u64,
}

/// Parses `/proc/net/dev`, whose lines look like:
/// `  eth0: 1234 5 0 0 0 0 0 0  5678 9 0 0 0 0 0 0`
/// columns: face|rx bytes packets errs drop fifo frame compressed multicast|tx bytes packets errs drop fifo colls carrier compressed
fn parse_proc_net_dev() -> Result<HashMap<String, ProcCounters>, CollectorError> {
    let content = fs::read_to_string("/proc/net/dev")?;
    Ok(parse_proc_net_dev_str(&content))
}

fn parse_proc_net_dev_str(content: &str) -> HashMap<String, ProcCounters> {
    let mut map = HashMap::new();

    for line in content.lines().skip(2) {
        let Some((name, rest)) = line.split_once(':') else {
            continue;
        };
        let name = name.trim().to_string();
        let fields: Vec<u64> = rest
            .split_whitespace()
            .map(|f| f.parse::<u64>().unwrap_or(0))
            .collect();
        if fields.len() < 16 {
            continue;
        }

        map.insert(
            name,
            ProcCounters {
                rx_bytes: fields[0],
                rx_packets: fields[1],
                rx_errors: fields[2],
                rx_drops: fields[3],
                rx_multicast_packets: fields[7],
                tx_bytes: fields[8],
                tx_packets: fields[9],
                tx_errors: fields[10],
                tx_drops: fields[11],
            },
        );
    }

    map
}

fn sys_read_trimmed(iface: &str, file: &str) -> Option<String> {
    fs::read_to_string(format!("/sys/class/net/{iface}/{file}"))
        .ok()
        .map(|s| s.trim().to_string())
}

fn oper_status_from_str(s: &str) -> OperStatus {
    match s {
        "up" => OperStatus::Up,
        "down" => OperStatus::Down,
        "testing" => OperStatus::Testing,
        "dormant" => OperStatus::Dormant,
        "notpresent" => OperStatus::NotPresent,
        "lowerlayerdown" => OperStatus::LowerLayerDown,
        _ => OperStatus::Unknown,
    }
}

/// Subset of ARPHRD_* values (linux/if_arp.h) relevant to typical NICs.
fn if_type_name(raw_type: u32) -> String {
    match raw_type {
        1 => "Ethernet".to_string(),
        772 => "Loopback".to_string(),
        801 => "WiFi".to_string(),
        65534 => "Tunnel".to_string(),
        other => format!("Other({other})"),
    }
}

impl InterfaceCollector for LinuxCollector {
    fn snapshot(&self) -> Result<Snapshot, CollectorError> {
        let counters = parse_proc_net_dev()?;
        let now = Utc::now();
        let mut interfaces = Vec::new();

        let all_addrs = if_addrs::get_if_addrs().unwrap_or_default();

        for (idx, (name, counter)) in counters.into_iter().enumerate() {
            let mac_address = sys_read_trimmed(&name, "address").unwrap_or_default();
            let mtu = sys_read_trimmed(&name, "mtu")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            let oper_status = sys_read_trimmed(&name, "operstate")
                .map(|s| oper_status_from_str(&s))
                .unwrap_or(OperStatus::Unknown);
            let link_speed_bps = sys_read_trimmed(&name, "speed")
                .and_then(|s| s.parse::<i64>().ok())
                .filter(|mbps| *mbps > 0)
                .map(|mbps| mbps as u64 * 1_000_000);
            let duplex = sys_read_trimmed(&name, "duplex").and_then(|s| match s.as_str() {
                "full" => Some(Duplex::Full),
                "half" => Some(Duplex::Half),
                _ => None,
            });
            let raw_type = sys_read_trimmed(&name, "type")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            let ifindex = sys_read_trimmed(&name, "ifindex")
                .and_then(|s| s.parse().ok())
                .unwrap_or(idx as u32);

            let mut ipv4_addresses = Vec::new();
            let mut ipv6_addresses = Vec::new();
            for addr in all_addrs.iter().filter(|a| a.name == name) {
                match addr.ip() {
                    std::net::IpAddr::V4(v4) => ipv4_addresses.push(v4.to_string()),
                    std::net::IpAddr::V6(v6) => ipv6_addresses.push(v6.to_string()),
                }
            }

            interfaces.push(InterfaceStats {
                index: ifindex,
                name: name.clone(),
                description: name,
                mac_address,
                mtu,
                oper_status,
                link_speed_bps,
                duplex,
                if_type: if_type_name(raw_type),
                ipv4_addresses,
                ipv6_addresses,
                rx_bytes: counter.rx_bytes,
                tx_bytes: counter.tx_bytes,
                rx_packets: counter.rx_packets,
                tx_packets: counter.tx_packets,
                rx_errors: counter.rx_errors,
                tx_errors: counter.tx_errors,
                rx_drops: counter.rx_drops,
                tx_drops: counter.tx_drops,
                rx_broadcast_packets: None,
                rx_multicast_packets: Some(counter.rx_multicast_packets),
                timestamp: now,
                collector_source: CollectorSource::LinuxProcSys,
            });
        }

        Ok(Snapshot {
            interfaces,
            taken_at: now,
        })
    }

    fn platform_name(&self) -> &'static str {
        "LinuxProcSys"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_PROC_NET_DEV: &str = "Inter-|   Receive                                                |  Transmit\n face |bytes    packets errs drop fifo frame compressed multicast|bytes    packets errs drop fifo colls carrier compressed\n    lo: 1234       10    0    0    0     0          0         0     1234       10    0    0    0     0       0          0\n  eth0: 1000000   500    1    2    0     0          0         5   200000      300    0    0    0     0       0          0\n";

    #[test]
    fn parses_interface_counters_from_proc_net_dev() {
        let counters = parse_proc_net_dev_str(SAMPLE_PROC_NET_DEV);

        assert_eq!(counters.len(), 2);
        let eth0 = counters.get("eth0").expect("eth0 present");
        assert_eq!(eth0.rx_bytes, 1_000_000);
        assert_eq!(eth0.rx_packets, 500);
        assert_eq!(eth0.rx_errors, 1);
        assert_eq!(eth0.rx_drops, 2);
        assert_eq!(eth0.rx_multicast_packets, 5);
        assert_eq!(eth0.tx_bytes, 200_000);
        assert_eq!(eth0.tx_packets, 300);
    }

    #[test]
    fn if_type_name_maps_known_arphrd_values() {
        assert_eq!(if_type_name(1), "Ethernet");
        assert_eq!(if_type_name(772), "Loopback");
        assert_eq!(if_type_name(9999), "Other(9999)");
    }

    #[test]
    fn oper_status_from_str_handles_known_and_unknown_values() {
        assert_eq!(oper_status_from_str("up"), OperStatus::Up);
        assert_eq!(oper_status_from_str("down"), OperStatus::Down);
        assert_eq!(oper_status_from_str("bogus"), OperStatus::Unknown);
    }
}
