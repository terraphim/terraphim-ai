//! GenAgent behavior trait and message handling patterns
//!
//! Implements the core GenAgent trait following OTP GenServer patterns.

use std::any::Any;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{AgentState, GenAgentError, GenAgentResult, StateTransition};

// Re-export types from other crates
use terraphim_agent_messaging::{AgentMessage, MessageId};
use terraphim_agent_supervisor::{AgentPid, InitArgs, SupervisorId};

/// Reply types for GenAgent call messages
pub type CallReply<T> = GenAgentResult<T>;

/// Reasons for agent termination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TerminateReason {
    Normal,
    Shutdown,
    Error(String),
    Timeout,
    SupervisorRequest,
    UserRequest,
}

/// System messages that agents can receive
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemMessage {
    /// Shutdown request
    Shutdown,
    /// Restart request
    Restart,
    /// Health check request
    HealthCheck,
    /// Status update
    StatusUpdate(String),
    /// Supervisor message
    SupervisorMessage(String),
    /// Custom system message
    Custom {
        message_type: String,
        data: serde_json::Value,
    },
}

/// Context for call messages
#[derive(Debug, Clone)]
pub struct CallContext {
    pub message_id: MessageId,
    pub sender: AgentPid,
    pub timeout: Duration,
}

/// Context for cast messages  
#[derive(Debug, Clone)]
pub struct CastContext {
    pub message_id: MessageId,
    pub sender: AgentPid,
}

/// Context for info messages
#[derive(Debug, Clone)]
pub struct InfoContext {
    pub message_id: MessageId,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Behavior specification for agents
#[derive(Debug, Clone)]
pub struct BehaviorSpec {
    pub name: String,
    pub version: String,
    pub capabilities: Vec<String>,
}

/// Core GenAgent trait following OTP GenServer patterns
#[async_trait]
pub trait GenAgent<State>: Send + Sync
where
    State: AgentState + 'static,
{
    /// Message type this agent handles
    type Message: Send + Sync + 'static;

    /// Reply type for call messages
    type Reply: Send + Sync + 'static;

    /// Initialize the agent (gen_server:init)
    async fn init(&mut self, args: InitArgs) -> GenAgentResult<State>;

    /// Handle synchronous call messages (gen_server:handle_call)
    async fn handle_call(
        &mut self,
        message: Self::Message,
        from: AgentPid,
        state: State,
    ) -> GenAgentResult<(CallReply<Self::Reply>, StateTransition<State>)>;

    /// Handle asynchronous cast messages (gen_server:handle_cast)
    async fn handle_cast(
        &mut self,
        message: Self::Message,
        state: State,
    ) -> GenAgentResult<StateTransition<State>>;

    /// Handle system info messages (gen_server:handle_info)
    async fn handle_info(
        &mut self,
        info: SystemMessage,
        state: State,
    ) -> GenAgentResult<StateTransition<State>>;

    /// Handle agent termination (gen_server:terminate)
    async fn terminate(&mut self, reason: TerminateReason, state: State) -> GenAgentResult<()>;

    /// Get agent configuration
    fn config(&self) -> GenAgentConfig {
        GenAgentConfig::default()
    }

    /// Handle timeout (optional)
    async fn handle_timeout(&mut self, state: State) -> GenAgentResult<StateTransition<State>> {
        Ok(StateTransition::Continue(state))
    }

    /// Code change handler for hot reloading (optional)
    async fn code_change(
        &mut self,
        old_version: u32,
        state: State,
        extra: serde_json::Value,
    ) -> GenAgentResult<State> {
        let _ = (old_version, extra);
        Ok(state)
    }
}

/// Configuration for GenAgent behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenAgentConfig {
    /// Timeout for call messages
    pub call_timeout: Duration,
    /// Whether to enable state persistence
    pub enable_persistence: bool,
    /// Hibernate timeout (pause message processing)
    pub hibernate_timeout: Option<Duration>,
    /// Maximum message queue size
    pub max_queue_size: Option<usize>,
    /// Whether to enable debug logging
    pub debug_logging: bool,
}

impl Default for GenAgentConfig {
    fn default() -> Self {
        Self {
            call_timeout: Duration::from_secs(30),
            enable_persistence: false,
            hibernate_timeout: None,
            max_queue_size: None,
            debug_logging: false,
        }
    }
}

/// GenAgent runtime statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenAgentStats {
    pub agent_id: AgentPid,
    pub supervisor_id: SupervisorId,
    pub state_type: String,
    pub messages_handled: u64,
    pub calls_handled: u64,
    pub casts_handled: u64,
    pub info_handled: u64,
    pub errors_encountered: u64,
    pub last_message_time: Option<chrono::DateTime<chrono::Utc>>,
    pub uptime: Duration,
    pub current_state_size: usize,
}

