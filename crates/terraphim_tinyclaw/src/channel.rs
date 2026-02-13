use crate::bus::{MessageBus, OutboundMessage};
use crate::config::ChannelsConfig;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

/// Trait for chat platform adapters.
#[async_trait]
pub trait Channel: Send + Sync {
    /// Get the channel name (e.g., "telegram", "discord", "cli").
    fn name(&self) -> &str;

    /// Start the channel and begin listening for messages.
    async fn start(&self, bus: Arc<MessageBus>) -> anyhow::Result<()>;

    /// Stop the channel gracefully.
    async fn stop(&self) -> anyhow::Result<()>;

    /// Send a message to the chat.
    async fn send(&self, msg: OutboundMessage) -> anyhow::Result<()>;

    /// Check if the channel is currently running.
    fn is_running(&self) -> bool;

    /// Check if a sender is allowed to use this channel.
    fn is_allowed(&self, sender_id: &str) -> bool;
}

/// Manages multiple channels and dispatches outbound messages.
pub struct ChannelManager {
    channels: HashMap<String, Box<dyn Channel>>,
}

impl ChannelManager {
    /// Create a new channel manager.
    pub fn new() -> Self {
        Self {
            channels: HashMap::new(),
        }
    }

    /// Register a channel.
    pub fn register(&mut self, channel: Box<dyn Channel>) {
        let name = channel.name().to_string();
        self.channels.insert(name, channel);
    }

    /// Get a channel by name.
    pub fn get(&self, name: &str) -> Option<&dyn Channel> {
        self.channels.get(name).map(|c| c.as_ref())
    }

    /// Send a message to the appropriate channel.
    pub async fn send(&self, msg: OutboundMessage) -> anyhow::Result<()> {
        let channel = self
            .channels
            .get(&msg.channel)
            .ok_or_else(|| anyhow::anyhow!("Unknown channel: {}", msg.channel))?;

        channel.send(msg).await
    }

    /// Start all registered channels.
    pub async fn start_all(&self, bus: Arc<MessageBus>) -> anyhow::Result<()> {
        for (name, channel) in &self.channels {
            log::info!("Starting channel: {}", name);
            channel.start(bus.clone()).await?;
        }
        Ok(())
    }

    /// Stop all registered channels.
    pub async fn stop_all(&self) -> anyhow::Result<()> {
        for (name, channel) in &self.channels {
            log::info!("Stopping channel: {}", name);
            channel.stop().await?;
        }
        Ok(())
    }
}

impl Default for ChannelManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Build channels from configuration.
pub fn build_channels_from_config(
    config: &ChannelsConfig,
) -> anyhow::Result<Vec<Box<dyn Channel>>> {
    let mut channels: Vec<Box<dyn Channel>> = Vec::new();

    #[cfg(feature = "telegram")]
    {
        use crate::channels::telegram::TelegramChannel;

        if let Some(ref cfg) = config.telegram {
            channels.push(Box::new(TelegramChannel::new(cfg.clone())));
        }
    }

    #[cfg(feature = "discord")]
    {
        use crate::channels::discord::DiscordChannel;

        if let Some(ref cfg) = config.discord {
            channels.push(Box::new(DiscordChannel::new(cfg.clone())));
        }
    }

    // Note: matrix channel disabled due to sqlite dependency conflict
    // Re-enable when matrix-sdk updates to compatible rusqlite version
    // #[cfg(feature = "matrix")]
    // {
    //     use crate::channels::matrix::MatrixChannel;
    //
    //     if let Some(ref cfg) = config.matrix {
    //         channels.push(Box::new(MatrixChannel::new(cfg.clone())));
    //     }
    // }

    Ok(channels)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockChannel {
        name: String,
        allowed: Vec<String>,
        running: bool,
    }

    #[async_trait]
    impl Channel for MockChannel {
        fn name(&self) -> &str {
            &self.name
        }

        async fn start(&self, _bus: Arc<MessageBus>) -> anyhow::Result<()> {
            Ok(())
        }

        async fn stop(&self) -> anyhow::Result<()> {
            Ok(())
        }

        async fn send(&self, _msg: OutboundMessage) -> anyhow::Result<()> {
            Ok(())
        }

        fn is_running(&self) -> bool {
            self.running
        }

        fn is_allowed(&self, sender_id: &str) -> bool {
            self.allowed.contains(&sender_id.to_string())
        }
    }

    #[test]
    fn test_channel_manager_register_and_get() {
        let mut manager = ChannelManager::new();
        let channel = MockChannel {
            name: "test".to_string(),
            allowed: vec![],
            running: false,
        };

        manager.register(Box::new(channel));
        assert!(manager.get("test").is_some());
        assert!(manager.get("other").is_none());
    }

    #[test]
    fn test_is_allowed_whitelist() {
        let channel = MockChannel {
            name: "test".to_string(),
            allowed: vec!["user1".to_string(), "user2".to_string()],
            running: false,
        };

        assert!(channel.is_allowed("user1"));
        assert!(channel.is_allowed("user2"));
        assert!(!channel.is_allowed("user3"));
        assert!(!channel.is_allowed("attacker"));
    }

    #[tokio::test]
    async fn test_channel_manager_send() {
        let mut manager = ChannelManager::new();
        let channel = MockChannel {
            name: "test".to_string(),
            allowed: vec![],
            running: false,
        };

        manager.register(Box::new(channel));

        let msg = OutboundMessage::new("test", "chat123", "Hello");
        assert!(manager.send(msg).await.is_ok());

        let msg = OutboundMessage::new("unknown", "chat123", "Hello");
        assert!(manager.send(msg).await.is_err());
    }
}
