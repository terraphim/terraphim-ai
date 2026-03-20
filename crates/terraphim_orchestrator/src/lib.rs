pub mod compound;
pub mod config;
pub mod convergence_detector;
pub mod dispatcher;
pub mod drift_detection;
pub mod error;
pub mod handoff;
pub mod issue_mode;
pub mod nightwatch;
pub mod scheduler;
pub mod session_rotation;

pub use compound::{CompoundReviewResult, CompoundReviewWorkflow};
pub use config::{
    AgentDefinition, AgentLayer, CompoundReviewConfig, ConcurrencyConfig, ConvergenceConfig,
    DriftDetectionConfig, NightwatchConfig, OrchestratorConfig, ReviewPair, SessionRotationConfig,
    TrackerConfig, TrackerType, WorkflowConfig, WorkflowMode,
};
pub use convergence_detector::{ConvergenceDetector, ConvergenceSignal};
pub use dispatcher::{ConcurrencyController, DispatchQueue, DispatchTask, DispatcherError};
pub use drift_detection::{DriftDetector, DriftReport};
pub use error::OrchestratorError;
pub use issue_mode::IssueMode;
pub use session_rotation::{AgentSession, SessionRotationManager};
pub use handoff::HandoffContext;
pub use nightwatch::{
    CorrectionAction, CorrectionLevel, DriftAlert, DriftMetrics, DriftScore, NightwatchMonitor,
    RateLimitTracker, RateLimitWindow,
};
pub use scheduler::{ScheduleEvent, TimeMode, TimeScheduler};

use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, Instant};

use terraphim_router::RoutingEngine;
use terraphim_spawner::health::HealthStatus;
use terraphim_spawner::output::OutputEvent;
use terraphim_spawner::{AgentHandle, AgentSpawner};
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

/// A request for cross-agent review.
#[derive(Debug, Clone)]
pub struct ReviewRequest {
    pub from_agent: String,
    pub to_agent: String,
    pub artifact_path: String,
    pub review_type: String,
}



/// Status of a single agent in the fleet.
#[derive(Debug, Clone)]
pub struct AgentStatus {
    pub name: String,
    pub layer: AgentLayer,
    pub running: bool,
    pub health: HealthStatus,
    pub drift_score: Option<f64>,
    pub uptime: Duration,
    pub restart_count: u32,
    /// API calls remaining per provider (None if no limit known).
    pub api_calls_remaining: HashMap<String, Option<u32>>,
}

/// Runtime state for a managed agent.
struct ManagedAgent {
    definition: AgentDefinition,
    handle: AgentHandle,
    started_at: Instant,
    restart_count: u32,
    /// Broadcast receiver for draining output events to nightwatch.
    output_rx: broadcast::Receiver<OutputEvent>,
}

/// Coordinates between TimeMode and IssueMode in dual mode operation.
pub struct ModeCoordinator {
    /// Time-based scheduler mode.
    pub time_mode: Option<TimeMode>,
    /// Issue-driven mode.
    pub issue_mode: Option<IssueMode>,
    /// Shared dispatch queue.
    pub dispatch_queue: DispatchQueue,
    /// Current workflow mode.
    pub workflow_mode: WorkflowMode,
    /// Shutdown signal sender for issue mode (only available when issue mode is active).
    pub issue_shutdown_tx: Option<tokio::sync::mpsc::Sender<()>>,
    /// Concurrency controller for limiting parallel execution.
    pub concurrency_controller: ConcurrencyController,
}

impl ModeCoordinator {
    /// Create a new mode coordinator based on workflow configuration.
    pub fn new(
        workflow_config: WorkflowConfig,
        agents: Vec<AgentDefinition>,
        tracker_config: Option<terraphim_tracker::TrackerConfig>,
        compound_schedule: Option<String>,
    ) -> Result<(Self, tokio::sync::mpsc::Receiver<()>), OrchestratorError> {
        let dispatch_queue = DispatchQueue::new(workflow_config.max_concurrent_tasks as usize * 10);
        let concurrency_controller = ConcurrencyController::new(
            workflow_config.max_concurrent_tasks as usize,
            300, // 5 minute starvation timeout
        );

        let (_coord_shutdown_tx, coord_shutdown_rx) = tokio::sync::mpsc::channel(1);

        let time_mode = if matches!(workflow_config.mode, WorkflowMode::TimeOnly | WorkflowMode::Dual) {
            let tm = TimeMode::new_with_queue(
                &agents,
                compound_schedule.as_deref(),
                dispatch_queue.clone(),
            )?;
            Some(tm)
        } else {
            None
        };

        let (issue_mode, issue_shutdown_tx) = if matches!(workflow_config.mode, WorkflowMode::IssueOnly | WorkflowMode::Dual) {
            if let Some(tracker_cfg) = tracker_config {
                let (im, tx) = IssueMode::new(
                    tracker_cfg,
                    dispatch_queue.clone(),
                    agents,
                    workflow_config.poll_interval_secs,
                )?;
                (Some(im), Some(tx))
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

        Ok((Self {
            time_mode,
            issue_mode,
            dispatch_queue,
            workflow_mode: workflow_config.mode,
            issue_shutdown_tx,
            concurrency_controller,
        }, coord_shutdown_rx))
    }

    /// Get the next task from the dispatch queue.
    /// Note: This requires mutable access due to the BinaryHeap's pop operation.
    pub fn next_task(&mut self) -> Option<DispatchTask> {
        self.dispatch_queue.next()
    }

    /// Check if queue is above stall threshold.
    pub fn is_stalled(&self, threshold: usize) -> bool {
        self.dispatch_queue.len() > threshold
    }

    /// Get current queue depth.
    pub fn queue_depth(&self) -> usize {
        self.dispatch_queue.len()
    }

    /// Try to acquire concurrency permit.
    pub fn try_acquire_permit(&self) -> Option<tokio::sync::SemaphorePermit<'_>> {
        self.concurrency_controller.try_acquire()
    }

    /// Signal shutdown to issue mode if active.
    pub async fn shutdown(&self) {
        if let Some(ref tx) = self.issue_shutdown_tx {
            let _ = tx.send(()).await;
        }
    }
}

/// The main orchestrator that runs the dark factory.
pub struct AgentOrchestrator {
    config: OrchestratorConfig,
    spawner: AgentSpawner,
    router: RoutingEngine,
    nightwatch: NightwatchMonitor,
    scheduler: TimeScheduler,
    compound_workflow: CompoundReviewWorkflow,
    active_agents: HashMap<String, ManagedAgent>,
    rate_limiter: RateLimitTracker,
    shutdown_requested: bool,
    /// Total restart count per agent (persists across agent lifecycle).
    restart_counts: HashMap<String, u32>,
    /// Last exit time per agent (for cooldown enforcement).
    restart_cooldowns: HashMap<String, Instant>,
    /// Timestamp of the last reconciliation tick (for cron comparison).
    last_tick_time: chrono::DateTime<chrono::Utc>,
    /// Queue of pending cross-agent review requests.
    review_queue: Vec<ReviewRequest>,
    /// Strategic drift detector.
    drift_detector: DriftDetector,
    /// Session rotation manager for fresh eyes.
    session_rotation: SessionRotationManager,
    /// Mode coordinator for dual mode operation.
    mode_coordinator: Option<ModeCoordinator>,
    /// Shared dispatch queue for dual mode operation.
    dispatch_queue: Option<DispatchQueue>,
    /// Shutdown signal senders for mode tasks.
    mode_shutdown_tx: Option<tokio::sync::mpsc::Sender<()>>,
    /// Stall detection threshold.
    stall_threshold: usize,
}

impl AgentOrchestrator {
    /// Create a new orchestrator from configuration.
    pub fn new(config: OrchestratorConfig) -> Result<Self, OrchestratorError> {
        let spawner = AgentSpawner::new().with_working_dir(&config.working_dir);
        let router = RoutingEngine::new();
        let nightwatch = NightwatchMonitor::new(config.nightwatch.clone());
        let scheduler = TimeScheduler::new(&config.agents, Some(&config.compound_review.schedule))?;
        let compound_workflow = CompoundReviewWorkflow::new(config.compound_review.clone());
        let drift_detector = DriftDetector::new(
            config.drift_detection.check_interval_ticks,
            config.drift_detection.drift_threshold,
            &config.drift_detection.plans_dir,
        );
        let mut session_rotation = SessionRotationManager::new(
            config.session_rotation.max_sessions_before_rotation,
        );
        if let Some(duration_secs) = config.session_rotation.max_session_duration_secs {
            session_rotation = session_rotation.with_duration(Duration::from_secs(duration_secs));
        }

        // Initialize mode coordinator if workflow config is present
        let mode_coordinator = if let Some(workflow) = &config.workflow {
            let tracker_cfg = config.tracker.as_ref().map(|t| terraphim_tracker::TrackerConfig {
                url: t.url.clone(),
                token: std::env::var(&t.token_env_var).unwrap_or_default(),
                owner: t.owner.clone(),
                repo: t.repo.clone(),
                robot_url: None,
            });
            let (coord, _) = ModeCoordinator::new(
                workflow.clone(),
                config.agents.clone(),
                tracker_cfg,
                Some(config.compound_review.schedule.clone()),
            )?;
            Some(coord)
        } else {
            None
        };

        let stall_threshold = config.concurrency.as_ref()
            .map(|c| c.queue_depth as usize)
            .unwrap_or(100);

        Ok(Self {
            config,
            spawner,
            router,
            nightwatch,
            scheduler,
            compound_workflow,
            active_agents: HashMap::new(),
            rate_limiter: RateLimitTracker::default(),
            shutdown_requested: false,
            restart_counts: HashMap::new(),
            restart_cooldowns: HashMap::new(),
            last_tick_time: chrono::Utc::now(),
            review_queue: Vec::new(),
            drift_detector,
            session_rotation,
            mode_coordinator,
            dispatch_queue: None,
            mode_shutdown_tx: None,
            stall_threshold,
        })
    }

