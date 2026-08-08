//! Email channel adapter (JMAP inbound / SMTP outbound).
//!
//! Inbound side leverages the `jmap_client` crate from the
//! `terraphim-private` workspace (path dep). Outbound is a stub since
//! sending JMAP `Email/set` requires SMTP for cross-provider delivery.
//!
//! The architectural shape matches Hermes' `gateway/channels/email.py`:
//! - Inbound: email-search poll via `jmap_client::JMAPClient::search_emails`
//! - Outbound: SMTP send (stub)
//! - Allowlist: `jmap_client::Email::from` field drives `is_allowed`
//!
//! Type re-use: `jmap_client::{Email, EmailAddress}` are used throughout
//! so the channel's data shape matches the JMAP spec.

use crate::bus::{InboundMessage, MessageBus, OutboundMessage};
use crate::channel::{Channel, is_sender_allowed};
use async_trait::async_trait;
use jmap_client::{Email, JMAPClient};
use std::sync::Arc;

/// Email channel identifier.
pub const CHANNEL_NAME: &str = "email";

/// Configuration for the email channel.
#[derive(Debug, Clone)]
pub struct EmailConfig {
    /// JMAP access token (Bearer credential).
    pub jmap_access_token: String,
    /// SMTP server hostname (for outbound).
    pub smtp_host: String,
    /// From-address to send as.
    pub from_address: String,
    /// Allowed sender email addresses (must be non-empty).
    pub allow_from: Vec<String>,
}

impl Default for EmailConfig {
    fn default() -> Self {
        Self {
            jmap_access_token: String::new(),
            smtp_host: "smtp.example.com".into(),
            from_address: "agent@example.com".into(),
            allow_from: vec!["alice@example.com".into()],
        }
    }
}

/// Email channel — uses `jmap_client::JMAPClient` for inbound searches.
pub struct EmailChannel {
    config: EmailConfig,
    /// Optional pre-connected JMAP client (None means not yet connected).
    client: Arc<tokio::sync::Mutex<Option<JMAPClient>>>,
    running: Arc<std::sync::atomic::AtomicBool>,
}

impl EmailChannel {
    pub fn new(config: EmailConfig) -> Self {
        Self {
            config,
            client: Arc::new(tokio::sync::Mutex::new(None)),
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Connect to the JMAP server. Stores the client handle.
    ///
    /// This is hermetic: it returns Ok(client) only when the JMAP server
    /// accepts the token. For Wave 4 parity tests we don't call this —
    /// we work directly with parsed `Email` fixtures.
    pub async fn connect(&self) -> anyhow::Result<()> {
        let client = JMAPClient::new(self.config.jmap_access_token.clone()).await?;
        *self.client.lock().await = Some(client);
        Ok(())
    }

    /// Search for emails matching a query. Requires the client to be connected.
    pub async fn search_emails(&self, query: &str) -> anyhow::Result<Vec<Email>> {
        let guard = self.client.lock().await;
        match guard.as_ref() {
            Some(client) => Ok(client.search_emails(query).await?),
            None => Ok(Vec::new()),
        }
    }

    /// Convert a JMAP `Email` into an `InboundMessage` for the bus.
    pub fn email_to_inbound(email: &Email, chat_id: &str) -> Option<InboundMessage> {
        let from = email
            .from
            .as_ref()
            .and_then(|v| v.first())
            .map(|a| a.email.clone())
            .unwrap_or_default();
        let content = email
            .body_values
            .values()
            .next()
            .map(|b| b.value.clone())
            .unwrap_or_default();
        if from.is_empty() {
            return None;
        }
        Some(InboundMessage::new(CHANNEL_NAME, from, chat_id, content))
    }
}

#[async_trait]
impl Channel for EmailChannel {
    fn name(&self) -> &str {
        CHANNEL_NAME
    }

    async fn start(&self, _bus: Arc<MessageBus>) -> anyhow::Result<()> {
        // Real implementation: poll JMAP for new messages, push to bus.
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
        // Real implementation: SMTP send.
        Ok(())
    }

    fn is_running(&self) -> bool {
        self.running.load(std::sync::atomic::Ordering::SeqCst)
    }

    fn is_allowed(&self, sender_id: &str) -> bool {
        is_sender_allowed(&self.config.allow_from, sender_id)
    }
}

/// Re-export common JMAP types so channel consumers don't need jmap_client.
pub use jmap_client::{BodyValue, Email as JmapEmail, EmailAddress as JmapEmailAddress};

#[cfg(test)]
mod tests {
    use super::*;
    use jmap_client::{BodyValue, EmailAddress};
    use std::collections::HashMap;

    fn make_email(from_email: &str, body: &str) -> Email {
        Email {
            id: "m1".into(),
            subject: Some("test".into()),
            from: Some(vec![EmailAddress {
                name: Some("Alice".into()),
                email: from_email.into(),
            }]),
            to: None,
            body_values: {
                let mut map = HashMap::new();
                map.insert(
                    "1".to_string(),
                    BodyValue {
                        value: body.into(),
                        is_truncated: Some(false),
                    },
                );
                map
            },
            text_body: Vec::new(),
            received_at: Some("2026-08-08T00:00:00Z".into()),
        }
    }

    #[test]
    fn email_to_inbound_extracts_from_and_body() {
        let email = make_email("alice@example.com", "hello");
        let inbound = EmailChannel::email_to_inbound(&email, "mailbox-1").unwrap();
        assert_eq!(inbound.channel, "email");
        assert_eq!(inbound.sender_id, "alice@example.com");
        assert_eq!(inbound.content, "hello");
    }

    #[test]
    fn email_to_inbound_returns_none_for_missing_from() {
        let email = Email {
            id: "m2".into(),
            subject: None,
            from: None,
            to: None,
            body_values: HashMap::new(),
            text_body: Vec::new(),
            received_at: None,
        };
        assert!(EmailChannel::email_to_inbound(&email, "mailbox-1").is_none());
    }

    #[test]
    fn is_allowed_respects_allowlist() {
        let ch = EmailChannel::new(EmailConfig {
            allow_from: vec!["alice@example.com".into()],
            ..Default::default()
        });
        assert!(ch.is_allowed("alice@example.com"));
        assert!(!ch.is_allowed("bob@example.com"));
    }

    #[test]
    fn is_allowed_wildcard() {
        let ch = EmailChannel::new(EmailConfig {
            allow_from: vec!["*".into()],
            ..Default::default()
        });
        assert!(ch.is_allowed("anyone@example.com"));
    }
}
