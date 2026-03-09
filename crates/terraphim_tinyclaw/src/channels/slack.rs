//! Slack channel adapter using slack-morphism Socket Mode.

use crate::bus::{MessageBus, OutboundMessage};
use crate::channel::Channel;
use crate::config::SlackConfig;
use async_trait::async_trait;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Mutex;

/// Queued outbound message for retry on reconnect or send failure.
struct QueuedMessage {
    chat_id: String,
    content: String,
}

/// Slack channel adapter using slack-morphism Socket Mode.
pub struct SlackChannel {
    config: SlackConfig,
    running: Arc<AtomicBool>,
    outgoing_queue: Arc<Mutex<Vec<QueuedMessage>>>,
}

impl SlackChannel {
    /// Create a new Slack channel adapter.
    pub fn new(config: SlackConfig) -> Self {
        Self {
            config,
            running: Arc::new(AtomicBool::new(false)),
            outgoing_queue: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

#[async_trait]
impl Channel for SlackChannel {
    fn name(&self) -> &str {
        "slack"
    }

    async fn start(&self, _bus: Arc<MessageBus>) -> anyhow::Result<()> {
        self.running.store(true, Ordering::SeqCst);
        // TODO: Step 4 will add Socket Mode listener
        Ok(())
    }

    async fn stop(&self) -> anyhow::Result<()> {
        log::info!("Slack channel stopping");
        self.running.store(false, Ordering::SeqCst);
        Ok(())
    }

    async fn send(&self, _msg: OutboundMessage) -> anyhow::Result<()> {
        // TODO: Step 4 will add chat.postMessage with retry queue
        Ok(())
    }

    fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    fn is_allowed(&self, sender_id: &str) -> bool {
        self.config.is_allowed(sender_id)
    }
}
