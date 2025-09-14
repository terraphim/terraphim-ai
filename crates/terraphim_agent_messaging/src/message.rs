//! Message types and patterns for agent communication
//!
//! Implements Erlang-style message patterns: call, cast, and info.

use std::any::Any;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;
use uuid::Uuid;

use crate::AgentPid;

/// Unique identifier for messages
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MessageId(pub Uuid);

impl MessageId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn as_str(&self) -> String {
        self.0.to_string()
    }
}

impl Default for MessageId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for MessageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Message priority levels
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum MessagePriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

impl Default for MessagePriority {
    fn default() -> Self {
        MessagePriority::Normal
    }
}

/// Message delivery options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryOptions {
    /// Message priority
    pub priority: MessagePriority,
    /// Timeout for message delivery
    pub timeout: Duration,
    /// Whether to require acknowledgment
    pub require_ack: bool,
    /// Maximum retry attempts
    pub max_retries: u32,
    /// Retry delay
    pub retry_delay: Duration,
}

impl Default for DeliveryOptions {
    fn default() -> Self {
        Self {
            priority: MessagePriority::Normal,
            timeout: Duration::from_secs(30),
            require_ack: false,
            max_retries: 3,
            retry_delay: Duration::from_millis(100),
        }
    }
}

/// Core agent message types following Erlang patterns
#[derive(Debug)]
pub enum AgentMessage {
    /// Synchronous call (gen_server:call) - expects a response
    Call {
        id: MessageId,
        from: AgentPid,
        payload: Box<dyn Any + Send>,
        reply_to: oneshot::Sender<Box<dyn Any + Send>>,
        timeout: Duration,
    },

    /// Asynchronous cast (gen_server:cast) - fire and forget
    Cast {
        id: MessageId,
        from: AgentPid,
        payload: Box<dyn Any + Send>,
    },

    /// System info message (gen_server:info) - system notifications
    Info { id: MessageId, info: SystemInfo },

    /// Response to a call message
    Reply {
        id: MessageId,
        to: AgentPid,
        payload: Box<dyn Any + Send>,
    },

    /// Acknowledgment message
    Ack {
        id: MessageId,
        original_message_id: MessageId,
    },
}

impl AgentMessage {
    /// Get the message ID
    pub fn id(&self) -> &MessageId {
        match self {
            AgentMessage::Call { id, .. } => id,
            AgentMessage::Cast { id, .. } => id,
            AgentMessage::Info { id, .. } => id,
            AgentMessage::Reply { id, .. } => id,
            AgentMessage::Ack { id, .. } => id,
        }
    }

    /// Get the sender (if applicable)
    pub fn from(&self) -> Option<&AgentPid> {
        match self {
            AgentMessage::Call { from, .. } => Some(from),
            AgentMessage::Cast { from, .. } => Some(from),
            AgentMessage::Info { .. } => None,
            AgentMessage::Reply { .. } => None,
            AgentMessage::Ack { .. } => None,
        }
    }

    /// Check if this is a call message that expects a response
    pub fn expects_response(&self) -> bool {
        matches!(self, AgentMessage::Call { .. })
    }

    /// Create a call message
    pub fn call<T>(
        from: AgentPid,
        payload: T,
        timeout: Duration,
    ) -> (Self, oneshot::Receiver<Box<dyn Any + Send>>)
    where
        T: Any + Send + 'static,
    {
        let (reply_tx, reply_rx) = oneshot::channel();
        let message = AgentMessage::Call {
            id: MessageId::new(),
            from,
            payload: Box::new(payload),
            reply_to: reply_tx,
            timeout,
        };
        (message, reply_rx)
    }

    /// Create a cast message
    pub fn cast<T>(from: AgentPid, payload: T) -> Self
    where
        T: Any + Send + 'static,
    {
        AgentMessage::Cast {
            id: MessageId::new(),
            from,
            payload: Box::new(payload),
        }
    }

    /// Create an info message
    pub fn info(info: SystemInfo) -> Self {
        AgentMessage::Info {
            id: MessageId::new(),
            info,
        }
    }

    /// Create a reply message
    pub fn reply<T>(to: AgentPid, payload: T) -> Self
    where
        T: Any + Send + 'static,
    {
        AgentMessage::Reply {
            id: MessageId::new(),
            to,
            payload: Box::new(payload),
        }
    }

    /// Create an acknowledgment message
    pub fn ack(original_message_id: MessageId) -> Self {
        AgentMessage::Ack {
            id: MessageId::new(),
            original_message_id,
        }
    }
}

/// System information messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemInfo {
    /// Agent started
    AgentStarted {
        agent_id: AgentPid,
        timestamp: DateTime<Utc>,
    },

    /// Agent stopped
    AgentStopped {
        agent_id: AgentPid,
        timestamp: DateTime<Utc>,
        reason: String,
    },

    /// Agent health check
    HealthCheck {
        agent_id: AgentPid,
        timestamp: DateTime<Utc>,
    },

    /// System shutdown
    SystemShutdown {
        timestamp: DateTime<Utc>,
        reason: String,
    },

    /// Custom system message
    Custom {
        message_type: String,
        data: serde_json::Value,
        timestamp: DateTime<Utc>,
    },
}

