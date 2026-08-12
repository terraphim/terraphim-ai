//! GitHub channel adapter (webhook + REST API).
//!
//! Minimal stub that satisfies the `Channel` trait contract. A real
//! implementation would receive webhook events from GitHub and respond
//! to issues/PRs via the REST API.

use crate::bus::{InboundMessage, MessageBus, OutboundMessage};
use crate::channel::{Channel, is_sender_allowed};
use async_trait::async_trait;
use std::sync::Arc;

/// GitHub channel identifier.
pub const CHANNEL_NAME: &str = "github";
const MAX_COMMENT_CHARS: usize = 65_536;

/// Configuration for the GitHub channel.
#[derive(Clone)]
pub struct GithubConfig {
    /// GitHub personal access token or GitHub App token.
    pub token: String,
    /// Webhook secret for HMAC verification.
    pub webhook_secret: String,
    /// Allowed GitHub user logins (must be non-empty).
    pub allow_from: Vec<String>,
}

/// Custom Debug that redacts the GitHub token.
/// Prevents accidental credential leakage via `dbg!()` or `tracing::debug!()`.
impl std::fmt::Debug for GithubConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GithubConfig")
            .field("token", &"***REDACTED***")
            .field("webhook_secret", &"***REDACTED***")
            .field("allow_from", &self.allow_from)
            .finish()
    }
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
    sent: Arc<std::sync::Mutex<Vec<GithubComment>>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GithubComment {
    pub repo: String,
    pub issue_number: u64,
    pub body: String,
}

