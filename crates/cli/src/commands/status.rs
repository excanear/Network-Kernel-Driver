use serde::Deserialize;

#[derive(Deserialize)]
struct StatusResponse {
    uptime_seconds: u64,
    collector_backend: String,
    poll_interval_ms: u64,
    interface_count: usize,
}

pub async fn run(base_url: &str) -> anyhow::Result<()> {
    let resp: StatusResponse = reqwest::get(format!("{base_url}/status"))
        .await?
        .error_for_status()?
        .json()
        .await?;

    println!("Network Observatory — status");
    println!("  backend           : {}", resp.collector_backend);
    println!("  uptime            : {}s", resp.uptime_seconds);
    println!("  poll interval     : {}ms", resp.poll_interval_ms);
    println!("  interfaces tracked: {}", resp.interface_count);
    Ok(())
}
