use std::sync::Arc;

use anyhow::Result;
use clap::{Parser, ValueEnum};
use rmcp::ServiceExt;
use terraphim_config::{ConfigBuilder, ConfigState};
use terraphim_mcp_server::McpService;
use tokio::io;
use tracing_subscriber::{self, EnvFilter, fmt, prelude::*};
use tracing_log;

#[derive(Parser)]
#[command(name = "terraphim_mcp_server")]
#[command(about = "Terraphim MCP server with configurable profile")]
#[command(version)]
struct Args {
    /// Configuration profile to use
    #[arg(short, long, value_enum, default_value_t = ConfigProfile::Desktop)]
    profile: ConfigProfile,
    
    /// Enable verbose logging (INFO level instead of WARN)
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum ConfigProfile {
    /// Use desktop configuration (Terraphim Engineer role with local KG)
    Desktop,
    /// Use server configuration (Default role without KG)
    Server,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    // Log to a file
    let log_dir = std::env::var("TERRAPHIM_LOG_DIR").unwrap_or_else(|_| {
        "/tmp/terraphim-logs".to_string()
    });
    std::fs::create_dir_all(&log_dir)?;
    let file_appender = tracing_appender::rolling::daily(log_dir, "terraphim-mcp-server.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // Forward `log` crate events (used throughout library code) to `tracing`
    let _ = tracing_log::LogTracer::init();

    // Set log level based on verbose flag
    let log_level = if args.verbose {
        tracing::Level::INFO
    } else {
        tracing::Level::WARN
    };

    let subscriber = tracing_subscriber::registry()
        .with(fmt::layer().with_writer(non_blocking))
        .with(EnvFilter::from_default_env().add_directive(log_level.into()));

    // If a subscriber is already set (e.g. in test harness), ignore the error.
    let _ = subscriber.try_init();

    tracing::info!("Starting Terraphim MCP server with {:?} profile", args.profile);

    // Build configuration based on selected profile
    let config = match args.profile {
        ConfigProfile::Desktop => {
            tracing::info!("Using desktop configuration (Terraphim Engineer role with local KG)");
            ConfigBuilder::new()
                .build_default_desktop()
                .build()
                .expect("Failed to build default desktop configuration")
        },
        ConfigProfile::Server => {
            tracing::info!("Using server configuration (Default role without KG)");
            ConfigBuilder::new()
                .build_default_server()
                .build()
                .expect("Failed to build default server configuration")
        },
    };

    // Initialize ConfigState from the config
    let mut temp_config = config.clone();
    let config_state = ConfigState::new(&mut temp_config)
        .await
        .expect("Failed to create config state from config");

    // Create the router
    let service = McpService::new(Arc::new(config_state));
    tracing::info!("Initialized Terraphim MCP service");

    // Create and run MCP server using stdout/stdin transport
    let server = service.serve((io::stdin(), io::stdout())).await?;
    tracing::info!("MCP server initialized and ready to handle requests");

    let reason = server.waiting().await?;
    tracing::info!("MCP server shut down with reason: {:?}", reason);

    Ok(())
} 