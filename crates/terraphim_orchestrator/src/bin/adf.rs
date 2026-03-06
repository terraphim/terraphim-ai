use std::path::PathBuf;

use terraphim_orchestrator::AgentOrchestrator;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let config_path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/opt/ai-dark-factory/orchestrator.toml"));

    tracing::info!(config = %config_path.display(), "loading orchestrator config");

    let mut orchestrator = AgentOrchestrator::from_config_file(&config_path)?;

    // Handle SIGTERM/SIGINT for graceful shutdown
    let shutdown_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let flag = shutdown_flag.clone();

    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        tracing::info!("received shutdown signal");
        flag.store(true, std::sync::atomic::Ordering::SeqCst);
    });

    tracing::info!("starting AI Dark Factory orchestrator");
    orchestrator.run().await?;

    Ok(())
}
