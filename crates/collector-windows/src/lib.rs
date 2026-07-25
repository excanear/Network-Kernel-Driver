#![cfg(windows)]

use std::collections::HashMap;
use std::net::IpAddr;

use chrono::Utc;
use collector_core::{
    CollectorError, CollectorSource, Duplex, InterfaceCollector, InterfaceStats, OperStatus,
    Snapshot,
};

use windows::Win32::Foundation::{ERROR_BUFFER_OVERFLOW, ERROR_SUCCESS};
use windows::Win32::NetworkManagement::IpHelper::{
    FreeMibTable, GetAdaptersAddresses, GetIfTable2, GAA_FLAG_INCLUDE_PREFIX,
    IP_ADAPTER_ADDRESSES_LH, MIB_IF_TABLE2,
};
use windows::Win32::NetworkManagement::Ndis::IF_OPER_STATUS;
use windows::Win32::Networking::WinSock::{AF_UNSPEC, SOCKADDR_IN, SOCKADDR_IN6};

pub struct WindowsCollector;

impl WindowsCollector {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WindowsCollector {
    fn default() -> Self {
        Self::new()
    }
}

fn oper_status_from_win32(status: IF_OPER_STATUS) -> OperStatus {
    match status.0 {
        1 => OperStatus::Up,
        2 => OperStatus::Down,
        3 => OperStatus::Testing,
        4 => OperStatus::Unknown,
        5 => OperStatus::Dormant,
        6 => OperStatus::NotPresent,
        7 => OperStatus::LowerLayerDown,
        _ => OperStatus::Unknown,
    }
}

fn if_type_name(if_type: u32) -> String {
    // Subset of the IANA ifType registry values relevant to typical desktop/server NICs.
    match if_type {
        6 => "Ethernet".to_string(),
        24 => "Loopback".to_string(),
        71 => "WiFi".to_string(),
        131 => "Tunnel".to_string(),
        144 => "IEEE1394".to_string(),
        53 => "VirtualBridge".to_string(),
        other => format!("Other({other})"),
    }
}

unsafe fn wide_ptr_to_string(ptr: *const u16) -> String {
    if ptr.is_null() {
        return String::new();
    }
    let mut len = 0usize;
    while *ptr.add(len) != 0 {
        len += 1;
    }
    let slice = std::slice::from_raw_parts(ptr, len);
    String::from_utf16_lossy(slice)
}

unsafe fn sockaddr_to_ip(lp_sockaddr: *const windows::Win32::Networking::WinSock::SOCKADDR) -> Option<String> {
    if lp_sockaddr.is_null() {
        return None;
    }
    let family = (*lp_sockaddr).sa_family;
    if family == windows::Win32::Networking::WinSock::AF_INET {
        let sockaddr_in = &*(lp_sockaddr as *const SOCKADDR_IN);
        let bytes = sockaddr_in.sin_addr.S_un.S_addr.to_ne_bytes();
        Some(IpAddr::from([bytes[0], bytes[1], bytes[2], bytes[3]]).to_string())
    } else if family == windows::Win32::Networking::WinSock::AF_INET6 {
        let sockaddr_in6 = &*(lp_sockaddr as *const SOCKADDR_IN6);
        Some(IpAddr::from(sockaddr_in6.sin6_addr.u.Byte).to_string())
    } else {
        None
    }
}

unsafe fn read_ip_addresses(adapter: &IP_ADAPTER_ADDRESSES_LH) -> (Vec<String>, Vec<String>) {
    let mut ipv4 = Vec::new();
    let mut ipv6 = Vec::new();
    let mut unicast = adapter.FirstUnicastAddress;
    while !unicast.is_null() {
        let addr = (*unicast).Address;
        match sockaddr_to_ip(addr.lpSockaddr) {
            Some(ip) if ip.contains(':') => ipv6.push(ip),
            Some(ip) => ipv4.push(ip),
            None => {}
        }
        unicast = (*unicast).Next;
    }
    (ipv4, ipv6)
}

/// Default gateway per interface index, plus the DNS servers configured
/// system-wide — used by `crates/topology` to build the network graph
/// without requiring the real kernel driver (Phase 2).
pub fn discover_network_config() -> (Vec<(u32, String)>, Vec<String>) {
    unsafe { discover_network_config_inner().unwrap_or_default() }
}

unsafe fn discover_network_config_inner() -> Result<(Vec<(u32, String)>, Vec<String>), CollectorError> {
    let mut size: u32 = 15_000;
    let mut buffer: Vec<u8>;
    let flags = GAA_FLAG_INCLUDE_PREFIX;

    loop {
        buffer = vec![0u8; size as usize];
        let adapter_addresses = buffer.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH;
        let result = GetAdaptersAddresses(AF_UNSPEC.0 as u32, flags, None, Some(adapter_addresses), &mut size);
        if result == ERROR_SUCCESS.0 {
            break;
        } else if result == ERROR_BUFFER_OVERFLOW.0 {
            continue;
        } else {
            return Err(CollectorError::PlatformApi(format!("GetAdaptersAddresses failed: {result}")));
        }
    }

    let mut gateways = Vec::new();
    let mut dns_servers = Vec::new();
    let mut current = buffer.as_ptr() as *const IP_ADAPTER_ADDRESSES_LH;

    while !current.is_null() {
        let adapter = &*current;
        let index = adapter.Anonymous1.Anonymous.IfIndex;

        let mut gw = adapter.FirstGatewayAddress;
        while !gw.is_null() {
            if let Some(ip) = sockaddr_to_ip((*gw).Address.lpSockaddr) {
                gateways.push((index, ip));
            }
            gw = (*gw).Next;
        }

        let mut dns = adapter.FirstDnsServerAddress;
        while !dns.is_null() {
            if let Some(ip) = sockaddr_to_ip((*dns).Address.lpSockaddr) {
                if !dns_servers.contains(&ip) {
                    dns_servers.push(ip);
                }
            }
            dns = (*dns).Next;
        }

        current = adapter.Next;
    }

    Ok((gateways, dns_servers))
}

struct CounterRow {
    rx_bytes: u64,
    tx_bytes: u64,
    rx_packets: u64,
    tx_packets: u64,
    rx_errors: u64,
    tx_errors: u64,
    rx_drops: u64,
    tx_drops: u64,
    rx_multicast_packets: u64,
}

unsafe fn read_counters() -> Result<HashMap<u32, CounterRow>, CollectorError> {
    let mut table: *mut MIB_IF_TABLE2 = std::ptr::null_mut();
    let result = GetIfTable2(&mut table);
    if result != ERROR_SUCCESS || table.is_null() {
        return Err(CollectorError::PlatformApi(format!(
            "GetIfTable2 failed: {result:?}"
        )));
    }

    let num_entries = (*table).NumEntries as usize;
    let first_row_ptr = (*table).Table.as_ptr();
    let mut map = HashMap::with_capacity(num_entries);

    for i in 0..num_entries {
        let row = &*first_row_ptr.add(i);
        map.insert(
            row.InterfaceIndex,
            CounterRow {
                rx_bytes: row.InOctets,
                tx_bytes: row.OutOctets,
                rx_packets: row.InUcastPkts + row.InNUcastPkts,
                tx_packets: row.OutUcastPkts + row.OutNUcastPkts,
                rx_errors: row.InErrors,
                tx_errors: row.OutErrors,
                rx_drops: row.InDiscards,
                tx_drops: row.OutDiscards,
                rx_multicast_packets: row.InNUcastPkts,
            },
        );
    }

    FreeMibTable(table as *const _);
    Ok(map)
}

unsafe fn read_adapters() -> Result<Vec<InterfaceStats>, CollectorError> {
    let counters = read_counters()?;

    let mut size: u32 = 15_000;
    let mut buffer: Vec<u8>;
    let flags = GAA_FLAG_INCLUDE_PREFIX;

    loop {
        buffer = vec![0u8; size as usize];
        let adapter_addresses = buffer.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH;
        let result = GetAdaptersAddresses(
            AF_UNSPEC.0 as u32,
            flags,
            None,
            Some(adapter_addresses),
            &mut size,
        );

        if result == ERROR_SUCCESS.0 {
            break;
        } else if result == ERROR_BUFFER_OVERFLOW.0 {
            continue;
        } else {
            return Err(CollectorError::PlatformApi(format!(
                "GetAdaptersAddresses failed: {result}"
            )));
        }
    }

    let mut stats = Vec::new();
    let mut current = buffer.as_ptr() as *const IP_ADAPTER_ADDRESSES_LH;
    let now = Utc::now();

    while !current.is_null() {
        let adapter = &*current;
        let index = adapter.Anonymous1.Anonymous.IfIndex;
        let name = wide_ptr_to_string(adapter.FriendlyName.as_ptr());
        let description = wide_ptr_to_string(adapter.Description.as_ptr());

        let mac_len = adapter.PhysicalAddressLength as usize;
        let mac_address = if mac_len > 0 {
            adapter.PhysicalAddress[..mac_len.min(8)]
                .iter()
                .map(|b| format!("{b:02X}"))
                .collect::<Vec<_>>()
                .join(":")
        } else {
            String::new()
        };

        let (ipv4_addresses, ipv6_addresses) = read_ip_addresses(adapter);

        let counter = counters.get(&index);
        let (tx_speed, rx_speed) = (adapter.TransmitLinkSpeed, adapter.ReceiveLinkSpeed);
        let link_speed_bps = if rx_speed > 0 && rx_speed != u64::MAX {
            Some(rx_speed)
        } else if tx_speed > 0 && tx_speed != u64::MAX {
            Some(tx_speed)
        } else {
            None
        };

        // Full/half duplex is not reliably exposed via GetAdaptersAddresses; the IP Helper
        // backend reports it as unknown (best-effort per docs/data-model.md). The Phase 2
        // kernel driver backend is expected to populate this from NDIS OID queries.
        let duplex: Option<Duplex> = None;

        stats.push(InterfaceStats {
            index,
            name,
            description,
            mac_address,
            mtu: adapter.Mtu,
            oper_status: oper_status_from_win32(adapter.OperStatus),
            link_speed_bps,
            duplex,
            if_type: if_type_name(adapter.IfType),
            ipv4_addresses,
            ipv6_addresses,
            rx_bytes: counter.map(|c| c.rx_bytes).unwrap_or_default(),
            tx_bytes: counter.map(|c| c.tx_bytes).unwrap_or_default(),
            rx_packets: counter.map(|c| c.rx_packets).unwrap_or_default(),
            tx_packets: counter.map(|c| c.tx_packets).unwrap_or_default(),
            rx_errors: counter.map(|c| c.rx_errors).unwrap_or_default(),
            tx_errors: counter.map(|c| c.tx_errors).unwrap_or_default(),
            rx_drops: counter.map(|c| c.rx_drops).unwrap_or_default(),
            tx_drops: counter.map(|c| c.tx_drops).unwrap_or_default(),
            rx_broadcast_packets: None,
            rx_multicast_packets: counter.map(|c| c.rx_multicast_packets),
            timestamp: now,
            collector_source: CollectorSource::WindowsIpHelper,
        });

        current = adapter.Next;
    }

    Ok(stats)
}

impl InterfaceCollector for WindowsCollector {
    fn snapshot(&self) -> Result<Snapshot, CollectorError> {
        let interfaces = unsafe { read_adapters()? };
        Ok(Snapshot {
            interfaces,
            taken_at: Utc::now(),
        })
    }

    fn platform_name(&self) -> &'static str {
        "WindowsIpHelper"
    }
}
