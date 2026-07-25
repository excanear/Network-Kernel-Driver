use collector_core::{InterfaceStats, OperStatus};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeType {
    Host,
    Nic,
    Gateway,
    Dns,
    Vpn,
    HyperV,
    Docker,
    Wsl,
    VMware,
    VirtualBox,
    VirtualSwitch,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyNode {
    pub id: String,
    pub label: String,
    pub node_type: NodeType,
    pub if_index: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyEdge {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyGraph {
    pub nodes: Vec<TopologyNode>,
    pub edges: Vec<TopologyEdge>,
}

#[derive(Debug, Clone, Default)]
pub struct GatewayInfo {
    pub if_index: u32,
    pub address: String,
}

/// Heuristic classification of virtual/tunnel interfaces by name+description,
/// since Windows/Linux don't expose a first-class "this is a Docker/WSL/VPN
/// adapter" flag — matches the detection approach documented in
/// docs/architecture.md for topology (Phase D).
fn classify_virtual(name: &str, description: &str) -> Option<NodeType> {
    let haystack = format!("{name} {description}").to_lowercase();
    if haystack.contains("hyper-v") || haystack.contains("hyperv") {
        Some(NodeType::HyperV)
    } else if haystack.contains("docker") {
        Some(NodeType::Docker)
    } else if haystack.contains("wsl") {
        Some(NodeType::Wsl)
    } else if haystack.contains("vmware") {
        Some(NodeType::VMware)
    } else if haystack.contains("virtualbox") || haystack.contains("vbox") {
        Some(NodeType::VirtualBox)
    } else if haystack.contains("tap-windows")
        || haystack.contains("tun")
        || haystack.contains("openvpn")
        || haystack.contains("wireguard")
        || haystack.contains("wintun")
    {
        Some(NodeType::Vpn)
    } else if haystack.contains("vethernet") || haystack.contains("virtual") {
        Some(NodeType::VirtualSwitch)
    } else {
        None
    }
}

pub fn build_topology(
    interfaces: &[InterfaceStats],
    gateways: &[GatewayInfo],
    dns_servers: &[String],
) -> TopologyGraph {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    let host_id = "host".to_string();
    nodes.push(TopologyNode {
        id: host_id.clone(),
        label: "This host".to_string(),
        node_type: NodeType::Host,
        if_index: None,
    });

    for iface in interfaces {
        if iface.oper_status != OperStatus::Up {
            continue;
        }
        let node_id = format!("if-{}", iface.index);
        let node_type = classify_virtual(&iface.name, &iface.description).unwrap_or(NodeType::Nic);

        nodes.push(TopologyNode {
            id: node_id.clone(),
            label: iface.name.clone(),
            node_type,
            if_index: Some(iface.index),
        });
        edges.push(TopologyEdge {
            from: host_id.clone(),
            to: node_id.clone(),
        });

        if let Some(gw) = gateways.iter().find(|g| g.if_index == iface.index) {
            let gw_id = format!("gw-{}", gw.address);
            if !nodes.iter().any(|n| n.id == gw_id) {
                nodes.push(TopologyNode {
                    id: gw_id.clone(),
                    label: format!("Gateway {}", gw.address),
                    node_type: NodeType::Gateway,
                    if_index: None,
                });
            }
            edges.push(TopologyEdge {
                from: node_id.clone(),
                to: gw_id,
            });
        }
    }

    for dns in dns_servers {
        let dns_id = format!("dns-{dns}");
        if !nodes.iter().any(|n| n.id == dns_id) {
            nodes.push(TopologyNode {
                id: dns_id.clone(),
                label: format!("DNS {dns}"),
                node_type: NodeType::Dns,
                if_index: None,
            });
        }
        edges.push(TopologyEdge {
            from: host_id.clone(),
            to: dns_id,
        });
    }

    TopologyGraph { nodes, edges }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use collector_core::CollectorSource;

    fn iface(index: u32, name: &str, description: &str, up: bool) -> InterfaceStats {
        InterfaceStats {
            index,
            name: name.to_string(),
            description: description.to_string(),
            mac_address: String::new(),
            mtu: 1500,
            oper_status: if up { OperStatus::Up } else { OperStatus::Down },
            link_speed_bps: None,
            duplex: None,
            if_type: "Ethernet".into(),
            ipv4_addresses: vec!["192.168.1.10".into()],
            ipv6_addresses: vec![],
            rx_bytes: 0,
            tx_bytes: 0,
            rx_packets: 0,
            tx_packets: 0,
            rx_errors: 0,
            tx_errors: 0,
            rx_drops: 0,
            tx_drops: 0,
            rx_broadcast_packets: None,
            rx_multicast_packets: None,
            timestamp: Utc::now(),
            collector_source: CollectorSource::WindowsIpHelper,
        }
    }

    #[test]
    fn classifies_and_links_gateway_dns() {
        let interfaces = vec![
            iface(1, "Ethernet", "Realtek PCIe", true),
            iface(2, "vEthernet (Default Switch)", "Hyper-V Virtual Switch", true),
            iface(3, "Loopback", "loop", false),
        ];
        let gateways = vec![GatewayInfo { if_index: 1, address: "192.168.1.1".into() }];
        let dns = vec!["8.8.8.8".to_string()];

        let graph = build_topology(&interfaces, &gateways, &dns);

        assert!(graph.nodes.iter().any(|n| n.id == "if-1" && n.node_type == NodeType::Nic));
        assert!(graph.nodes.iter().any(|n| n.id == "if-2" && n.node_type == NodeType::HyperV));
        assert!(!graph.nodes.iter().any(|n| n.id == "if-3")); // down interface excluded
        assert!(graph.nodes.iter().any(|n| n.node_type == NodeType::Gateway));
        assert!(graph.nodes.iter().any(|n| n.node_type == NodeType::Dns));
        assert!(graph.edges.iter().any(|e| e.from == "if-1" && e.to.starts_with("gw-")));
    }
}
