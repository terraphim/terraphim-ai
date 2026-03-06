pub mod compound;
pub mod config;
pub mod error;
pub mod handoff;
pub mod nightwatch;
pub mod scheduler;

pub use compound::{CompoundReviewResult, CompoundReviewWorkflow};
pub use config::{
    AgentDefinition, AgentLayer, CompoundReviewConfig, NightwatchConfig, OrchestratorConfig,
};
pub use error::OrchestratorError;
pub use handoff::HandoffContext;
pub use nightwatch::{
    CorrectionAction, CorrectionLevel, DriftAlert, DriftMetrics, DriftScore, NightwatchMonitor,
    RateLimitTracker, RateLimitWindow,
};
pub use scheduler::{ScheduleEvent, TimeScheduler};

use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, Instant};

use terraphim_router::RoutingEngine;
use terraphim_spawner::health::HealthStatus;
use terraphim_spawner::{AgentHandle, AgentSpawner};
use tracing::{error, info, warn};

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
}

impl AgentOrchestrator {
    /// Create a new orchestrator from configuration.
    pub fn new(config: OrchestratorConfig) -> Result<Self, OrchestratorError> {
        let spawner = AgentSpawner::new().with_working_dir(&config.working_dir);
        let router = RoutingEngine::new();
        let nightwatch = NightwatchMonitor::new(config.nightwatch.clone());
        let scheduler = TimeScheduler::new(&config.agents, Some(&config.compound_review.schedule))?;
        let compound_workflow = CompoundReviewWorkflow::new(config.compound_review.clone());

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
    /// 2. Enters the select! loop handling schedule events, drift alerts
    pub async fn run(&mut self) -> Result<(), OrchestratorError> {
        info!(
            "starting orchestrator with {} agent definitions",
            self.config.agents.len()
        );

        // Spawn Safety-layer agents immediately
        let immediate = self.scheduler.immediate_agents();
        for agent_def in &immediate {
            if let Err(e) = self.spawn_agent(agent_def).await {
                error!(agent = %agent_def.name, error = %e, "failed to spawn safety agent");
            }
        }

        info!(
            safety_agents = immediate.len(),
            active = self.active_agents.len(),
            "safety agents spawned, entering reconciliation loop"
        );

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

        self.active_agents.insert(
            def.name.clone(),
            ManagedAgent {
                definition: def.clone(),
                handle,
                started_at: Instant::now(),
                restart_count: 0,
            },
        );

        Ok(())
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
                },
            ],
            restart_cooldown_secs: 60,
            max_restart_count: 10,
            tick_interval_secs: 30,
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
}
