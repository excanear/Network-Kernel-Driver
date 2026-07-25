use std::path::PathBuf;

pub async fn run(base_url: &str, format: &str, output: Option<PathBuf>) -> anyhow::Result<()> {
    let resp = reqwest::get(format!("{base_url}/reports?format={format}"))
        .await?
        .error_for_status()?;

    let bytes = resp.bytes().await?;
    let output = output.unwrap_or_else(|| PathBuf::from(format!("network-observatory-report.{format}")));
    std::fs::write(&output, &bytes)?;
    println!("Report written to {} ({} bytes)", output.display(), bytes.len());
    Ok(())
}
