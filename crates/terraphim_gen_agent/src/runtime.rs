//! GenAgent runtime system
//!
//! Provides the runtime environment for executing GenAgent instances with
//! message processing, state management, and supervision integration.

use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use futures_util::future::BoxFuture;
use tokio::sync::{mpsc, oneshot, Mutex, RwLock};
use tokio::task::JoinHandle;

use crate::{
    AgentState, GenAgent, GenAgentError, GenAgentResult, LifecycleManager, LifecyclePhase,
    LifecycleStats, StateManager, StateManagerStats, SystemMessage, TerminateReason,
};

// Re-export types from other crates
use terraphim_agent_messaging::{AgentMessage, MessageId, MessagingError};
use terraphim_agent_supervisor::{
    AgentPid, AgentStatus, InitArgs as GenAgentInitArgs, SupervisorId,
};

/// GenAgent message types
#[derive(Debug, Clone)]
pub enum GenAgentMessage {
    Call {
        message: Box<dyn Any + Send>,
        reply_to: oneshot::Sender<Box<dyn Any + Send>>,
    },
    Cast {
        message: Box<dyn Any + Send>,
    },
    Info {
        message: SystemMessage,
    },
}

/// Mailbox sender for agents
pub type MailboxSender = mpsc::Sender<GenAgentMessage>;

/// Runtime configuration for GenAgent
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub message_buffer_size: usize,
    pub max_concurrent_messages: usize,
    pub message_timeout: Duration,
    pub hibernation_timeout: Option<Duration>,
    pub enable_tracing: bool,
    pub enable_metrics: bool,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            message_buffer_size: 1000,
            max_concurrent_messages: 100,
            message_timeout: Duration::from_secs(30),
            hibernation_timeout: Some(Duration::from_secs(300)), // 5 minutes
            enable_tracing: false,
            enable_metrics: true,
        }
    }
}

/// GenAgent runtime instance
pub struct GenAgentRuntime<State: AgentState> {
    agent_id: AgentPid,
    supervisor_id: SupervisorId,
    config: RuntimeConfig,
    behavior_spec: BehaviorSpec,

    // Message handling
    message_receiver: Arc<Mutex<mpsc::Receiver<GenAgentMessage>>>,
    message_sender: mpsc::Sender<GenAgentMessage>,

    // State and lifecycle management
    lifecycle_manager: Arc<LifecycleManager<State>>,
    state_manager: Arc<dyn StateManager>,

    // Runtime control
    shutdown_sender: Option<oneshot::Sender<()>>,
    runtime_handle: Option<JoinHandle<GenAgentResult<()>>>,

    // Metrics and monitoring
    message_processing_times: Arc<Mutex<Vec<Duration>>>,
    last_health_check: Arc<Mutex<Instant>>,
}

