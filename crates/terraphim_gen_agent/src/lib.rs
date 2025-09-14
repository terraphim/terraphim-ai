//! # Terraphim GenAgent Framework
//!
//! OTP GenServer-inspired agent behavior framework for standardized agent patterns.
//!
//! This crate provides the core abstractions for building agents that follow
//! Erlang/OTP GenServer patterns, including standardized message handling,
//! state management, and lifecycle management.
//!
//! ## Core Concepts
//!
//! - **GenAgent Trait**: Core behavior pattern similar to OTP GenServer
//! - **State Management**: Immutable state transitions with persistence
//! - **Message Handling**: Standardized call, cast, and info message patterns
//! - **Lifecycle Management**: Init, handle messages, terminate with supervision integration
//! - **Error Handling**: Comprehensive error categorization and recovery strategies

pub mod behavior;
pub mod error;
pub mod lifecycle;
pub mod runtime;
pub mod state;

pub use behavior::*;
pub use error::*;
pub use lifecycle::*;
pub use runtime::*;
pub use state::*;

// Re-export supervisor and messaging types for convenience
pub use terraphim_agent_messaging::{AgentMailbox, AgentMessage, MessageId};
pub use terraphim_agent_supervisor::{AgentPid, InitArgs, SupervisorId};

/// Result type for GenAgent operations
pub type GenAgentResult<T> = Result<T, GenAgentError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_imports() {
        // Test that all modules compile and basic types are available
        let _pid = AgentPid::new();
        let _supervisor_id = SupervisorId::new();
        let _message_id = MessageId::new();
    }
}
