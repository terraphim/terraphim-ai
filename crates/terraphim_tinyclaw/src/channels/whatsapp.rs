//! WhatsApp Cloud API channel adapter.

use crate::bus::{InboundMessage, MessageBus, OutboundMessage};
use crate::channel::Channel;
use crate::config::WhatsAppConfig;
use async_trait::async_trait;
use hmac::{Hmac, Mac};
use serde::Deserialize;
use serde_json::json;
use sha2::Sha256;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

type HmacSha256 = Hmac<Sha256>;

pub const CHANNEL_NAME: &str = "whatsapp";
const MAX_TEXT_CHARS: usize = 4096;
const MAX_SEND_ATTEMPTS: usize = 3;

/// WhatsApp Cloud API adapter.
pub struct WhatsAppChannel {
    config: WhatsAppConfig,
    client: reqwest::Client,
    running: Arc<AtomicBool>,
}

impl WhatsAppChannel {
    pub fn new(config: WhatsAppConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    fn messages_url(&self) -> String {
        format!(
            "{}/{}/{}/messages",
            self.config.graph_base_url.trim_end_matches('/'),
            self.config.api_version.trim_matches('/'),
            self.config.phone_number_id
        )
    }

    /// Verify Meta webhook subscription challenge parameters.
    pub fn verify_subscription(&self, mode: &str, verify_token: &str) -> bool {
        mode == "subscribe" && verify_token == self.config.verify_token
    }

    /// Verify `X-Hub-Signature-256` for a raw webhook body.
    pub fn verify_webhook_signature(&self, body: &[u8], signature_header: &str) -> bool {
        verify_hmac_sha256(self.config.app_secret.as_bytes(), body, signature_header)
    }

    /// Parse WhatsApp webhook JSON into TinyClaw inbound messages.
    pub fn parse_webhook(&self, body: &[u8]) -> anyhow::Result<Vec<InboundMessage>> {
        let webhook: WhatsAppWebhook = serde_json::from_slice(body)?;
        let mut messages = Vec::new();

        for entry in webhook.entry {
            for change in entry.changes {
                for msg in change.value.messages.unwrap_or_default() {
                    if !self.is_allowed(&msg.from) {
                        continue;
                    }
                    let Some(text) = msg.text.and_then(|t| t.body) else {
                        continue;
                    };
                    if text.is_empty() {
                        continue;
                    }

                    let mut inbound =
                        InboundMessage::new(CHANNEL_NAME, msg.from.clone(), msg.from, text);
                    inbound.metadata.insert("message_id".into(), msg.id);
                    inbound.metadata.insert("message_type".into(), msg.kind);
                    if let Some(profile_name) = change
                        .value
                        .contacts
                        .as_ref()
                        .and_then(|contacts| contacts.first())
                        .and_then(|contact| contact.profile.as_ref())
                        .and_then(|profile| profile.name.clone())
                    {
                        inbound.metadata.insert("profile_name".into(), profile_name);
                    }
                    messages.push(inbound);
                }
            }
        }

        Ok(messages)
    }

    async fn send_chunk(&self, chat_id: &str, chunk: &str) -> anyhow::Result<()> {
        let body = json!({
            "messaging_product": "whatsapp",
            "recipient_type": "individual",
            "to": chat_id,
            "type": "text",
            "text": {
                "preview_url": false,
                "body": chunk
            }
        });

        for attempt in 0..MAX_SEND_ATTEMPTS {
            let response = self
                .client
                .post(self.messages_url())
                .bearer_auth(&self.config.access_token)
                .json(&body)
                .send()
                .await;

            match response {
                Ok(resp) if resp.status().is_success() => return Ok(()),
                Ok(resp) if is_transient(resp.status()) && attempt + 1 < MAX_SEND_ATTEMPTS => {
                    tokio::time::sleep(backoff(attempt)).await;
                }
                Ok(resp) => {
                    let status = resp.status();
                    let text = resp.text().await.unwrap_or_default();
                    anyhow::bail!("WhatsApp send failed with status {status}: {text}");
                }
                Err(err) if attempt + 1 < MAX_SEND_ATTEMPTS => {
                    log::warn!("WhatsApp send transport error, retrying: {}", err);
                    tokio::time::sleep(backoff(attempt)).await;
                }
                Err(err) => return Err(err.into()),
            }
        }

        unreachable!("send loop always returns")
    }
}

#[async_trait]
impl Channel for WhatsAppChannel {
    fn name(&self) -> &str {
        CHANNEL_NAME
    }

