//! Agent lifecycle management
//!
//! Handles the complete lifecycle of GenAgent instances including initialization,
//! message processing, state transitions, and termination.

use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, RwLock};

use crate::{
    AgentPid, AgentState, AgentStatus, GenAgent, GenAgentError, GenAgentInitArgs, GenAgentResult,
    StateContainer, StateManager, SupervisorId, TerminateReason,
};

/// Agent lifecycle phases
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LifecyclePhase {
    Created,
    Initializing,
    Running,
    Hibernating,
    Terminating,
    Terminated,
    Failed(String),
}

/// Agent lifecycle statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleStats {
    pub agent_id: AgentPid,
    pub phase: LifecyclePhase,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub terminated_at: Option<DateTime<Utc>>,
    pub total_messages_handled: u64,
    pub total_calls: u64,
    pub total_casts: u64,
    pub total_infos: u64,
    pub total_errors: u64,
    pub average_message_time: Duration,
    pub last_message_at: Option<DateTime<Utc>>,
    pub state_version: u64,
}

impl LifecycleStats {
    pub fn new(agent_id: AgentPid) -> Self {
        Self {
            agent_id,
            phase: LifecyclePhase::Created,
            created_at: Utc::now(),
            started_at: None,
            terminated_at: None,
            total_messages_handled: 0,
            total_calls: 0,
            total_casts: 0,
            total_infos: 0,
            total_errors: 0,
            average_message_time: Duration::ZERO,
            last_message_at: None,
            state_version: 0,
        }
    }

    pub fn update_phase(&mut self, phase: LifecyclePhase) {
        self.phase = phase;
        match &self.phase {
            LifecyclePhase::Running => {
                if self.started_at.is_none() {
                    self.started_at = Some(Utc::now());
                }
            }
            LifecyclePhase::Terminated | LifecyclePhase::Failed(_) => {
                self.terminated_at = Some(Utc::now());
            }
            _ => {}
        }
    }

    pub fn record_message(&mut self, message_type: &str, processing_time: Duration) {
        self.total_messages_handled += 1;
        self.last_message_at = Some(Utc::now());

        match message_type {
            "call" => self.total_calls += 1,
            "cast" => self.total_casts += 1,
            "info" => self.total_infos += 1,
            _ => {}
        }

        // Update average processing time (simple moving average)
        if self.total_messages_handled == 1 {
            self.average_message_time = processing_time;
        } else {
            let total_time = self.average_message_time.as_nanos() as f64
                * (self.total_messages_handled - 1) as f64;
            let new_average = (total_time + processing_time.as_nanos() as f64)
                / self.total_messages_handled as f64;
            self.average_message_time = Duration::from_nanos(new_average as u64);
        }
    }

    pub fn record_error(&mut self) {
        self.total_errors += 1;
    }

    pub fn update_state_version(&mut self, version: u64) {
        self.state_version = version;
    }

    pub fn uptime(&self) -> Option<chrono::Duration> {
        if let Some(started_at) = self.started_at {
            if let Some(terminated_at) = self.terminated_at {
                Some(terminated_at - started_at)
            } else {
                Some(Utc::now() - started_at)
            }
        } else {
            None
        }
    }
}

/// Agent lifecycle manager
pub struct LifecycleManager<State: AgentState> {
    agent_id: AgentPid,
    supervisor_id: SupervisorId,
    stats: Arc<Mutex<LifecycleStats>>,
    state_container: Arc<RwLock<Option<StateContainer<State>>>>,
    state_manager: Arc<StateManager>,
    hibernation_timeout: Option<Duration>,
    last_activity: Arc<Mutex<Instant>>,
}

