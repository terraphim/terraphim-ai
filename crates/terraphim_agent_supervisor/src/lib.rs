//! # Terraphim Agent Supervisor
//!
//! OTP-inspired supervision trees for fault-tolerant AI agent management.
//!
//! This crate provides Erlang/OTP-style supervision patterns for managing AI agents,
//! including automatic restart strategies, fault isolation, and hierarchical supervision.
//!
//! ## Core Concepts
//!
//! - **Supervision Trees**: Hierarchical fault tolerance with automatic restart
//! - **"Let It Crash"**: Fast failure detection with supervisor recovery
//! - **Restart Strategies**: OneForOne, OneForAll, RestForOne patterns
//! - **Agent Lifecycle**: Spawn, monitor, restart, terminate with state persistence

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod agent;
pub mod error;
pub mod restart_strategy;
pub mod supervisor;

pub use agent::*;
pub use error::*;
pub use restart_strategy::*;
pub use supervisor::*;

/// Unique identifier for agents in the supervision system
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentPid(pub Uuid);

impl AgentPid {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn as_str(&self) -> String {
        self.0.to_string()
    }
}

impl Default for AgentPid {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for AgentPid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for supervisors
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SupervisorId(pub Uuid);

impl SupervisorId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn as_str(&self) -> String {
        self.0.to_string()
    }
}

impl Default for SupervisorId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SupervisorId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Agent execution state
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentStatus {
    Starting,
    Running,
    Stopping,
    Stopped,
    Failed(String),
    Restarting,
}

/// Reasons for agent termination
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExitReason {
    Normal,
    Shutdown,
    Kill,
    Error(String),
    Timeout,
    SupervisorShutdown,
}

/// Reasons for agent termination in supervision context
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TerminateReason {
    Normal,
    Shutdown,
    Error(String),
    Timeout,
    SupervisorRequest,
}

/// System messages for agent supervision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemMessage {
    Shutdown,
    Restart,
    HealthCheck,
    StatusUpdate(AgentStatus),
    SupervisorMessage(String),
}

/// Agent initialization arguments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitArgs {
    pub agent_id: AgentPid,
    pub supervisor_id: SupervisorId,
    pub config: serde_json::Value,
}

/// Result type for supervision operations
pub type SupervisionResult<T> = Result<T, SupervisionError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_pid_creation() {
        let pid1 = AgentPid::new();
        let pid2 = AgentPid::new();

        assert_ne!(pid1, pid2);
        assert!(!pid1.as_str().is_empty());
    }

    #[test]
    fn test_supervisor_id_creation() {
        let id1 = SupervisorId::new();
        let id2 = SupervisorId::new();

        assert_ne!(id1, id2);
    }

    #[test]
    fn test_agent_status_serialization() {
        let status = AgentStatus::Failed("test error".to_string());
        let serialized = serde_json::to_string(&status).unwrap();
        let deserialized: AgentStatus = serde_json::from_str(&serialized).unwrap();

        assert_eq!(status, deserialized);
    }
}
