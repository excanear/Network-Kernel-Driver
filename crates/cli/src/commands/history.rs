use collector_core::InterfaceStats;

use super::interfaces::human_bytes;

pub async fn run(
    base_url: &str,
    interface_index: u32,
    from: Option<String>,
    to: Option<String>,
    limit: u32,
) -> anyhow::Result<()> {
    let mut url = format!("{base_url}/interfaces/{interface_index}/history?limit={limit}");
    if let Some(from) = from {
        url.push_str(&format!("&from={from}"));
    }
    if let Some(to) = to {
        url.push_str(&format!("&to={to}"));
    }

    let samples: Vec<InterfaceStats> = reqwest::get(url).await?.error_for_status()?.json().await?;

    if samples.is_empty() {
        println!("No history found for interface {interface_index}. Samples are persisted roughly every 10s while network-observatoryd runs.");
        return Ok(());
    }

    println!("{:<25} {:<12} {:<12}", "TIMESTAMP", "RX", "TX");
    for sample in samples {
        println!(
            "{:<25} {:<12} {:<12}",
            sample.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
            human_bytes(sample.rx_bytes),
            human_bytes(sample.tx_bytes),
        );
    }
    Ok(())
}
