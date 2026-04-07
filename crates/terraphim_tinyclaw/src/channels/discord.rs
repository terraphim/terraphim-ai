//! Discord channel adapter using serenity.

use crate::bus::{MessageBus, OutboundMessage};
use crate::channel::Channel;
use crate::config::DiscordConfig;
use async_trait::async_trait;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Discord channel adapter.
pub struct DiscordChannel {
    config: DiscordConfig,
    running: Arc<AtomicBool>,
}

impl DiscordChannel {
    /// Create a new Discord channel.
    pub fn new(config: DiscordConfig) -> Self {
        Self {
            config,
            running: Arc::new(AtomicBool::new(false)),
        }
    }
}

#[async_trait]
impl Channel for DiscordChannel {
    fn name(&self) -> &str {
        "discord"
    }

    async fn start(&self, bus: Arc<MessageBus>) -> anyhow::Result<()> {
        log::info!("Discord channel starting");
        self.running.store(true, Ordering::SeqCst);
        // Discord support temporarily disabled: serenity 0.12 pulls rustls-webpki 0.102.8
        // (RUSTSEC-2026-0049). Re-enable once serenity 0.13+ with rustls 0.23 is available.
        // See: https://git.terraphim.cloud/terraphim/terraphim-ai/issues/363
        let _ = bus;
        anyhow::bail!(
            "Discord channel is not available: pending serenity 0.13+ upgrade (RUSTSEC-2026-0049)"
        )
    }

    async fn stop(&self) -> anyhow::Result<()> {
        log::info!("Discord channel stopping");
        self.running.store(false, Ordering::SeqCst);
        Ok(())
    }

    async fn send(&self, msg: OutboundMessage) -> anyhow::Result<()> {
        // Discord support temporarily disabled: see start() for reason.
        let _ = msg;
        anyhow::bail!(
            "Discord channel is not available: pending serenity 0.13+ upgrade (RUSTSEC-2026-0049)"
        )
    }

    fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    fn is_allowed(&self, sender_id: &str) -> bool {
        self.config.is_allowed(sender_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discord_channel_name() {
        let config = DiscordConfig {
            token: "test".to_string(),
            allow_from: vec!["user1".to_string()],
        };
        let channel = DiscordChannel::new(config);
        assert_eq!(channel.name(), "discord");
    }

    #[test]
    fn test_discord_is_allowed() {
        let config = DiscordConfig {
            token: "test".to_string(),
            allow_from: vec!["user1".to_string(), "user2".to_string()],
        };
        let channel = DiscordChannel::new(config);
        assert!(channel.is_allowed("user1"));
        assert!(!channel.is_allowed("user3"));
    }
}
