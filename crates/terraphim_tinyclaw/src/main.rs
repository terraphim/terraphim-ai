mod agent;
mod bus;
mod channel;
mod channels;
mod config;
mod session;
mod tools;

use crate::bus::MessageBus;
use crate::channel::{Channel, ChannelManager};
use crate::channels::cli::CliChannel;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::sync::Arc;

/// Multi-channel AI assistant powered by Terraphim.
#[derive(Parser, Debug)]
#[command(name = "terraphim-tinyclaw")]
#[command(about = "Multi-channel AI assistant for Telegram, Discord, and CLI")]
#[command(version)]
struct Cli {
    /// Path to configuration file.
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Enable verbose logging.
    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run in interactive CLI mode.
    Agent,
    /// Run as gateway server with all enabled channels.
    Gateway,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    if cli.verbose {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();
    } else {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    }

    log::info!("terraphim-tinyclaw starting");

    // Load configuration
    let config_path = cli.config.or_else(config::Config::default_path);

    let _config = match config_path {
        Some(path) if path.exists() => {
            log::info!("Loading configuration from {}", path.display());
            config::Config::from_file_with_env(&path)?
        }
        Some(path) => {
            log::warn!(
                "Config file not found at {}, using defaults",
                path.display()
            );
            config::Config::default()
        }
        None => {
            log::warn!("Could not determine config path, using defaults");
            config::Config::default()
        }
    };

    match cli.command {
        Commands::Agent => {
            log::info!("Starting in agent mode (CLI)");
            run_agent_mode().await?;
        }
        Commands::Gateway => {
            log::info!("Starting in gateway mode");
            run_gateway_mode().await?;
        }
    }

    log::info!("terraphim-tinyclaw shutting down");
    Ok(())
}

async fn run_agent_mode() -> anyhow::Result<()> {
    println!("TinyClaw Agent Mode");
    println!("===================");

    // Create message bus
    let bus = Arc::new(MessageBus::new());

    // Create CLI channel
    let cli_channel = CliChannel::new();

    // For now, just run the CLI channel directly
    // In the full implementation, we'd also start the agent loop
    cli_channel.start(bus).await?;

    Ok(())
}

async fn run_gateway_mode() -> anyhow::Result<()> {
    println!("TinyClaw Gateway Mode");
    println!("=====================");
    println!("(Implementation in progress - Step 1 complete)");
    Ok(())
}
