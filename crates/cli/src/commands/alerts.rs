use serde::Deserialize;

#[derive(Deserialize)]
struct Alert {
    if_name: String,
    kind: String,
    severity: String,
    message: String,
    triggered_at: String,
    resolved_at: Option<String>,
}

pub async fn run(base_url: &str, all: bool) -> anyhow::Result<()> {
    let path = if all { "/alerts/recent" } else { "/alerts" };
    let alerts: Vec<Alert> = reqwest::get(format!("{base_url}{path}"))
        .await?
        .error_for_status()?
        .json()
        .await?;

    if alerts.is_empty() {
        println!("No {} alerts.", if all { "recent" } else { "active" });
        return Ok(());
    }

    println!(
        "{:<10} {:<18} {:<20} {:<10} {}",
        "SEVERITY", "INTERFACE", "KIND", "STATUS", "MESSAGE"
    );
    for alert in alerts {
        let status = if alert.resolved_at.is_some() { "resolved" } else { "active" };
        println!(
            "{:<10} {:<18} {:<20} {:<10} {} ({})",
            alert.severity, alert.if_name, alert.kind, status, alert.message, alert.triggered_at
        );
    }
    Ok(())
}