    /// Create from a TOML config file path.
    pub fn from_config_file(path: impl AsRef<Path>) -> Result<Self, OrchestratorError> {
        let config = OrchestratorConfig::from_file(path)?;
        Self::new(config)
    }

    /// Run the orchestrator (blocks until shutdown signal).
    ///
    /// 1. Spawns all Safety-layer agents immediately
    /// 2. Enters the select! loop handling schedule events, drift alerts, and periodic tick
    pub async fn run(&mut self) -> Result<(), OrchestratorError> {
        info!(
            "starting orchestrator with {} agent definitions",
            self.config.agents.len()
        );

        // Spawn Safety-layer agents with stagger delay (thundering herd prevention)
        let immediate = self.scheduler.immediate_agents();
        let stagger_delay = Duration::from_millis(self.config.stagger_delay_ms);
        for (idx, agent_def) in immediate.iter().enumerate() {
            if idx > 0 {
                // Stagger spawns to prevent thundering herd
                tokio::time::sleep(stagger_delay).await;
            }
            if let Err(e) = self.spawn_agent(agent_def).await {
                error!(agent = %agent_def.name, error = %e, "failed to spawn safety agent");
            }
        }

        info!(
            safety_agents = immediate.len(),
            active = self.active_agents.len(),
            "safety agents spawned, entering reconciliation loop"
        );

        // Reconciliation tick interval
        let mut tick = tokio::time::interval(Duration::from_secs(self.config.tick_interval_secs));

        // Main reconciliation loop
        loop {
            if self.shutdown_requested {
                info!("shutdown requested, stopping reconciliation loop");
                break;
            }

            tokio::select! {
                event = self.scheduler.next_event() => {
                    self.handle_schedule_event(event).await;
                }
                alert = self.nightwatch.next_alert() => {
                    self.handle_drift_alert(alert).await;
                }
                _ = tick.tick() => {
                    self.reconcile_tick().await;
                }
            }
        }

        // Graceful shutdown of all agents
        self.shutdown_all_agents().await;
        Ok(())
    }

    /// Request graceful shutdown of all agents and the orchestrator.
    pub fn shutdown(&mut self) {
        info!("shutdown requested");
        self.shutdown_requested = true;
    }

