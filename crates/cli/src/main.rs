mod commands;

use clap::{Parser, Subcommand};

const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 7878;

#[derive(Parser)]
#[command(name = "network", version, about = "Network Observatory CLI")]
struct Cli {
    /// network-observatoryd host
    #[arg(long, global = true, default_value = DEFAULT_HOST)]
    host: String,

    /// network-observatoryd port
    #[arg(long, global = true, default_value_t = DEFAULT_PORT)]
    port: u16,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show service health and collector backend info
    Status,
    /// List current interface snapshot
    Interfaces {
        #[arg(long)]
        json: bool,
    },
    /// Live-updating terminal view of interface throughput (like `docker stats`)
    Monitor {
        #[arg(long, default_value = "1s")]
        interval: String,
        #[arg(long)]
        interface: Option<String>,
    },
    /// Query historical samples for one interface
    History {
        #[arg(long)]
        interface: u32,
        #[arg(long)]
        from: Option<String>,
        #[arg(long)]
        to: Option<String>,
        #[arg(long, default_value_t = 100)]
        limit: u32,
    },
    /// Network topology graph (planned — see docs/roadmap.md)
    Topology,
    /// Export reports (planned — see docs/roadmap.md)
    Export,
    /// Alert rules (planned — see docs/roadmap.md)
    Alerts,
    /// Aggregate statistics (planned — see docs/roadmap.md)
    Statistics,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let base_url = format!("http://{}:{}/api/v1", cli.host, cli.port);
    let ws_url = format!("ws://{}:{}/api/v1/ws/interfaces", cli.host, cli.port);

    match cli.command {
        Commands::Status => commands::status::run(&base_url).await?,
        Commands::Interfaces { json } => commands::interfaces::run(&base_url, json).await?,
        Commands::Monitor { interval, interface } => {
            commands::monitor::run(&ws_url, interface.as_deref(), &interval).await?
        }
        Commands::History {
            interface,
            from,
            to,
            limit,
        } => commands::history::run(&base_url, interface, from, to, limit).await?,
        Commands::Topology => commands::not_implemented("topology"),
        Commands::Export => commands::not_implemented("export"),
        Commands::Alerts => commands::not_implemented("alerts"),
        Commands::Statistics => commands::not_implemented("statistics"),
    }

    Ok(())
}