impl GenAgentStats {
    pub fn new(agent_id: AgentPid, supervisor_id: SupervisorId, state_type: String) -> Self {
        Self {
            agent_id,
            supervisor_id,
            state_type,
            messages_handled: 0,
            calls_handled: 0,
            casts_handled: 0,
            info_handled: 0,
            errors_encountered: 0,
            last_message_time: None,
            uptime: Duration::ZERO,
            current_state_size: 0,
        }
    }

    pub fn record_call(&mut self) {
        self.messages_handled += 1;
        self.calls_handled += 1;
        self.last_message_time = Some(chrono::Utc::now());
    }

    pub fn record_cast(&mut self) {
        self.messages_handled += 1;
        self.casts_handled += 1;
        self.last_message_time = Some(chrono::Utc::now());
    }

    pub fn record_info(&mut self) {
        self.messages_handled += 1;
        self.info_handled += 1;
        self.last_message_time = Some(chrono::Utc::now());
    }

    pub fn record_error(&mut self) {
        self.errors_encountered += 1;
    }

    pub fn update_state_size(&mut self, size: usize) {
        self.current_state_size = size;
    }
}

/// Example GenAgent implementation for testing
pub struct ExampleGenAgent {
    agent_id: AgentPid,
    supervisor_id: SupervisorId,
    config: GenAgentConfig,
}

impl ExampleGenAgent {
    pub fn new() -> Self {
        Self {
            agent_id: AgentPid::new(),
            supervisor_id: SupervisorId::new(),
            config: GenAgentConfig::default(),
        }
    }

    pub fn with_config(mut self, config: GenAgentConfig) -> Self {
        self.config = config;
        self
    }
}

impl Default for ExampleGenAgent {
    fn default() -> Self {
        Self::new()
    }
}

/// Example message type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExampleMessage {
    Increment,
    Decrement,
    GetCount,
    SetName(String),
    Reset,
}

/// Example reply type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExampleReply {
    Ok,
    Count(u64),
    Name(String),
}

#[async_trait]
impl GenAgent<crate::state::ExampleState> for ExampleGenAgent {
    type Message = ExampleMessage;
    type Reply = ExampleReply;

    async fn init(&mut self, args: InitArgs) -> GenAgentResult<crate::state::ExampleState> {
        self.agent_id = args.agent_id;
        self.supervisor_id = args.supervisor_id;

        let name = args
            .config
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("example_agent")
            .to_string();

        Ok(crate::state::ExampleState::new(name))
    }

    async fn handle_call(
        &mut self,
        message: Self::Message,
        _from: AgentPid,
        mut state: crate::state::ExampleState,
    ) -> GenAgentResult<(
        CallReply<Self::Reply>,
        StateTransition<crate::state::ExampleState>,
    )> {
        match message {
            ExampleMessage::GetCount => {
                let reply = Ok(ExampleReply::Count(state.counter));
                Ok((reply, StateTransition::Continue(state)))
            }
            ExampleMessage::Increment => {
                state.increment();
                let reply = Ok(ExampleReply::Ok);
                Ok((reply, StateTransition::Continue(state)))
            }
            ExampleMessage::SetName(name) => {
                state.name = name.clone();
                let reply = Ok(ExampleReply::Name(name));
                Ok((reply, StateTransition::Continue(state)))
            }
            _ => {
                let reply = Err(GenAgentError::InvalidMessageType(
                    self.agent_id.clone(),
                    "Message not supported in call".to_string(),
                ));
                Ok((reply, StateTransition::Continue(state)))
            }
        }
    }

    async fn handle_cast(
        &mut self,
        message: Self::Message,
        mut state: crate::state::ExampleState,
    ) -> GenAgentResult<StateTransition<crate::state::ExampleState>> {
        match message {
            ExampleMessage::Increment => {
                state.increment();
                Ok(StateTransition::Continue(state))
            }
            ExampleMessage::Decrement => {
                if state.counter > 0 {
                    state.counter -= 1;
                }
                Ok(StateTransition::Continue(state))
            }
            ExampleMessage::Reset => {
                state.counter = 0;
                Ok(StateTransition::Continue(state))
            }
            _ => {
                log::warn!("Unsupported cast message: {:?}", message);
                Ok(StateTransition::Continue(state))
            }
        }
    }

    async fn handle_info(
        &mut self,
        info: SystemMessage,
        state: crate::state::ExampleState,
    ) -> GenAgentResult<StateTransition<crate::state::ExampleState>> {
        match info {
            SystemMessage::Shutdown => Ok(StateTransition::Stop("Shutdown requested".to_string())),
            SystemMessage::HealthCheck => {
                log::info!("Agent {} health check: OK", self.agent_id);
                Ok(StateTransition::Continue(state))
            }
            SystemMessage::StatusUpdate(status) => {
                log::info!("Agent {} status update: {}", self.agent_id, status);
                Ok(StateTransition::Continue(state))
            }
            _ => {
                log::debug!(
                    "Agent {} received system message: {:?}",
                    self.agent_id,
                    info
                );
                Ok(StateTransition::Continue(state))
            }
        }
    }