    /// Unified shutdown with queue draining and active task waiting.
    /// Signals all modes, drains the dispatch queue, and waits for active tasks to complete.
    pub async fn unified_shutdown(&mut self) {
        info!("starting unified shutdown");

        // Set shutdown flag
        self.shutdown_requested = true;

        // Signal mode shutdown
        if let Some(ref coord) = self.mode_coordinator {
            coord.shutdown().await;
            info!("mode coordinator shutdown signaled");
        }

        // Drain dispatch queue
        if let Some(ref mut coord) = self.mode_coordinator {
            let mut drained = 0;
            while coord.next_task().is_some() {
                drained += 1;
            }
            info!("drained {} tasks from dispatch queue", drained);
        }

        // Wait for active tasks to complete (up to timeout)
        let shutdown_timeout = Duration::from_secs(30);
        let deadline = Instant::now() + shutdown_timeout;

        while Instant::now() < deadline {
            let active_count = self.active_agents.len();
            if active_count == 0 {
                info!("all agents completed, shutdown complete");
                break;
            }
            info!(
                active_count = active_count,
                "waiting for agents to complete..."
            );
            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        // Force shutdown any remaining agents
        let remaining = self.active_agents.len();
        if remaining > 0 {
            warn!(
                remaining = remaining,
                "timeout reached, force stopping remaining agents"
            );
            self.shutdown_all_agents().await;
        }

        info!("unified shutdown complete");
    }

    /// Check for stall condition and log warning if detected.
    /// Returns true if stalled.
    pub fn check_stall(&self) -> bool {
        if let Some(ref coord) = self.mode_coordinator {
            if coord.is_stalled(self.stall_threshold) {
                let depth = coord.queue_depth();
                warn!(
                    queue_depth = depth,
                    threshold = self.stall_threshold,
                    "STALL DETECTED: Queue depth exceeded threshold"
                );
                return true;
            }
        }
        false
    }

    /// Dispatch tasks from the queue to agents using the spawner.
    /// Returns the number of tasks dispatched.
    pub async fn dispatch_from_queue(&mut self) -> usize {
        let mut dispatched = 0;

        // Try to acquire a concurrency permit first
        let has_permit = self
            .mode_coordinator
            .as_ref()
            .and_then(|c| c.try_acquire_permit())
            .is_some();

        if !has_permit {
            debug!("concurrency limit reached, skipping dispatch");
            return 0;
        }

        // Get next task from queue - this needs mutable borrow
        let task = self
            .mode_coordinator
            .as_mut()
            .and_then(|c| c.next_task());

        if let Some(task) = task {
            match task {
                DispatchTask::TimeTask(agent_name, _schedule) => {
                    if let Some(agent_def) =
                        self.config.agents.iter().find(|a| a.name == agent_name).cloned()
                    {
                        if let Err(e) = self.spawn_agent(&agent_def).await {
                            error!(agent = %agent_name, error = %e, "failed to dispatch time task");
                        } else {
                            info!(agent = %agent_name, "dispatched time task");
                            dispatched += 1;
                        }
                    } else {
                        warn!(agent = %agent_name, "agent not found for time task");
                    }
                }
                DispatchTask::IssueTask(agent_name, issue_id, _priority) => {
                    if let Some(agent_def) =
                        self.config.agents.iter().find(|a| a.name == agent_name).cloned()
                    {
                        if let Err(e) = self.spawn_agent(&agent_def).await {
                            error!(agent = %agent_name, issue_id = issue_id, error = %e, "failed to dispatch issue task");
                        } else {
                            info!(agent = %agent_name, issue_id = issue_id, "dispatched issue task");
                            dispatched += 1;
                        }
                    } else {
                        warn!(agent = %agent_name, "agent not found for issue task");
                    }
                }
            }
        }

        dispatched
    }

    /// Get the current mode coordinator (if dual mode is configured).
    pub fn mode_coordinator(&self) -> Option<&ModeCoordinator> {
        self.mode_coordinator.as_ref()
    }

    /// Get a mutable reference to the mode coordinator.
    pub fn mode_coordinator_mut(&mut self) -> Option<&mut ModeCoordinator> {
        self.mode_coordinator.as_mut()
    }

    /// Get the current workflow mode (if configured).
    pub fn workflow_mode(&self) -> Option<WorkflowMode> {
        self.mode_coordinator.as_ref().map(|c| c.workflow_mode)
    }

    /// Get current status of all agents.
    pub fn agent_statuses(&self) -> Vec<AgentStatus> {
        self.active_agents
            .values()
            .map(|managed| {
                let drift = self.nightwatch.drift_score(&managed.definition.name);
                AgentStatus {
                    name: managed.definition.name.clone(),
                    layer: managed.definition.layer,
                    running: true,
                    health: managed.handle.health_status(),
                    drift_score: drift.map(|d| d.score),
                    uptime: managed.started_at.elapsed(),
                    restart_count: managed.restart_count,
                    api_calls_remaining: HashMap::new(),
                }
            })
            .collect()
    }

    /// Manually trigger a compound review (outside normal schedule).
    pub async fn trigger_compound_review(
        &mut self,
    ) -> Result<CompoundReviewResult, OrchestratorError> {
        info!("triggering manual compound review");
        self.compound_workflow.run().await
    }

    /// Hand off a task from one agent to another.
    pub async fn handoff(
        &mut self,
        from_agent: &str,
        to_agent: &str,
        context: HandoffContext,
    ) -> Result<(), OrchestratorError> {
        if !self.active_agents.contains_key(from_agent) {
            return Err(OrchestratorError::AgentNotFound(from_agent.to_string()));
        }

        // Find the target agent definition
        let to_def = self
            .config
            .agents
            .iter()
            .find(|a| a.name == to_agent)
            .cloned()
            .ok_or_else(|| OrchestratorError::AgentNotFound(to_agent.to_string()))?;

        // If target isn't running, spawn it
        if !self.active_agents.contains_key(to_agent) {
            self.spawn_agent(&to_def).await?;
        }

        // Write handoff context to file for the target agent
        let handoff_path = self
            .config
            .working_dir
            .join(format!(".handoff-{}.json", to_agent));
        context
            .write_to_file(&handoff_path)
            .map_err(|e| OrchestratorError::HandoffFailed {
                from: from_agent.to_string(),
                to: to_agent.to_string(),
                reason: e.to_string(),
            })?;

        info!(
            from = from_agent,
            to = to_agent,
            handoff_file = %handoff_path.display(),
            "handoff context written"
        );

        Ok(())
    }

    /// Get a reference to the routing engine.
    pub fn router(&self) -> &RoutingEngine {
        &self.router
    }

    /// Get a mutable reference to the routing engine.
    pub fn router_mut(&mut self) -> &mut RoutingEngine {
        &mut self.router
    }

    /// Get a reference to the rate limiter.
    pub fn rate_limiter(&self) -> &RateLimitTracker {
        &self.rate_limiter
    }

    /// Get a mutable reference to the rate limiter.
    pub fn rate_limiter_mut(&mut self) -> &mut RateLimitTracker {
        &mut self.rate_limiter
    }

    /// Submit a cross-agent review request.
    pub fn submit_review_request(&mut self, request: ReviewRequest) {
        info!(
            from = %request.from_agent,
            to = %request.to_agent,
            artifact = %request.artifact_path,
            review_type = %request.review_type,
            "review request submitted"
        );
        self.review_queue.push(request);
    }

    /// Get a reference to the review queue.
    pub fn review_queue(&self) -> &[ReviewRequest] {
        &self.review_queue
    }

    /// Process pending review requests.
    /// Returns the number of requests processed.
    pub async fn process_review_queue(&mut self) -> usize {
        let mut processed = 0;
        let to_process: Vec<ReviewRequest> = self.review_queue.drain(..).collect();

        for request in to_process {
            info!(
                from = %request.from_agent,
                to = %request.to_agent,
                "processing review request"
            );

            // Find the reviewer agent definition
            if let Some(reviewer_def) = self
                .config
                .agents
                .iter()
                .find(|a| a.name == request.to_agent)
                .cloned()
            {
                // Spawn the reviewer agent (in a real implementation, we'd pass the artifact info)
                if let Err(e) = self.spawn_agent(&reviewer_def).await {
                    error!(reviewer = %request.to_agent, error = %e, "failed to spawn reviewer agent");
                } else {
                    processed += 1;
                }
            } else {
                warn!(reviewer = %request.to_agent, "reviewer agent not found in config");
            }
        }

        processed
    }

    /// Check if a review should be triggered when an agent completes.
    fn check_review_trigger(&mut self, agent_name: &str, artifact_path: &str) {
        let matching_pairs: Vec<(String, String)> = self
            .config
            .review_pairs
            .iter()
            .filter(|pair| pair.producer == agent_name)
            .map(|pair| (pair.reviewer.clone(), pair.producer.clone()))
            .collect();

        for (reviewer, producer) in matching_pairs {
            let request = ReviewRequest {
                from_agent: producer,
                to_agent: reviewer,
                artifact_path: artifact_path.to_string(),
                review_type: "post_completion".to_string(),
            };
            self.submit_review_request(request);
        }
    }

    /// Spawn an agent from its definition.
    ///
    /// Model selection: if the agent has an explicit `model` field, use it.
    /// Otherwise, route the task prompt through the RoutingEngine to select
    /// a model based on keyword matching.
    async fn spawn_agent(&mut self, def: &AgentDefinition) -> Result<(), OrchestratorError> {
        // Select model via keyword routing or explicit config.
        // Skip keyword routing for CLIs that use OAuth and don't support -m
        // (e.g. codex with ChatGPT account). Only apply routed models when the
        // CLI tool is known to accept --model flags with arbitrary model IDs.
        let cli_name = std::path::Path::new(&def.cli_tool)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&def.cli_tool);
        let supports_model_flag = matches!(cli_name, "claude" | "claude-code");

        let model = if let Some(m) = &def.model {
            info!(agent = %def.name, model = %m, "using explicit model");
            Some(m.clone())
        } else if supports_model_flag {
            // Route the task prompt to find the best model
            let context = terraphim_router::RoutingContext::default();
            match self.router.route(&def.task, &context) {
                Ok(decision) => {
                    if let terraphim_types::capability::ProviderType::Llm { model_id, .. } =
                        &decision.provider.provider_type
                    {
                        info!(
                            agent = %def.name,
                            model = %model_id,
                            confidence = decision.confidence,
                            "model selected via keyword routing"
                        );
                        Some(model_id.clone())
                    } else {
                        None
                    }
                }
                Err(_) => {
                    info!(agent = %def.name, "no model matched via routing, using CLI default");
                    None
                }
            }
        } else {
            info!(agent = %def.name, cli = %def.cli_tool, "skipping model routing (CLI uses OAuth/default)");
            None
        };

        info!(agent = %def.name, layer = ?def.layer, cli = %def.cli_tool, model = ?model, "spawning agent");

        // Build a Provider from the agent definition for the spawner
        let provider = terraphim_types::capability::Provider {
            id: def.name.clone(),
            name: def.name.clone(),
            provider_type: terraphim_types::capability::ProviderType::Agent {
                agent_id: def.name.clone(),
                cli_command: def.cli_tool.clone(),
                working_dir: self.config.working_dir.clone(),
            },
            capabilities: vec![],
            cost_level: terraphim_types::capability::CostLevel::Cheap,
            latency: terraphim_types::capability::Latency::Medium,
            keywords: def.capabilities.clone(),
        };

        let handle = self
            .spawner
            .spawn_with_model(&provider, &def.task, model.as_deref())
            .await
            .map_err(|e| OrchestratorError::SpawnFailed {
                agent: def.name.clone(),
                reason: e.to_string(),
            })?;

        // Subscribe to the output broadcast for nightwatch drain
        let output_rx = handle.subscribe_output();

        // Get the restart count from the orchestrator-level counter
        let restart_count = self.restart_counts.get(&def.name).copied().unwrap_or(0);

        self.active_agents.insert(
            def.name.clone(),
            ManagedAgent {
                definition: def.clone(),
                handle,
                started_at: Instant::now(),
                restart_count,
                output_rx,
            },
        );

        Ok(())
    }