impl<State: AgentState + 'static> GenAgentRuntime<State> {
    /// Create a new GenAgent runtime
    pub fn new(
        agent_id: AgentPid,
        supervisor_id: SupervisorId,
        state_manager: Arc<dyn StateManager>,
        config: RuntimeConfig,
        behavior_spec: BehaviorSpec,
    ) -> Self {
        let (message_sender, message_receiver) = mpsc::channel(config.message_buffer_size);

        let lifecycle_manager = Arc::new(LifecycleManager::new(
            agent_id.clone(),
            supervisor_id.clone(),
            state_manager.clone(),
            config.hibernation_timeout,
        ));

        Self {
            agent_id: agent_id.clone(),
            supervisor_id,
            config,
            behavior_spec,
            message_receiver: Arc::new(Mutex::new(message_receiver)),
            message_sender,
            lifecycle_manager,
            state_manager,
            shutdown_sender: None,
            runtime_handle: None,
            message_processing_times: Arc::new(Mutex::new(Vec::new())),
            last_health_check: Arc::new(Mutex::new(Instant::now())),
        }
    }

    /// Start the GenAgent runtime
    pub async fn start<A>(
        &mut self,
        mut agent: A,
        init_args: GenAgentInitArgs,
    ) -> GenAgentResult<()>
    where
        A: GenAgent<State> + Send + 'static,
    {
        // Initialize the agent
        self.lifecycle_manager
            .initialize(&mut agent, init_args)
            .await?;

        // Create shutdown channel
        let (shutdown_sender, shutdown_receiver) = oneshot::channel();
        self.shutdown_sender = Some(shutdown_sender);

        // Clone necessary components for the runtime task
        let agent_id = self.agent_id.clone();
        let config = self.config.clone();
        let behavior_spec = self.behavior_spec.clone();
        let message_receiver = self.message_receiver.clone();
        let lifecycle_manager = self.lifecycle_manager.clone();
        let message_processing_times = self.message_processing_times.clone();
        let last_health_check = self.last_health_check.clone();

        // Start the runtime task
        let runtime_handle = tokio::spawn(async move {
            Self::run_agent_loop(
                agent,
                agent_id,
                config,
                behavior_spec,
                message_receiver,
                lifecycle_manager,
                message_processing_times,
                last_health_check,
                shutdown_receiver,
            )
            .await
        });

        self.runtime_handle = Some(runtime_handle);

        log::info!("GenAgent runtime started for agent {}", self.agent_id);
        Ok(())
    }

    /// Stop the GenAgent runtime
    pub async fn stop(&mut self) -> GenAgentResult<()> {
        if let Some(shutdown_sender) = self.shutdown_sender.take() {
            let _ = shutdown_sender.send(());
        }

        if let Some(runtime_handle) = self.runtime_handle.take() {
            match runtime_handle.await {
                Ok(result) => result?,
                Err(e) => {
                    return Err(GenAgentError::System(format!("Runtime task failed: {}", e)));
                }
            }
        }

        log::info!("GenAgent runtime stopped for agent {}", self.agent_id);
        Ok(())
    }

    /// Send a call message to the agent
    pub async fn call<M, R>(&self, message: M, timeout: Duration) -> GenAgentResult<R>
    where
        M: Send + 'static,
        R: Send + 'static,
    {
        let (reply_sender, reply_receiver) = oneshot::channel();

        let context = CallContext {
            message_id: MessageId::new(),
            from: self.agent_id.clone(), // Self-call for now
            timeout,
        };

        let gen_message = GenAgentMessage::Call {
            message: Box::new(message),
            context,
            reply_sender: reply_sender,
        };

        self.message_sender
            .send(gen_message)
            .await
            .map_err(|_| GenAgentError::AgentNotRunning(self.agent_id.clone()))?;

        let reply = tokio::time::timeout(timeout, reply_receiver)
            .await
            .map_err(|_| GenAgentError::MessageTimeout(MessageId::new(), self.agent_id.clone()))?
            .map_err(|_| {
                GenAgentError::MessageHandlingFailed(
                    self.agent_id.clone(),
                    "Reply channel closed".to_string(),
                )
            })?;

        reply.downcast::<R>().map(|boxed| *boxed).map_err(|_| {
            GenAgentError::InvalidMessageType(
                self.agent_id.clone(),
                "Expected reply type".to_string(),
                "Unknown type".to_string(),
            )
        })
    }

    /// Send a cast message to the agent
    pub async fn cast<M>(&self, message: M) -> GenAgentResult<()>
    where
        M: Send + 'static,
    {
        let context = CastContext {
            message_id: MessageId::new(),
            from: self.agent_id.clone(), // Self-cast for now
        };

        let gen_message = GenAgentMessage::Cast {
            message: Box::new(message),
            context,
        };

        self.message_sender
            .send(gen_message)
            .await
            .map_err(|_| GenAgentError::AgentNotRunning(self.agent_id.clone()))?;

        Ok(())
    }

    /// Send an info message to the agent
    pub async fn info<M>(&self, message: M) -> GenAgentResult<()>
    where
        M: Send + 'static,
    {
        let context = InfoContext {
            message_id: MessageId::new(),
        };

        let gen_message = GenAgentMessage::Info {
            message: Box::new(message),
            context,
        };

        self.message_sender
            .send(gen_message)
            .await
            .map_err(|_| GenAgentError::AgentNotRunning(self.agent_id.clone()))?;

        Ok(())
    }

    /// Send a system message to the agent
    pub async fn system(&self, message: SystemMessage) -> GenAgentResult<()> {
        let gen_message = GenAgentMessage::System { message };

        self.message_sender
            .send(gen_message)
            .await
            .map_err(|_| GenAgentError::AgentNotRunning(self.agent_id.clone()))?;

        Ok(())
    }

    /// Get agent status
    pub async fn get_status(&self) -> AgentStatus {
        let phase = self.lifecycle_manager.get_phase().await;
        match phase {
            LifecyclePhase::Created => AgentStatus::Starting,
            LifecyclePhase::Initializing => AgentStatus::Starting,
            LifecyclePhase::Running => AgentStatus::Running,
            LifecyclePhase::Hibernating => AgentStatus::Running, // Still considered running
            LifecyclePhase::Terminating => AgentStatus::Stopping,
            LifecyclePhase::Terminated => AgentStatus::Stopped,
            LifecyclePhase::Failed(_) => AgentStatus::Failed,
        }
    }

    /// Get runtime statistics
    pub async fn get_stats(&self) -> RuntimeStats {
        let lifecycle_stats = self.lifecycle_manager.get_stats().await;
        let processing_times = self.message_processing_times.lock().await;
        let last_health_check = *self.last_health_check.lock().await;

        RuntimeStats {
            agent_id: self.agent_id.clone(),
            lifecycle_stats,
            average_processing_time: if processing_times.is_empty() {
                Duration::ZERO
            } else {
                let total: Duration = processing_times.iter().sum();
                total / processing_times.len() as u32
            },
            message_queue_size: self.message_sender.capacity() - self.message_sender.max_capacity(),
            last_health_check,
        }
    }

    /// Main agent execution loop
    async fn run_agent_loop<A>(
        mut agent: A,
        agent_id: AgentPid,
        config: RuntimeConfig,
        behavior_spec: BehaviorSpec,
        message_receiver: Arc<Mutex<mpsc::Receiver<GenAgentMessage>>>,
        lifecycle_manager: Arc<LifecycleManager<State>>,
        message_processing_times: Arc<Mutex<Vec<Duration>>>,
        last_health_check: Arc<Mutex<Instant>>,
        mut shutdown_receiver: oneshot::Receiver<()>,
    ) -> GenAgentResult<()>
    where
        A: GenAgent<State> + Send,
    {
        let mut hibernation_timer = if let Some(timeout) = config.hibernation_timeout {
            Some(tokio::time::interval(timeout))
        } else {
            None
        };

        loop {
            tokio::select! {
                // Handle shutdown signal
                _ = &mut shutdown_receiver => {
                    log::info!("Agent {} received shutdown signal", agent_id);
                    break;
                }

                // Handle hibernation check
                _ = async {
                    if let Some(ref mut timer) = hibernation_timer {
                        timer.tick().await;
                    } else {
                        futures_util::future::pending::<()>().await;
                    }
                } => {
                    if lifecycle_manager.should_hibernate().await {
                        lifecycle_manager.hibernate().await?;

                        // Wait for next message to wake up
                        let mut receiver = message_receiver.lock().await;
                        if let Some(message) = receiver.recv().await {
                            lifecycle_manager.wake_up().await?;
                            drop(receiver);
                            Self::process_message(&mut agent, message, &lifecycle_manager, &message_processing_times).await?;
                        }
                    }
                }

                // Handle incoming messages
                message = async {
                    let mut receiver = message_receiver.lock().await;
                    receiver.recv().await
                } => {
                    if let Some(message) = message {
                        Self::process_message(&mut agent, message, &lifecycle_manager, &message_processing_times).await?;
                    } else {
                        // Channel closed, exit loop
                        break;
                    }
                }
            }

            // Update health check timestamp
            {
                let mut last_check = last_health_check.lock().await;
                *last_check = Instant::now();
            }
        }

        // Terminate the agent
        lifecycle_manager
            .terminate(&mut agent, TerminateReason::Normal)
            .await?;

        Ok(())
    }

    /// Process a single message
    async fn process_message<A>(
        agent: &mut A,
        message: GenAgentMessage,
        lifecycle_manager: &Arc<LifecycleManager<State>>,
        message_processing_times: &Arc<Mutex<Vec<Duration>>>,
    ) -> GenAgentResult<()>
    where
        A: GenAgent<State>,
    {
        let start_time = Instant::now();
        let message_type = match &message {
            GenAgentMessage::Call { .. } => "call",
            GenAgentMessage::Cast { .. } => "cast",
            GenAgentMessage::Info { .. } => "info",
            GenAgentMessage::System { .. } => "system",
        };

        let result = match message {
            GenAgentMessage::Call {
                message,
                context,
                reply_sender,
            } => {
                let state = lifecycle_manager.get_state().await?;

                // This is a simplified version - in practice, you'd need proper type handling
                let (reply, new_state) = agent
                    .handle_call(
                        message, // This would need proper downcasting
                        context, state,
                    )
                    .await?;

                lifecycle_manager.update_state(new_state).await?;

                let _ = reply_sender.send(Box::new(reply));
                Ok(())
            }

            GenAgentMessage::Cast { message, context } => {
                let state = lifecycle_manager.get_state().await?;

                let new_state = agent
                    .handle_cast(
                        message, // This would need proper downcasting
                        state,
                    )
                    .await?;

                lifecycle_manager.update_state(new_state).await?;
                Ok(())
            }

            GenAgentMessage::Info { message, context } => {
                let state = lifecycle_manager.get_state().await?;

                let new_state = agent
                    .handle_info(
                        message, // This would need proper downcasting
                        state,
                    )
                    .await?;

                lifecycle_manager.update_state(new_state).await?;
                Ok(())
            }

            GenAgentMessage::System { message } => {
                let state = lifecycle_manager.get_state().await?;

                let new_state = agent.handle_info(message, state).await?;

                lifecycle_manager.update_state(new_state).await?;
                Ok(())
            }
        };

        let processing_time = start_time.elapsed();

        // Record processing time
        lifecycle_manager
            .record_message_processing(message_type, processing_time)
            .await;

        // Store processing time for statistics
        {
            let mut times = message_processing_times.lock().await;
            times.push(processing_time);

            // Keep only the last 1000 processing times
            if times.len() > 1000 {
                times.remove(0);
            }
        }

        if let Err(e) = result {
            lifecycle_manager.record_error().await;
            return Err(e);
        }

        Ok(())
    }

    /// Get the message sender for external communication
    pub fn get_message_sender(&self) -> mpsc::Sender<GenAgentMessage> {
        self.message_sender.clone()
    }

    /// Check if the runtime is running
    pub fn is_running(&self) -> bool {
        self.runtime_handle.is_some()
    }
}

