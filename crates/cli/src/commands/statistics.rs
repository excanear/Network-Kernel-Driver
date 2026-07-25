use serde::Deserialize;

#[derive(Deserialize)]
struct HealthScore {
    if_index: u32,
    score: f64,
    availability_pct: f64,
    stability_score: f64,
    packet_loss_pct: f64,
    error_rate_pct: f64,
    link_state_changes: u32,
    sample_count: usize,
}

pub async fn run(base_url: &str) -> anyhow::Result<()> {
    let scores: Vec<HealthScore> = reqwest::get(format!("{base_url}/health"))
        .await?
        .error_for_status()?
        .json()
        .await?;

    if scores.is_empty() {
        println!("No statistics available yet — the service needs at least one poll tick of history.");
        return Ok(());
    }

    let avg_score = scores.iter().map(|s| s.score).sum::<f64>() / scores.len() as f64;
    let worst = scores
        .iter()
        .min_by(|a, b| a.score.partial_cmp(&b.score).unwrap())
        .unwrap();

    println!("Network Observatory — aggregate statistics");
    println!("  interfaces tracked : {}", scores.len());
    println!("  average health     : {avg_score:.1}/100");
    println!(
        "  least healthy      : interface {} ({:.1}/100)",
        worst.if_index, worst.score
    );
    println!();
    println!(
        "{:<6} {:<8} {:<8} {:<8} {:<10} {:<10} {:<8} {}",
        "IDX", "SCORE", "AVAIL%", "STABLE", "LOSS%", "ERR%", "FLAPS", "SAMPLES"
    );
    for s in scores {
        println!(
            "{:<6} {:<8.1} {:<8.1} {:<8.1} {:<10.2} {:<10.2} {:<8} {}",
            s.if_index,
            s.score,
            s.availability_pct,
            s.stability_score,
            s.packet_loss_pct,
            s.error_rate_pct,
            s.link_state_changes,
            s.sample_count
        );
    }
    Ok(())
}