impl GithubChannel {
    pub fn new(config: GithubConfig) -> Self {
        Self {
            config,
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            sent: Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    /// Verify a webhook signature (HMAC-SHA256).
    /// Hermes contract: `gateway/channels/github.py` requires the
    /// `X-Hub-Signature-256` header to match HMAC-SHA256 of the body
    /// with the configured secret. Returns true if the signature is valid.
    ///
    /// Uses `hmac::Mac::verify_slice` for constant-time comparison
    /// (avoids timing-attackable `String ==`).
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
        // Constant-time comparison via the hmac crate's verify_slice.
        let mut provided_bytes = [0u8; 32];
        if !hex_decode_32(provided, &mut provided_bytes) {
            return false;
        }
        mac.verify_slice(&provided_bytes).is_ok()
    }

    pub fn parse_issue_comment_event(&self, body: &[u8]) -> anyhow::Result<Option<InboundMessage>> {
        let v: serde_json::Value = serde_json::from_slice(body)?;
        let Some(comment) = v.get("comment") else {
            return Ok(None);
        };
        let sender = v["sender"]["login"].as_str().unwrap_or("");
        if !self.is_allowed(sender) {
            return Ok(None);
        }
        let repo = v["repository"]["full_name"].as_str().unwrap_or("");
        let issue = v["issue"]["number"].as_u64().unwrap_or(0);
        let body = comment["body"].as_str().unwrap_or("");
        let mut msg = InboundMessage::new(
            CHANNEL_NAME,
            sender,
            format!("{repo}#{issue}"),
            body.to_string(),
        );
        msg.metadata.insert("repo".into(), repo.into());
        msg.metadata
            .insert("issue_number".into(), issue.to_string());
        Ok(Some(msg))
    }

    pub fn sent_comments(&self) -> Vec<GithubComment> {
        self.sent.lock().unwrap().clone()
    }
}

pub fn format_github_comment(content: &str) -> String {
    content.trim().to_string()
}

pub fn chunk_github_comment(content: &str) -> Vec<String> {
    chunk_on_char_boundary(content, MAX_COMMENT_CHARS)
}

fn parse_github_chat_id(chat_id: &str) -> anyhow::Result<(String, u64)> {
    let (repo, issue) = chat_id
        .rsplit_once('#')
        .ok_or_else(|| anyhow::anyhow!("GitHub chat_id must be '<owner>/<repo>#<number>'"))?;
    Ok((repo.to_string(), issue.parse()?))
}

/// Decode a hex string into a 32-byte buffer (SHA-256 size).
/// Returns false if length is wrong or chars aren't hex.
fn hex_decode_32(s: &str, out: &mut [u8; 32]) -> bool {
    if s.len() != 64 {
        return false;
    }
    let bytes = s.as_bytes();
    for (i, chunk) in bytes.chunks(2).enumerate() {
        let hi = hex_nibble(chunk[0]);
        let lo = hex_nibble(chunk[1]);
        match (hi, lo) {
            (Some(h), Some(l)) => out[i] = (h << 4) | l,
            _ => return false,
        }
    }
    true
}

/// Convert a single hex character to its 0-15 value.
fn hex_nibble(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
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
    async fn send(&self, msg: OutboundMessage) -> anyhow::Result<()> {
        let (repo, issue_number) = parse_github_chat_id(&msg.chat_id)?;
        let mut sent = self.sent.lock().unwrap();
        for body in chunk_github_comment(&format_github_comment(&msg.content)) {
            sent.push(GithubComment {
                repo: repo.clone(),
                issue_number,
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

fn chunk_on_char_boundary(content: &str, max_chars: usize) -> Vec<String> {
    if content.is_empty() {
        return vec![String::new()];
    }
    let chars: Vec<char> = content.chars().collect();
    chars
        .chunks(max_chars)
        .map(|chunk| chunk.iter().collect())
        .collect()
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
    fn webhook_verification_rejects_malformed_hex() {
        let ch = GithubChannel::new(GithubConfig::default());
        assert!(!ch.verify_webhook(b"hello", "sha256=not-hex-chars-zzzz"));
    }

    #[test]
    fn webhook_verification_rejects_wrong_length_hex() {
        let ch = GithubChannel::new(GithubConfig::default());
        assert!(!ch.verify_webhook(b"hello", "sha256=deadbeefdeadbeef"));
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

    #[tokio::test]
    async fn send_formats_and_chunks_comments() {
        let ch = GithubChannel::new(GithubConfig::default());
        let msg = OutboundMessage::new("github", "terraphim/terraphim-ai#3165", " hi ");
        ch.send(msg).await.unwrap();
        let sent = ch.sent_comments();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].repo, "terraphim/terraphim-ai");
        assert_eq!(sent[0].issue_number, 3165);
        assert_eq!(sent[0].body, "hi");

        let long = "x".repeat(MAX_COMMENT_CHARS + 1);
        ch.send(OutboundMessage::new(
            "github",
            "terraphim/terraphim-ai#3165",
            long,
        ))
        .await
        .unwrap();
        let sent = ch.sent_comments();
        assert_eq!(sent.len(), 3);
        assert_eq!(sent[1].body.chars().count(), MAX_COMMENT_CHARS);
        assert_eq!(sent[2].body, "x");
    }

    #[test]
    fn receive_issue_comment_parses_inbound_message() {
        let ch = GithubChannel::new(GithubConfig {
            allow_from: vec!["alice".into()],
            ..Default::default()
        });
        let payload = serde_json::json!({
            "sender": {"login": "alice"},
            "repository": {"full_name": "terraphim/terraphim-ai"},
            "issue": {"number": 3165},
            "comment": {"body": "@tinyclaw help"}
        });
        let msg = ch
            .parse_issue_comment_event(payload.to_string().as_bytes())
            .unwrap()
            .unwrap();
        assert_eq!(msg.channel, "github");
        assert_eq!(msg.sender_id, "alice");
        assert_eq!(msg.chat_id, "terraphim/terraphim-ai#3165");
        assert_eq!(msg.content, "@tinyclaw help");
    }

    #[tokio::test]
    #[ignore]
    async fn live_tier_github_send_hook_is_explicitly_gated() {
        if std::env::var("LIVE_TINYCLAW_GITHUB").ok().as_deref() != Some("1") {
            eprintln!("set LIVE_TINYCLAW_GITHUB=1 with GITHUB_TOKEN and GITHUB_TEST_CHAT_ID");
            return;
        }
        let token = std::env::var("GITHUB_TOKEN").expect("GITHUB_TOKEN");
        let chat = std::env::var("GITHUB_TEST_CHAT_ID").expect("GITHUB_TEST_CHAT_ID");
        let ch = GithubChannel::new(GithubConfig {
            token,
            webhook_secret: "live".into(),
            allow_from: vec!["*".into()],
        });
        ch.send(OutboundMessage::new(
            "github",
            chat,
            "TinyClaw live-tier smoke",
        ))
        .await
        .unwrap();
    }
}
