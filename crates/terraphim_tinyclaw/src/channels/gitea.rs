//! Gitea channel adapter (webhook + REST API).
//!
//! Gitea uses the same `X-Gitea-Signature` HMAC-SHA256 pattern as GitHub.
//! Hermes' `gateway/channels/gitea.py` accepts both GitHub-style and
//! Gitea-style signature headers for compatibility.

use crate::bus::{MessageBus, OutboundMessage};
use crate::channel::{Channel, is_sender_allowed};
use async_trait::async_trait;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::sync::Arc;
type HmacSha256 = Hmac<Sha256>;

/// Gitea channel identifier.
pub const CHANNEL_NAME: &str = "gitea";

/// Configuration for the Gitea channel.
#[derive(Debug, Clone)]
pub struct GiteaConfig {
    /// Gitea API token.
    pub token: String,
    /// Gitea base URL (e.g. https://git.terraphim.cloud).
    pub base_url: String,
    /// Webhook secret for HMAC verification.
    pub webhook_secret: String,
    /// Allowed Gitea user logins (must be non-empty).
    pub allow_from: Vec<String>,
}

impl Default for GiteaConfig {
    fn default() -> Self {
        Self {
            token: "gitea_token_xxx".into(),
            base_url: "https://git.example.com".into(),
            webhook_secret: "secret".into(),
            allow_from: vec!["alex".into()],
        }
    }
}

/// Stub Gitea channel.
pub struct GiteaChannel {
    config: GiteaConfig,
    running: Arc<std::sync::atomic::AtomicBool>,
}

impl GiteaChannel {
    pub fn new(config: GiteaConfig) -> Self {
        Self {
            config,
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Verify a Gitea webhook signature (HMAC-SHA256).
    ///
    /// Gitea signature header format: `sha256=<hex>` (same as GitHub).
    /// Uses `hmac::Mac::verify_slice` for constant-time comparison
    /// (avoids timing-attackable `String ==`).
    pub fn verify_webhook(&self, body: &[u8], signature_header: &str) -> bool {
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
        let mut provided_bytes = [0u8; 32];
        if !hex_decode_32(provided, &mut provided_bytes) {
            return false;
        }
        mac.verify_slice(&provided_bytes).is_ok()
    }
}

/// Decode a hex string into a 32-byte buffer (SHA-256 size).
/// Returns false if length is wrong or chars aren't hex.
fn hex_decode_32(s: &str, out: &mut [u8; 32]) -> bool {
    if s.len() != 64 {
        return false;
    }
    let bytes = s.as_bytes();
    for (i, chunk) in bytes.chunks(2).enumerate() {
        let hi = hex_nibble_gitea(chunk[0]);
        let lo = hex_nibble_gitea(chunk[1]);
        match (hi, lo) {
            (Some(h), Some(l)) => out[i] = (h << 4) | l,
            _ => return false,
        }
    }
    true
}

/// Convert a single hex character to its 0-15 value (gitea helper).
fn hex_nibble_gitea(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

#[async_trait]
impl Channel for GiteaChannel {
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
        // Real implementation: POST a comment via Gitea REST API.
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
    fn channel_name_is_gitea() {
        let ch = GiteaChannel::new(GiteaConfig::default());
        assert_eq!(ch.name(), "gitea");
    }

    #[test]
    fn webhook_verification_accepts_valid_signature() {
        let ch = GiteaChannel::new(GiteaConfig::default());
        let body = b"hello";
        let sig = make_sig("secret", body);
        assert!(ch.verify_webhook(body, &sig));
    }

    #[test]
    fn webhook_verification_rejects_invalid_signature() {
        let ch = GiteaChannel::new(GiteaConfig::default());
        assert!(!ch.verify_webhook(b"hello", "sha256=deadbeef"));
    }

    #[test]
    fn is_allowed_respects_allowlist() {
        let ch = GiteaChannel::new(GiteaConfig {
            allow_from: vec!["alice".into()],
            ..Default::default()
        });
        assert!(ch.is_allowed("alice"));
        assert!(!ch.is_allowed("bob"));
    }

    #[test]
    fn is_allowed_wildcard() {
        let ch = GiteaChannel::new(GiteaConfig {
            allow_from: vec!["*".into()],
            ..Default::default()
        });
        assert!(ch.is_allowed("anyone"));
    }
}
