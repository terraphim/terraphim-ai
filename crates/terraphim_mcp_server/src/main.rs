use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use axum::{
    routing::get,
    Router,
};
use mcp_server::{ByteTransport, Server};
use mcp_server::router::RouterService;
use terraphim_config::{ConfigBuilder, ConfigId, ConfigState};
use terraphim_persistence::Persistable;
use terraphim_mcp_server::TerraphimMcpRouter;
use tokio::io;
use tokio::net::TcpListener;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{self, EnvFilter};

/// Default host for the MCP server
const DEFAULT_HOST: &str = "127.0.0.1";
/// Default port for the MCP server
const DEFAULT_PORT: u16 = 3000;

#[tokio::main]
async fn main() -> Result<()> {
    // Set up file appender for logging
    let log_dir = std::env::var("TERRAPHIM_LOG_DIR").unwrap_or_else(|_| "/tmp/terraphim_logs".to_string());

    // Create the log directory if it doesn't exist
    std::fs::create_dir_all(&log_dir)?;

    // Initialize the file appender with the log directory
    let file_appender = RollingFileAppender::new(Rotation::DAILY, log_dir.clone(), "terraphim-mcp-server.log");

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
    let fixtures_dir = std::env::var("TERRAPHIM_FIXTURES_DIR").unwrap_or_else(|_| "not set".to_string());
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
    let config_path = std::env::var("TERRAPHIM_CONFIG")
        .unwrap_or_else(|_| "terraphim_config.json".to_string());
    
    tracing::info!("Loading configuration from {}", config_path);
    
    // Load or create configuration
    let mut config = match ConfigBuilder::new_with_id(ConfigId::Server).build_default_server().build() {
        Ok(mut local_config) => {
            match local_config.load().await {
                Ok(config) => {
                    tracing::info!("Configuration loaded successfully");
                    config
                },
                Err(err) => {
                    tracing::error!("Failed to load configuration: {}", err);
                    tracing::info!("Using default server configuration");
                    ConfigBuilder::new().build_default_server().build().unwrap()
                }
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
    let config_state = ConfigState::new(&mut config).await
        .expect("Failed to create config state from config");
    
    // Determine host and port
    let host = std::env::var("TERRAPHIM_MCP_HOST").unwrap_or_else(|_| DEFAULT_HOST.to_string());
    let port = std::env::var("TERRAPHIM_MCP_PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(DEFAULT_PORT);
    
    let addr = format!("{}:{}", host, port).parse::<SocketAddr>()?;
    
    // Create the router
    let router = RouterService(TerraphimMcpRouter::new(Arc::new(config_state)));
    tracing::info!("Initialized Terraphim MCP router");
    
    // Set up HTTP server
    let app = Router::new()
        .route("/", get(health_check))
        .route("/health", get(health_check));
    
    // Start HTTP server on a separate task
    let http_addr = addr;
    tokio::spawn(async move {
        tracing::info!("Starting HTTP server on {}", http_addr);
        let listener = TcpListener::bind(http_addr).await.unwrap();
        axum::serve(listener, app).await.unwrap();
    });
    
    // Create and run MCP server using stdout/stdin transport
    let server = Server::new(router);
    tracing::info!("MCP server initialized and ready to handle requests");
    let transport = ByteTransport::new(io::stdin(), io::stdout());
    
    server.run(transport).await?;
    
    Ok(())
}

/// Health check endpoint
async fn health_check() -> &'static str {
    "Terraphim MCP Server is running"
} 