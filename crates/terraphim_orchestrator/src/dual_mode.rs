//! Dual-mode orchestrator with full integration.
//!
//! Manages both time-driven and issue-driven agent execution modes
//! with shared concurrency control and unified status.

use crate::{
    AgentDefinition, AgentOrchestrator, CompoundReviewResult, ConcurrencyController,
    DispatcherStats, FairnessPolicy, HandoffContext, ModeQuotas, OrchestratorConfig, ScheduleEvent,
    TimeScheduler, WorkflowConfig,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use terraphim_tracker::{GiteaTracker, IssueTracker};
use tokio::sync::{mpsc, watch, Mutex};
use tracing::{error, info, warn};

/// Shared state between time and issue modes.
#[derive(Clone)]
pub struct SharedState {
    /// Concurrency controller for agent limits.
    pub concurrency: ConcurrencyController,
    /// Statistics from both modes.
    pub stats: Arc<Mutex<DualModeStats>>,
    /// Shutdown signal.
    pub shutdown_tx: watch::Sender<bool>,
}

/// Statistics for dual-mode operation.
#[derive(Debug, Default)]
pub struct DualModeStats {
    /// Time-driven statistics.
    pub time_stats: Option<DispatcherStats>,
    /// Issue-driven statistics.
    pub issue_stats: Option<DispatcherStats>,
    /// Total agents spawned.
    pub total_agents_spawned: u64,
    /// Active agents by mode.
    pub active_by_mode: HashMap<String, usize>,
}

/// Agent identifier with mode.
#[derive(Debug, Clone)]
pub struct AgentId {
    /// Agent name or issue identifier.
    pub name: String,
    /// Source mode.
    pub mode: ExecutionMode,
}

/// Execution mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionMode {
    /// Time-driven (cron-based).
    TimeDriven,
    /// Issue-driven (tracker-based).
    IssueDriven,
}

impl std::fmt::Display for ExecutionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionMode::TimeDriven => write!(f, "time"),
            ExecutionMode::IssueDriven => write!(f, "issue"),
        }
    }
}

/// Task spawned by either mode.
#[derive(Debug, Clone)]
pub enum SpawnTask {
    /// Time-driven agent task.
    TimeTask { agent: Box<AgentDefinition> },
    /// Issue-driven agent task.
    IssueTask { issue_id: String, title: String },
}

/// Full dual-mode orchestrator.
pub struct DualModeOrchestrator {
    /// Configuration.
    config: OrchestratorConfig,
    /// Base orchestrator for time-driven mode.
    base: AgentOrchestrator,
    /// Shared state.
    state: SharedState,
    /// Time mode controller (if enabled).
    time_mode: Option<TimeModeComponents>,
    /// Issue mode controller (if enabled).
    issue_mode: Option<IssueModeComponents>,
    /// Task receiver from both modes.
    task_rx: mpsc::Receiver<SpawnTask>,
    /// Task sender for dispatching from modes.
    task_tx: mpsc::Sender<SpawnTask>,
    /// Active agents.
    active_agents: Arc<Mutex<HashMap<String, AgentId>>>,
}

/// Components for time mode.
struct TimeModeComponents {
    scheduler: TimeScheduler,
    shutdown_rx: watch::Receiver<bool>,
}

/// Components for issue mode.
struct IssueModeComponents {
    tracker: Box<dyn IssueTracker>,
    workflow: WorkflowConfig,
    shutdown_rx: watch::Receiver<bool>,
}

