//! Types for the routing engine.

use serde::{Deserialize, Serialize};
use std::fmt;
use terraphim_types::capability::{Capability, Provider};
use thiserror::Error;

/// Result of a routing decision
#[derive(Debug, Clone)]
pub struct RoutingDecision {
    /// The selected provider
    pub provider: Provider,
    /// Capabilities that matched
    pub matched_capabilities: Vec<Capability>,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
    /// Reason for the routing decision
    pub reason: RoutingReason,
}

/// Reason for routing decision
#[derive(Debug, Clone, PartialEq)]
pub enum RoutingReason {
    /// Matched by keyword
    KeywordMatch { keyword: String },
    /// Matched by capability
    CapabilityMatch { capabilities: Vec<Capability> },
    /// Explicit @mention
    ExplicitMention { mention: String },
    /// Fallback to default
    Fallback,
}

impl fmt::Display for RoutingReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RoutingReason::KeywordMatch { keyword } => {
                write!(f, "keyword match: '{}'", keyword)
            }
            RoutingReason::CapabilityMatch { capabilities } => {
                write!(f, "capability match: {:?}", capabilities)
            }
            RoutingReason::ExplicitMention { mention } => {
                write!(f, "explicit mention: {}", mention)
            }
            RoutingReason::Fallback => {
                write!(f, "fallback to default")
            }
        }
    }
}

/// Context for routing decisions
#[derive(Debug, Clone, Default)]
pub struct RoutingContext {
    /// Source agent (if routing from agent output)
    pub source_agent: Option<String>,
    /// Conversation ID
    pub conversation_id: Option<String>,
    /// User ID
    pub user_id: Option<String>,
    /// Preferred strategy override
    pub strategy_override: Option<String>,
}

/// Result of executing a routing decision
#[derive(Debug)]
pub enum RoutingResult {
    /// LLM response
    LlmResponse(String),
    /// Agent spawned
    AgentSpawned(ProcessId),
    /// Error occurred
    Error(String),
}

/// Process ID for spawned agents
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProcessId(pub String);

impl ProcessId {
    /// Generate a new random process ID
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

impl Default for ProcessId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ProcessId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Routing errors
#[derive(Error, Debug, Clone)]
pub enum RoutingError {
    #[error("No provider found for capabilities: {0:?}")]
    NoProviderFound(Vec<Capability>),

    #[error("Provider not found: {0}")]
    ProviderNotFound(String),

    #[error("Registry error: {0}")]
    RegistryError(String),

    #[error("Execution error: {0}")]
    ExecutionError(String),

    #[error("IO error: {0}")]
    Io(String),

    #[error("Serialization error: {0}")]
    Serialization(String),
}

/// Task for routing
#[derive(Debug, Clone)]
pub struct Task {
    pub id: String,
    pub source: String,
    pub target: String,
    pub context: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_id_generation() {
        let id1 = ProcessId::new();
        let id2 = ProcessId::new();

        assert_ne!(id1.0, id2.0);
        assert!(!id1.0.is_empty());
    }

    #[test]
    fn test_routing_reason_display() {
        let reason = RoutingReason::KeywordMatch {
            keyword: "think".to_string(),
        };
        assert_eq!(reason.to_string(), "keyword match: 'think'");

        let reason = RoutingReason::Fallback;
        assert_eq!(reason.to_string(), "fallback to default");
    }
}