    async fn start(&self, _bus: Arc<MessageBus>) -> anyhow::Result<()> {
        self.running.store(true, Ordering::SeqCst);
        Ok(())
    }

    async fn stop(&self) -> anyhow::Result<()> {
        self.running.store(false, Ordering::SeqCst);
        Ok(())
    }

    async fn send(&self, msg: OutboundMessage) -> anyhow::Result<()> {
        for chunk in crate::format::chunk_message(&msg.content, MAX_TEXT_CHARS) {
            self.send_chunk(&msg.chat_id, &chunk).await?;
        }
        Ok(())
    }

    fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    fn is_allowed(&self, sender_id: &str) -> bool {
        self.config.is_allowed(sender_id)
    }
}

fn is_transient(status: reqwest::StatusCode) -> bool {
    status == reqwest::StatusCode::REQUEST_TIMEOUT
        || status == reqwest::StatusCode::CONFLICT
        || status.as_u16() == 425
        || status == reqwest::StatusCode::TOO_MANY_REQUESTS
        || status.is_server_error()
}

fn backoff(attempt: usize) -> Duration {
    Duration::from_millis(50 * 2_u64.pow(attempt as u32))
}

fn verify_hmac_sha256(secret: &[u8], body: &[u8], signature_header: &str) -> bool {
    let prefix = "sha256=";
    if !signature_header.starts_with(prefix) {
        return false;
    }
    let Some(provided) = hex_to_32_bytes(&signature_header[prefix.len()..]) else {
        return false;
    };
    let mut mac = match HmacSha256::new_from_slice(secret) {
        Ok(mac) => mac,
        Err(_) => return false,
    };
    mac.update(body);
    mac.verify_slice(&provided).is_ok()
}

fn hex_to_32_bytes(hex: &str) -> Option<[u8; 32]> {
    if hex.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, chunk) in hex.as_bytes().chunks(2).enumerate() {
        let hi = hex_nibble(chunk[0])?;
        let lo = hex_nibble(chunk[1])?;
        out[i] = (hi << 4) | lo;
    }
    Some(out)
}

