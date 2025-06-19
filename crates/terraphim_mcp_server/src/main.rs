use std::sync::Arc;

use anyhow::Result;
use rmcp::ServiceExt;
use terraphim_config::{ConfigBuilder, ConfigState};
use terraphim_mcp_server::McpService;
use tokio::io;
use tracing_subscriber::{self, EnvFilter, fmt, prelude::*};

#[tokio::main]
async fn main() -> Result<()> {
    // Log to a file
    let log_dir = std::env::var("TERRAPHIM_LOG_DIR").unwrap_or_else(|_| "logs".to_string());
    std::fs::create_dir_all(&log_dir)?;
    let file_appender = tracing_appender::rolling::daily(log_dir, "terraphim-mcp-server.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(non_blocking))
        .with(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .init();

    tracing::info!("Starting Terraphim MCP server");

    // Create a minimal default configuration
    let config = ConfigBuilder::new()
        .build_default_server()
        .build()
        .expect("Failed to build default configuration");

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