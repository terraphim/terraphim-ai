//! Symphony CLI entrypoint.
//!
//! Parses command-line arguments, loads the WORKFLOW.md, and starts
//! the orchestrator main loop.

use clap::Parser;
use std::path::PathBuf;
use tracing::info;
use tracing_subscriber::EnvFilter;

use terraphim_symphony::config::ServiceConfig;
use terraphim_symphony::orchestrator::SymphonyOrchestrator;
use terraphim_symphony::tracker::gitea::GiteaTracker;
use terraphim_symphony::tracker::linear::LinearTracker;
use terraphim_symphony::workspace::WorkspaceManager;
use terraphim_symphony::SymphonyError;

/// Symphony orchestration service.
///
/// Continuously reads work from issue trackers and dispatches
/// coding agent sessions for each issue.
#[derive(Parser)]
#[command(name = "symphony", version, about)]
struct Cli {
    /// Path to WORKFLOW.md
    #[arg(default_value = "WORKFLOW.md")]
    workflow: PathBuf,

    /// HTTP server port (overrides server.port in WORKFLOW.md)
    #[arg(long)]
    port: Option<u16>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialise logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(true)
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();

    info!(workflow = %cli.workflow.display(), "loading configuration");

    // Load and validate configuration
    let config = ServiceConfig::load(&cli.workflow)?;
    config.validate_for_dispatch()?;

    // Build the tracker client
    let tracker: Box<dyn terraphim_symphony::IssueTracker> =
        match config.tracker_kind().as_deref() {
            Some("linear") => Box::new(LinearTracker::from_config(&config)?),
            Some("gitea") => Box::new(GiteaTracker::from_config(&config)?),
            Some(kind) => {
                return Err(SymphonyError::UnsupportedTrackerKind {
                    kind: kind.into(),
                }
                .into());
            }
            None => {
                return Err(SymphonyError::ValidationFailed {
                    checks: vec!["tracker.kind is required".into()],
                }
                .into());
            }
        };

    // Build the workspace manager
    let workspace_mgr = WorkspaceManager::new(&config)?;

    // Build and run the orchestrator
    let mut orchestrator = SymphonyOrchestrator::new(config, tracker, workspace_mgr);

    info!("starting orchestrator");
    orchestrator.run().await?;

    info!("orchestrator shut down");
    Ok(())
}
