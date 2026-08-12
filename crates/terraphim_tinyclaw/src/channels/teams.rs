//! Microsoft Teams Bot Framework channel adapter.

use crate::bus::{InboundMessage, MessageBus, OutboundMessage};
use crate::channel::Channel;
use crate::config::TeamsConfig;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

pub const CHANNEL_NAME: &str = "teams";
const MAX_TEXT_CHARS: usize = 28_000;
const MAX_SEND_ATTEMPTS: usize = 3;

/// Microsoft Teams adapter using documented Bot Framework HTTP contracts.
pub struct TeamsChannel {
    config: TeamsConfig,
    client: reqwest::Client,
    running: Arc<AtomicBool>,
}

impl TeamsChannel {
    pub fn new(config: TeamsConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Parse Bot Framework Activity JSON into a TinyClaw inbound message.
    pub fn parse_activity(&self, body: &[u8]) -> anyhow::Result<Option<InboundMessage>> {
        let activity: TeamsActivity = serde_json::from_slice(body)?;
        if activity.kind != "message" {
            return Ok(None);
        }

        let sender_id = activity.from.id;
        if !self.is_allowed(&sender_id) {
            return Ok(None);
        }

        let Some(text) = activity.text else {
            return Ok(None);
        };
        if text.trim().is_empty() {
            return Ok(None);
        }

        let conversation_id = activity.conversation.id;
        let chat_id = match activity.service_url.as_deref() {
            Some(service_url) if !service_url.trim().is_empty() => {
                format!("{service_url}|{conversation_id}")
            }
            _ => conversation_id,
        };
        let mut inbound = InboundMessage::new(CHANNEL_NAME, sender_id, chat_id, text);
        if let Some(id) = activity.id {
            inbound.metadata.insert("activity_id".into(), id);
        }
        if let Some(service_url) = activity.service_url {
            inbound.metadata.insert("service_url".into(), service_url);
        }
        if let Some(channel_id) = activity.channel_id {
            inbound.metadata.insert("channel_id".into(), channel_id);
        }
        if let Some(tenant_id) = activity
            .channel_data
            .as_ref()
            .and_then(|data| data.tenant.as_ref())
            .and_then(|tenant| tenant.id.clone())
        {
            inbound.metadata.insert("tenant_id".into(), tenant_id);
        }
        Ok(Some(inbound))
    }

    /// Minimal production guard for Bot Framework webhook handlers.
    ///
    /// A deployable HTTP route must reject missing/non-bearer headers before
    /// parsing activities, then validate the JWT with Microsoft OpenID
    /// metadata at the route boundary. This helper enforces the non-optional
    /// bearer shape without logging or returning token material.
    pub fn has_bearer_authorization(&self, authorization_header: &str) -> bool {
        authorization_header.starts_with("Bearer ")
            && authorization_header["Bearer ".len()..].trim().len() > 20
    }

    async fn acquire_access_token(&self) -> anyhow::Result<String> {
        #[derive(Deserialize)]
        struct TokenResponse {
            access_token: String,
        }

        let params = [
            ("grant_type", "client_credentials"),
            ("client_id", self.config.app_id.as_str()),
            ("client_secret", self.config.app_password.as_str()),
            ("scope", self.config.scope.as_str()),
        ];

        let response = self
            .client
            .post(&self.config.token_url)
            .form(&params)
            .send()
            .await?;
        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("Teams token request failed with status {status}: {text}");
        }
        let token = response.json::<TokenResponse>().await?;
        if token.access_token.trim().is_empty() {
            anyhow::bail!("Teams token response did not include access_token");
        }
        Ok(token.access_token)
    }

    async fn send_chunk(
        &self,
        service_url: &str,
        conversation_id: &str,
        chunk: &str,
    ) -> anyhow::Result<()> {
        let token = self.acquire_access_token().await?;
        let url = format!(
            "{}/v3/conversations/{}/activities",
            service_url.trim_end_matches('/'),
            conversation_id
        );
        let body = json!({
            "type": "message",
            "text": chunk
        });

        for attempt in 0..MAX_SEND_ATTEMPTS {
            let response = self
                .client
                .post(&url)
                .bearer_auth(&token)
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
                    anyhow::bail!("Teams send failed with status {status}: {text}");
                }
                Err(err) if attempt + 1 < MAX_SEND_ATTEMPTS => {
                    log::warn!("Teams send transport error, retrying: {}", err);
                    tokio::time::sleep(backoff(attempt)).await;
                }
                Err(err) => return Err(err.into()),
            }
        }

        unreachable!("send loop always returns")
    }
}

