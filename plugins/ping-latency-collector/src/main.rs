//! Example Network Observatory plugin: active latency/jitter/loss probing.
//!
//! Real ICMP echo requires raw sockets, which need admin/root privileges on
//! both Windows and Linux. To keep the plugin runnable without elevation
//! (matching the rest of Network Observatory's least-privilege design), this
//! plugin measures **TCP connect time** to a target host:port instead — a
//! well-established unprivileged proxy for path latency, at the cost of
//! including TCP handshake overhead rather than pure ICMP RTT. This
//! trade-off is documented here and in docs/roadmap.md rather than hidden.

use std::net::TcpStream;
use std::time::{Duration, Instant};

use chrono::Utc;
use clap::{Parser, Subcommand};
use plugin_api::{PluginKind, PluginManifest, PluginMetric, PluginResult};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Manifest,
    Run {
        #[arg(long, default_value = "1.1.1.1:443")]
        target: String,
        #[arg(long, default_value_t = 5)]
        samples: u32,
    },
}

fn manifest() -> PluginManifest {
    PluginManifest {
        name: "ping-latency-collector".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        kind: PluginKind::Collector,
        description: "Active latency/jitter/loss probing via TCP connect timing (unprivileged ICMP proxy)".to_string(),
    }
}

fn run(target: &str, samples: u32) -> PluginResult {
    let mut latencies_ms = Vec::new();
    let mut failures = 0u32;

    for _ in 0..samples {
        let start = Instant::now();
        match TcpStream::connect_timeout(
            &target.parse().unwrap_or_else(|_| {
                // Fall back to DNS resolution via std if target isn't a bare socket addr.
                use std::net::ToSocketAddrs;
                target
                    .to_socket_addrs()
                    .ok()
                    .and_then(|mut a| a.next())
                    .unwrap_or_else(|| "1.1.1.1:443".parse().unwrap())
            }),
            Duration::from_secs(2),
        ) {
            Ok(_) => latencies_ms.push(start.elapsed().as_secs_f64() * 1000.0),
            Err(_) => failures += 1,
        }
    }

    let mut metrics = Vec::new();
    let error = if latencies_ms.is_empty() {
        Some(format!("all {samples} connection attempts to {target} failed"))
    } else {
        let avg = latencies_ms.iter().sum::<f64>() / latencies_ms.len() as f64;
        let variance = latencies_ms.iter().map(|v| (v - avg).powi(2)).sum::<f64>() / latencies_ms.len() as f64;
        let jitter = variance.sqrt();
        let loss_pct = 100.0 * failures as f64 / samples as f64;

        metrics.push(PluginMetric { key: "latency_ms".into(), value: avg, unit: "ms".into() });
        metrics.push(PluginMetric { key: "jitter_ms".into(), value: jitter, unit: "ms".into() });
        metrics.push(PluginMetric { key: "packet_loss_pct".into(), value: loss_pct, unit: "%".into() });
        None
    };

    PluginResult {
        plugin: "ping-latency-collector".to_string(),
        target: target.to_string(),
        metrics,
        collected_at: Utc::now(),
        error,
    }
}

fn main() {
    let cli = Cli::parse();
    let output = match cli.command {
        Command::Manifest => serde_json::to_string(&manifest()),
        Command::Run { target, samples } => serde_json::to_string(&run(&target, samples)),
    };
    println!("{}", output.expect("serialization should not fail"));
}
