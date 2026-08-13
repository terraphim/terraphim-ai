//! Microsoft Teams Bot Framework channel adapter.

use crate::bus::{InboundMessage, MessageBus, OutboundMessage};
use crate::channel::Channel;
use crate::config::TeamsConfig;
use async_trait::async_trait;
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

pub const CHANNEL_NAME: &str = "teams";
const MAX_TEXT_CHARS: usize = 28_000;
const MAX_SEND_ATTEMPTS: usize = 3;

/// Microsoft Teams adapter using documented Bot Framework HTTP contracts.
pub struct TeamsChannel {
    config: TeamsConfig,
    client: reqwest::Client,
    running: Arc<AtomicBool>,
    token_cache: Arc<Mutex<Option<CachedToken>>>,
    jwks_cache: Arc<Mutex<Option<CachedJwks>>>,
}

#[derive(Debug, Clone)]
struct CachedToken {
    access_token: String,
    expires_at: Instant,
}

#[derive(Debug, Clone)]
struct CachedJwks {
    jwks: JwksDocument,
    expires_at: Instant,
}

impl TeamsChannel {
    pub fn new(config: TeamsConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
            running: Arc::new(AtomicBool::new(false)),
            token_cache: Arc::new(Mutex::new(None)),
            jwks_cache: Arc::new(Mutex::new(None)),
        }
    }

    /// Parse Bot Framework Activity JSON into a TinyClaw inbound message.
    pub fn parse_activity(&self, body: &[u8]) -> anyhow::Result<Option<InboundMessage>> {
        let activity: TeamsActivity = serde_json::from_slice(body)?;
        if activity.kind != "message" {
            return Ok(None);
        }

        let sender_id = activity
            .from
            .ok_or_else(|| anyhow::anyhow!("Teams message activity missing from"))?
            .id;
        if !self.is_allowed(&sender_id) {
            return Ok(None);
        }

        let text = activity
            .text
            .ok_or_else(|| anyhow::anyhow!("Teams message activity missing text"))?;
        if text.trim().is_empty() {
            return Ok(None);
        }

        let conversation_id = activity
            .conversation
            .ok_or_else(|| anyhow::anyhow!("Teams message activity missing conversation"))?
            .id;
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

    /// Validate a Bot Framework webhook JWT before activity parse/dispatch.
    pub async fn validate_webhook_authorization(
        &self,
        authorization_header: Option<&str>,
        body: &[u8],
    ) -> anyhow::Result<()> {
        let token = bearer_token(authorization_header)?;
        let header = decode_header(token)?;
        if header.alg != Algorithm::RS256 {
            anyhow::bail!("Teams webhook JWT must use RS256");
        }
        let kid = header
            .kid
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("Teams webhook JWT missing kid"))?;
        let jwks = self.jwks().await?;
        let key = jwks
            .keys
            .iter()
            .find(|key| key.kid.as_deref() == Some(kid))
            .ok_or_else(|| anyhow::anyhow!("Teams webhook JWT signing key not found"))?;

        let decoding_key = DecodingKey::from_rsa_components(&key.n, &key.e)?;
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_audience(std::slice::from_ref(&self.config.app_id));
        validation.set_issuer(std::slice::from_ref(&self.config.jwt_issuer));
        validation.validate_exp = true;
        validation.validate_nbf = true;
        validation.leeway = 300;

        let token_data = decode::<TeamsWebhookClaims>(token, &decoding_key, &validation)?;
        if token_data.claims.aud != self.config.app_id {
            anyhow::bail!("Teams webhook JWT audience mismatch");
        }
        let activity = serde_json::from_slice::<TeamsActivityServiceUrl>(body)?;
        let body_service_url = activity
            .service_url
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("Teams activity missing serviceUrl"))?;
        if token_data.claims.service_url != body_service_url {
            anyhow::bail!("Teams webhook JWT serviceUrl mismatch");
        }

        Ok(())
    }

    async fn jwks(&self) -> anyhow::Result<JwksDocument> {
        let mut cache = self.jwks_cache.lock().await;
        if let Some(cached) = cache.as_ref()
            && Instant::now() < cached.expires_at
        {
            return Ok(cached.jwks.clone());
        }

        let metadata = self.fetch_openid_metadata().await?;
        if metadata.issuer != self.config.jwt_issuer {
            anyhow::bail!("Teams OpenID metadata issuer mismatch");
        }
        if !metadata
            .id_token_signing_alg_values_supported
            .iter()
            .any(|alg| alg == "RS256")
        {
            anyhow::bail!("Teams OpenID metadata does not advertise RS256");
        }
        let jwks_url = self
            .config
            .openid_jwks_url
            .as_deref()
            .unwrap_or(metadata.jwks_uri.as_str());
        let jwks = self.fetch_jwks(jwks_url).await?;
        if jwks.keys.is_empty() {
            anyhow::bail!("Teams JWKS is empty");
        }
        *cache = Some(CachedJwks {
            jwks: jwks.clone(),
            expires_at: Instant::now() + Duration::from_secs(24 * 60 * 60),
        });
        Ok(jwks)
    }

    async fn fetch_openid_metadata(&self) -> anyhow::Result<OpenIdMetadata> {
        let response = self
            .client
            .get(&self.config.openid_metadata_url)
            .send()
            .await?;
        let status = response.status();
        if !status.is_success() {
            anyhow::bail!("Teams OpenID metadata request failed with status {status}");
        }
        Ok(response.json::<OpenIdMetadata>().await?)
    }

    async fn fetch_jwks(&self, jwks_url: &str) -> anyhow::Result<JwksDocument> {
        let response = self.client.get(jwks_url).send().await?;
        let status = response.status();
        if !status.is_success() {
            anyhow::bail!("Teams JWKS request failed with status {status}");
        }
        Ok(response.json::<JwksDocument>().await?)
    }

    async fn acquire_access_token_uncached(&self) -> anyhow::Result<CachedToken> {
        #[derive(Deserialize)]
        struct TokenResponse {
            access_token: String,
            expires_in: Option<u64>,
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
        let lifetime = Duration::from_secs(token.expires_in.unwrap_or(3600));
        let skew = Duration::from_secs(60).min(lifetime / 2);
        Ok(CachedToken {
            access_token: token.access_token,
            expires_at: Instant::now() + lifetime.saturating_sub(skew),
        })
    }

    async fn access_token(&self) -> anyhow::Result<String> {
        let mut cache = self.token_cache.lock().await;
        if let Some(cached) = cache.as_ref()
            && Instant::now() < cached.expires_at
        {
            return Ok(cached.access_token.clone());
        }

        let token = self.acquire_access_token_uncached().await?;
        let access_token = token.access_token.clone();
        *cache = Some(token);
        Ok(access_token)
    }

    async fn send_chunk(
        &self,
        service_url: &str,
        conversation_id: &str,
        chunk: &str,
        token: &str,
    ) -> anyhow::Result<()> {
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
                .bearer_auth(token)
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
        let token = self.access_token().await?;
        for chunk in crate::format::chunk_message_with_hard_limit(&msg.content, MAX_TEXT_CHARS) {
            self.send_chunk(service_url, conversation_id, &chunk, &token)
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

fn bearer_token(authorization_header: Option<&str>) -> anyhow::Result<&str> {
    let header = authorization_header
        .ok_or_else(|| anyhow::anyhow!("Teams webhook missing authorization header"))?;
    let Some(token) = header.strip_prefix("Bearer ") else {
        anyhow::bail!("Teams webhook authorization must use Bearer scheme");
    };
    let token = token.trim();
    if token.is_empty() {
        anyhow::bail!("Teams webhook authorization bearer token is empty");
    }
    Ok(token)
}

#[derive(Debug, Deserialize)]
struct OpenIdMetadata {
    issuer: String,
    jwks_uri: String,
    #[serde(default)]
    id_token_signing_alg_values_supported: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct JwksDocument {
    keys: Vec<JwkKey>,
}

#[derive(Debug, Clone, Deserialize)]
struct JwkKey {
    kid: Option<String>,
    n: String,
    e: String,
}

#[derive(Debug, Clone, Deserialize)]
struct TeamsWebhookClaims {
    aud: String,
    #[serde(rename = "serviceUrl")]
    service_url: String,
}

#[derive(Debug, Deserialize)]
struct TeamsActivityServiceUrl {
    #[serde(rename = "serviceUrl")]
    service_url: Option<String>,
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
    from: Option<TeamsAccount>,
    conversation: Option<TeamsConversation>,
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
    fn ignores_non_message_activities_without_message_fields() {
        let ch = TeamsChannel::new(test_config("http://127.0.0.1/token"));
        for body in [
            br#"{"type": "conversationUpdate"}"#.as_slice(),
            br#"{"type": "invoke", "name": "composeExtension/query"}"#.as_slice(),
        ] {
            assert!(ch.parse_activity(body).unwrap().is_none());
        }
    }

    #[test]
    fn malformed_message_activity_fails_with_required_field_context() {
        let ch = TeamsChannel::new(test_config("http://127.0.0.1/token"));
        let body = br#"{"type": "message"}"#;

        let err = ch.parse_activity(body).unwrap_err().to_string();

        assert!(err.contains("Teams message activity missing"));
        assert!(err.contains("from"));
    }

    #[test]
    fn extracts_bearer_token_without_shape_only_acceptance() {
        assert_eq!(
            bearer_token(Some("Bearer abcdefghijklmnopqrstuvwxyz")).unwrap(),
            "abcdefghijklmnopqrstuvwxyz"
        );
        assert!(bearer_token(None).is_err());
        assert!(bearer_token(Some("Bearer ")).is_err());
        assert!(bearer_token(Some("Basic abcdefghijklmnopqrstuvwxyz")).is_err());
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
    async fn multi_chunk_send_reuses_one_oauth_token() {
        let (tx, mut rx) = mpsc::channel(8);
        let base = spawn_capture_server(tx).await;
        let ch = TeamsChannel::new(test_config(&format!("{base}/token")));
        let content = "x".repeat(MAX_TEXT_CHARS + 17);

        ch.send(OutboundMessage::new(
            "teams",
            format!("{base}|conv-1"),
            content,
        ))
        .await
        .unwrap();

        let mut token_requests = 0;
        let mut send_requests = 0;
        while let Some(request) = rx.recv().await {
            match request.path.as_str() {
                "/token" => token_requests += 1,
                "/v3/conversations/conv-1/activities" => send_requests += 1,
                _ => {}
            }
            if send_requests == 2 {
                break;
            }
        }

        assert_eq!(token_requests, 1);
        assert_eq!(send_requests, 2);
    }

    #[tokio::test]
    async fn expired_oauth_token_is_refreshed() {
        let (tx, mut rx) = mpsc::channel(8);
        let base = spawn_capture_server(tx).await;
        let ch = TeamsChannel::new(test_config(&format!("{base}/token")));

        ch.send(OutboundMessage::new(
            "teams",
            format!("{base}|conv-1"),
            "one",
        ))
        .await
        .unwrap();
        {
            let mut cache = ch.token_cache.lock().await;
            let cached = cache.as_mut().expect("first send cached a token");
            cached.expires_at = Instant::now() - Duration::from_secs(1);
        }
        ch.send(OutboundMessage::new(
            "teams",
            format!("{base}|conv-1"),
            "two",
        ))
        .await
        .unwrap();

        let mut token_requests = 0;
        let mut send_requests = 0;
        while let Some(request) = rx.recv().await {
            match request.path.as_str() {
                "/token" => token_requests += 1,
                "/v3/conversations/conv-1/activities" => send_requests += 1,
                _ => {}
            }
            if send_requests == 2 {
                break;
            }
        }

        assert_eq!(token_requests, 2);
        assert_eq!(send_requests, 2);
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
            openid_metadata_url:
                "https://login.botframework.com/v1/.well-known/openidconfiguration".into(),
            openid_jwks_url: None,
            jwt_issuer: "https://api.botframework.com".into(),
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
            openid_metadata_url:
                "https://login.botframework.com/v1/.well-known/openidconfiguration".into(),
            openid_jwks_url: None,
            jwt_issuer: "https://api.botframework.com".into(),
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
                (
                    StatusCode::OK,
                    r#"{"access_token":"fixture-token","expires_in":3600}"#,
                )
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