/// Message envelope for serialization and routing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageEnvelope {
    pub id: MessageId,
    pub from: Option<AgentPid>,
    pub to: AgentPid,
    pub message_type: String,
    pub payload: serde_json::Value,
    pub delivery_options: DeliveryOptions,
    pub created_at: DateTime<Utc>,
    pub attempts: u32,
}

impl MessageEnvelope {
    /// Create a new message envelope
    pub fn new(
        to: AgentPid,
        message_type: String,
        payload: serde_json::Value,
        delivery_options: DeliveryOptions,
    ) -> Self {
        Self {
            id: MessageId::new(),
            from: None,
            to,
            message_type,
            payload,
            delivery_options,
            created_at: Utc::now(),
            attempts: 0,
        }
    }

    /// Set the sender
    pub fn with_from(mut self, from: AgentPid) -> Self {
        self.from = Some(from);
        self
    }

    /// Increment attempt counter
    pub fn increment_attempts(&mut self) {
        self.attempts += 1;
    }

    /// Check if max retries exceeded
    pub fn max_retries_exceeded(&self) -> bool {
        self.attempts >= self.delivery_options.max_retries
    }

    /// Check if message has expired
    pub fn is_expired(&self) -> bool {
        let elapsed = Utc::now() - self.created_at;
        elapsed.to_std().unwrap_or(Duration::ZERO) > self.delivery_options.timeout
    }
}

/// Typed message wrapper for type-safe messaging
pub struct TypedMessage<T> {
    pub id: MessageId,
    pub from: Option<AgentPid>,
    pub payload: T,
    pub created_at: DateTime<Utc>,
}

impl<T> TypedMessage<T> {
    pub fn new(payload: T) -> Self {
        Self {
            id: MessageId::new(),
            from: None,
            payload,
            created_at: Utc::now(),
        }
    }

    pub fn with_from(mut self, from: AgentPid) -> Self {
        self.from = Some(from);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_message_id_creation() {
        let id1 = MessageId::new();
        let id2 = MessageId::new();

        assert_ne!(id1, id2);
        assert!(!id1.as_str().is_empty());
    }

    #[test]
    fn test_message_priority_ordering() {
        assert!(MessagePriority::Critical > MessagePriority::High);
        assert!(MessagePriority::High > MessagePriority::Normal);
        assert!(MessagePriority::Normal > MessagePriority::Low);
    }

    #[test]
    fn test_delivery_options_default() {
        let options = DeliveryOptions::default();
        assert_eq!(options.priority, MessagePriority::Normal);
        assert_eq!(options.timeout, Duration::from_secs(30));
        assert!(!options.require_ack);
        assert_eq!(options.max_retries, 3);
    }

    #[test]
    fn test_agent_message_creation() {
        let from = AgentPid::new();
        let payload = "test message";

        // Test cast message
        let cast_msg = AgentMessage::cast(from.clone(), payload);
        assert_eq!(cast_msg.from(), Some(&from));
        assert!(!cast_msg.expects_response());

        // Test call message
        let (call_msg, _reply_rx) =
            AgentMessage::call(from.clone(), payload, Duration::from_secs(5));
        assert_eq!(call_msg.from(), Some(&from));
        assert!(call_msg.expects_response());

        // Test info message
        let info_msg = AgentMessage::info(SystemInfo::HealthCheck {
            agent_id: from.clone(),
            timestamp: Utc::now(),
        });
        assert_eq!(info_msg.from(), None);
        assert!(!info_msg.expects_response());
    }

    #[test]
    fn test_message_envelope() {
        let to = AgentPid::new();
        let from = AgentPid::new();
        let payload = serde_json::json!({"test": "data"});
        let options = DeliveryOptions::default();

        let mut envelope =
            MessageEnvelope::new(to.clone(), "test_message".to_string(), payload, options)
                .with_from(from.clone());

        assert_eq!(envelope.to, to);
        assert_eq!(envelope.from, Some(from));
        assert_eq!(envelope.attempts, 0);
        assert!(!envelope.max_retries_exceeded());
        assert!(!envelope.is_expired());

        // Test attempt increment
        envelope.increment_attempts();
        assert_eq!(envelope.attempts, 1);
    }

    #[test]
    fn test_typed_message() {
        #[derive(Debug, PartialEq, Clone)]
        struct TestPayload {
            data: String,
        }

        let payload = TestPayload {
            data: "test".to_string(),
        };
        let from = AgentPid::new();

        let msg = TypedMessage::new(payload.clone()).with_from(from.clone());

        assert_eq!(msg.from, Some(from));
        assert_eq!(msg.payload.data, "test");
    }
}
