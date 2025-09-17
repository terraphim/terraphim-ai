//! Supervision tree implementation
//!
//! Implements Erlang/OTP-style supervision trees for fault-tolerant agent management.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, RwLock};
use tokio::time::timeout;

use crate::{
    AgentFactory, AgentPid, AgentSpec, ExitReason, RestartPolicy, RestartStrategy, SupervisedAgent,
    SupervisedAgentInfo, SupervisionError, SupervisionResult, SupervisorId, TerminateReason,
};

/// Supervisor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisorConfig {
    /// Unique identifier for the supervisor
    pub supervisor_id: SupervisorId,
    /// Restart policy for child agents
    pub restart_policy: RestartPolicy,
    /// Timeout for agent operations
    pub agent_timeout: Duration,
    /// Health check interval
    pub health_check_interval: Duration,
    /// Maximum number of child agents
    pub max_children: usize,
}

impl Default for SupervisorConfig {
    fn default() -> Self {
        Self {
            supervisor_id: SupervisorId::new(),
            restart_policy: RestartPolicy::default(),
            agent_timeout: Duration::from_secs(30),
            health_check_interval: Duration::from_secs(60),
            max_children: 100,
        }
    }
}

/// Supervisor state
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SupervisorStatus {
    Starting,
    Running,
    Stopping,
    Stopped,
    Failed(String),
}

/// Restart history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestartEntry {
    pub agent_id: AgentPid,
    pub timestamp: DateTime<Utc>,
    pub reason: ExitReason,
}

/// Agent supervisor implementing OTP-style supervision
pub struct AgentSupervisor {
    config: SupervisorConfig,
    status: SupervisorStatus,
    children: Arc<RwLock<HashMap<AgentPid, SupervisedAgentInfo>>>,
    agents: Arc<RwLock<HashMap<AgentPid, Box<dyn SupervisedAgent>>>>,
    agent_factory: Arc<dyn AgentFactory>,
    restart_history: Arc<Mutex<Vec<RestartEntry>>>,
    shutdown_signal: Arc<Mutex<Option<tokio::sync::oneshot::Receiver<()>>>>,
}

impl AgentSupervisor {
    /// Create a new agent supervisor
    pub fn new(config: SupervisorConfig, agent_factory: Arc<dyn AgentFactory>) -> Self {
        Self {
            config,
            status: SupervisorStatus::Stopped,
            children: Arc::new(RwLock::new(HashMap::new())),
            agents: Arc::new(RwLock::new(HashMap::new())),
            agent_factory,
            restart_history: Arc::new(Mutex::new(Vec::new())),
            shutdown_signal: Arc::new(Mutex::new(None)),
        }
    }

    /// Start the supervisor
    pub async fn start(&mut self) -> SupervisionResult<()> {
        if self.status != SupervisorStatus::Stopped {
            return Err(SupervisionError::System(
                "Supervisor is already running".to_string(),
            ));
        }

        self.status = SupervisorStatus::Starting;
        log::info!("Starting supervisor {}", self.config.supervisor_id.0);

        // Start health check task
        self.start_health_check_task().await;

        self.status = SupervisorStatus::Running;
        log::info!(
            "Supervisor {} started successfully",
            self.config.supervisor_id.0
        );

        Ok(())
    }

    /// Stop the supervisor and all child agents
    pub async fn stop(&mut self) -> SupervisionResult<()> {
        if self.status == SupervisorStatus::Stopped {
            return Ok(());
        }

        self.status = SupervisorStatus::Stopping;
        log::info!("Stopping supervisor {}", self.config.supervisor_id.0);

        // Stop all child agents
        let agent_pids: Vec<AgentPid> = {
            let children = self.children.read().await;
            children.keys().cloned().collect()
        };

        for pid in agent_pids {
            if let Err(e) = self.stop_agent(&pid).await {
                log::error!("Failed to stop agent {}: {}", pid, e);
            }
        }

        // Signal shutdown to background tasks
        if let Some(_sender) = self.shutdown_signal.lock().await.take() {
            // Sender will be dropped, signaling shutdown
        }

        self.status = SupervisorStatus::Stopped;
        log::info!("Supervisor {} stopped", self.config.supervisor_id.0);

        Ok(())
    }

    /// Spawn a new supervised agent
    pub async fn spawn_agent(&mut self, spec: AgentSpec) -> SupervisionResult<AgentPid> {
        // Check if we've reached the maximum number of children
        {
            let children = self.children.read().await;
            if children.len() >= self.config.max_children {
                return Err(SupervisionError::System(
                    "Maximum number of child agents reached".to_string(),
                ));
            }
        }

        self.spawn_agent_internal(spec, 0).await
    }

