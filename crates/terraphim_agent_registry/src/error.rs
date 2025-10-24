//! Error types for the agent registry

use crate::{AgentPid, SupervisorId};
use thiserror::Error;

/// Errors that can occur in the agent registry
#[derive(Error, Debug)]
pub enum RegistryError {
    #[error("Agent {0} not found in registry")]
    AgentNotFound(AgentPid),

    #[error("Agent {0} already registered")]
    AgentAlreadyExists(AgentPid),

    #[error("Supervisor {0} not found")]
    SupervisorNotFound(SupervisorId),

    #[error("Invalid agent specification for {0}: {1}")]
    InvalidAgentSpec(AgentPid, String),

    #[error("Capability matching failed: {0}")]
    CapabilityMatchingFailed(String),

    #[error("Knowledge graph operation failed: {0}")]
    KnowledgeGraphError(String),

    #[error("Role graph operation failed: {0}")]
    RoleGraphError(String),

    #[error("Agent discovery failed: {0}")]
    DiscoveryFailed(String),

    #[error("Metadata validation failed for agent {0}: {1}")]
    MetadataValidationFailed(AgentPid, String),

    #[error("Registry persistence failed: {0}")]
    PersistenceError(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("System error: {0}")]
    System(String),
}

impl RegistryError {
    /// Check if this error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            RegistryError::AgentNotFound(_) => true,
            RegistryError::AgentAlreadyExists(_) => false,
            RegistryError::SupervisorNotFound(_) => true,
            RegistryError::InvalidAgentSpec(_, _) => false,
            RegistryError::CapabilityMatchingFailed(_) => true,
            RegistryError::KnowledgeGraphError(_) => true,
            RegistryError::RoleGraphError(_) => true,
            RegistryError::DiscoveryFailed(_) => true,
            RegistryError::MetadataValidationFailed(_, _) => false,
            RegistryError::PersistenceError(_) => true,
            RegistryError::Serialization(_) => false,
            RegistryError::System(_) => false,
        }
    }

    /// Get error category for monitoring
    pub fn category(&self) -> ErrorCategory {
        match self {
            RegistryError::AgentNotFound(_) => ErrorCategory::NotFound,
            RegistryError::AgentAlreadyExists(_) => ErrorCategory::Conflict,
            RegistryError::SupervisorNotFound(_) => ErrorCategory::NotFound,
            RegistryError::InvalidAgentSpec(_, _) => ErrorCategory::Validation,
            RegistryError::CapabilityMatchingFailed(_) => ErrorCategory::Matching,
            RegistryError::KnowledgeGraphError(_) => ErrorCategory::KnowledgeGraph,
            RegistryError::RoleGraphError(_) => ErrorCategory::RoleGraph,
            RegistryError::DiscoveryFailed(_) => ErrorCategory::Discovery,
            RegistryError::MetadataValidationFailed(_, _) => ErrorCategory::Validation,
            RegistryError::PersistenceError(_) => ErrorCategory::Persistence,
            RegistryError::Serialization(_) => ErrorCategory::Serialization,
            RegistryError::System(_) => ErrorCategory::System,
        }
    }
}

/// Error categories for monitoring and alerting
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorCategory {
    NotFound,
    Conflict,
    Validation,
    Matching,
    KnowledgeGraph,
    RoleGraph,
    Discovery,
    Persistence,
    Serialization,
    System,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_recoverability() {
        let recoverable_error = RegistryError::AgentNotFound(AgentPid::new());
        assert!(recoverable_error.is_recoverable());

        let non_recoverable_error =
            RegistryError::InvalidAgentSpec(AgentPid::new(), "invalid spec".to_string());
        assert!(!non_recoverable_error.is_recoverable());
    }

    #[test]
    fn test_error_categorization() {
        let not_found_error = RegistryError::AgentNotFound(AgentPid::new());
        assert_eq!(not_found_error.category(), ErrorCategory::NotFound);

        let validation_error =
            RegistryError::InvalidAgentSpec(AgentPid::new(), "validation failed".to_string());
        assert_eq!(validation_error.category(), ErrorCategory::Validation);
    }
}
