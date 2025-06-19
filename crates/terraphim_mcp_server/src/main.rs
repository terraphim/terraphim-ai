use std::sync::Arc;

use anyhow::Result;
use rmcp::ServiceExt;
use terraphim_config::{ConfigBuilder, ConfigId, ConfigState};
use terraphim_mcp_server::McpService;
use terraphim_persistence::Persistable;
use tokio::io;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{self, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    // Set up file appender for logging
    let log_dir =
        std::env::var("TERRAPHIM_LOG_DIR").unwrap_or_else(|_| "/tmp/terraphim_logs".to_string());

    // Create the log directory if it doesn't exist
    std::fs::create_dir_all(&log_dir)?;

    // Initialize the file appender with the log directory
    let file_appender =
        RollingFileAppender::new(Rotation::DAILY, log_dir.clone(), "terraphim-mcp-server.log");

    // Initialize the tracing subscriber with file logging only (no stdout)
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .with_writer(file_appender)
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    tracing::info!("Starting Terraphim MCP server");
    tracing::info!("Logging to directory: {}", log_dir);

    // Log important environment variables
    let data_dir = std::env::var("TERRAPHIM_DATA_DIR").unwrap_or_else(|_| "not set".to_string());
    let fixtures_dir =
        std::env::var("TERRAPHIM_FIXTURES_DIR").unwrap_or_else(|_| "not set".to_string());
    let test_mode = std::env::var("TERRAPHIM_TEST_MODE").unwrap_or_else(|_| "not set".to_string());

    tracing::info!("Environment variables:");
    tracing::info!("  TERRAPHIM_DATA_DIR: {}", data_dir);
    tracing::info!("  TERRAPHIM_FIXTURES_DIR: {}", fixtures_dir);
    tracing::info!("  TERRAPHIM_TEST_MODE: {}", test_mode);

    // Create data directory if set and doesn't exist
    if data_dir != "not set" {
        let data_path = std::path::PathBuf::from(&data_dir);
        if !data_path.exists() {
            tracing::info!("Creating data directory: {:?}", data_path);
            std::fs::create_dir_all(&data_path)?;
        }
    }

    // Set environment variable to prevent println statements in dependencies
    std::env::set_var("TERRAPHIM_NO_CONSOLE_OUTPUT", "1");

    // Load Terraphim configuration
    let config_path =
        std::env::var("TERRAPHIM_CONFIG").unwrap_or_else(|_| "terraphim_config.json".to_string());

    tracing::info!("Loading configuration from {}", config_path);

    // Load or create configuration
    let mut config = match ConfigBuilder::new_with_id(ConfigId::Server)
        .build_default_server()
        .build()
    {
        Ok(mut local_config) => match local_config.load().await {
            Ok(config) => {
                tracing::info!("Configuration loaded successfully");
                config
            }
            Err(err) => {
                tracing::error!("Failed to load configuration: {}", err);
                tracing::info!("Using default server configuration");
                ConfigBuilder::new()
                    .build_default_server()
                    .build()
                    .unwrap()
            }
        },
        Err(err) => {
            tracing::error!("Failed to build configuration: {}", err);

            // Create a default configuration using ConfigBuilder
            tracing::info!("Creating default configuration");
            ConfigBuilder::new()
                .build_default_server()
                .build()
                .expect("Failed to build default configuration")
        }
    };

    // Initialize ConfigState from the config
    let config_state = ConfigState::new(&mut config)
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