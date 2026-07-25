mod commands;

use std::path::PathBuf;

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
    /// Export a report (csv, json, html, md, pdf)
    Export {
        #[arg(long, default_value = "html")]
        format: String,
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Show alerts (active by default)
    Alerts {
        /// Show recent alerts (active + resolved) instead of only active ones
        #[arg(long)]
        all: bool,
    },
    /// Aggregate health statistics per interface
    Statistics,
    /// List discovered plugins
    Plugins,
    /// Run a plugin now
    PluginRun {
        name: String,
        #[arg(long)]
        target: Option<String>,
    },
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
        Commands::Topology => commands::topology::run(&base_url).await?,
        Commands::Export { format, output } => commands::export::run(&base_url, &format, output).await?,
        Commands::Alerts { all } => commands::alerts::run(&base_url, all).await?,
        Commands::Statistics => commands::statistics::run(&base_url).await?,
        Commands::Plugins => commands::plugins::list(&base_url).await?,
        Commands::PluginRun { name, target } => commands::plugins::run(&base_url, &name, target).await?,
    }

    Ok(())
}
