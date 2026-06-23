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

    /// Attach media URLs to the message.
    pub fn with_media(mut self, media: Vec<String>) -> Self {
        self.media = media;
        self
    }

    /// Check if the message has media attachments.
    pub fn has_media(&self) -> bool {
        !self.media.is_empty()
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

    #[tokio::test]
    async fn test_message_bus_multiple_senders() {
        let bus = MessageBus::new();
        let sender1 = bus.inbound_sender();
        let sender2 = bus.inbound_sender();

        sender1
            .send(InboundMessage::new("telegram", "user1", "chat1", "First"))
            .await
            .unwrap();
        sender2
            .send(InboundMessage::new("discord", "user2", "chat2", "Second"))
            .await
            .unwrap();

        let mut rx = bus.inbound_rx.lock().await;
        let first = rx.recv().await.unwrap();
        let second = rx.recv().await.unwrap();

        // Both messages must be delivered regardless of which sender sent them.
        let contents: Vec<String> = vec![first.content, second.content];
        assert!(contents.contains(&"First".to_string()));
        assert!(contents.contains(&"Second".to_string()));
    }

    #[tokio::test]
    async fn test_message_bus_empty_receiver_times_out() {
        let bus = MessageBus::new();
        let mut rx = bus.inbound_rx.lock().await;

        let result = tokio::time::timeout(std::time::Duration::from_millis(50), rx.recv()).await;

        assert!(result.is_err(), "empty bus should not produce messages");
    }

    #[test]
    fn test_inbound_with_media() {
        let msg = InboundMessage::new("telegram", "user123", "chat456", "[Voice message]")
            .with_media(vec!["https://example.com/voice.ogg".to_string()]);
        assert!(msg.has_media());
        assert_eq!(msg.media.len(), 1);
        assert_eq!(msg.media[0], "https://example.com/voice.ogg");
    }

    #[test]
    fn test_inbound_has_media() {
        let msg = InboundMessage::new("telegram", "user123", "chat456", "Hello");
        assert!(!msg.has_media());
        assert!(msg.media.is_empty());
    }

    #[test]
    fn test_outbound_message_builder() {
        let msg = OutboundMessage::new("telegram", "chat456", "Response").with_reply_to("msg123");

        assert_eq!(msg.channel, "telegram");
        assert_eq!(msg.reply_to, Some("msg123".to_string()));
    }

    /// AC: a message is routed to the correct channel. The bus exposes two
    /// independent channels (inbound/outbound); the outbound path — where the
    /// `OutboundMessage.channel` field is the routing key a dispatcher reads to
    /// pick the platform adapter — had zero coverage. This test verifies an
    /// outbound message survives the bus round-trip with its routing key intact.
    #[tokio::test]
    async fn test_outbound_message_routed_with_channel_key_intact() {
        let bus = MessageBus::new();
        let sender = bus.outbound_sender();

        let msg = OutboundMessage::new("discord", "chat789", "hello from the agent");
        sender.send(msg.clone()).await.unwrap();

        let received = bus.outbound_rx.lock().await.recv().await.unwrap();
        assert_eq!(received.channel, "discord", "routing key must be preserved");
        assert_eq!(received.chat_id, "chat789");
        assert_eq!(received.content, "hello from the agent");
        assert!(received.reply_to.is_none());
    }

    /// AC: multiple outbound messages to different channels preserve their
    /// distinct routing keys in FIFO order (no cross-channel bleed).
    #[tokio::test]
    async fn test_outbound_messages_keep_distinct_channels_in_order() {
        let bus = MessageBus::new();
        let sender = bus.outbound_sender();

        sender
            .send(OutboundMessage::new("telegram", "t1", "first"))
            .await
            .unwrap();
        sender
            .send(OutboundMessage::new("slack", "s1", "second"))
            .await
            .unwrap();
        sender
            .send(OutboundMessage::new("discord", "d1", "third"))
            .await
            .unwrap();

        let mut rx = bus.outbound_rx.lock().await;
        let first = rx.recv().await.unwrap();
        let second = rx.recv().await.unwrap();
        let third = rx.recv().await.unwrap();

        assert_eq!(first.channel, "telegram");
        assert_eq!(first.content, "first");
        assert_eq!(second.channel, "slack");
        assert_eq!(second.content, "second");
        assert_eq!(third.channel, "discord");
        assert_eq!(third.content, "third");
    }

    /// AC: inbound and outbound channels are independent — an inbound message
    /// never appears on the outbound receiver (and vice versa).
    #[tokio::test]
    async fn test_inbound_and_outbound_channels_are_independent() {
        let bus = MessageBus::new();

        bus.inbound_sender()
            .send(InboundMessage::new("telegram", "u", "c", "in"))
            .await
            .unwrap();
        bus.outbound_sender()
            .send(OutboundMessage::new("telegram", "c", "out"))
            .await
            .unwrap();

        // Outbound receiver must yield the outbound message, not the inbound one.
        let out = bus.outbound_rx.lock().await.recv().await.unwrap();
        assert_eq!(out.content, "out");
        // Inbound receiver must yield the inbound message, not the outbound one.
        let inc = bus.inbound_rx.lock().await.recv().await.unwrap();
        assert_eq!(inc.content, "in");

        // Both receivers must now be drained.
        let out_drained = tokio::time::timeout(
            std::time::Duration::from_millis(50),
            bus.outbound_rx.lock().await.recv(),
        )
        .await;
        assert!(out_drained.is_err(), "outbound channel must be drained");
    }
}
