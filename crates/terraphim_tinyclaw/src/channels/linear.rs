//! Linear channel adapter (Linear GraphQL API).
//!
//! Minimal stub that satisfies the `Channel` trait contract. A real
//! implementation needs the Linear GraphQL endpoint + OAuth token.

use crate::bus::{MessageBus, OutboundMessage};
use crate::channel::{Channel, is_sender_allowed};
use async_trait::async_trait;
use std::sync::Arc;

/// Linear channel identifier.
pub const CHANNEL_NAME: &str = "linear";

/// Configuration for the Linear channel.
#[derive(Debug, Clone)]
pub struct LinearConfig {
    /// Linear API key.
    pub api_key: String,
    /// Linear team ID to monitor.
    pub team_id: String,
    /// Allowed Linear user IDs (must be non-empty).
    pub allow_from: Vec<String>,
}

impl Default for LinearConfig {
    fn default() -> Self {
        Self {
            api_key: "lin_api_xxx".into(),
            team_id: "team-uuid".into(),
            allow_from: vec!["user-uuid-1".into()],
        }
    }
}

/// Stub Linear channel.
pub struct LinearChannel {
    config: LinearConfig,
    running: Arc<std::sync::atomic::AtomicBool>,
}

impl LinearChannel {
    pub fn new(config: LinearConfig) -> Self {
        Self {
            config,
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }
}

#[async_trait]
impl Channel for LinearChannel {
    fn name(&self) -> &str {
        CHANNEL_NAME
    }
    async fn start(&self, _bus: Arc<MessageBus>) -> anyhow::Result<()> {
        // Real implementation: GraphQL subscription on Issue updates.
        self.running
            .store(true, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }
    async fn stop(&self) -> anyhow::Result<()> {
        self.running
            .store(false, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }
    async fn send(&self, _msg: OutboundMessage) -> anyhow::Result<()> {
        // Real implementation: GraphQL mutation to create a comment.
        Ok(())
    }
    fn is_running(&self) -> bool {
        self.running.load(std::sync::atomic::Ordering::SeqCst)
    }
    fn is_allowed(&self, sender_id: &str) -> bool {
        is_sender_allowed(&self.config.allow_from, sender_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_name_is_linear() {
        let ch = LinearChannel::new(LinearConfig::default());
        assert_eq!(ch.name(), "linear");
    }

    #[test]
    fn is_allowed_respects_allowlist() {
        let ch = LinearChannel::new(LinearConfig {
            allow_from: vec!["user-1".into()],
            ..Default::default()
        });
        assert!(ch.is_allowed("user-1"));
        assert!(!ch.is_allowed("user-2"));
    }

    #[test]
    fn is_allowed_wildcard() {
        let ch = LinearChannel::new(LinearConfig {
            allow_from: vec!["*".into()],
            ..Default::default()
        });
        assert!(ch.is_allowed("anyone"));
    }
}
