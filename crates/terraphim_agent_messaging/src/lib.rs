//! # Terraphim Agent Messaging
//!
//! Erlang-style asynchronous message passing system for AI agents.
//!
//! This crate provides message-based communication patterns inspired by Erlang/OTP,
//! including agent mailboxes, message routing, and delivery guarantees.
//!
//! ## Core Concepts
//!
//! - **Agent Mailboxes**: Unbounded message queues with delivery guarantees
//! - **Message Patterns**: Call (synchronous), Cast (asynchronous), Info (system messages)
//! - **Message Routing**: Cross-agent message delivery with timeout handling
//! - **Delivery Guarantees**: At-least-once delivery with acknowledgments

pub mod delivery;
pub mod error;
pub mod mailbox;
pub mod message;
pub mod router;

pub use delivery::*;
pub use error::*;
pub use mailbox::*;
pub use message::*;
pub use router::*;

// Re-export supervisor types for convenience
pub use terraphim_agent_supervisor::{AgentPid, SupervisorId};

/// Result type for messaging operations
pub type MessagingResult<T> = Result<T, MessagingError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_imports() {
        // Test that all modules compile and basic types are available
        let _pid = AgentPid::new();
        let _supervisor_id = SupervisorId::new();
    }
}
