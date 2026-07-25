use collector_core::{InterfaceStats, Snapshot};

pub async fn run(base_url: &str, json: bool) -> anyhow::Result<()> {
    let snapshot: Snapshot = reqwest::get(format!("{base_url}/interfaces"))
        .await?
        .error_for_status()?
        .json()
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&snapshot)?);
        return Ok(());
    }

    print_table(&snapshot.interfaces);
    Ok(())
}

pub fn print_table(interfaces: &[InterfaceStats]) {
    println!(
        "{:<5} {:<20} {:<10} {:<10} {:<18} {:<12} {:<12}",
        "IDX", "NAME", "STATUS", "SPEED", "MAC", "RX", "TX"
    );
    for iface in interfaces {
        let speed = iface
            .link_speed_bps
            .map(|s| format!("{:.0} Mbps", s as f64 / 1_000_000.0))
            .unwrap_or_else(|| "-".to_string());
        println!(
            "{:<5} {:<20} {:<10?} {:<10} {:<18} {:<12} {:<12}",
            iface.index,
            truncate(&iface.name, 20),
            iface.oper_status,
            speed,
            iface.mac_address,
            human_bytes(iface.rx_bytes),
            human_bytes(iface.tx_bytes),
        );
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}…", &s[..max.saturating_sub(1)])
    } else {
        s.to_string()
    }
}

pub fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit_idx = 0;
    while value >= 1024.0 && unit_idx < UNITS.len() - 1 {
        value /= 1024.0;
        unit_idx += 1;
    }
    format!("{value:.1}{}", UNITS[unit_idx])
}
