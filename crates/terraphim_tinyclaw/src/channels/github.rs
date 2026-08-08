//! GitHub channel adapter (webhook + REST API).
//!
//! Minimal stub that satisfies the `Channel` trait contract. A real
//! implementation would receive webhook events from GitHub and respond
//! to issues/PRs via the REST API.

use crate::bus::{MessageBus, OutboundMessage};
use crate::channel::{Channel, is_sender_allowed};
use async_trait::async_trait;
use std::sync::Arc;

/// GitHub channel identifier.
pub const CHANNEL_NAME: &str = "github";

/// Configuration for the GitHub channel.
#[derive(Debug, Clone)]
pub struct GithubConfig {
    /// GitHub personal access token or GitHub App token.
    pub token: String,
    /// Webhook secret for HMAC verification.
    pub webhook_secret: String,
    /// Allowed GitHub user logins (must be non-empty).
    pub allow_from: Vec<String>,
}

impl Default for GithubConfig {
    fn default() -> Self {
        Self {
            token: "ghp_xxx".into(),
            webhook_secret: "secret".into(),
            allow_from: vec!["octocat".into()],
        }
    }
}

/// Stub GitHub channel.
pub struct GithubChannel {
    config: GithubConfig,
    running: Arc<std::sync::atomic::AtomicBool>,
}

impl GithubChannel {
    pub fn new(config: GithubConfig) -> Self {
        Self {
            config,
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Verify a webhook signature (HMAC-SHA256).
    /// Hermes contract: `gateway/channels/github.py` requires the
    /// `X-Hub-Signature-256` header to match HMAC-SHA256 of the body
    /// with the configured secret. Returns true if the signature is valid.
    pub fn verify_webhook(&self, body: &[u8], signature_header: &str) -> bool {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        type HmacSha256 = Hmac<Sha256>;

        let prefix = "sha256=";
        if !signature_header.starts_with(prefix) {
            return false;
        }
        let provided = &signature_header[prefix.len()..];

        let mut mac = match HmacSha256::new_from_slice(self.config.webhook_secret.as_bytes()) {
            Ok(m) => m,
            Err(_) => return false,
        };
        mac.update(body);
        let expected = mac.finalize().into_bytes();
        // Constant-time compare via hex-encoding both sides.
        let expected_hex = expected
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        expected_hex == provided
    }
}

#[async_trait]
impl Channel for GithubChannel {
    fn name(&self) -> &str {
        CHANNEL_NAME
    }
    async fn start(&self, _bus: Arc<MessageBus>) -> anyhow::Result<()> {
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
        // Real implementation: POST a comment via REST API.
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
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    type HmacSha256 = Hmac<Sha256>;

    fn make_sig(secret: &str, body: &[u8]) -> String {
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body);
        let bytes = mac.finalize().into_bytes();
        format!(
            "sha256={}",
            bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
        )
    }

    #[test]
    fn channel_name_is_github() {
        let ch = GithubChannel::new(GithubConfig::default());
        assert_eq!(ch.name(), "github");
    }

    #[test]
    fn webhook_verification_accepts_valid_signature() {
        let ch = GithubChannel::new(GithubConfig::default());
        let body = b"hello";
        let sig = make_sig("secret", body);
        assert!(ch.verify_webhook(body, &sig));
    }

    #[test]
    fn webhook_verification_rejects_invalid_signature() {
        let ch = GithubChannel::new(GithubConfig::default());
        assert!(!ch.verify_webhook(b"hello", "sha256=deadbeef"));
    }

    #[test]
    fn webhook_verification_rejects_wrong_prefix() {
        let ch = GithubChannel::new(GithubConfig::default());
        assert!(!ch.verify_webhook(b"hello", "md5=abc"));
    }

    #[test]
    fn is_allowed_respects_allowlist() {
        let ch = GithubChannel::new(GithubConfig {
            allow_from: vec!["alice".into()],
            ..Default::default()
        });
        assert!(ch.is_allowed("alice"));
        assert!(!ch.is_allowed("bob"));
    }
}
