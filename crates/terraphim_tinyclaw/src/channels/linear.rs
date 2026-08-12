//! Linear channel adapter (Linear GraphQL API).
//!
//! Minimal stub that satisfies the `Channel` trait contract. A real
//! implementation needs the Linear GraphQL endpoint + OAuth token.

use crate::bus::{InboundMessage, MessageBus, OutboundMessage};
use crate::channel::{Channel, is_sender_allowed};
use async_trait::async_trait;
use std::sync::Arc;

/// Linear channel identifier.
pub const CHANNEL_NAME: &str = "linear";
const MAX_COMMENT_CHARS: usize = 32_000;

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
    sent: Arc<std::sync::Mutex<Vec<LinearComment>>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinearComment {
    pub issue_id: String,
    pub body: String,
}

impl LinearChannel {
    pub fn new(config: LinearConfig) -> Self {
        Self {
            config,
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            sent: Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    pub fn parse_issue_event(&self, body: &[u8]) -> anyhow::Result<Option<InboundMessage>> {
        let v: serde_json::Value = serde_json::from_slice(body)?;
        let sender = v["actor"]["id"].as_str().unwrap_or("");
        if !self.is_allowed(sender) {
            return Ok(None);
        }
        let issue_id = v["data"]["id"].as_str().unwrap_or("");
        let text = v["data"]["description"]
            .as_str()
            .or_else(|| v["data"]["comment"]["body"].as_str())
            .unwrap_or("");
        let mut msg = InboundMessage::new(CHANNEL_NAME, sender, issue_id, text);
        msg.metadata
            .insert("team_id".into(), self.config.team_id.clone());
        Ok(Some(msg))
    }

    pub fn sent_comments(&self) -> Vec<LinearComment> {
        self.sent.lock().unwrap().clone()
    }
}

pub fn format_linear_comment(content: &str) -> String {
    content.trim().to_string()
}

pub fn chunk_linear_comment(content: &str) -> Vec<String> {
    if content.is_empty() {
        return vec![String::new()];
    }
    let chars: Vec<char> = content.chars().collect();
    chars
        .chunks(MAX_COMMENT_CHARS)
        .map(|chunk| chunk.iter().collect())
        .collect()
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
    async fn send(&self, msg: OutboundMessage) -> anyhow::Result<()> {
        let mut sent = self.sent.lock().unwrap();
        for body in chunk_linear_comment(&format_linear_comment(&msg.content)) {
            sent.push(LinearComment {
                issue_id: msg.chat_id.clone(),
                body,
            });
        }
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

    #[tokio::test]
    async fn send_formats_and_chunks_comments() {
        let ch = LinearChannel::new(LinearConfig::default());
        ch.send(OutboundMessage::new("linear", "LIN-123", " hello "))
            .await
            .unwrap();
        let sent = ch.sent_comments();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].issue_id, "LIN-123");
        assert_eq!(sent[0].body, "hello");

        let long = "x".repeat(MAX_COMMENT_CHARS + 1);
        ch.send(OutboundMessage::new("linear", "LIN-123", long))
            .await
            .unwrap();
        let sent = ch.sent_comments();
        assert_eq!(sent.len(), 3);
        assert_eq!(sent[1].body.chars().count(), MAX_COMMENT_CHARS);
        assert_eq!(sent[2].body, "x");
    }

    #[test]
    fn receive_issue_event_parses_inbound_message() {
        let ch = LinearChannel::new(LinearConfig {
            allow_from: vec!["user-1".into()],
            ..Default::default()
        });
        let payload = serde_json::json!({
            "actor": {"id": "user-1"},
            "data": {"id": "LIN-123", "description": "@tinyclaw triage this"}
        });
        let msg = ch
            .parse_issue_event(payload.to_string().as_bytes())
            .unwrap()
            .unwrap();
        assert_eq!(msg.channel, "linear");
        assert_eq!(msg.sender_id, "user-1");
        assert_eq!(msg.chat_id, "LIN-123");
        assert_eq!(msg.content, "@tinyclaw triage this");
    }

    #[tokio::test]
    #[ignore]
    async fn live_tier_linear_send_hook_is_explicitly_gated() {
        if std::env::var("LIVE_TINYCLAW_LINEAR").ok().as_deref() != Some("1") {
            eprintln!("set LIVE_TINYCLAW_LINEAR=1 with LINEAR_API_KEY and LINEAR_TEST_ISSUE_ID");
            return;
        }
        let ch = LinearChannel::new(LinearConfig {
            api_key: std::env::var("LINEAR_API_KEY").expect("LINEAR_API_KEY"),
            team_id: std::env::var("LINEAR_TEAM_ID").unwrap_or_else(|_| "live".into()),
            allow_from: vec!["*".into()],
        });
        ch.send(OutboundMessage::new(
            "linear",
            std::env::var("LINEAR_TEST_ISSUE_ID").expect("LINEAR_TEST_ISSUE_ID"),
            "TinyClaw live-tier smoke",
        ))
        .await
        .unwrap();
    }
}
