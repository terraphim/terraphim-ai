use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::{Mutex, mpsc};

/// Message received from a chat channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboundMessage {
    pub channel: String,
    pub sender_id: String,
    pub chat_id: String,
    pub content: String,
    pub media: Vec<String>,
    pub metadata: HashMap<String, String>,
    pub timestamp: DateTime<Utc>,
}

impl InboundMessage {
    /// Create a new inbound message with current timestamp.
    pub fn new(
        channel: impl Into<String>,
        sender_id: impl Into<String>,
        chat_id: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            channel: channel.into(),
            sender_id: sender_id.into(),
            chat_id: chat_id.into(),
            content: content.into(),
            media: Vec::new(),
            metadata: HashMap::new(),
            timestamp: Utc::now(),
        }
    }

    /// Generate a session key in the format `channel:chat_id`.
    pub fn session_key(&self) -> String {
        format!("{}:{}", self.channel, self.chat_id)
    }

    /// Check if the message is a slash command.
    pub fn is_slash_command(&self) -> bool {
        self.content.starts_with('/')
    }

    /// Get the slash command name if this is a slash command.
    pub fn slash_command(&self) -> Option<&str> {
        if self.is_slash_command() {
            self.content.split_whitespace().next().map(|s| &s[1..])
        } else {
            None
        }
    }
}

/// Message to send to a chat channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboundMessage {
    pub channel: String,
    pub chat_id: String,
    pub content: String,
    pub reply_to: Option<String>,
}

impl OutboundMessage {
    /// Create a new outbound message.
    pub fn new(
        channel: impl Into<String>,
        chat_id: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            channel: channel.into(),
            chat_id: chat_id.into(),
            content: content.into(),
            reply_to: None,
        }
    }

    /// Set the reply-to message ID.
    pub fn with_reply_to(mut self, message_id: impl Into<String>) -> Self {
        self.reply_to = Some(message_id.into());
        self
    }
}

/// Capacity for message channels.
const CHANNEL_CAPACITY: usize = 1000;

/// Async message bus using tokio mpsc channels.
pub struct MessageBus {
    pub inbound_tx: mpsc::Sender<InboundMessage>,
    pub inbound_rx: Mutex<mpsc::Receiver<InboundMessage>>,
    pub outbound_tx: mpsc::Sender<OutboundMessage>,
    pub outbound_rx: Mutex<mpsc::Receiver<OutboundMessage>>,
}

impl MessageBus {
    /// Create a new message bus with bounded channels.
    pub fn new() -> Self {
        let (inbound_tx, inbound_rx) = mpsc::channel(CHANNEL_CAPACITY);
        let (outbound_tx, outbound_rx) = mpsc::channel(CHANNEL_CAPACITY);

        Self {
            inbound_tx,
            inbound_rx: Mutex::new(inbound_rx),
            outbound_tx,
            outbound_rx: Mutex::new(outbound_rx),
        }
    }

    /// Get a sender handle for inbound messages.
    pub fn inbound_sender(&self) -> mpsc::Sender<InboundMessage> {
        self.inbound_tx.clone()
    }

    /// Get a sender handle for outbound messages.
    pub fn outbound_sender(&self) -> mpsc::Sender<OutboundMessage> {
        self.outbound_tx.clone()
    }
}

impl Default for MessageBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inbound_session_key() {
        let msg = InboundMessage::new("telegram", "user123", "chat456", "Hello");
        assert_eq!(msg.session_key(), "telegram:chat456");
    }

    #[test]
    fn test_slash_command_parsing() {
        let msg = InboundMessage::new("telegram", "user123", "chat456", "/role list");
        assert!(msg.is_slash_command());
        assert_eq!(msg.slash_command(), Some("role"));

        let msg2 = InboundMessage::new("telegram", "user123", "chat456", "Hello");
        assert!(!msg2.is_slash_command());
        assert_eq!(msg2.slash_command(), None);
    }

    #[tokio::test]
    async fn test_message_bus_roundtrip() {
        let bus = MessageBus::new();

        // Send a message through inbound
        let sender = bus.inbound_sender();
        let msg = InboundMessage::new("telegram", "user123", "chat456", "Hello");
        sender.send(msg.clone()).await.unwrap();

        // Receive it
        let received = bus.inbound_rx.lock().await.recv().await.unwrap();
        assert_eq!(received.channel, "telegram");
        assert_eq!(received.sender_id, "user123");
        assert_eq!(received.content, "Hello");
    }

    #[test]
    fn test_outbound_message_builder() {
        let msg = OutboundMessage::new("telegram", "chat456", "Response").with_reply_to("msg123");

        assert_eq!(msg.channel, "telegram");
        assert_eq!(msg.reply_to, Some("msg123".to_string()));
    }
}
