//! Discord channel adapter (placeholder - will be implemented in Step 10).

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
        log::info!("Discord channel starting (placeholder)");
        self.running.store(true, Ordering::SeqCst);
        // TODO: Implement in Step 10
        anyhow::bail!("Discord channel not yet implemented - coming in Step 10")
    }

    async fn stop(&self) -> anyhow::Result<()> {
        log::info!("Discord channel stopping");
        self.running.store(false, Ordering::SeqCst);
        Ok(())
    }

    async fn send(&self, _msg: OutboundMessage) -> anyhow::Result<()> {
        // TODO: Implement in Step 10
        anyhow::bail!("Discord send not yet implemented")
    }

    fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    fn is_allowed(&self, sender_id: &str) -> bool {
        self.config.is_allowed(sender_id)
    }
}
