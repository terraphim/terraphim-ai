//! Email channel adapter (JMAP inbound / SMTP outbound).
//!
//! Inbound side leverages the `haystack_jmap` crate (terraphim registry;
//! moved out of `terraphim-private` into the `terraphim-service` crates
//! repo, #3198). Outbound is a stub since sending JMAP `Email/set`
//! requires SMTP for cross-provider delivery.
//!
//! The architectural shape matches Hermes' `gateway/channels/email.py`:
//! - Inbound: email-search poll via `haystack_jmap::JMAPClient::search_emails`
//! - Outbound: SMTP send (stub)
//! - Allowlist: `haystack_jmap::Email::from` field drives `is_allowed`
//!
//! Type re-use: `haystack_jmap::{Email, EmailAddress}` are used throughout
//! so the channel's data shape matches the JMAP spec.

use crate::bus::{InboundMessage, MessageBus, OutboundMessage};
use crate::channel::Channel;
use async_trait::async_trait;
use haystack_jmap::{Email, JMAPClient};
use std::sync::Arc;

/// Email channel identifier.
pub const CHANNEL_NAME: &str = "email";

/// Default bound on inbound search results per poll.
const SEARCH_LIMIT: u32 = 20;

/// Configuration for the email channel.
#[derive(Clone)]
pub struct EmailConfig {
    /// JMAP access token (Bearer credential).
    pub jmap_access_token: String,
    /// JMAP session endpoint URL (required to connect; e.g.
    /// `https://api.fastmail.com/jmap/session`).
    pub jmap_session_url: String,
    /// SMTP server hostname (for outbound).
    pub smtp_host: String,
    /// From-address to send as.
    pub from_address: String,
    /// Allowed sender email addresses (must be non-empty).
    pub allow_from: Vec<String>,
}

/// Custom Debug that redacts the JMAP token (mirrors `TelegramConfig::fmt`).
/// Prevents accidental credential leakage via `dbg!()` or `tracing::debug!()`.
impl std::fmt::Debug for EmailConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EmailConfig")
            .field("jmap_access_token", &"***REDACTED***")
            .field("jmap_session_url", &self.jmap_session_url)
            .field("smtp_host", &self.smtp_host)
            .field("from_address", &self.from_address)
            .field("allow_from", &self.allow_from)
            .finish()
    }
}

impl Default for EmailConfig {
    fn default() -> Self {
        Self {
            jmap_access_token: String::new(),
            jmap_session_url: String::new(),
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
        if self.config.jmap_session_url.is_empty() {
            anyhow::bail!(
                "jmap_session_url is required to connect (set it in the email channel config)"
            );
        }
        let client = JMAPClient::new(
            self.config.jmap_access_token.clone(),
            &self.config.jmap_session_url,
        )
        .await?;
        *self.client.lock().await = Some(client);
        Ok(())
    }

    /// Search for emails matching a query. Requires the client to be connected.
    pub async fn search_emails(&self, query: &str) -> anyhow::Result<Vec<Email>> {
        let guard = self.client.lock().await;
        match guard.as_ref() {
            Some(client) => Ok(client.search_emails(query, SEARCH_LIMIT).await?),
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
        // Email local-parts are case-insensitive per RFC 5321 §2.4.
        let id_lower = sender_id.to_lowercase();
        self.config
            .allow_from
            .iter()
            .any(|a| a == "*" || a.to_lowercase() == id_lower)
    }
}

/// Re-export common JMAP types so channel consumers don't need the crate.
pub use haystack_jmap::{BodyValue, Email as JmapEmail, EmailAddress as JmapEmailAddress};

#[cfg(test)]
mod tests {
    use super::*;
    use haystack_jmap::{BodyValue, EmailAddress};
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

    #[tokio::test]
    async fn connect_requires_session_url() {
        let ch = EmailChannel::new(EmailConfig::default());
        let err = ch.connect().await.expect_err("empty session url must fail");
        assert!(
            err.to_string().contains("jmap_session_url"),
            "error should mention jmap_session_url, got: {err}"
        );
    }
}