    /// Periodic reconciliation: detect exits, check cron, evaluate drift, drain output.
    async fn reconcile_tick(&mut self) {
        // 1. Poll all active agents for exit and handle exits per layer
        self.poll_agent_exits().await;

        // 2. Restart pending Safety agents (cooldown-aware)
        self.restart_pending_safety_agents().await;

        // 3. Check cron schedules for Core agents
        self.check_cron_schedules().await;

        // 4. Drain output events to nightwatch
        self.drain_output_events();

        // 5. Evaluate nightwatch drift
        self.nightwatch.evaluate();

        // 6. Check for stall condition (if dual mode is active)
        self.check_stall();

        // 7. Dispatch tasks from queue to agents (if dual mode is active)
        self.dispatch_from_queue().await;

        // 8. Update last_tick_time
        self.last_tick_time = chrono::Utc::now();
    }

    /// Poll all active agents for exit and handle exits per layer.
    async fn poll_agent_exits(&mut self) {
        // Collect exited agents first to avoid borrow conflict
        let mut exited: Vec<(String, AgentDefinition, std::process::ExitStatus)> = Vec::new();
        for (name, managed) in &mut self.active_agents {
            match managed.handle.try_wait() {
                Ok(Some(status)) => {
                    exited.push((name.clone(), managed.definition.clone(), status));
                }
                Ok(None) => {} // still running
                Err(e) => {
                    warn!(agent = %name, error = %e, "try_wait failed");
                }
            }
        }

        // Process exits
        for (name, def, status) in exited {
            self.active_agents.remove(&name);
            self.handle_agent_exit(&name, &def, status);
        }
    }

    /// Handle an agent exit based on its layer.
    fn handle_agent_exit(
        &mut self,
        name: &str,
        def: &AgentDefinition,
        status: std::process::ExitStatus,
    ) {
        match def.layer {
            AgentLayer::Safety => {
                let count = self.restart_counts.entry(name.to_string()).or_insert(0);
                *count += 1;
                self.restart_cooldowns
                    .insert(name.to_string(), Instant::now());
                if *count <= self.config.max_restart_count {
                    info!(
                        agent = %name,
                        exit_status = %status,
                        restart_count = *count,
                        cooldown_secs = self.config.restart_cooldown_secs,
                        "safety agent exited, will restart after cooldown"
                    );
                } else {
                    error!(
                        agent = %name,
                        exit_status = %status,
                        restart_count = *count,
                        max = self.config.max_restart_count,
                        "safety agent exceeded max restarts, not restarting"
                    );
                }
            }
            AgentLayer::Core => {
                info!(agent = %name, exit_status = %status, "core agent completed");
            }
            AgentLayer::Growth => {
                info!(agent = %name, exit_status = %status, "growth agent completed");
            }
        }
    }

