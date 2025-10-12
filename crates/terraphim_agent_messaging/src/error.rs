//! Error types for the messaging system

use crate::AgentPid;
use thiserror::Error;

/// Errors that can occur in the messaging system
#[derive(Error, Debug)]
pub enum MessagingError {
    #[error("Agent {0} not found")]
    AgentNotFound(AgentPid),

    #[error("Message delivery failed to agent {0}: {1}")]
    DeliveryFailed(AgentPid, String),

    #[error("Message timeout waiting for response from agent {0}")]
    MessageTimeout(AgentPid),

    #[error("Mailbox full for agent {0}")]
    MailboxFull(AgentPid),

    #[error("Invalid message format: {0}")]
    InvalidMessage(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Channel closed for agent {0}")]
    ChannelClosed(AgentPid),

    #[error("Router not initialized")]
    RouterNotInitialized,

    #[error("Duplicate agent registration: {0}")]
    DuplicateAgent(AgentPid),

    #[error("System error: {0}")]
    System(String),
}

impl MessagingError {
    /// Check if this error is recoverable through retry
    pub fn is_recoverable(&self) -> bool {
        match self {
            MessagingError::DeliveryFailed(_, _) => true,
            MessagingError::MessageTimeout(_) => true,
            MessagingError::MailboxFull(_) => true,
            MessagingError::ChannelClosed(_) => false,
            MessagingError::AgentNotFound(_) => false,
            MessagingError::InvalidMessage(_) => false,
            MessagingError::Serialization(_) => false,
            MessagingError::RouterNotInitialized => false,
            MessagingError::DuplicateAgent(_) => false,
            MessagingError::System(_) => false,
        }
    }

    /// Get error category for monitoring and alerting
    pub fn category(&self) -> ErrorCategory {
        match self {
            MessagingError::AgentNotFound(_) => ErrorCategory::NotFound,
            MessagingError::DeliveryFailed(_, _) => ErrorCategory::Delivery,
            MessagingError::MessageTimeout(_) => ErrorCategory::Timeout,
            MessagingError::MailboxFull(_) => ErrorCategory::ResourceLimit,
            MessagingError::InvalidMessage(_) => ErrorCategory::Validation,
            MessagingError::Serialization(_) => ErrorCategory::Serialization,
            MessagingError::ChannelClosed(_) => ErrorCategory::Connection,
            MessagingError::RouterNotInitialized => ErrorCategory::Configuration,
            MessagingError::DuplicateAgent(_) => ErrorCategory::Configuration,
            MessagingError::System(_) => ErrorCategory::System,
        }
    }
}

/// Error categories for monitoring and alerting
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorCategory {
    NotFound,
    Delivery,
    Timeout,
    ResourceLimit,
    Validation,
    Serialization,
    Connection,
    Configuration,
    System,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_recoverability() {
        let recoverable_error =
            MessagingError::DeliveryFailed(AgentPid::new(), "network error".to_string());
        assert!(recoverable_error.is_recoverable());

        let non_recoverable_error = MessagingError::InvalidMessage("malformed message".to_string());
        assert!(!non_recoverable_error.is_recoverable());
    }

    #[test]
    fn test_error_categorization() {
        let timeout_error = MessagingError::MessageTimeout(AgentPid::new());
        assert_eq!(timeout_error.category(), ErrorCategory::Timeout);

        let delivery_error =
            MessagingError::DeliveryFailed(AgentPid::new(), "connection failed".to_string());
        assert_eq!(delivery_error.category(), ErrorCategory::Delivery);
    }
}