    /// Stop a specific agent
    pub async fn stop_agent(&mut self, agent_id: &AgentPid) -> SupervisionResult<()> {
        log::info!("Stopping agent {}", agent_id);

        // Get agent instance
        let mut agent = {
            let mut agents = self.agents.write().await;
            agents
                .remove(agent_id)
                .ok_or_else(|| SupervisionError::AgentNotFound(agent_id.clone()))?
        };

        // Stop the agent with timeout
        let stop_result = timeout(self.config.agent_timeout, agent.stop()).await;

        match stop_result {
            Ok(Ok(())) => {
                log::info!("Agent {} stopped successfully", agent_id);
            }
            Ok(Err(e)) => {
                log::error!("Agent {} stop failed: {}", agent_id, e);
                // Force termination
                let _ = agent.terminate(TerminateReason::Error(e.to_string())).await;
            }
            Err(_) => {
                log::error!("Agent {} stop timed out", agent_id);
                // Force termination
                let _ = agent.terminate(TerminateReason::Timeout).await;
            }
        }

        // Remove from children
        {
            let mut children = self.children.write().await;
            children.remove(agent_id);
        }

        Ok(())
    }

    /// Handle agent failure and apply restart strategy
    pub async fn handle_agent_exit(
        &mut self,
        agent_id: AgentPid,
        reason: ExitReason,
    ) -> SupervisionResult<()> {
        log::warn!("Agent {} exited with reason: {:?}", agent_id, reason);

        // Record restart entry
        {
            let mut history = self.restart_history.lock().await;
            history.push(RestartEntry {
                agent_id: agent_id.clone(),
                timestamp: Utc::now(),
                reason: reason.clone(),
            });
        }

        // Check if restart is allowed
        if !self.should_restart(&agent_id, &reason).await? {
            log::info!("Not restarting agent {} due to policy", agent_id);
            // Remove the failed agent
            self.stop_agent(&agent_id).await?;
            return Ok(());
        }

        // Apply restart strategy
        match self.config.restart_policy.strategy {
            RestartStrategy::OneForOne => {
                self.restart_agent(&agent_id).await?;
            }
            RestartStrategy::OneForAll => {
                self.restart_all_agents().await?;
            }
            RestartStrategy::RestForOne => {
                self.restart_from_agent(&agent_id).await?;
            }
        }

        Ok(())
    }

    /// Check if agent should be restarted based on policy
    async fn should_restart(
        &self,
        agent_id: &AgentPid,
        reason: &ExitReason,
    ) -> SupervisionResult<bool> {
        // Don't restart on normal shutdown
        if matches!(reason, ExitReason::Normal | ExitReason::Shutdown) {
            return Ok(false);
        }

        // Get agent info
        let agent_info = {
            let children = self.children.read().await;
            children
                .get(agent_id)
                .cloned()
                .ok_or_else(|| SupervisionError::AgentNotFound(agent_id.clone()))?
        };

        // Check restart intensity - use time since first restart if available, otherwise time since start
        let time_since_first_restart = if let Some(first_restart) = agent_info.last_restart {
            let duration = Utc::now() - first_restart;
            Duration::from_secs(duration.num_seconds().max(0) as u64)
        } else {
            // No previous restarts, so this would be the first
            Duration::from_secs(0)
        };

        let is_allowed = self
            .config
            .restart_policy
            .intensity
            .is_restart_allowed(agent_info.restart_count, time_since_first_restart);

        if !is_allowed {
            log::warn!(
                "Agent {} exceeded restart limits (count: {}, time_window: {:?})",
                agent_id,
                agent_info.restart_count,
                time_since_first_restart
            );
            return Err(SupervisionError::MaxRestartsExceeded(
                self.config.supervisor_id.clone(),
            ));
        }

        Ok(true)
    }

    /// Restart a single agent
    async fn restart_agent(&mut self, agent_id: &AgentPid) -> SupervisionResult<()> {
        log::info!("Restarting agent {}", agent_id);

        // Get agent spec and current restart count
        let (spec, current_restart_count) = {
            let children = self.children.read().await;
            if let Some(info) = children.get(agent_id) {
                (info.spec.clone(), info.restart_count)
            } else {
                return Err(SupervisionError::AgentNotFound(agent_id.clone()));
            }
        };

        // Stop existing agent if still running
        if self.agents.read().await.contains_key(agent_id) {
            self.stop_agent(agent_id).await?;
        }

        // Create new agent with same spec
        let mut new_spec = spec.clone();
        new_spec.agent_id = agent_id.clone(); // Keep the same agent ID for tracking

        // Spawn new agent with incremented restart count
        self.spawn_agent_internal(new_spec, current_restart_count + 1)
            .await?;

        log::info!("Agent {} restarted successfully", agent_id);
        Ok(())
    }

