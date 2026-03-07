mod config;
mod platform;

use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

use crate::config::MyakuConfig;

#[derive(Parser)]
#[command(name = "myaku", about = "Myaku (脈) — GPU system monitor")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Run the metrics collection daemon.
    Daemon,
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    // Load config via shikumi
    let config = match shikumi::ConfigDiscovery::new("myaku")
        .env_override("MYAKU_CONFIG")
        .discover()
    {
        Ok(path) => {
            tracing::info!("loading config from {}", path.display());
            let store =
                shikumi::ConfigStore::<MyakuConfig>::load(&path, "MYAKU_").unwrap_or_else(|e| {
                    tracing::warn!("failed to load config: {e}, using defaults");
                    let tmp = std::env::temp_dir().join("myaku-default.yaml");
                    std::fs::write(&tmp, "{}").ok();
                    shikumi::ConfigStore::load(&tmp, "MYAKU_").unwrap()
                });
            MyakuConfig::clone(&store.get())
        }
        Err(_) => {
            tracing::info!("no config file found, using defaults");
            MyakuConfig::default()
        }
    };

    match cli.command {
        Some(Command::Daemon) => {
            tracing::info!("starting myaku daemon");
            let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
            rt.block_on(async {
                tracing::info!(
                    "metrics daemon on port {}, retention {}h",
                    config.daemon.metrics_port,
                    config.daemon.history_retention_hours
                );
                // Daemon event loop will be implemented here
                tokio::signal::ctrl_c()
                    .await
                    .expect("failed to listen for ctrl-c");
                tracing::info!("daemon shutting down");
            });
        }
        None => {
            tracing::info!("launching myaku GUI");
            tracing::info!(
                "refresh rate: {}ms",
                config.appearance.refresh_rate_ms
            );
            // GUI event loop will be implemented here
        }
    }
}