#[async_trait]
impl Channel for TeamsChannel {
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
        let (service_url, conversation_id) = parse_chat_id(&msg.chat_id)?;
        for chunk in crate::format::chunk_message_with_hard_limit(&msg.content, MAX_TEXT_CHARS) {
            self.send_chunk(service_url, conversation_id, &chunk)
                .await?;
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

fn parse_chat_id(chat_id: &str) -> anyhow::Result<(&str, &str)> {
    let Some((service_url, conversation_id)) = chat_id.rsplit_once('|') else {
        anyhow::bail!("Teams chat_id must be '<service_url>|<conversation_id>'");
    };
    if service_url.trim().is_empty() || conversation_id.trim().is_empty() {
        anyhow::bail!("Teams chat_id must include both service_url and conversation_id");
    }
    Ok((service_url, conversation_id))
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

#[derive(Debug, Deserialize)]
struct TeamsActivity {
    #[serde(rename = "type")]
    kind: String,
    id: Option<String>,
    #[serde(rename = "serviceUrl")]
    service_url: Option<String>,
    #[serde(rename = "channelId")]
    channel_id: Option<String>,
    from: TeamsAccount,
    conversation: TeamsConversation,
    text: Option<String>,
    #[serde(rename = "channelData")]
    channel_data: Option<TeamsChannelData>,
}

#[derive(Debug, Deserialize)]
struct TeamsAccount {
    id: String,
}

#[derive(Debug, Deserialize)]
struct TeamsConversation {
    id: String,
}

#[derive(Debug, Deserialize)]
struct TeamsChannelData {
    tenant: Option<TeamsTenant>,
}

#[derive(Debug, Deserialize)]
struct TeamsTenant {
    id: Option<String>,
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
    fn debug_redacts_teams_secret() {
        let cfg = test_config("http://127.0.0.1/token");
        let out = format!("{cfg:?}");
        assert!(out.contains("app-123"));
        assert!(!out.contains("secret-456"));
    }

    #[test]
    fn parse_chat_id_requires_service_url_and_conversation_id() {
        assert_eq!(
            parse_chat_id("https://smba.trafficmanager.net/emea|conv-1").unwrap(),
            ("https://smba.trafficmanager.net/emea", "conv-1")
        );
        assert!(parse_chat_id("conv-1").is_err());
        assert!(parse_chat_id("|conv-1").is_err());
        assert!(parse_chat_id("https://service|").is_err());
    }

    #[test]
    fn parses_message_activity() {
        let ch = TeamsChannel::new(test_config("http://127.0.0.1/token"));
        let body = br#"{
          "type": "message",
          "id": "activity-1",
          "serviceUrl": "https://smba.trafficmanager.net/emea/",
          "channelId": "msteams",
          "from": {"id": "29:user"},
          "conversation": {"id": "conv-1"},
          "text": "hello teams",
          "channelData": {"tenant": {"id": "tenant-1"}}
        }"#;

        let msg = ch.parse_activity(body).unwrap().unwrap();
        assert_eq!(msg.channel, "teams");
        assert_eq!(msg.sender_id, "29:user");
        assert_eq!(msg.chat_id, "https://smba.trafficmanager.net/emea/|conv-1");
        assert_eq!(msg.content, "hello teams");
        assert_eq!(msg.metadata["activity_id"], "activity-1");
        assert_eq!(
            msg.metadata["service_url"],
            "https://smba.trafficmanager.net/emea/"
        );
        assert_eq!(msg.metadata["tenant_id"], "tenant-1");
    }

    #[test]
    fn ignores_non_message_or_unauthorized_activity() {
        let ch = TeamsChannel::new(test_config("http://127.0.0.1/token"));
        let non_message = br#"{
          "type": "conversationUpdate",
          "from": {"id": "29:user"},
          "conversation": {"id": "conv-1"}
        }"#;
        assert!(ch.parse_activity(non_message).unwrap().is_none());

        let blocked = br#"{
          "type": "message",
          "from": {"id": "29:blocked"},
          "conversation": {"id": "conv-1"},
          "text": "blocked"
        }"#;
        assert!(ch.parse_activity(blocked).unwrap().is_none());
    }

    #[test]
    fn requires_bearer_authorization_shape() {
        let ch = TeamsChannel::new(test_config("http://127.0.0.1/token"));
        assert!(ch.has_bearer_authorization("Bearer abcdefghijklmnopqrstuvwxyz"));
        assert!(!ch.has_bearer_authorization("Bearer short"));
        assert!(!ch.has_bearer_authorization("Basic abcdefghijklmnopqrstuvwxyz"));
    }

    #[tokio::test]
    async fn sends_oauth_backed_activity_payload() {
        let (tx, mut rx) = mpsc::channel(2);
        let base = spawn_capture_server(tx).await;
        let ch = TeamsChannel::new(test_config(&format!("{base}/token")));

        ch.send(OutboundMessage::new(
            "teams",
            format!("{base}|conv-1"),
            "hello teams",
        ))
        .await
        .unwrap();

        let token_request = rx.recv().await.unwrap();
        assert_eq!(token_request.path, "/token");
        assert!(
            token_request
                .body_text
                .contains("grant_type=client_credentials")
        );
        assert!(token_request.body_text.contains("client_id=app-123"));
        assert!(token_request.body_text.contains("client_secret=secret-456"));
        assert!(
            token_request
                .body_text
                .contains("scope=https%3A%2F%2Fapi.botframework.com%2F.default")
        );

        let send_request = rx.recv().await.unwrap();
        assert_eq!(send_request.path, "/v3/conversations/conv-1/activities");
        assert_eq!(send_request.auth, "Bearer fixture-token");
        let body: serde_json::Value = serde_json::from_str(&send_request.body_text).unwrap();
        assert_eq!(body["type"], "message");
        assert_eq!(body["text"], "hello teams");
    }