    /// Internal spawn method that preserves restart count
    async fn spawn_agent_internal(
        &mut self,
        spec: AgentSpec,
        restart_count: u32,
    ) -> SupervisionResult<AgentPid> {
        if self.status != SupervisorStatus::Running {
            return Err(SupervisionError::System(
                "Supervisor is not running".to_string(),
            ));
        }

        // Validate agent specification
        self.agent_factory.validate_spec(&spec)?;

        let agent_id = spec.agent_id.clone();
        log::info!(
            "Spawning agent {} of type {} (restart count: {})",
            agent_id,
            spec.agent_type,
            restart_count
        );

        // Create agent info with preserved restart count
        let mut agent_info = SupervisedAgentInfo::new(
            agent_id.clone(),
            self.config.supervisor_id.clone(),
            spec.clone(),
        );
        agent_info.restart_count = restart_count;

        // If this is a restart, record it
        if restart_count > 0 {
            agent_info.record_restart();
            // Set the restart count explicitly since record_restart doesn't increment it anymore
            agent_info.restart_count = restart_count;
        }

        // Create and initialize agent
        let mut agent = self.agent_factory.create_agent(&spec).await?;

        let init_args = crate::InitArgs {
            agent_id: agent_id.clone(),
            supervisor_id: self.config.supervisor_id.clone(),
            config: spec.config.clone(),
        };

        agent
            .init(init_args)
            .await
            .map_err(|e| SupervisionError::AgentStartFailed(agent_id.clone(), e.to_string()))?;

        // Start the agent
        agent
            .start()
            .await
            .map_err(|e| SupervisionError::AgentStartFailed(agent_id.clone(), e.to_string()))?;

        // Store agent info and instance
        {
            let mut children = self.children.write().await;
            children.insert(agent_id.clone(), agent_info);
        }
        {
            let mut agents = self.agents.write().await;
            agents.insert(agent_id.clone(), agent);
        }

        log::info!(
            "Agent {} spawned successfully with restart count {}",
            agent_id,
            restart_count
        );
        Ok(agent_id)
    }

    /// Restart all agents
    async fn restart_all_agents(&mut self) -> SupervisionResult<()> {
        log::info!("Restarting all agents");

        let agent_specs: Vec<AgentSpec> = {
            let children = self.children.read().await;
            children.values().map(|info| info.spec.clone()).collect()
        };

        // Stop all agents
        let agent_pids: Vec<AgentPid> = {
            let children = self.children.read().await;
            children.keys().cloned().collect()
        };

        for pid in agent_pids {
            if let Err(e) = self.stop_agent(&pid).await {
                log::error!("Failed to stop agent {} during restart all: {}", pid, e);
            }
        }

        // Restart all agents
        for spec in agent_specs {
            if let Err(e) = self.spawn_agent(spec.clone()).await {
                log::error!("Failed to restart agent {}: {}", spec.agent_id, e);
            }
        }

        log::info!("All agents restarted");
        Ok(())
    }

    /// Restart agents from a specific point
    async fn restart_from_agent(&mut self, failed_agent_id: &AgentPid) -> SupervisionResult<()> {
        log::info!("Restarting from agent {}", failed_agent_id);

        // Get all agent specs in order
        let mut agent_specs: Vec<AgentSpec> = {
            let children = self.children.read().await;
            children.values().map(|info| info.spec.clone()).collect()
        };

        // Sort by start time to maintain order
        agent_specs.sort_by_key(|spec| spec.agent_id.0);

        // Find the failed agent index
        let failed_index = agent_specs
            .iter()
            .position(|spec| spec.agent_id == *failed_agent_id)
            .ok_or_else(|| SupervisionError::AgentNotFound(failed_agent_id.clone()))?;

        // Stop agents from failed agent onwards
        for spec in &agent_specs[failed_index..] {
            if let Err(e) = self.stop_agent(&spec.agent_id).await {
                log::error!(
                    "Failed to stop agent {} during restart from: {}",
                    spec.agent_id,
                    e
                );
            }
        }

        // Restart agents from failed agent onwards
        for spec in &agent_specs[failed_index..] {
            if let Err(e) = self.spawn_agent(spec.clone()).await {
                log::error!("Failed to restart agent {}: {}", spec.agent_id, e);
            }
        }

        log::info!("Restarted agents from {}", failed_agent_id);
        Ok(())
    }