/// Runtime statistics
#[derive(Debug, Clone)]
pub struct RuntimeStats {
    pub agent_id: AgentPid,
    pub lifecycle_stats: crate::LifecycleStats,
    pub average_processing_time: Duration,
    pub message_queue_size: usize,
    pub last_health_check: Instant,
}

/// GenAgent factory for creating and managing agent runtimes
pub struct GenAgentFactory {
    state_manager: Arc<dyn StateManager>,
    default_config: RuntimeConfig,
    runtimes: Arc<RwLock<HashMap<AgentPid, Box<dyn Any + Send + Sync>>>>,
}

impl GenAgentFactory {
    pub fn new(state_manager: Arc<dyn StateManager>, default_config: RuntimeConfig) -> Self {
        Self {
            state_manager,
            default_config,
            runtimes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new GenAgent runtime
    pub async fn create_agent<A, State>(
        &self,
        agent: A,
        agent_id: AgentPid,
        supervisor_id: SupervisorId,
        init_args: GenAgentInitArgs,
        behavior_spec: Option<BehaviorSpec>,
        config: Option<RuntimeConfig>,
    ) -> GenAgentResult<Arc<GenAgentRuntime<State>>>
    where
        A: GenAgent<State> + Send + 'static,
        State: AgentState + 'static,
    {
        let config = config.unwrap_or_else(|| self.default_config.clone());
        let behavior_spec = behavior_spec.unwrap_or_default();

        let mut runtime = GenAgentRuntime::new(
            agent_id.clone(),
            supervisor_id,
            self.state_manager.clone(),
            config,
            behavior_spec,
        );

        runtime.start(agent, init_args).await?;

        let runtime = Arc::new(runtime);

        // Store runtime
        {
            let mut runtimes = self.runtimes.write().await;
            runtimes.insert(agent_id.clone(), Box::new(runtime.clone()));
        }

        Ok(runtime)
    }

    /// Get an existing runtime
    pub async fn get_runtime<State: AgentState + 'static>(
        &self,
        agent_id: &AgentPid,
    ) -> Option<Arc<GenAgentRuntime<State>>> {
        let runtimes = self.runtimes.read().await;
        runtimes
            .get(agent_id)
            .and_then(|runtime_any| runtime_any.downcast_ref::<Arc<GenAgentRuntime<State>>>())
            .cloned()
    }

    /// Stop and remove a runtime
    pub async fn stop_agent(&self, agent_id: &AgentPid) -> GenAgentResult<()> {
        let runtime_any = {
            let mut runtimes = self.runtimes.write().await;
            runtimes.remove(agent_id)
        };

        if let Some(_runtime_any) = runtime_any {
            // In practice, we'd need to properly handle the type-erased runtime
            // For now, we'll just log the removal
            log::info!("Agent {} runtime removed", agent_id);
        }

        Ok(())
    }

    /// List all active runtimes
    pub async fn list_agents(&self) -> Vec<AgentPid> {
        let runtimes = self.runtimes.read().await;
        runtimes.keys().cloned().collect()
    }

    /// Get factory statistics
    pub async fn get_stats(&self) -> FactoryStats {
        let runtimes = self.runtimes.read().await;
        FactoryStats {
            total_agents: runtimes.len(),
            state_manager_stats: self.state_manager.get_stats().await,
        }
    }
}

/// Factory statistics
#[derive(Debug, Clone)]
pub struct FactoryStats {
    pub total_agents: usize,
    pub state_manager_stats: crate::StateManagerStats,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CallContext, CastContext, GenAgent, InfoContext, TestAgentState};

