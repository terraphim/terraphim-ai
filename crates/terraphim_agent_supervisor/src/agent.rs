//! Agent trait and lifecycle management
//!
//! Defines the core agent interface and lifecycle management for supervised agents.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    AgentPid, AgentStatus, InitArgs, SupervisionError, SupervisionResult, SupervisorId,
    SystemMessage, TerminateReason,
};

/// Core agent trait for supervised agents
#[async_trait]
pub trait SupervisedAgent: Send + Sync {
    /// Initialize the agent with the given arguments
    async fn init(&mut self, args: InitArgs) -> SupervisionResult<()>;

    /// Start the agent's main execution loop
    async fn start(&mut self) -> SupervisionResult<()>;

    /// Stop the agent gracefully
    async fn stop(&mut self) -> SupervisionResult<()>;

    /// Handle system messages from supervisor
    async fn handle_system_message(&mut self, message: SystemMessage) -> SupervisionResult<()>;

    /// Get the agent's current status
    fn status(&self) -> AgentStatus;

    /// Get the agent's unique identifier
    fn pid(&self) -> &AgentPid;

    /// Get the agent's supervisor identifier
    fn supervisor_id(&self) -> &SupervisorId;

    /// Perform health check
    async fn health_check(&self) -> SupervisionResult<bool>;

    /// Cleanup resources on termination
    async fn terminate(&mut self, reason: TerminateReason) -> SupervisionResult<()>;
}

/// Agent specification for creating supervised agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSpec {
    /// Unique identifier for the agent
    pub agent_id: AgentPid,
    /// Agent type identifier
    pub agent_type: String,
    /// Agent configuration
    pub config: serde_json::Value,
    /// Agent name for debugging
    pub name: Option<String>,
}

impl AgentSpec {
    /// Create a new agent specification
    pub fn new(agent_type: String, config: serde_json::Value) -> Self {
        Self {
            agent_id: AgentPid::new(),
            agent_type,
            config,
            name: None,
        }
    }

    /// Set the agent name
    pub fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    /// Set the agent ID
    pub fn with_id(mut self, agent_id: AgentPid) -> Self {
        self.agent_id = agent_id;
        self
    }
}

/// Information about a supervised agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisedAgentInfo {
    pub pid: AgentPid,
    pub supervisor_id: SupervisorId,
    pub spec: AgentSpec,
    pub status: AgentStatus,
    pub start_time: DateTime<Utc>,
    pub restart_count: u32,
    pub last_restart: Option<DateTime<Utc>>,
    pub last_health_check: Option<DateTime<Utc>>,
}

impl SupervisedAgentInfo {
    /// Create new agent info
    pub fn new(pid: AgentPid, supervisor_id: SupervisorId, spec: AgentSpec) -> Self {
        Self {
            pid,
            supervisor_id,
            spec,
            status: AgentStatus::Starting,
            start_time: Utc::now(),
            restart_count: 0,
            last_restart: None,
            last_health_check: None,
        }
    }

    /// Update agent status
    pub fn update_status(&mut self, status: AgentStatus) {
        self.status = status;
    }

    /// Record a restart
    pub fn record_restart(&mut self) {
        self.restart_count += 1;
        self.last_restart = Some(Utc::now());
    }

    /// Record health check
    pub fn record_health_check(&mut self) {
        self.last_health_check = Some(Utc::now());
    }

    /// Check if agent is running
    pub fn is_running(&self) -> bool {
        matches!(self.status, AgentStatus::Running)
    }

    /// Check if agent has failed
    pub fn is_failed(&self) -> bool {
        matches!(self.status, AgentStatus::Failed(_))
    }

    /// Get uptime duration
    pub fn uptime(&self) -> chrono::Duration {
        Utc::now() - self.start_time
    }
}

/// Factory trait for creating supervised agents
#[async_trait]
pub trait AgentFactory: Send + Sync {
    /// Create a new agent instance from specification
    async fn create_agent(&self, spec: &AgentSpec) -> SupervisionResult<Box<dyn SupervisedAgent>>;

    /// Validate agent specification
    fn validate_spec(&self, spec: &AgentSpec) -> SupervisionResult<()>;

    /// Get supported agent types
    fn supported_types(&self) -> Vec<String>;
}

/// Basic agent implementation for testing
#[derive(Debug)]
pub struct TestAgent {
    pid: AgentPid,
    supervisor_id: SupervisorId,
    status: AgentStatus,
    config: serde_json::Value,
}

impl Default for TestAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl TestAgent {
    pub fn new() -> Self {
        Self {
            pid: AgentPid::new(),
            supervisor_id: SupervisorId::new(),
            status: AgentStatus::Stopped,
            config: serde_json::Value::Null,
        }
    }
}