    #[tokio::test]
    async fn parsed_activity_chat_id_can_be_used_for_direct_reply() {
        let (tx, mut rx) = mpsc::channel(2);
        let base = spawn_capture_server(tx).await;
        let ch = TeamsChannel::new(test_config(&format!("{base}/token")));
        let body = format!(
            r#"{{
              "type": "message",
              "id": "activity-1",
              "serviceUrl": "{base}/",
              "from": {{"id": "29:user"}},
              "conversation": {{"id": "conv-1"}},
              "text": "hello teams"
            }}"#
        );

        let inbound = ch.parse_activity(body.as_bytes()).unwrap().unwrap();
        ch.send(OutboundMessage::new("teams", inbound.chat_id, "reply"))
            .await
            .unwrap();

        let token_request = rx.recv().await.unwrap();
        assert_eq!(token_request.path, "/token");

        let send_request = rx.recv().await.unwrap();
        assert_eq!(send_request.path, "/v3/conversations/conv-1/activities");
        let body: serde_json::Value = serde_json::from_str(&send_request.body_text).unwrap();
        assert_eq!(body["text"], "reply");
    }

    #[tokio::test]
    async fn splits_uninterrupted_unicode_text_within_teams_limit_losslessly() {
        let (tx, mut rx) = mpsc::channel(8);
        let base = spawn_capture_server(tx).await;
        let ch = TeamsChannel::new(test_config(&format!("{base}/token")));
        let content = "漢".repeat(MAX_TEXT_CHARS + 17);

        ch.send(OutboundMessage::new(
            "teams",
            format!("{base}|conv-1"),
            content.clone(),
        ))
        .await
        .unwrap();

        let mut reconstructed = String::new();
        while let Some(request) = rx.recv().await {
            if request.path == "/token" {
                continue;
            }
            let body: serde_json::Value = serde_json::from_str(&request.body_text).unwrap();
            let chunk = body["text"].as_str().unwrap();
            assert!(chunk.chars().count() <= MAX_TEXT_CHARS);
            assert!(chunk.len() <= MAX_TEXT_CHARS);
            reconstructed.push_str(chunk);
            if reconstructed.chars().count() == content.chars().count() {
                break;
            }
        }

        assert_eq!(reconstructed, content);
    }

    #[tokio::test]
    #[ignore]
    async fn live_teams_send_text() {
        if std::env::var("TERRAPHIM_TEST_LIVE").ok().as_deref() != Some("1") {
            eprintln!("set TERRAPHIM_TEST_LIVE=1 to run live Teams test");
            return;
        }
        let cfg = TeamsConfig {
            app_id: std::env::var("TEAMS_APP_ID").unwrap(),
            app_password: std::env::var("TEAMS_APP_PASSWORD").unwrap(),
            token_url: "https://login.microsoftonline.com/botframework.com/oauth2/v2.0/token"
                .into(),
            scope: "https://api.botframework.com/.default".into(),
            allow_from: vec!["*".into()],
        };
        let service_url = std::env::var("TEAMS_TEST_SERVICE_URL").unwrap();
        let conversation_id = std::env::var("TEAMS_TEST_CONVERSATION_ID").unwrap();
        TeamsChannel::new(cfg)
            .send(OutboundMessage::new(
                "teams",
                format!("{service_url}|{conversation_id}"),
                "TinyClaw live Teams channel test",
            ))
            .await
            .unwrap();
    }

    fn test_config(token_url: &str) -> TeamsConfig {
        TeamsConfig {
            app_id: "app-123".into(),
            app_password: "secret-456".into(),
            token_url: token_url.into(),
            scope: "https://api.botframework.com/.default".into(),
            allow_from: vec!["29:user".into()],
        }
    }

    #[derive(Debug)]
    struct CapturedRequest {
        path: String,
        auth: String,
        body_text: String,
    }

    async fn spawn_capture_server(tx: mpsc::Sender<CapturedRequest>) -> String {
        async fn capture(
            State(tx): State<mpsc::Sender<CapturedRequest>>,
            headers: HeaderMap,
            uri: axum::http::Uri,
            body: Bytes,
        ) -> (StatusCode, &'static str) {
            let auth = headers
                .get(axum::http::header::AUTHORIZATION)
                .and_then(|v| v.to_str().ok())
                .unwrap_or_default()
                .to_string();
            tx.send(CapturedRequest {
                path: uri.path().to_string(),
                auth,
                body_text: String::from_utf8(body.to_vec()).unwrap(),
            })
            .await
            .unwrap();
            if uri.path() == "/token" {
                (StatusCode::OK, r#"{"access_token":"fixture-token"}"#)
            } else {
                (StatusCode::CREATED, r#"{"id":"activity-reply"}"#)
            }
        }

        let app = Router::new()
            .route("/token", post(capture))
            .route(
                "/v3/conversations/{conversation_id}/activities",
                post(capture),
            )
            .with_state(tx);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr: SocketAddr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        format!("http://{addr}")
    }
}