    /// Restart Safety agents that have exited and passed their cooldown.
    async fn restart_pending_safety_agents(&mut self) {
        let cooldown = Duration::from_secs(self.config.restart_cooldown_secs);
        let max_restarts = self.config.max_restart_count;

        // Find Safety agents that need restarting
        let to_restart: Vec<AgentDefinition> = self
            .config
            .agents
            .iter()
            .filter(|def| {
                // Must be Safety layer
                if def.layer != AgentLayer::Safety {
                    return false;
                }
                // Must not be currently active
                if self.active_agents.contains_key(&def.name) {
                    return false;
                }
                // Must have a restart cooldown entry (meaning it exited)
                let last_exit = match self.restart_cooldowns.get(&def.name) {
                    Some(t) => t,
                    None => return false,
                };
                // Must have passed the cooldown
                if last_exit.elapsed() < cooldown {
                    return false;
                }
                // Must be under max restart count
                let count = self.restart_counts.get(&def.name).copied().unwrap_or(0);
                count <= max_restarts
            })
            .cloned()
            .collect();

        for def in to_restart {
            info!(
                agent = %def.name,
                restart_count = self.restart_counts.get(&def.name).copied().unwrap_or(0),
                "restarting safety agent after cooldown"
            );
            if let Err(e) = self.spawn_agent(&def).await {
                error!(agent = %def.name, error = %e, "failed to restart safety agent");
            }
        }
    }

    /// Check cron schedules and spawn due Core agents.
    async fn check_cron_schedules(&mut self) {
        let now = chrono::Utc::now();
        let scheduled = self.scheduler.scheduled_agents();

        // Collect agents that should fire
        let to_spawn: Vec<AgentDefinition> = scheduled
            .into_iter()
            .filter(|(def, _schedule)| {
                // Skip if already active
                !self.active_agents.contains_key(&def.name)
            })
            .filter(|(_def, schedule)| {
                // Check if a fire time exists between last_tick and now
                schedule
                    .after(&self.last_tick_time)
                    .take_while(|t| *t <= now)
                    .next()
                    .is_some()
            })
            .map(|(def, _)| def.clone())
            .collect();

        for def in to_spawn {
            // Add random jitter to prevent thundering herd for Core agents
            let jitter_ms = rand::random::<u64>() % self.config.stagger_delay_ms;
            if jitter_ms > 0 {
                tokio::time::sleep(Duration::from_millis(jitter_ms)).await;
            }
            info!(agent = %def.name, jitter_ms = jitter_ms, "cron schedule fired");
            if let Err(e) = self.spawn_agent(&def).await {
                error!(agent = %def.name, error = %e, "cron spawn failed");
            }
        }

        // Also check compound review schedule
        if let Some(compound_sched) = self.scheduler.compound_review_schedule() {
            let should_fire = compound_sched
                .after(&self.last_tick_time)
                .take_while(|t| *t <= now)
                .next()
                .is_some();
            if should_fire {
                self.handle_schedule_event(ScheduleEvent::CompoundReview)
                    .await;
            }
        }
    }

    /// Drain broadcast output events from all active agents into nightwatch.
    fn drain_output_events(&mut self) {
        // Collect events first to avoid borrow conflict (active_agents + nightwatch)
        let mut events: Vec<(String, OutputEvent)> = Vec::new();
        for (name, managed) in &mut self.active_agents {
            loop {
                match managed.output_rx.try_recv() {
                    Ok(event) => events.push((name.clone(), event)),
                    Err(broadcast::error::TryRecvError::Empty) => break,
                    Err(broadcast::error::TryRecvError::Lagged(n)) => {
                        warn!(agent = %name, skipped = n, "output events lagged");
                        break;
                    }
                    Err(broadcast::error::TryRecvError::Closed) => break,
                }
            }
        }
        for (name, event) in &events {
            self.nightwatch.observe(name, event);
        }
    }

    /// Handle a schedule event from the TimeScheduler.
    async fn handle_schedule_event(&mut self, event: ScheduleEvent) {
        match event {
            ScheduleEvent::Spawn(def) => {
                info!(agent = %def.name, "scheduled spawn");
                if let Err(e) = self.spawn_agent(&def).await {
                    error!(agent = %def.name, error = %e, "scheduled spawn failed");
                }
            }
            ScheduleEvent::Stop { agent_name } => {
                info!(agent = %agent_name, "scheduled stop");
                self.stop_agent(&agent_name).await;
            }
            ScheduleEvent::CompoundReview => {
                info!("scheduled compound review starting");
                match self.compound_workflow.run().await {
                    Ok(result) => {
                        info!(
                            findings = result.findings.len(),
                            pr_created = result.pr_created,
                            duration = ?result.duration,
                            "compound review completed"
                        );
                    }
                    Err(e) => {
                        error!(error = %e, "compound review failed");
                    }
                }
            }
        }
    }

    /// Handle a drift alert from the NightwatchMonitor.
    async fn handle_drift_alert(&mut self, alert: DriftAlert) {
        warn!(
            agent = %alert.agent_name,
            score = alert.drift_score.score,
            level = ?alert.drift_score.level,
            "drift alert"
        );

        match alert.recommended_action {
            CorrectionAction::LogWarning(msg) => {
                warn!(agent = %alert.agent_name, message = %msg, "drift warning");
            }
            CorrectionAction::RestartAgent => {
                info!(agent = %alert.agent_name, "restarting agent due to drift");
                self.stop_agent(&alert.agent_name).await;
                self.nightwatch.reset(&alert.agent_name);

                // Find definition and respawn
                if let Some(def) = self
                    .config
                    .agents
                    .iter()
                    .find(|a| a.name == alert.agent_name)
                    .cloned()
                {
                    if let Err(e) = self.spawn_agent(&def).await {
                        error!(
                            agent = %alert.agent_name,
                            error = %e,
                            "failed to restart agent after drift correction"
                        );
                    }
                }
            }
            CorrectionAction::PauseAndEscalate(msg) => {
                error!(
                    agent = %alert.agent_name,
                    message = %msg,
                    "CRITICAL: pausing agent and escalating to human"
                );
                self.stop_agent(&alert.agent_name).await;
                self.nightwatch.reset(&alert.agent_name);
            }
        }
    }

    /// Stop a specific agent by name.
    async fn stop_agent(&mut self, name: &str) {
        if let Some(managed) = self.active_agents.remove(name) {
            info!(agent = %name, "stopping agent");
            let grace = Duration::from_secs(5);
            let mut handle = managed.handle;
            match handle.shutdown(grace).await {
                Ok(_) => info!(agent = %name, "agent stopped gracefully"),
                Err(e) => warn!(agent = %name, error = %e, "agent stop had issues"),
            }
        }
    }

