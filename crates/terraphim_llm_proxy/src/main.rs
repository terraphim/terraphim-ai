//! Terraphim LLM Proxy
//!
//! A production-ready LLM proxy that functions as a drop-in replacement for Claude Code,
//! with intelligent routing, cost optimization, and Terraphim integration.

use clap::Parser;
use std::net::SocketAddr;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use terraphim_llm_proxy::{config::ProxyConfig, server::create_server, Result};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to configuration file
    #[arg(short, long, default_value = "config.toml")]
    config: String,

    /// Host to bind to
    #[arg(long, env = "PROXY_HOST")]
    host: Option<String>,

    /// Port to bind to
    #[arg(short, long, env = "PROXY_PORT")]
    port: Option<u16>,

    /// Log level (trace, debug, info, warn, error)
    #[arg(long, env = "LOG_LEVEL", default_value = "info")]
    log_level: String,

    /// Enable JSON logging
    #[arg(long, env = "LOG_JSON")]
    log_json: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file if present
    let _ = dotenvy::dotenv();

    // Parse CLI arguments
    let args = Args::parse();

    // Initialize logging
    init_logging(&args.log_level, args.log_json)?;

    info!(
        "Starting Terraphim LLM Proxy v{}",
        env!("CARGO_PKG_VERSION")
    );

    // Load configuration
    info!("Loading configuration from: {}", args.config);
    let mut config = ProxyConfig::load(&args.config)?;

    // Override config with CLI args if provided
    if let Some(host) = args.host {
        config.proxy.host = host;
    }
    if let Some(port) = args.port {
        config.proxy.port = port;
    }

    // Validate configuration
    info!("Validating configuration...");
    config.validate()?;
    info!("Configuration validated successfully");

    // Log configuration (sanitized)
    info!(
        host = %config.proxy.host,
        port = config.proxy.port,
        providers = config.providers.len(),
        "Proxy configuration"
    );

    // Create server address
    let addr = SocketAddr::new(
        config.proxy.host.parse().map_err(|e| {
            terraphim_llm_proxy::error::ProxyError::ConfigError(format!("Invalid host: {}", e))
        })?,
        config.proxy.port,
    );

    // Create and start server
    info!("Starting HTTP server on {}", addr);
    let app = create_server(config).await?;

    info!("✓ Terraphim LLM Proxy is running on http://{}", addr);
    info!("Ready to accept connections");

    // Run server (Axum 0.7+ uses axum::serve)
    let listener = tokio::net::TcpListener::bind(&addr).await.map_err(|e| {
        terraphim_llm_proxy::error::ProxyError::Internal(format!(
            "Failed to bind to {}: {}",
            addr, e
        ))
    })?;

    axum::serve(listener, app).await.map_err(|e| {
        terraphim_llm_proxy::error::ProxyError::Internal(format!("Server error: {}", e))
    })?;

    Ok(())
}

fn init_logging(level: &str, json: bool) -> Result<()> {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(level));

    if json {
        // JSON logging for production
        tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer().json())
            .init();
    } else {
        // Pretty logging for development
        tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer())
            .init();
    }

    Ok(())
}