    async fn terminate(
        &mut self,
        reason: TerminateReason,
        _state: crate::state::ExampleState,
    ) -> GenAgentResult<()> {
        log::info!("Agent {} terminating: {:?}", self.agent_id, reason);
        Ok(())
    }

    fn config(&self) -> GenAgentConfig {
        self.config.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::ExampleState;

    #[tokio::test]
    async fn test_example_gen_agent_init() {
        let mut agent = ExampleGenAgent::new();
        let args = InitArgs {
            agent_id: AgentPid::new(),
            supervisor_id: SupervisorId::new(),
            config: serde_json::json!({"name": "test_agent"}),
        };

        let state = agent.init(args).await.unwrap();
        assert_eq!(state.name, "test_agent");
        assert_eq!(state.counter, 0);
        assert!(state.active);
    }

    #[tokio::test]
    async fn test_example_gen_agent_call() {
        let mut agent = ExampleGenAgent::new();
        let state = ExampleState::new("test".to_string());

        // Test GetCount call
        let (reply, new_state) = agent
            .handle_call(ExampleMessage::GetCount, AgentPid::new(), state.clone())
            .await
            .unwrap();

        match reply.unwrap() {
            ExampleReply::Count(count) => assert_eq!(count, 0),
            _ => panic!("Expected Count reply"),
        }
        assert!(new_state.is_continue());

        // Test Increment call
        let (reply, new_state) = agent
            .handle_call(ExampleMessage::Increment, AgentPid::new(), state)
            .await
            .unwrap();

        assert!(matches!(reply.unwrap(), ExampleReply::Ok));
        let state = new_state.state().unwrap();
        assert_eq!(state.counter, 1);
    }

    #[tokio::test]
    async fn test_example_gen_agent_cast() {
        let mut agent = ExampleGenAgent::new();
        let state = ExampleState::new("test".to_string());

        // Test Increment cast
        let new_state = agent
            .handle_cast(ExampleMessage::Increment, state)
            .await
            .unwrap();
        let state = new_state.state().unwrap();
        assert_eq!(state.counter, 1);

        // Test Decrement cast
        let new_state = agent
            .handle_cast(ExampleMessage::Decrement, state)
            .await
            .unwrap();
        let state = new_state.state().unwrap();
        assert_eq!(state.counter, 0);

        // Test Reset cast
        let mut state = state;
        state.counter = 10;
        let new_state = agent
            .handle_cast(ExampleMessage::Reset, state)
            .await
            .unwrap();
        let state = new_state.state().unwrap();
        assert_eq!(state.counter, 0);
    }

    #[tokio::test]
    async fn test_example_gen_agent_info() {
        let mut agent = ExampleGenAgent::new();
        let state = ExampleState::new("test".to_string());

        // Test HealthCheck info
        let new_state = agent
            .handle_info(SystemMessage::HealthCheck, state.clone())
            .await
            .unwrap();
        assert!(new_state.is_continue());

        // Test Shutdown info
        let new_state = agent
            .handle_info(SystemMessage::Shutdown, state)
            .await
            .unwrap();
        assert!(new_state.is_stop());
        assert_eq!(new_state.stop_reason(), Some("Shutdown requested"));
    }

    #[tokio::test]
    async fn test_gen_agent_stats() {
        let agent_id = AgentPid::new();
        let supervisor_id = SupervisorId::new();
        let mut stats =
            GenAgentStats::new(agent_id.clone(), supervisor_id, "ExampleState".to_string());

        assert_eq!(stats.messages_handled, 0);
        assert_eq!(stats.calls_handled, 0);

        stats.record_call();
        assert_eq!(stats.messages_handled, 1);
        assert_eq!(stats.calls_handled, 1);
        assert!(stats.last_message_time.is_some());

        stats.record_cast();
        assert_eq!(stats.messages_handled, 2);
        assert_eq!(stats.casts_handled, 1);

        stats.record_error();
        assert_eq!(stats.errors_encountered, 1);
    }

    #[test]
    fn test_gen_agent_config() {
        let config = GenAgentConfig::default();
        assert_eq!(config.call_timeout, Duration::from_secs(30));
        assert!(!config.enable_persistence);
        assert!(config.hibernate_timeout.is_none());

        let custom_config = GenAgentConfig {
            call_timeout: Duration::from_secs(10),
            enable_persistence: true,
            hibernate_timeout: Some(Duration::from_secs(60)),
            max_queue_size: Some(1000),
            debug_logging: true,
        };

        assert_eq!(custom_config.call_timeout, Duration::from_secs(10));
        assert!(custom_config.enable_persistence);
        assert_eq!(
            custom_config.hibernate_timeout,
            Some(Duration::from_secs(60))
        );
    }
}
