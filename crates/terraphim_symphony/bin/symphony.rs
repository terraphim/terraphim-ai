//! Symphony CLI entrypoint.
//!
//! Parses command-line arguments, loads the WORKFLOW.md, and starts
//! the orchestrator main loop.

use async_trait::async_trait;
use clap::Parser;
use std::path::PathBuf;
use tracing::info;
use tracing_subscriber::EnvFilter;

use terraphim_symphony::SymphonyError;
use terraphim_symphony::config::ServiceConfig;
use terraphim_symphony::orchestrator::SymphonyOrchestrator;
use terraphim_symphony::tracker::gitea::GiteaTracker;
use terraphim_symphony::workspace::WorkspaceManager;
use terraphim_tracker::{IssueTracker as _, LinearConfig, LinearTracker};

struct LinearTrackerAdapter {
    inner: LinearTracker,
}

impl LinearTrackerAdapter {
    fn new(inner: LinearTracker) -> Self {
        Self { inner }
    }

    fn map_issue(issue: terraphim_tracker::Issue) -> terraphim_symphony::Issue {
        terraphim_symphony::Issue {
            id: issue.id,
            identifier: issue.identifier,
            title: issue.title,
            description: issue.description,
            priority: issue.priority,
            state: issue.state,
            branch_name: issue.branch_name,
            url: issue.url,
            labels: issue.labels,
            blocked_by: issue
                .blocked_by
                .into_iter()
                .map(|blocker| terraphim_symphony::tracker::BlockerRef {
                    id: blocker.id,
                    identifier: blocker.identifier,
                    state: blocker.state,
                })
                .collect(),
            pagerank_score: issue.pagerank_score,
            created_at: None,
            updated_at: None,
        }
    }

    fn map_error(error: terraphim_tracker::TrackerError) -> SymphonyError {
        SymphonyError::Tracker {
            kind: "linear".into(),
            message: error.to_string(),
        }
    }
}

#[async_trait]
impl terraphim_symphony::IssueTracker for LinearTrackerAdapter {
    async fn fetch_candidate_issues(
        &self,
    ) -> terraphim_symphony::Result<Vec<terraphim_symphony::Issue>> {
        self.inner
            .fetch_candidate_issues()
            .await
            .map(|issues| issues.into_iter().map(Self::map_issue).collect())
            .map_err(Self::map_error)
    }

    async fn fetch_issue_states_by_ids(
        &self,
        ids: &[String],
    ) -> terraphim_symphony::Result<Vec<terraphim_symphony::Issue>> {
        self.inner
            .fetch_issue_states_by_ids(ids)
            .await
            .map(|issues| issues.into_iter().map(Self::map_issue).collect())
            .map_err(Self::map_error)
    }

    async fn fetch_issues_by_states(
        &self,
        states: &[String],
    ) -> terraphim_symphony::Result<Vec<terraphim_symphony::Issue>> {
        self.inner
            .fetch_issues_by_states(states)
            .await
            .map(|issues| issues.into_iter().map(Self::map_issue).collect())
            .map_err(Self::map_error)
    }
}

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
            Some("linear") => {
                let api_key = config.tracker_api_key().ok_or_else(|| {
                    SymphonyError::AuthenticationMissing {
                        service: "linear".into(),
                    }
                })?;
                let project_slug = config.tracker_project_slug().ok_or_else(|| {
                    SymphonyError::ValidationFailed {
                        checks: vec!["tracker.project_slug is required for linear".into()],
                    }
                })?;

                Box::new(LinearTrackerAdapter::new(LinearTracker::new(
                    LinearConfig {
                        endpoint: config.tracker_endpoint(),
                        api_key,
                        project_slug,
                        active_states: config.active_states(),
                        terminal_states: config.terminal_states(),
                    },
                )?))
            }
            Some("gitea") => Box::new(GiteaTracker::from_config(&config)?),
            Some(kind) => {
                return Err(SymphonyError::UnsupportedTrackerKind { kind: kind.into() }.into());
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