impl<State: AgentState + 'static> LifecycleManager<State> {
    pub fn new(
        agent_id: AgentPid,
        supervisor_id: SupervisorId,
        state_manager: Arc<StateManager>,
        hibernation_timeout: Option<Duration>,
    ) -> Self {
        let stats = LifecycleStats::new(agent_id.clone());

        Self {
            agent_id: agent_id.clone(),
            supervisor_id,
            stats: Arc::new(Mutex::new(stats)),
            state_container: Arc::new(RwLock::new(None)),
            state_manager,
            hibernation_timeout,
            last_activity: Arc::new(Mutex::new(Instant::now())),
        }
    }

    /// Initialize the agent lifecycle
    pub async fn initialize<A>(&self, agent: &mut A, args: GenAgentInitArgs) -> GenAgentResult<()>
    where
        A: GenAgent<State>,
    {
        // Update phase to initializing
        {
            let mut stats = self.stats.lock().await;
            stats.update_phase(LifecyclePhase::Initializing);
        }

        // Initialize the agent
        let initial_state = agent.init(args).await.map_err(|e| {
            GenAgentError::InitializationFailed(self.agent_id.clone(), e.to_string())
        })?;

        // Create state container
        let state_container = StateContainer::new(self.agent_id.clone(), initial_state)?;

        // Store state
        {
            let mut container_guard = self.state_container.write().await;
            *container_guard = Some(state_container.clone());
        }

        // Persist state
        self.state_manager
            .store_state(self.agent_id.clone(), state_container)
            .await?;

        // Update phase to running
        {
            let mut stats = self.stats.lock().await;
            stats.update_phase(LifecyclePhase::Running);
            stats.update_state_version(1);
        }

        log::info!("Agent {} initialized successfully", self.agent_id);
        Ok(())
    }

    /// Get the current state
    pub async fn get_state(&self) -> GenAgentResult<State> {
        let container_guard = self.state_container.read().await;
        if let Some(container) = container_guard.as_ref() {
            Ok(container.state.clone())
        } else {
            Err(GenAgentError::AgentNotRunning(self.agent_id.clone()))
        }
    }

    /// Update the agent state
    pub async fn update_state(&self, new_state: State) -> GenAgentResult<()> {
        let mut container_guard = self.state_container.write().await;
        if let Some(container) = container_guard.as_mut() {
            container.update_state(new_state)?;

            // Persist updated state
            self.state_manager
                .store_state(self.agent_id.clone(), container.clone())
                .await?;

            // Update stats
            {
                let mut stats = self.stats.lock().await;
                stats.update_state_version(container.version());
            }

            // Update activity timestamp
            {
                let mut last_activity = self.last_activity.lock().await;
                *last_activity = Instant::now();
            }

            Ok(())
        } else {
            Err(GenAgentError::AgentNotRunning(self.agent_id.clone()))
        }
    }

    /// Record message processing
    pub async fn record_message_processing(&self, message_type: &str, processing_time: Duration) {
        let mut stats = self.stats.lock().await;
        stats.record_message(message_type, processing_time);

        // Update activity timestamp
        {
            let mut last_activity = self.last_activity.lock().await;
            *last_activity = Instant::now();
        }
    }

    /// Record error
    pub async fn record_error(&self) {
        let mut stats = self.stats.lock().await;
        stats.record_error();
    }

    /// Check if agent should hibernate
    pub async fn should_hibernate(&self) -> bool {
        if let Some(timeout) = self.hibernation_timeout {
            let last_activity = self.last_activity.lock().await;
            last_activity.elapsed() > timeout
        } else {
            false
        }
    }

    /// Hibernate the agent
    pub async fn hibernate(&self) -> GenAgentResult<()> {
        let mut stats = self.stats.lock().await;
        stats.update_phase(LifecyclePhase::Hibernating);

        log::info!("Agent {} hibernating", self.agent_id);
        Ok(())
    }

    /// Wake up the agent from hibernation
    pub async fn wake_up(&self) -> GenAgentResult<()> {
        let mut stats = self.stats.lock().await;
        stats.update_phase(LifecyclePhase::Running);

        // Update activity timestamp
        {
            let mut last_activity = self.last_activity.lock().await;
            *last_activity = Instant::now();
        }

        log::info!("Agent {} waking up from hibernation", self.agent_id);
        Ok(())
    }

    /// Terminate the agent
    pub async fn terminate<A>(&self, agent: &mut A, reason: TerminateReason) -> GenAgentResult<()>
    where
        A: GenAgent<State>,
    {
        // Update phase to terminating
        {
            let mut stats = self.stats.lock().await;
            stats.update_phase(LifecyclePhase::Terminating);
        }

        // Get current state for termination
        let state = self.get_state().await?;

        // Call agent's terminate method
        agent
            .terminate(reason.clone(), state)
            .await
            .map_err(|e| GenAgentError::TerminationFailed(self.agent_id.clone(), e.to_string()))?;

        // Clean up state
        {
            let mut container_guard = self.state_container.write().await;
            *container_guard = None;
        }

        // Remove from state manager
        self.state_manager.remove_state(&self.agent_id).await?;

        // Update phase to terminated
        {
            let mut stats = self.stats.lock().await;
            stats.update_phase(LifecyclePhase::Terminated);
        }

        log::info!("Agent {} terminated due to: {:?}", self.agent_id, reason);
        Ok(())
    }

    /// Mark agent as failed
    pub async fn mark_failed(&self, error: String) {
        let mut stats = self.stats.lock().await;
        stats.update_phase(LifecyclePhase::Failed(error.clone()));
        stats.record_error();

        log::error!("Agent {} failed: {}", self.agent_id, error);
    }

    /// Get lifecycle statistics
    pub async fn get_stats(&self) -> LifecycleStats {
        self.stats.lock().await.clone()
    }

    /// Get current lifecycle phase
    pub async fn get_phase(&self) -> LifecyclePhase {
        let stats = self.stats.lock().await;
        stats.phase.clone()
    }

    /// Check if agent is running
    pub async fn is_running(&self) -> bool {
        let stats = self.stats.lock().await;
        matches!(
            stats.phase,
            LifecyclePhase::Running | LifecyclePhase::Hibernating
        )
    }

    /// Check if agent is terminated
    pub async fn is_terminated(&self) -> bool {
        let stats = self.stats.lock().await;
        matches!(
            stats.phase,
            LifecyclePhase::Terminated | LifecyclePhase::Failed(_)
        )
    }

    /// Get agent uptime
    pub async fn get_uptime(&self) -> Option<chrono::Duration> {
        let stats = self.stats.lock().await;
        stats.uptime()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CallContext, CastContext, GenAgent, InfoContext, TestAgentState};
    use std::sync::Arc;

    // Test GenAgent implementation
    struct TestLifecycleAgent;

    #[async_trait::async_trait]
    impl GenAgent<TestAgentState> for TestLifecycleAgent {
        type CallMessage = String;
        type CallReply = String;
        type CastMessage = String;
        type InfoMessage = String;

        async fn init(&mut self, args: GenAgentInitArgs) -> GenAgentResult<TestAgentState> {
            Ok(TestAgentState {
                counter: 0,
                name: "test_lifecycle_agent".to_string(),
                active: true,
            })
        }

        async fn handle_call(
            &mut self,
            message: Self::CallMessage,
            context: CallContext,
            mut state: TestAgentState,
        ) -> GenAgentResult<(Self::CallReply, TestAgentState)> {
            state.counter += 1;
            Ok((format!("Reply: {}", message), state))
        }

        async fn handle_cast(
            &mut self,
            message: Self::CastMessage,
            context: CastContext,
            mut state: TestAgentState,
        ) -> GenAgentResult<TestAgentState> {
            state.counter += 1;
            Ok(state)
        }

        async fn handle_info(
            &mut self,
            message: Self::InfoMessage,
            context: InfoContext,
            state: TestAgentState,
        ) -> GenAgentResult<TestAgentState> {
            Ok(state)
        }
    }

    #[tokio::test]
    async fn test_lifecycle_stats() {
        let agent_id = AgentPid::new();
        let mut stats = LifecycleStats::new(agent_id.clone());

        assert_eq!(stats.agent_id, agent_id);
        assert_eq!(stats.phase, LifecyclePhase::Created);
        assert_eq!(stats.total_messages_handled, 0);

        // Test phase update
        stats.update_phase(LifecyclePhase::Running);
        assert_eq!(stats.phase, LifecyclePhase::Running);
        assert!(stats.started_at.is_some());

        // Test message recording
        stats.record_message("call", Duration::from_millis(10));
        assert_eq!(stats.total_messages_handled, 1);
        assert_eq!(stats.total_calls, 1);
        assert_eq!(stats.average_message_time, Duration::from_millis(10));

        stats.record_message("cast", Duration::from_millis(20));
        assert_eq!(stats.total_messages_handled, 2);
        assert_eq!(stats.total_casts, 1);
        assert_eq!(stats.average_message_time, Duration::from_millis(15));

        // Test error recording
        stats.record_error();
        assert_eq!(stats.total_errors, 1);
    }

    #[tokio::test]
    async fn test_lifecycle_manager() {
        let agent_id = AgentPid::new();
        let supervisor_id = SupervisorId::new();
        let state_manager = Arc::new(StateManager::new(false));

        let lifecycle = LifecycleManager::<TestAgentState>::new(
            agent_id.clone(),
            supervisor_id.clone(),
            state_manager,
            Some(Duration::from_secs(60)),
        );

        let mut agent = TestLifecycleAgent;

        // Test initialization
        let args = GenAgentInitArgs {
            agent_id: agent_id.clone(),
            supervisor_id,
            config: serde_json::json!({}),
            timeout: Duration::from_secs(30),
        };

        lifecycle.initialize(&mut agent, args).await.unwrap();

        // Test that agent is running
        assert!(lifecycle.is_running().await);
        assert!(!lifecycle.is_terminated().await);

        let phase = lifecycle.get_phase().await;
        assert_eq!(phase, LifecyclePhase::Running);

        // Test state retrieval
        let state = lifecycle.get_state().await.unwrap();
        assert_eq!(state.name, "test_lifecycle_agent");
        assert_eq!(state.counter, 0);

        // Test state update
        let new_state = TestAgentState {
            counter: 42,
            name: "updated_agent".to_string(),
            active: false,
        };
        lifecycle.update_state(new_state.clone()).await.unwrap();

        let updated_state = lifecycle.get_state().await.unwrap();
        assert_eq!(updated_state, new_state);

        // Test message processing recording
        lifecycle
            .record_message_processing("call", Duration::from_millis(10))
            .await;

        let stats = lifecycle.get_stats().await;
        assert_eq!(stats.total_messages_handled, 1);
        assert_eq!(stats.total_calls, 1);

        // Test termination
        lifecycle
            .terminate(&mut agent, TerminateReason::Normal)
            .await
            .unwrap();

        assert!(!lifecycle.is_running().await);
        assert!(lifecycle.is_terminated().await);

        let final_phase = lifecycle.get_phase().await;
        assert_eq!(final_phase, LifecyclePhase::Terminated);
    }

    #[tokio::test]
    async fn test_hibernation() {
        let agent_id = AgentPid::new();
        let supervisor_id = SupervisorId::new();
        let state_manager = Arc::new(StateManager::new(false));

        let lifecycle = LifecycleManager::<TestAgentState>::new(
            agent_id.clone(),
            supervisor_id,
            state_manager,
            Some(Duration::from_millis(100)), // Short hibernation timeout for testing
        );

        let mut agent = TestLifecycleAgent;

        let args = GenAgentInitArgs {
            agent_id,
            supervisor_id: SupervisorId::new(),
            config: serde_json::json!({}),
            timeout: Duration::from_secs(30),
        };

        lifecycle.initialize(&mut agent, args).await.unwrap();

        // Initially should not hibernate
        assert!(!lifecycle.should_hibernate().await);

        // Wait for hibernation timeout
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Now should hibernate
        assert!(lifecycle.should_hibernate().await);

        // Test hibernation
        lifecycle.hibernate().await.unwrap();
        let phase = lifecycle.get_phase().await;
        assert_eq!(phase, LifecyclePhase::Hibernating);

        // Test wake up
        lifecycle.wake_up().await.unwrap();
        let phase = lifecycle.get_phase().await;
        assert_eq!(phase, LifecyclePhase::Running);
    }

    #[tokio::test]
    async fn test_error_handling() {
        let agent_id = AgentPid::new();
        let supervisor_id = SupervisorId::new();
        let state_manager = Arc::new(StateManager::new(false));

        let lifecycle =
            LifecycleManager::<TestAgentState>::new(agent_id, supervisor_id, state_manager, None);

        // Test marking as failed
        lifecycle.mark_failed("Test error".to_string()).await;

        let phase = lifecycle.get_phase().await;
        assert_eq!(phase, LifecyclePhase::Failed("Test error".to_string()));

        let stats = lifecycle.get_stats().await;
        assert_eq!(stats.total_errors, 1);

        assert!(!lifecycle.is_running().await);
        assert!(lifecycle.is_terminated().await);
    }
}
