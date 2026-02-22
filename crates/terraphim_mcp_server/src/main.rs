//! ## Logging and OpenDAL Warning Messages
//!
//! This server uses OpenDAL library for storage operations. You may see
//! WARN-level messages about "NotFound" errors when reading configuration files:
//! ```text
//! [WARN  opendal::services] service=memory name=0x... path=embedded_config.json: read failed NotFound (permanent)
//! ```
//!
//! These messages are **expected and harmless** - they occur when OpenDAL attempts
//! to read configuration files that don't exist yet. The system correctly falls back
//! to default values and continues normal operation.
//!
//! ### Why These Warnings Appear
//!
//! OpenDAL has an internal `LoggingLayer` that logs directly to the Rust `log` crate.
//! This logging is independent of application logging configuration and occurs before
//! our tracing setup takes effect.
//!
//! ### Suppressing These Warnings
//!
//! If you want cleaner logs (without these expected warnings), you can set the
//! `RUST_LOG` environment variable:
//!
//! ```bash
//! # Option 1: Suppress all warnings (includes real ones)
//! RUST_LOG=error terraphim-mcp-server
//!
//! # Option 2: Suppress OpenDAL-specific warnings
//! RUST_LOG="opendal=error" terraphim-mcp-server
//!
//! # Option 3: Use quieter mode
//! RUST_LOG=warn terraphim-mcp-server
//! ```

use std::sync::Arc;

use anyhow::Result;
use clap::{Parser, ValueEnum};
use rmcp::{
    ServiceExt,
    transport::{
        sse_server::{SseServer, SseServerConfig},
        stdio,
    },
};
use terraphim_config::{ConfigBuilder, ConfigState};
use terraphim_mcp_server::McpService;
use tracing::{Level, info};

#[derive(Parser, Debug)]
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

    /// Start SSE server instead of stdio transport
    #[arg(long, default_value_t = false)]
    sse: bool,

    /// SSE bind address (when --sse)
    #[arg(long, default_value = "127.0.0.1:8000")]
    bind: String,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum ConfigProfile {
    /// Use desktop configuration (Terraphim Engineer role with local KG)
    Desktop,
    /// Use server configuration (Default role without KG)
    Server,
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to listen for ctrl+c");
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let args = Args::parse();

    // Standardized tracing setup
    let level = if args.verbose {
        Level::DEBUG
    } else {
        Level::INFO
    };
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(level)
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env().add_directive(level.into()),
        );

    if args.sse {
        // SSE mode needs timestamps for server logs - write to stdout
        subscriber.init();
    } else {
        // Stdio mode: write logs to stderr to avoid mixing with JSON-RPC responses on stdout
        subscriber
            .without_time()
            .with_writer(std::io::stderr)
            .init();
    }

    info!("Starting Terraphim MCP Server...");
    info!("Args: {:?}", args);

    // Build configuration based on selected profile
    let config = match args.profile {
        ConfigProfile::Desktop => {
            info!("Using desktop configuration (Terraphim Engineer role with local KG)");
            ConfigBuilder::new()
                .build_default_desktop()
                .build()
                .expect("Failed to build default desktop configuration")
        }
        ConfigProfile::Server => {
            info!("Using server configuration (Default role without KG)");
            ConfigBuilder::new()
                .build_default_server()
                .build()
                .expect("Failed to build default server configuration")
        }
    };

    // Initialize ConfigState from the config
    let mut temp_config = config.clone();
    let config_state = ConfigState::new(&mut temp_config)
        .await
        .expect("Failed to create config state from config");

    // Create the MCP service
    let service = McpService::new(Arc::new(config_state));

    if args.sse {
        info!("Starting SSE server on {}", args.bind);

        // Start SSE server
        let config = SseServerConfig {
            bind: args.bind.parse().expect("Invalid bind address"),
            sse_path: "/sse".to_string(),
            post_path: "/message".to_string(),
            ct: tokio_util::sync::CancellationToken::new(),
            sse_keep_alive: None,
        };

        let (sse_server, router) = SseServer::new(config);
        let listener = tokio::net::TcpListener::bind(sse_server.config.bind).await?;
        let ct = sse_server.config.ct.child_token();

        let server = axum::serve(listener, router).with_graceful_shutdown(async move {
            ct.cancelled().await;
            info!("SSE server cancelled");
        });

        tokio::spawn(async move {
            if let Err(e) = server.await {
                tracing::error!(error = %e, "SSE server shutdown with error");
            }
        });

        let _ct = sse_server.with_service(move || service.clone());

        // Wait for shutdown signal
        shutdown_signal().await;
    } else {
        info!("Starting stdio server");

        // Initialize autocomplete index by default
        service.init_autocomplete_default().await;
        info!("Initialized Terraphim MCP service");

        // Start stdio server
        let mcp_service = service.serve(stdio()).await?;
        mcp_service.waiting().await?;
    }

    Ok(())
}
