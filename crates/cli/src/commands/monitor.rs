use collector_core::Snapshot;
use futures_util::StreamExt;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

use super::interfaces::print_table;

pub async fn run(ws_url: &str, interface: Option<&str>, requested_interval: &str) -> anyhow::Result<()> {
    println!(
        "Connecting to {ws_url} — press Ctrl+C to stop\n\
         (note: cadence is currently driven by the service's poll interval; \
         --interval={requested_interval} is accepted for forward-compatibility, see docs/roadmap.md)"
    );
    let (ws_stream, _) = connect_async(ws_url).await?;
    let (_, mut read) = ws_stream.split();

    while let Some(msg) = read.next().await {
        let msg = msg?;
        if let Message::Text(text) = msg {
            let snapshot: Snapshot = serde_json::from_str(&text)?;
            let interfaces: Vec<_> = snapshot
                .interfaces
                .into_iter()
                .filter(|i| interface.map(|n| i.name == n).unwrap_or(true))
                .collect();

            // Clear screen (ANSI) for a live-updating view, like `nvidia-smi -l`/`docker stats`.
            print!("\x1B[2J\x1B[1;1H");
            println!(
                "Network Observatory — live monitor ({})",
                snapshot.taken_at.format("%Y-%m-%d %H:%M:%S UTC")
            );
            print_table(&interfaces);
        }
    }

    Ok(())
}
