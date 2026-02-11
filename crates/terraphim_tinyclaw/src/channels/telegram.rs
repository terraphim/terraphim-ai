//! Telegram channel adapter (placeholder - will be implemented in Step 9).

use crate::bus::{InboundMessage, MessageBus, OutboundMessage};
use crate::channel::Channel;
use crate::config::TelegramConfig;
use async_trait::async_trait;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Telegram channel adapter.
pub struct TelegramChannel {
    config: TelegramConfig,
    running: Arc<AtomicBool>,
}

impl TelegramChannel {
    /// Create a new Telegram channel.
    pub fn new(config: TelegramConfig) -> Self {
        Self {
            config,
            running: Arc::new(AtomicBool::new(false)),
        }
    }
}

#[async_trait]
impl Channel for TelegramChannel {
    fn name(&self) -> &str {
        "telegram"
    }

    async fn start(&self, _bus: Arc<MessageBus>) -> anyhow::Result<()> {
        log::info!("Telegram channel starting (placeholder)");
        self.running.store(true, Ordering::SeqCst);
        // TODO: Implement in Step 9
        anyhow::bail!("Telegram channel not yet implemented - coming in Step 9")
    }

    async fn stop(&self) -> anyhow::Result<()> {
        log::info!("Telegram channel stopping");
        self.running.store(false, Ordering::SeqCst);
        Ok(())
    }

    async fn send(&self, _msg: OutboundMessage) -> anyhow::Result<()> {
        // TODO: Implement in Step 9
        anyhow::bail!("Telegram send not yet implemented")
    }

    fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    fn is_allowed(&self, sender_id: &str) -> bool {
        self.config.is_allowed(sender_id)
    }
}