#[async_trait]
impl SupervisedAgent for TestAgent {
    async fn init(&mut self, args: InitArgs) -> SupervisionResult<()> {
        self.pid = args.agent_id;
        self.supervisor_id = args.supervisor_id;
        self.config = args.config;
        self.status = AgentStatus::Starting;
        Ok(())
    }

    async fn start(&mut self) -> SupervisionResult<()> {
        self.status = AgentStatus::Running;
        log::info!("Test agent {} started", self.pid);
        Ok(())
    }

    async fn stop(&mut self) -> SupervisionResult<()> {
        self.status = AgentStatus::Stopping;
        // Simulate some cleanup work
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        self.status = AgentStatus::Stopped;
        log::info!("Test agent {} stopped", self.pid);
        Ok(())
    }

    async fn handle_system_message(&mut self, message: SystemMessage) -> SupervisionResult<()> {
        match message {
            SystemMessage::Shutdown => {
                self.stop().await?;
            }
            SystemMessage::Restart => {
                self.stop().await?;
                self.start().await?;
            }
            SystemMessage::HealthCheck => {
                // Health check handled by health_check method
            }
            SystemMessage::StatusUpdate(status) => {
                self.status = status;
            }
            SystemMessage::SupervisorMessage(msg) => {
                log::info!("Agent {} received supervisor message: {}", self.pid, msg);
            }
        }
        Ok(())
    }

    fn status(&self) -> AgentStatus {
        self.status.clone()
    }

    fn pid(&self) -> &AgentPid {
        &self.pid
    }

    fn supervisor_id(&self) -> &SupervisorId {
        &self.supervisor_id
    }

    async fn health_check(&self) -> SupervisionResult<bool> {
        // Simple health check - agent is healthy if running
        Ok(matches!(self.status, AgentStatus::Running))
    }

    async fn terminate(&mut self, reason: TerminateReason) -> SupervisionResult<()> {
        log::info!("Agent {} terminating due to: {:?}", self.pid, reason);
        self.status = AgentStatus::Stopped;
        Ok(())
    }
}

/// Test agent factory
pub struct TestAgentFactory;

#[async_trait]
impl AgentFactory for TestAgentFactory {
    async fn create_agent(&self, _spec: &AgentSpec) -> SupervisionResult<Box<dyn SupervisedAgent>> {
        Ok(Box::new(TestAgent::new()))
    }

    fn validate_spec(&self, spec: &AgentSpec) -> SupervisionResult<()> {
        if spec.agent_type != "test" {
            return Err(SupervisionError::InvalidAgentSpec(format!(
                "Unsupported agent type: {}",
                spec.agent_type
            )));
        }
        Ok(())
    }

    fn supported_types(&self) -> Vec<String> {
        vec!["test".to_string()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_agent_spec_creation() {
        let spec = AgentSpec::new("test".to_string(), json!({"key": "value"}))
            .with_name("test-agent".to_string());

        assert_eq!(spec.agent_type, "test");
        assert_eq!(spec.name, Some("test-agent".to_string()));
        assert_eq!(spec.config, json!({"key": "value"}));
    }

    #[test]
    fn test_supervised_agent_info() {
        let pid = AgentPid::new();
        let supervisor_id = SupervisorId::new();
        let spec = AgentSpec::new("test".to_string(), json!({}));

        let mut info = SupervisedAgentInfo::new(pid.clone(), supervisor_id, spec);

        assert_eq!(info.pid, pid);
        assert_eq!(info.restart_count, 0);
        assert!(!info.is_running());

        info.update_status(AgentStatus::Running);
        assert!(info.is_running());

        info.record_restart();
        assert_eq!(info.restart_count, 1);
        assert!(info.last_restart.is_some());
    }

    #[tokio::test]
    async fn test_test_agent_lifecycle() {
        let mut agent = TestAgent::new();
        let args = InitArgs {
            agent_id: AgentPid::new(),
            supervisor_id: SupervisorId::new(),
            config: json!({}),
        };

        // Initialize agent
        agent.init(args).await.unwrap();
        assert_eq!(agent.status(), AgentStatus::Starting);

        // Start agent
        agent.start().await.unwrap();
        assert_eq!(agent.status(), AgentStatus::Running);

        // Health check
        assert!(agent.health_check().await.unwrap());

        // Stop agent
        agent.stop().await.unwrap();
        assert_eq!(agent.status(), AgentStatus::Stopped);
    }

    #[tokio::test]
    async fn test_test_agent_factory() {
        let factory = TestAgentFactory;
        let spec = AgentSpec::new("test".to_string(), json!({}));

        // Validate spec
        factory.validate_spec(&spec).unwrap();

        // Create agent
        let agent = factory.create_agent(&spec).await.unwrap();
        assert_eq!(agent.status(), AgentStatus::Stopped);

        // Check supported types
        assert_eq!(factory.supported_types(), vec!["test"]);
    }
}