impl DualModeOrchestrator {
    /// Create a new dual-mode orchestrator.
    pub fn new(config: OrchestratorConfig) -> Result<Self, crate::OrchestratorError> {
        let base = AgentOrchestrator::new(config.clone())?;

        // Create concurrency controller
        let concurrency = if let Some(ref workflow) = config.workflow {
            ConcurrencyController::new(
                workflow.concurrency.global_max,
                ModeQuotas {
                    time_max: workflow
                        .concurrency
                        .global_max
                        .saturating_sub(workflow.concurrency.issue_max),
                    issue_max: workflow.concurrency.issue_max,
                },
                workflow
                    .concurrency
                    .fairness
                    .parse()
                    .unwrap_or(FairnessPolicy::RoundRobin),
            )
        } else {
            ConcurrencyController::new(10, ModeQuotas::default(), FairnessPolicy::RoundRobin)
        };

        // Create shared state
        let (shutdown_tx, _shutdown_rx) = watch::channel(false);
        let state = SharedState {
            concurrency,
            stats: Arc::new(Mutex::new(DualModeStats::default())),
            shutdown_tx,
        };

        // Create task channel for modes to send spawned tasks to orchestrator
        let (task_tx, task_rx) = mpsc::channel(128);

        // Setup time mode
        let time_mode = {
            let scheduler =
                TimeScheduler::new(&config.agents, Some(&config.compound_review.schedule))?;
            let shutdown_rx = state.shutdown_tx.subscribe();
            Some(TimeModeComponents {
                scheduler,
                shutdown_rx,
            })
        };

        // Setup issue mode if configured
        let issue_mode = if let Some(ref workflow) = config.workflow {
            if workflow.enabled {
                match create_tracker(workflow) {
                    Ok(tracker) => {
                        let shutdown_rx = state.shutdown_tx.subscribe();
                        Some(IssueModeComponents {
                            tracker,
                            workflow: workflow.clone(),
                            shutdown_rx,
                        })
                    }
                    Err(e) => {
                        warn!("failed to create issue tracker: {}", e);
                        None
                    }
                }
            } else {
                None
            }
        } else {
            None
        };

        Ok(Self {
            config,
            base,
            state,
            time_mode,
            issue_mode,
            task_rx,
            task_tx,
            active_agents: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Access the orchestrator configuration.
    pub fn config(&self) -> &OrchestratorConfig {
        &self.config
    }

    /// Run the dual-mode orchestrator.
    pub async fn run(&mut self) -> Result<(), crate::OrchestratorError> {
        info!(
            agents = self.config.agents.len(),
            workflow_enabled = self.config.workflow.as_ref().is_some_and(|w| w.enabled),
            "starting dual-mode orchestrator"
        );

        // Start time mode task
        let mut time_handle = if let Some(time_components) = self.time_mode.take() {
            let state = self.state.clone();
            Some(tokio::spawn(run_time_mode(time_components, state)))
        } else {
            None
        };

        // Start issue mode task
        let mut issue_handle = if let Some(issue_components) = self.issue_mode.take() {
            let state = self.state.clone();
            Some(tokio::spawn(run_issue_mode(issue_components, state)))
        } else {
            None
        };

        // Wait for shutdown signal, task completion, or spawned tasks
        let ctrl_c = tokio::signal::ctrl_c();
        tokio::pin!(ctrl_c);

        // Pin the optional handles for select
        let mut time_done = false;
        let mut issue_done = false;

        loop {
            tokio::select! {
                // Receive spawned tasks from modes and track them
                Some(task) = self.task_rx.recv() => {
                    self.track_spawned_task(task).await;
                }
                // Wait for time mode completion
                result = async {
                    match &mut time_handle {
                        Some(h) => h.await,
                        None => std::future::pending().await,
                    }
                }, if !time_done => {
                    time_done = true;
                    match result {
                        Ok(()) => info!("time mode completed"),
                        Err(e) => error!("time mode panicked: {}", e),
                    }
                    if issue_done { break; }
                }
                // Wait for issue mode completion
                result = async {
                    match &mut issue_handle {
                        Some(h) => h.await,
                        None => std::future::pending().await,
                    }
                }, if !issue_done => {
                    issue_done = true;
                    match result {
                        Ok(()) => info!("issue mode completed"),
                        Err(e) => error!("issue mode panicked: {}", e),
                    }
                    if time_done { break; }
                }
                // Base orchestrator reconciliation loop
                result = self.base.run() => {
                    match result {
                        Ok(()) => info!("base orchestrator completed"),
                        Err(e) => error!("base orchestrator error: {}", e),
                    }
                    break;
                }
                _ = &mut ctrl_c => {
                    info!("shutdown signal received");
                    let _ = self.state.shutdown_tx.send(true);
                    break;
                }
            }
        }

        // Graceful shutdown
        info!("shutting down dual-mode orchestrator");
        self.shutdown().await;

        Ok(())
    }

    /// Track a spawned task from either mode.
    async fn track_spawned_task(&self, task: SpawnTask) {
        let mut stats = self.state.stats.lock().await;
        stats.total_agents_spawned += 1;
        match &task {
            SpawnTask::TimeTask { agent } => {
                info!(agent_name = %agent.name, "received time-driven spawn task");
                let mut agents = self.active_agents.lock().await;
                agents.insert(
                    agent.name.clone(),
                    AgentId {
                        name: agent.name.clone(),
                        mode: ExecutionMode::TimeDriven,
                    },
                );
                *stats.active_by_mode.entry("time".into()).or_insert(0) += 1;
            }
            SpawnTask::IssueTask { issue_id, title } => {
                info!(issue_id = %issue_id, title = %title, "received issue-driven spawn task");
                let mut agents = self.active_agents.lock().await;
                agents.insert(
                    issue_id.clone(),
                    AgentId {
                        name: issue_id.clone(),
                        mode: ExecutionMode::IssueDriven,
                    },
                );
                *stats.active_by_mode.entry("issue".into()).or_insert(0) += 1;
            }
        }
    }

    /// Get a clone of the task sender for external dispatch.
    pub fn task_sender(&self) -> mpsc::Sender<SpawnTask> {
        self.task_tx.clone()
    }

    /// Request shutdown.
    pub fn request_shutdown(&self) {
        let _ = self.state.shutdown_tx.send(true);
    }

    /// Shutdown gracefully.
    async fn shutdown(&mut self) {
        info!("initiating graceful shutdown");

        // Stop accepting new tasks
        self.request_shutdown();

        // Wait for active agents to complete
        let timeout = Duration::from_secs(30);
        let start = std::time::Instant::now();

        loop {
            let active_count = {
                let agents = self.active_agents.lock().await;
                agents.len()
            };

            if active_count == 0 {
                info!("all agents completed");
                break;
            }

            if start.elapsed() > timeout {
                warn!(
                    "shutdown timeout reached with {} agents still active",
                    active_count
                );
                break;
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        // Shutdown base orchestrator
        self.base.shutdown();

        info!("shutdown complete");
    }

    /// Get current statistics.
    pub async fn stats(&self) -> DualModeStats {
        let stats = self.state.stats.lock().await;
        stats.clone()
    }

    /// Get active agent count.
    pub async fn active_count(&self) -> usize {
        let agents = self.active_agents.lock().await;
        agents.len()
    }

    /// Trigger compound review.
    pub async fn trigger_compound_review(
        &mut self,
        git_ref: &str,
        base_ref: &str,
    ) -> Result<CompoundReviewResult, crate::OrchestratorError> {
        self.base.trigger_compound_review(git_ref, base_ref).await
    }

    /// Handoff task between agents.
    pub async fn handoff(
        &mut self,
        from_agent: &str,
        to_agent: &str,
        ctx: HandoffContext,
    ) -> Result<(), crate::OrchestratorError> {
        self.base.handoff(from_agent, to_agent, ctx).await
    }
}

/// Run time mode in background.
async fn run_time_mode(components: TimeModeComponents, state: SharedState) {
    info!("starting time mode task");

    let TimeModeComponents {
        mut scheduler,
        mut shutdown_rx,
    } = components;

    // Get immediate agents (Safety layer)
    let immediate = scheduler.immediate_agents();
    for agent in immediate {
        info!(agent_name = %agent.name, "spawning immediate Safety agent");
        // Safety agents spawn without concurrency limit
    }

    loop {
        tokio::select! {
            event = scheduler.next_event() => {
                match event {
                    ScheduleEvent::Spawn(agent) => {
                        // Try to acquire time-driven slot
                        match state.concurrency.acquire_time_driven().await {
                            Some(permit) => {
                                info!(agent_name = %agent.name, "spawning time-driven agent");
                                // Spawn agent here
                                drop(permit);
                            }
                            None => {
                                warn!(agent_name = %agent.name, "no slot available for time-driven agent");
                            }
                        }
                    }
                    ScheduleEvent::Stop { agent_name } => {
                        info!(agent_name = %agent_name, "stopping agent");
                    }
                    ScheduleEvent::CompoundReview => {
                        info!("compound review triggered");
                    }
                    ScheduleEvent::Flow(flow) => {
                        info!(flow_name = %flow.name, "flow triggered");
                    }
                }
            }
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    info!("time mode shutting down");
                    break;
                }
            }
        }
    }
}

/// Run issue mode in background.
async fn run_issue_mode(components: IssueModeComponents, state: SharedState) {
    info!("starting issue mode task");

    let IssueModeComponents {
        tracker,
        workflow,
        mut shutdown_rx,
    } = components;

    let poll_interval = Duration::from_secs(workflow.poll_interval_secs);

    loop {
        tokio::select! {
            _ = tokio::time::sleep(poll_interval) => {
                match tracker.fetch_candidate_issues().await {
                    Ok(issues) => {
                        info!(count = issues.len(), "fetched candidate issues");

                        for issue in issues {
                            // Skip blocked issues
                            if !issue.all_blockers_terminal(&workflow.tracker.states.terminal) {
                                continue;
                            }

                            // Try to acquire issue-driven slot
                            match state.concurrency.acquire_issue_driven().await {
                                Some(permit) => {
                                    info!(
                                        issue_id = %issue.id,
                                        title = %issue.title,
                                        "dispatching issue-driven agent"
                                    );
                                    // Spawn agent here
                                    drop(permit);
                                }
                                None => {
                                    warn!("no slot available for issue-driven agent");
                                    break; // Stop trying until slots free up
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("failed to fetch issues: {}", e);
                    }
                }
            }
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    info!("issue mode shutting down");
                    break;
                }
            }
        }
    }
}

/// Create tracker from workflow config.
fn create_tracker(workflow: &WorkflowConfig) -> Result<Box<dyn IssueTracker>, String> {
    match workflow.tracker.kind.as_str() {
        "gitea" => {
            use terraphim_tracker::gitea::GiteaConfig;
            let tracker = GiteaTracker::new(GiteaConfig {
                base_url: workflow.tracker.endpoint.clone(),
                token: workflow.tracker.api_key.clone(),
                owner: workflow.tracker.owner.clone(),
                repo: workflow.tracker.repo.clone(),
                active_states: workflow.tracker.states.active.clone(),
                terminal_states: workflow.tracker.states.terminal.clone(),
                use_robot_api: workflow.tracker.use_robot_api,
            })
            .map_err(|e| format!("failed to create Gitea tracker: {}", e))?;

            Ok(Box::new(tracker))
        }
        "linear" => {
            use terraphim_tracker::{LinearConfig, LinearTracker};
            let project_slug = workflow
                .tracker
                .project_slug
                .clone()
                .ok_or("project_slug required for linear tracker")?;
            let tracker = LinearTracker::new(LinearConfig {
                endpoint: workflow.tracker.endpoint.clone(),
                api_key: workflow.tracker.api_key.clone(),
                project_slug,
                active_states: workflow.tracker.states.active.clone(),
                terminal_states: workflow.tracker.states.terminal.clone(),
            })
            .map_err(|e| format!("failed to create Linear tracker: {}", e))?;

            Ok(Box::new(tracker))
        }
        _ => Err(format!(
            "unsupported tracker kind: {}",
            workflow.tracker.kind
        )),
    }
}

impl Clone for DualModeStats {
    fn clone(&self) -> Self {
        Self {
            time_stats: self.time_stats.clone(),
            issue_stats: self.issue_stats.clone(),
            total_agents_spawned: self.total_agents_spawned,
            active_by_mode: self.active_by_mode.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_mode_display() {
        assert_eq!(ExecutionMode::TimeDriven.to_string(), "time");
        assert_eq!(ExecutionMode::IssueDriven.to_string(), "issue");
    }

    #[test]
    fn test_dual_mode_stats_default() {
        let stats = DualModeStats::default();
        assert_eq!(stats.total_agents_spawned, 0);
        assert!(stats.time_stats.is_none());
        assert!(stats.issue_stats.is_none());
    }
}