    // Test GenAgent implementation
    struct TestRuntimeAgent;

    #[async_trait]
    impl GenAgent<TestAgentState> for TestRuntimeAgent {
        type CallMessage = String;
        type CallReply = String;
        type CastMessage = String;
        type InfoMessage = String;

        async fn init(&mut self, args: GenAgentInitArgs) -> GenAgentResult<TestAgentState> {
            Ok(TestAgentState {
                counter: 0,
                name: "test_runtime_agent".to_string(),
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
            Ok((format!("Processed: {}", message), state))
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
    async fn test_runtime_config() {
        let config = RuntimeConfig::default();
        assert_eq!(config.message_buffer_size, 1000);
        assert_eq!(config.max_concurrent_messages, 100);
        assert_eq!(config.message_timeout, Duration::from_secs(30));
    }

    #[tokio::test]
    async fn test_genagent_factory() {
        let state_manager = Arc::new(StateManager::new(false));
        let config = RuntimeConfig::default();
        let factory = GenAgentFactory::new(state_manager, config);

        let agent_id = AgentPid::new();
        let supervisor_id = SupervisorId::new();

        let init_args = GenAgentInitArgs {
            agent_id: agent_id.clone(),
            supervisor_id: supervisor_id.clone(),
            config: serde_json::json!({}),
            timeout: Duration::from_secs(30),
        };

        let agent = TestRuntimeAgent;

        // Create agent runtime
        let runtime = factory
            .create_agent(
                agent,
                agent_id.clone(),
                supervisor_id,
                init_args,
                None,
                None,
            )
            .await
            .unwrap();

        assert!(runtime.is_running());

        // Test factory stats
        let stats = factory.get_stats().await;
        assert_eq!(stats.total_agents, 1);

        // List agents
        let agents = factory.list_agents().await;
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0], agent_id);

        // Stop agent
        factory.stop_agent(&agent_id).await.unwrap();

        let final_stats = factory.get_stats().await;
        assert_eq!(final_stats.total_agents, 0);
    }
}
