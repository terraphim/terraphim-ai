//! Minimal TUI surface for TinyClaw.
//!
//! The TUI does not implement its own response logic. It submits messages to
//! the shared [`crate::bus::MessageBus`], which is consumed by
//! [`crate::agent::agent_loop::ToolCallingLoop`].

use crate::agent::entry::dispatch_to_agent_loop;
use crate::bus::{InboundMessage, MessageBus};
use std::sync::Arc;

/// Local terminal UI surface.
#[derive(Debug, Clone)]
pub struct TuiSurface {
    sender_id: String,
    chat_id: String,
}

impl TuiSurface {
    /// Create a new TUI surface using the default local identity.
    pub fn new() -> Self {
        Self {
            sender_id: "local".to_string(),
            chat_id: "tui".to_string(),
        }
    }

    /// Submit one user message to the shared agent loop bus.
    pub async fn submit(
        &self,
        bus: Arc<MessageBus>,
        content: impl Into<String>,
    ) -> anyhow::Result<()> {
        dispatch_to_agent_loop(
            &bus,
            InboundMessage::new(
                "tui",
                self.sender_id.clone(),
                self.chat_id.clone(),
                content.into(),
            ),
        )
        .await
    }
}

impl Default for TuiSurface {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn submit_uses_shared_agent_loop_bus() {
        let bus = Arc::new(MessageBus::new());
        let tui = TuiSurface::new();

        tui.submit(bus.clone(), "hello from tui").await.unwrap();

        let received = bus.inbound_rx.lock().await.recv().await.unwrap();
        assert_eq!(received.channel, "tui");
        assert_eq!(received.session_key(), "tui:tui");
        assert_eq!(received.content, "hello from tui");
    }
}
