//! Discord channel adapter using serenity.

use crate::bus::{InboundMessage, MessageBus, OutboundMessage};
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

    async fn start(&self, _bus: Arc<MessageBus>) -> anyhow::Result<()> {
        log::info!("Discord channel starting");
        self.running.store(true, Ordering::SeqCst);

        #[cfg(feature = "discord")]
        {
            // Simplified implementation - just log for now
            log::info!("Discord bot would start here");
            Ok(())
        }

        #[cfg(not(feature = "discord"))]
        {
            anyhow::bail!("Discord feature not enabled")
        }
    }

    async fn stop(&self) -> anyhow::Result<()> {
        log::info!("Discord channel stopping");
        self.running.store(false, Ordering::SeqCst);
        Ok(())
    }

    async fn send(&self, msg: OutboundMessage) -> anyhow::Result<()> {
        #[cfg(feature = "discord")]
        {
            use serenity::http::Http;

            let http = Http::new(&self.config.token);
            let channel_id = msg.chat_id.parse::<u64>()?;

            // Discord supports markdown natively, but chunk if too long
            let chunks = crate::format::chunk_message(&msg.content, 2000);

            for chunk in chunks {
                serenity::model::id::ChannelId::from(channel_id)
                    .say(&http, &chunk)
                    .await?;
            }
            Ok(())
        }

        #[cfg(not(feature = "discord"))]
        {
            let _ = msg;
            anyhow::bail!("Discord feature not enabled")
        }
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