    /// Start health check background task
    async fn start_health_check_task(&mut self) {
        let children = Arc::clone(&self.children);
        let agents = Arc::clone(&self.agents);
        let interval = self.config.health_check_interval;
        let _supervisor_id = self.config.supervisor_id.clone();

        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);

            loop {
                interval_timer.tick().await;

                let agent_pids: Vec<AgentPid> = {
                    let children_guard = children.read().await;
                    children_guard.keys().cloned().collect()
                };

                for pid in agent_pids {
                    let health_result = {
                        let agents_guard = agents.read().await;
                        if let Some(agent) = agents_guard.get(&pid) {
                            agent.health_check().await
                        } else {
                            continue;
                        }
                    };

                    match health_result {
                        Ok(true) => {
                            // Agent is healthy, update health check time
                            let mut children_guard = children.write().await;
                            if let Some(info) = children_guard.get_mut(&pid) {
                                info.record_health_check();
                            }
                        }
                        Ok(false) => {
                            log::warn!("Agent {} failed health check", pid);
                            // TODO: Handle unhealthy agent
                        }
                        Err(e) => {
                            log::error!("Health check error for agent {}: {}", pid, e);
                            // TODO: Handle health check error
                        }
                    }
                }
            }
        });
    }

    /// Get supervisor status
    pub fn status(&self) -> SupervisorStatus {
        self.status.clone()
    }

    /// Get supervisor configuration
    pub fn config(&self) -> &SupervisorConfig {
        &self.config
    }

    /// Get information about all child agents
    pub async fn get_children(&self) -> HashMap<AgentPid, SupervisedAgentInfo> {
        self.children.read().await.clone()
    }

    /// Get information about a specific child agent
    pub async fn get_child(&self, agent_id: &AgentPid) -> Option<SupervisedAgentInfo> {
        self.children.read().await.get(agent_id).cloned()
    }

    /// Get restart history
    pub async fn get_restart_history(&self) -> Vec<RestartEntry> {
        self.restart_history.lock().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TestAgentFactory;
    use serde_json::json;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_supervisor_lifecycle() {
        let config = SupervisorConfig::default();
        let factory = Arc::new(TestAgentFactory);
        let mut supervisor = AgentSupervisor::new(config, factory);

        // Start supervisor
        supervisor.start().await.unwrap();
        assert_eq!(supervisor.status(), SupervisorStatus::Running);

        // Stop supervisor
        supervisor.stop().await.unwrap();
        assert_eq!(supervisor.status(), SupervisorStatus::Stopped);
    }

    #[tokio::test]
    async fn test_agent_spawning() {
        let config = SupervisorConfig::default();
        let factory = Arc::new(TestAgentFactory);
        let mut supervisor = AgentSupervisor::new(config, factory);

        supervisor.start().await.unwrap();

        // Spawn an agent
        let spec = AgentSpec::new("test".to_string(), json!({}));
        let agent_id = supervisor.spawn_agent(spec).await.unwrap();

        // Check agent was created
        let children = supervisor.get_children().await;
        assert!(children.contains_key(&agent_id));

        // Stop agent
        supervisor.stop_agent(&agent_id).await.unwrap();

        // Check agent was removed
        let children = supervisor.get_children().await;
        assert!(!children.contains_key(&agent_id));

        supervisor.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_restart_strategy_one_for_one() {
        let mut config = SupervisorConfig::default();
        config.restart_policy.strategy = RestartStrategy::OneForOne;

        let factory = Arc::new(TestAgentFactory);
        let mut supervisor = AgentSupervisor::new(config, factory);

        supervisor.start().await.unwrap();

        // Spawn two agents
        let spec1 = AgentSpec::new("test".to_string(), json!({}));
        let spec2 = AgentSpec::new("test".to_string(), json!({}));

        let agent_id1 = supervisor.spawn_agent(spec1).await.unwrap();
        let _agent_id2 = supervisor.spawn_agent(spec2).await.unwrap();

        // Simulate agent failure
        supervisor
            .handle_agent_exit(
                agent_id1.clone(),
                ExitReason::Error("test error".to_string()),
            )
            .await
            .unwrap();

        // Check that both agents are still present (one restarted)
        let children = supervisor.get_children().await;
        assert_eq!(children.len(), 2);

        supervisor.stop().await.unwrap();
    }
}
