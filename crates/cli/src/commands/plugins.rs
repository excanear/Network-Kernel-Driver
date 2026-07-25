use serde::Deserialize;

#[derive(Deserialize)]
struct Manifest {
    name: String,
    version: String,
    kind: String,
    description: String,
}

#[derive(Deserialize)]
struct Metric {
    key: String,
    value: f64,
    unit: String,
}

#[derive(Deserialize)]
struct PluginResult {
    target: String,
    metrics: Vec<Metric>,
    error: Option<String>,
}

pub async fn list(base_url: &str) -> anyhow::Result<()> {
    let manifests: Vec<Manifest> = reqwest::get(format!("{base_url}/plugins"))
        .await?
        .error_for_status()?
        .json()
        .await?;

    if manifests.is_empty() {
        println!("No plugins discovered. Drop plugin executables next to network-observatoryd, or set NETOBS_PLUGINS_DIR.");
        return Ok(());
    }

    println!("{:<28} {:<10} {:<12} {}", "NAME", "VERSION", "KIND", "DESCRIPTION");
    for m in manifests {
        println!("{:<28} {:<10} {:<12} {}", m.name, m.version, m.kind, m.description);
    }
    Ok(())
}

pub async fn run(base_url: &str, name: &str, target: Option<String>) -> anyhow::Result<()> {
    let mut url = format!("{base_url}/plugins/{name}/run");
    if let Some(target) = target {
        url.push_str(&format!("?target={target}"));
    }
    let result: PluginResult = reqwest::get(url).await?.error_for_status()?.json().await?;

    if let Some(err) = result.error {
        println!("Plugin '{name}' error against {}: {err}", result.target);
        return Ok(());
    }

    println!("Plugin '{name}' against {}:", result.target);
    for metric in result.metrics {
        println!("  {:<20} {:.2} {}", metric.key, metric.value, metric.unit);
    }
    Ok(())
}