    /// Shutdown all active agents.
    async fn shutdown_all_agents(&mut self) {
        let names: Vec<String> = self.active_agents.keys().cloned().collect();
        for name in names {
            self.stop_agent(&name).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> OrchestratorConfig {
        OrchestratorConfig {
            working_dir: std::path::PathBuf::from("/tmp/test-orchestrator"),
            nightwatch: NightwatchConfig::default(),
            compound_review: CompoundReviewConfig {
                schedule: "0 2 * * *".to_string(),
                max_duration_secs: 60,
                repo_path: std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
                create_prs: false,
            },
            agents: vec![
                AgentDefinition {
                    name: "sentinel".to_string(),
                    layer: AgentLayer::Safety,
                    cli_tool: "echo".to_string(),
                    task: "safety watch".to_string(),
                    model: None,
                    schedule: None,
                    capabilities: vec!["security".to_string()],
                    max_memory_bytes: None,
                    provider: None,
                    fallback_provider: None,
                    fallback_model: None,
                    provider_tier: None,
                    persona_name: None,
                    persona_symbol: None,
                    persona_vibe: None,
                    meta_cortex_connections: vec![],
                    skill_chain: vec![],
                },
                AgentDefinition {
                    name: "sync".to_string(),
                    layer: AgentLayer::Core,
                    cli_tool: "echo".to_string(),
                    task: "sync upstream".to_string(),
                    model: None,
                    schedule: Some("0 3 * * *".to_string()),
                    capabilities: vec!["sync".to_string()],
                    max_memory_bytes: None,
                    provider: None,
                    fallback_provider: None,
                    fallback_model: None,
                    provider_tier: None,
                    persona_name: None,
                    persona_symbol: None,
                    persona_vibe: None,
                    meta_cortex_connections: vec![],
                    skill_chain: vec![],
                },
            ],
            restart_cooldown_secs: 60,
            max_restart_count: 10,
            tick_interval_secs: 30,
            allowed_providers: vec![],
            banned_providers: vec!["opencode".to_string()],
            skill_registry: Default::default(),
            stagger_delay_ms: 5000,
            review_pairs: vec![],
            drift_detection: DriftDetectionConfig::default(),
            session_rotation: SessionRotationConfig::default(),
            convergence: ConvergenceConfig::default(),
            workflow: None,
            tracker: None,
            concurrency: None,
        }
    }

    #[test]
    fn test_orchestrator_creates_from_config() {
        let config = test_config();
        let orch = AgentOrchestrator::new(config);
        assert!(orch.is_ok());
    }

    #[test]
    fn test_orchestrator_initial_state() {
        let config = test_config();
        let orch = AgentOrchestrator::new(config).unwrap();
        assert!(orch.active_agents.is_empty());
        assert!(!orch.shutdown_requested);
        let statuses = orch.agent_statuses();
        assert!(statuses.is_empty());
    }

    #[test]
    fn test_orchestrator_shutdown_flag() {
        let config = test_config();
        let mut orch = AgentOrchestrator::new(config).unwrap();
        assert!(!orch.shutdown_requested);
        orch.shutdown();
        assert!(orch.shutdown_requested);
    }

    #[tokio::test]
    async fn test_orchestrator_compound_review_manual() {
        let config = test_config();
        let mut orch = AgentOrchestrator::new(config).unwrap();
        let result = orch.trigger_compound_review().await.unwrap();
        assert!(!result.pr_created);
    }

    #[test]
    fn test_orchestrator_from_toml() {
        let toml_str = r#"
working_dir = "/tmp"

[nightwatch]

[compound_review]
schedule = "0 2 * * *"
repo_path = "/tmp"

[[agents]]
name = "test"
layer = "Safety"
cli_tool = "echo"
task = "test"
"#;
        let config = OrchestratorConfig::from_toml(toml_str).unwrap();
        let orch = AgentOrchestrator::new(config);
        assert!(orch.is_ok());
    }

    #[test]
    fn test_agent_status_fields() {
        let status = AgentStatus {
            name: "test".to_string(),
            layer: AgentLayer::Safety,
            running: true,
            health: HealthStatus::Healthy,
            drift_score: Some(0.05),
            uptime: Duration::from_secs(3600),
            restart_count: 0,
            api_calls_remaining: HashMap::new(),
        };
        assert_eq!(status.name, "test");
        assert!(status.running);
        assert_eq!(status.drift_score, Some(0.05));
    }

    /// Helper: create a config with a single Safety echo agent and short cooldown.
    fn test_config_fast_lifecycle() -> OrchestratorConfig {
        OrchestratorConfig {
            working_dir: std::path::PathBuf::from("/tmp"),
            nightwatch: NightwatchConfig::default(),
            compound_review: CompoundReviewConfig {
                schedule: "0 2 * * *".to_string(),
                max_duration_secs: 60,
                repo_path: std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
                create_prs: false,
            },
            agents: vec![AgentDefinition {
                name: "echo-safety".to_string(),
                layer: AgentLayer::Safety,
                cli_tool: "echo".to_string(),
                task: "safety watch".to_string(),
                model: None,
                schedule: None,
                capabilities: vec![],
                max_memory_bytes: None,
                provider: None,
                fallback_provider: None,
                fallback_model: None,
                provider_tier: None,
                persona_name: None,
                persona_symbol: None,
                persona_vibe: None,
                meta_cortex_connections: vec![],
                skill_chain: vec![],
            }],
            restart_cooldown_secs: 0, // instant restart for testing
            max_restart_count: 3,
            tick_interval_secs: 1,
            allowed_providers: vec![],
            banned_providers: vec!["opencode".to_string()],
            skill_registry: Default::default(),
            stagger_delay_ms: 5000,
            review_pairs: vec![],
            drift_detection: DriftDetectionConfig::default(),
            session_rotation: SessionRotationConfig::default(),
            convergence: ConvergenceConfig::default(),
            workflow: None,
            tracker: None,
            concurrency: None,
        }
    }

    #[tokio::test]
    async fn test_reconcile_detects_agent_exit() {
        let config = test_config_fast_lifecycle();
        let mut orch = AgentOrchestrator::new(config).unwrap();

        // Spawn the echo agent (exits immediately)
        let def = orch.config.agents[0].clone();
        orch.spawn_agent(&def).await.unwrap();
        assert!(orch.active_agents.contains_key("echo-safety"));

        // Give echo time to exit
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Poll for exits
        orch.poll_agent_exits().await;

        // Agent should be removed from active_agents
        assert!(
            !orch.active_agents.contains_key("echo-safety"),
            "exited agent should be removed from active_agents"
        );

        // Restart count should be recorded
        assert_eq!(
            orch.restart_counts.get("echo-safety").copied(),
            Some(1),
            "restart count should be incremented"
        );
    }

    #[tokio::test]
    async fn test_safety_agent_restarts_after_cooldown() {
        let config = test_config_fast_lifecycle();
        let mut orch = AgentOrchestrator::new(config).unwrap();

        // Spawn and let it exit
        let def = orch.config.agents[0].clone();
        orch.spawn_agent(&def).await.unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
        orch.poll_agent_exits().await;
        assert!(!orch.active_agents.contains_key("echo-safety"));

        // Restart pending (cooldown is 0, so immediate)
        orch.restart_pending_safety_agents().await;
        assert!(
            orch.active_agents.contains_key("echo-safety"),
            "safety agent should be restarted after cooldown"
        );
    }

    #[tokio::test]
    async fn test_core_agent_no_auto_restart() {
        let mut config = test_config_fast_lifecycle();
        config.agents = vec![AgentDefinition {
            name: "echo-core".to_string(),
            layer: AgentLayer::Core,
            cli_tool: "echo".to_string(),
            task: "core task".to_string(),
            model: None,
            schedule: Some("0 3 * * *".to_string()),
            capabilities: vec![],
            max_memory_bytes: None,
            provider: None,
            fallback_provider: None,
            fallback_model: None,
            provider_tier: None,
            persona_name: None,
            persona_symbol: None,
            persona_vibe: None,
            meta_cortex_connections: vec![],
            skill_chain: vec![],
        }];
        let mut orch = AgentOrchestrator::new(config).unwrap();

        // Spawn core agent and let it exit
        let def = orch.config.agents[0].clone();
        orch.spawn_agent(&def).await.unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
        orch.poll_agent_exits().await;
        assert!(!orch.active_agents.contains_key("echo-core"));

        // Restart pending should NOT restart a Core agent
        orch.restart_pending_safety_agents().await;
        assert!(
            !orch.active_agents.contains_key("echo-core"),
            "core agent should not auto-restart"
        );
    }

    #[tokio::test]
    async fn test_max_restart_count_respected() {
        let mut config = test_config_fast_lifecycle();
        config.max_restart_count = 2;
        let mut orch = AgentOrchestrator::new(config).unwrap();
        let def = orch.config.agents[0].clone();

        // Cycle through max_restart_count + 1 exits
        for i in 0..3 {
            orch.spawn_agent(&def).await.unwrap();
            tokio::time::sleep(Duration::from_millis(100)).await;
            orch.poll_agent_exits().await;
            assert!(
                !orch.active_agents.contains_key("echo-safety"),
                "agent should have exited on cycle {}",
                i
            );
        }

        // After 3 exits, restart count = 3, max = 2
        // restart_pending should NOT restart (count > max)
        orch.restart_pending_safety_agents().await;
        assert!(
            !orch.active_agents.contains_key("echo-safety"),
            "agent should not restart after exceeding max_restart_count"
        );
        assert_eq!(orch.restart_counts.get("echo-safety").copied(), Some(3));
    }

    #[tokio::test]
    async fn test_output_events_fed_to_nightwatch() {
        let config = test_config_fast_lifecycle();
        let mut orch = AgentOrchestrator::new(config).unwrap();

        // Spawn echo agent (writes "safety watch" to stdout)
        let def = orch.config.agents[0].clone();
        orch.spawn_agent(&def).await.unwrap();

        // Give the output capture time to process
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Drain events
        orch.drain_output_events();

        // Nightwatch should have observations for the agent
        let drift = orch.nightwatch.drift_score("echo-safety");
        assert!(
            drift.is_some(),
            "nightwatch should have drift data after draining output events"
        );
        let drift = drift.unwrap();
        assert!(
            drift.metrics.sample_count > 0,
            "nightwatch should have at least one sample from drained output"
        );
    }

    #[tokio::test]
    async fn test_reconcile_tick_full_cycle() {
        let config = test_config_fast_lifecycle();
        let mut orch = AgentOrchestrator::new(config).unwrap();

        // Spawn echo agent
        let def = orch.config.agents[0].clone();
        orch.spawn_agent(&def).await.unwrap();

        // Give echo time to exit and produce output
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Run a full reconciliation tick
        orch.reconcile_tick().await;

        // After tick: echo exited, so it was detected and marked for restart.
        // With 0 cooldown, it should have been restarted in the same tick.
        assert!(
            orch.active_agents.contains_key("echo-safety"),
            "safety agent should be restarted by reconcile_tick"
        );
        assert_eq!(
            orch.restart_counts.get("echo-safety").copied(),
            Some(1),
            "restart count should be 1 after first exit+restart cycle"
        );
    }

    /// Test: verify stagger_delay_ms is configurable
    #[test]
    fn test_stagger_delay_configurable() {
        let mut config = test_config();
        config.stagger_delay_ms = 100;
        assert_eq!(config.stagger_delay_ms, 100);

        config.stagger_delay_ms = 0;
        assert_eq!(config.stagger_delay_ms, 0);
    }

    /// Test: verify default stagger delay is 5000ms
    #[test]
    fn test_stagger_delay_default() {
        let config = OrchestratorConfig {
            working_dir: std::path::PathBuf::from("/tmp"),
            nightwatch: NightwatchConfig::default(),
            compound_review: CompoundReviewConfig {
                schedule: "0 0 * * *".to_string(),
                max_duration_secs: 1800,
                repo_path: std::path::PathBuf::from("/tmp"),
                create_prs: false,
            },
            agents: vec![],
            restart_cooldown_secs: 60,
            max_restart_count: 10,
            tick_interval_secs: 30,
            allowed_providers: vec![],
            banned_providers: vec!["opencode".to_string()],
            skill_registry: Default::default(),
            stagger_delay_ms: crate::config::default_stagger_delay_ms(),
            review_pairs: vec![],
            drift_detection: DriftDetectionConfig::default(),
            session_rotation: SessionRotationConfig::default(),
            convergence: ConvergenceConfig::default(),
            workflow: None,
            tracker: None,
            concurrency: None,
        };
        assert_eq!(config.stagger_delay_ms, 5000);
    }

    /// Test: verify review queue functionality
    #[test]
    fn test_review_queue_operations() {
        let config = test_config();
        let mut orch = AgentOrchestrator::new(config).unwrap();

        // Queue should start empty
        assert!(orch.review_queue.is_empty());

        // Submit a review request
        let request = ReviewRequest {
            from_agent: "implementation-swarm".to_string(),
            to_agent: "security-sentinel".to_string(),
            artifact_path: "/tmp/report.md".to_string(),
            review_type: "security".to_string(),
        };
        orch.review_queue.push(request.clone());

        // Queue should now have one item
        assert_eq!(orch.review_queue.len(), 1);
        assert_eq!(orch.review_queue[0].from_agent, "implementation-swarm");
        assert_eq!(orch.review_queue[0].to_agent, "security-sentinel");

        // Process the queue (pop the request)
        let processed = orch.review_queue.remove(0);
        assert_eq!(processed.from_agent, request.from_agent);
        assert!(orch.review_queue.is_empty());
    }

    /// Test: verify review pairs are loaded from config
    #[test]
    fn test_review_pairs_config() {
        let mut config = test_config();
        config.review_pairs = vec![
            crate::config::ReviewPair {
                producer: "implementation-swarm".to_string(),
                reviewer: "security-sentinel".to_string(),
            },
            crate::config::ReviewPair {
                producer: "code-writer".to_string(),
                reviewer: "quality-gate".to_string(),
            },
        ];

        let orch = AgentOrchestrator::new(config).unwrap();
        assert_eq!(orch.config.review_pairs.len(), 2);
        assert_eq!(orch.config.review_pairs[0].producer, "implementation-swarm");
        assert_eq!(orch.config.review_pairs[0].reviewer, "security-sentinel");
    }

    // =========================================================================
    // Issue #12: Dual Mode and ModeCoordinator Tests
    // =========================================================================

    /// Test: ModeCoordinator creation in dual mode
    #[test]
    fn test_mode_coordinator_dual_mode() {
        let workflow = WorkflowConfig {
            mode: WorkflowMode::Dual,
            poll_interval_secs: 60,
            max_concurrent_tasks: 5,
        };

        let agents = vec![AgentDefinition {
            name: "test-agent".to_string(),
            layer: AgentLayer::Growth,
            cli_tool: "echo".to_string(),
            task: "test".to_string(),
            model: None,
            schedule: None,
            capabilities: vec![],
            max_memory_bytes: None,
            provider: None,
            fallback_provider: None,
            fallback_model: None,
            provider_tier: None,
            persona_name: None,
            persona_symbol: None,
            persona_vibe: None,
            meta_cortex_connections: vec![],
            skill_chain: vec![],
        }];

        let (coord, _shutdown_rx) = ModeCoordinator::new(
            workflow,
            agents,
            None, // No tracker for this test
            Some("0 2 * * *".to_string()),
        ).unwrap();

        assert_eq!(coord.workflow_mode, WorkflowMode::Dual);
        assert!(coord.time_mode.is_some());
        assert!(coord.issue_mode.is_none()); // No tracker provided
        assert_eq!(coord.queue_depth(), 0);
        assert!(!coord.is_stalled(100));
    }

    /// Test: ModeCoordinator creation in time-only mode
    #[test]
    fn test_mode_coordinator_time_only_mode() {
        let workflow = WorkflowConfig {
            mode: WorkflowMode::TimeOnly,
            poll_interval_secs: 60,
            max_concurrent_tasks: 3,
        };

        let agents = vec![];

        let (coord, _shutdown_rx) = ModeCoordinator::new(
            workflow,
            agents,
            None,
            None,
        ).unwrap();

        assert_eq!(coord.workflow_mode, WorkflowMode::TimeOnly);
        assert!(coord.time_mode.is_some());
        assert!(coord.issue_mode.is_none());
        assert_eq!(coord.concurrency_controller.max_parallel(), 3);
    }

    /// Test: ModeCoordinator creation in issue-only mode
    #[test]
    fn test_mode_coordinator_issue_only_mode() {
        let workflow = WorkflowConfig {
            mode: WorkflowMode::IssueOnly,
            poll_interval_secs: 30,
            max_concurrent_tasks: 5,
        };

        let agents = vec![];

        // Without tracker, issue mode should not be created
        let (coord, _shutdown_rx) = ModeCoordinator::new(
            workflow,
            agents.clone(),
            None,
            None,
        ).unwrap();

        assert_eq!(coord.workflow_mode, WorkflowMode::IssueOnly);
        assert!(coord.time_mode.is_none());
        assert!(coord.issue_mode.is_none());
    }

    /// Test: Stall detection in ModeCoordinator
    #[test]
    fn test_stall_detection() {
        let workflow = WorkflowConfig {
            mode: WorkflowMode::Dual,
            poll_interval_secs: 60,
            max_concurrent_tasks: 5,
        };

        let agents = vec![];

        let (mut coord, _shutdown_rx) = ModeCoordinator::new(
            workflow,
            agents,
            None,
            None,
        ).unwrap();

        // Initially not stalled
        assert!(!coord.is_stalled(10));

        // Fill the queue above threshold
        for i in 0..15 {
            let task = DispatchTask::TimeTask(format!("agent-{}", i), "0 * * * *".to_string());
            coord.dispatch_queue.submit(task).unwrap();
        }

        // Now should be stalled with threshold of 10
        assert!(coord.is_stalled(10));
        assert!(!coord.is_stalled(20));
    }

    /// Test: Orchestrator stall detection integration
    #[test]
    fn test_orchestrator_stall_detection() {
        let mut config = test_config();
        config.workflow = Some(WorkflowConfig {
            mode: WorkflowMode::Dual,
            poll_interval_secs: 60,
            max_concurrent_tasks: 5,
        });
        config.concurrency = Some(ConcurrencyConfig {
            max_parallel_agents: 3,
            queue_depth: 10,
            starvation_timeout_secs: 300,
        });

        let mut orch = AgentOrchestrator::new(config).unwrap();

        // Initially not stalled
        assert!(!orch.check_stall());

        // Fill the mode coordinator queue if it exists
        if let Some(ref mut coord) = orch.mode_coordinator {
            for i in 0..15 {
                let task = DispatchTask::TimeTask(format!("agent-{}", i), "0 * * * *".to_string());
                coord.dispatch_queue.submit(task).unwrap();
            }
        }

        // Now should be stalled
        assert!(orch.check_stall());
    }

    /// Test: Unified shutdown signals issue mode
    #[tokio::test]
    async fn test_unified_shutdown() {
        let mut config = test_config();
        config.workflow = Some(WorkflowConfig {
            mode: WorkflowMode::Dual,
            poll_interval_secs: 60,
            max_concurrent_tasks: 5,
        });

        let mut orch = AgentOrchestrator::new(config).unwrap();

        // Start shutdown
        orch.unified_shutdown().await;

        // Should complete without errors
        assert!(orch.shutdown_requested);
    }

    /// Test: Dispatch queue operations
    #[test]
    fn test_dispatch_queue_next_task() {
        let workflow = WorkflowConfig {
            mode: WorkflowMode::Dual,
            poll_interval_secs: 60,
            max_concurrent_tasks: 5,
        };

        let agents = vec![];

        let (mut coord, _shutdown_rx) = ModeCoordinator::new(
            workflow,
            agents,
            None,
            None,
        ).unwrap();

        // Submit some tasks
        let task1 = DispatchTask::TimeTask("agent1".to_string(), "0 * * * *".to_string());
        let task2 = DispatchTask::IssueTask("agent2".to_string(), 42, 100);

        coord.dispatch_queue.submit(task1.clone()).unwrap();
        coord.dispatch_queue.submit(task2.clone()).unwrap();

        assert_eq!(coord.queue_depth(), 2);

        // Get next task - should be issue task due to higher priority
        let next = coord.next_task();
        assert!(next.is_some());
        assert_eq!(coord.queue_depth(), 1);

        // Get remaining task
        let next = coord.next_task();
        assert!(next.is_some());
        assert_eq!(coord.queue_depth(), 0);

        // Queue empty
        let next = coord.next_task();
        assert!(next.is_none());
    }

    /// Test: Concurrency permit acquisition
    #[test]
    fn test_concurrency_permit_acquisition() {
        let workflow = WorkflowConfig {
            mode: WorkflowMode::Dual,
            poll_interval_secs: 60,
            max_concurrent_tasks: 2,
        };

        let agents = vec![];

        let (coord, _shutdown_rx) = ModeCoordinator::new(
            workflow,
            agents,
            None,
            None,
        ).unwrap();

        // Should be able to acquire permits up to max
        let permit1 = coord.try_acquire_permit();
        assert!(permit1.is_some());

        let permit2 = coord.try_acquire_permit();
        assert!(permit2.is_some());

        // Third permit should fail
        let permit3 = coord.try_acquire_permit();
        assert!(permit3.is_none());
    }
}