fn hex_nibble(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

#[derive(Debug, Deserialize)]
struct WhatsAppWebhook {
    entry: Vec<WhatsAppEntry>,
}

#[derive(Debug, Deserialize)]
struct WhatsAppEntry {
    changes: Vec<WhatsAppChange>,
}

#[derive(Debug, Deserialize)]
struct WhatsAppChange {
    value: WhatsAppValue,
}

#[derive(Debug, Deserialize)]
struct WhatsAppValue {
    messages: Option<Vec<WhatsAppMessage>>,
    contacts: Option<Vec<WhatsAppContact>>,
}

#[derive(Debug, Deserialize)]
struct WhatsAppContact {
    profile: Option<WhatsAppProfile>,
}

#[derive(Debug, Deserialize)]
struct WhatsAppProfile {
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WhatsAppMessage {
    from: String,
    id: String,
    #[serde(rename = "type")]
    kind: String,
    text: Option<WhatsAppText>,
}

#[derive(Debug, Deserialize)]
struct WhatsAppText {
    body: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::body::Bytes;
    use axum::extract::State;
    use axum::http::{HeaderMap, StatusCode};
    use axum::routing::post;
    use std::net::SocketAddr;
    use tokio::net::TcpListener;
    use tokio::sync::mpsc;

    #[test]
    fn debug_redacts_whatsapp_secrets() {
        let cfg = test_config("http://127.0.0.1");
        let out = format!("{cfg:?}");
        assert!(!out.contains("test-access-token"));
        assert!(!out.contains("test-app-secret"));
        assert!(!out.contains("test-verify-token"));
    }

    #[test]
    fn verifies_subscription_and_hmac_signature() {
        let ch = WhatsAppChannel::new(test_config("http://127.0.0.1"));
        assert!(ch.verify_subscription("subscribe", "test-verify-token"));
        assert!(!ch.verify_subscription("subscribe", "wrong"));

        let body = br#"{"hello":"world"}"#;
        let sig = make_sig("test-app-secret", body);
        assert!(ch.verify_webhook_signature(body, &sig));
        assert!(!ch.verify_webhook_signature(body, "sha256=deadbeef"));
    }

    #[test]
    fn parses_text_webhook_and_ignores_unauthorized_sender() {
        let ch = WhatsAppChannel::new(test_config("http://127.0.0.1"));
        let body = br#"{
          "entry": [{
            "changes": [{
              "value": {
                "contacts": [{"profile": {"name": "Alice"}}],
                "messages": [
                  {"from":"15551234567","id":"wamid.1","type":"text","text":{"body":"hello"}},
                  {"from":"15550000000","id":"wamid.2","type":"text","text":{"body":"blocked"}},
                  {"from":"15551234567","id":"wamid.3","type":"image"}
                ]
              }
            }]
          }]
        }"#;

        let messages = ch.parse_webhook(body).unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].channel, "whatsapp");
        assert_eq!(messages[0].sender_id, "15551234567");
        assert_eq!(messages[0].chat_id, "15551234567");
        assert_eq!(messages[0].content, "hello");
        assert_eq!(messages[0].metadata["message_id"], "wamid.1");
        assert_eq!(messages[0].metadata["profile_name"], "Alice");
    }

    #[tokio::test]
    async fn sends_exact_cloud_api_payload() {
        let (tx, mut rx) = mpsc::channel(1);
        let base = spawn_capture_server(tx).await;
        let ch = WhatsAppChannel::new(test_config(&base));

        ch.send(OutboundMessage::new("whatsapp", "15551234567", "hello"))
            .await
            .unwrap();

        let (path, auth, body) = rx.recv().await.unwrap();
        assert_eq!(path, "/v20.0/pn-123/messages");
        assert_eq!(auth, "Bearer test-access-token");
        assert_eq!(body["messaging_product"], "whatsapp");
        assert_eq!(body["recipient_type"], "individual");
        assert_eq!(body["to"], "15551234567");
        assert_eq!(body["type"], "text");
        assert_eq!(body["text"]["preview_url"], false);
        assert_eq!(body["text"]["body"], "hello");
    }

    #[tokio::test]
    #[ignore]
    async fn live_whatsapp_send_text() {
        if std::env::var("TERRAPHIM_TEST_LIVE").ok().as_deref() != Some("1") {
            eprintln!("set TERRAPHIM_TEST_LIVE=1 to run live WhatsApp test");
            return;
        }
        let cfg = WhatsAppConfig {
            access_token: std::env::var("WHATSAPP_ACCESS_TOKEN").unwrap(),
            phone_number_id: std::env::var("WHATSAPP_PHONE_NUMBER_ID").unwrap(),
            verify_token: "unused-live-test".into(),
            app_secret: "unused-live-test".into(),
            graph_base_url: "https://graph.facebook.com".into(),
            api_version: "v20.0".into(),
            allow_from: vec!["*".into()],
        };
        let recipient = std::env::var("WHATSAPP_TEST_RECIPIENT").unwrap();
        WhatsAppChannel::new(cfg)
            .send(OutboundMessage::new(
                "whatsapp",
                recipient,
                "TinyClaw live WhatsApp channel test",
            ))
            .await
            .unwrap();
    }

    fn test_config(base: &str) -> WhatsAppConfig {
        WhatsAppConfig {
            access_token: "test-access-token".into(),
            phone_number_id: "pn-123".into(),
            verify_token: "test-verify-token".into(),
            app_secret: "test-app-secret".into(),
            graph_base_url: base.into(),
            api_version: "v20.0".into(),
            allow_from: vec!["15551234567".into()],
        }
    }

    fn make_sig(secret: &str, body: &[u8]) -> String {
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body);
        let bytes = mac.finalize().into_bytes();
        format!(
            "sha256={}",
            bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
        )
    }

    async fn spawn_capture_server(tx: mpsc::Sender<(String, String, serde_json::Value)>) -> String {
        async fn capture(
            State(tx): State<mpsc::Sender<(String, String, serde_json::Value)>>,
            headers: HeaderMap,
            uri: axum::http::Uri,
            body: Bytes,
        ) -> StatusCode {
            let auth = headers
                .get(axum::http::header::AUTHORIZATION)
                .and_then(|v| v.to_str().ok())
                .unwrap_or_default()
                .to_string();
            let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
            tx.send((uri.path().to_string(), auth, value))
                .await
                .unwrap();
            StatusCode::OK
        }

        let app = Router::new()
            .route("/{version}/{phone_id}/messages", post(capture))
            .with_state(tx);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr: SocketAddr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        format!("http://{addr}")
    }
}
