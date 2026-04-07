//! Discord channel adapter stub.
//!
//! Discord support is currently disabled pending serenity 0.13+ (rustls 0.23+).
//! serenity 0.12 transitively pulls rustls-webpki 0.102.8 (RUSTSEC-2026-0049).
//! Re-enable once serenity 0.13+ is released with clean rustls 0.23 support.

use crate::bus::{MessageBus, OutboundMessage};
use crate::channel::Channel;
use crate::config::DiscordConfig;
use async_trait::async_trait;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Discord channel adapter (stubbed; serenity removed to fix RUSTSEC-2026-0049).
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
        anyhow::bail!(
            "Discord support is disabled: serenity 0.12 carries RUSTSEC-2026-0049 \
             (rustls-webpki CRL bypass). Re-enable once serenity 0.13+ with \
             rustls 0.23+ support is released."
        )
    }

    async fn stop(&self) -> anyhow::Result<()> {
        self.running.store(false, Ordering::SeqCst);
        Ok(())
    }

    async fn send(&self, _msg: OutboundMessage) -> anyhow::Result<()> {
        anyhow::bail!(
            "Discord support is disabled: serenity 0.12 carries RUSTSEC-2026-0049 \
             (rustls-webpki CRL bypass). Re-enable once serenity 0.13+ with \
             rustls 0.23+ support is released."
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
