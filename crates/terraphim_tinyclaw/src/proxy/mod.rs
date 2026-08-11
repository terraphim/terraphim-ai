//! OpenAI-compatible HTTP proxy.
//!
//! Wave 5 (Phase C2) of the Hermes parity arc. Translates OpenAI Chat
//! Completions API requests into TinyClaw agent invocations and returns
//! OpenAI-shaped responses. Allows any OpenAI-compatible client (Cursor,
//! Continue.dev, Aider, etc.) to use TinyClaw as a backend.
//!
//! When an upstream proxy is configured (`TERRAPHIM_LLM_PROXY_URL`),
//! requests are forwarded to it verbatim — this is how TinyClaw
//! leverages the deployed `terraphim-llm-proxy` on bigbox
//! (http://100.106.66.7:3456, Anthropic- + OpenAI-compatible).
//! Without an upstream, the proxy falls back to the hermetic echo stub
//! so tests and offline use stay self-contained. See ADR-0010.
//!
//! Endpoints:
//! - `POST /v1/chat/completions` — main completion endpoint
//! - `GET  /v1/models` — list available models
//! - `GET  /v1/health` — health check

pub mod chat;
pub mod models;

use axum::Router;
use axum::routing::{get, post};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::session::SessionManager;

/// Env var for the upstream proxy base URL (e.g. `http://100.106.66.7:3456`).
pub const ENV_UPSTREAM_URL: &str = "TERRAPHIM_LLM_PROXY_URL";
/// Env var for the upstream proxy API key (`PROXY_API_KEY` on bigbox).
pub const ENV_UPSTREAM_API_KEY: &str = "TERRAPHIM_LLM_PROXY_API_KEY";

/// Upstream proxy configuration (forwarding mode).
#[derive(Debug, Clone)]
pub struct Upstream {
    /// Base URL of the upstream OpenAI-compatible proxy.
    pub base_url: String,
    /// Optional bearer token sent as `Authorization: Bearer <key>`.
    pub api_key: Option<String>,
}

/// Shared proxy state.
#[derive(Clone)]
pub struct ProxyState {
    /// Map of model name to agent identifier.
    pub models: Arc<Mutex<Vec<ModelInfo>>>,
    pub sessions: Arc<Mutex<SessionManager>>,
    /// When set, requests are forwarded to this upstream proxy instead of
    /// being answered by the local echo stub.
    pub upstream: Option<Upstream>,
    /// Shared HTTP client (connection-pooled; used for upstream calls).
    pub client: reqwest::Client,
}

/// Model metadata exposed via `/v1/models`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ModelInfo {
    pub id: String,
    pub object: &'static str,
    pub created: i64,
    pub owned_by: &'static str,
}

impl Default for ProxyState {
    fn default() -> Self {
        Self {
            models: Arc::new(Mutex::new(vec![ModelInfo {
                id: "tinyclaw-default".into(),
                object: "model",
                created: 0,
                owned_by: "tinyclaw",
            }])),
            sessions: Arc::new(Mutex::new(SessionManager::new(std::path::PathBuf::from(
                "/tmp",
            )))),
            upstream: None,
            client: reqwest::Client::new(),
        }
    }
}

impl ProxyState {
    /// Build state with an upstream proxy (forwarding mode).
    pub fn with_upstream(base_url: impl Into<String>, api_key: Option<String>) -> Self {
        Self {
            upstream: Some(Upstream {
                base_url: base_url.into(),
                api_key,
            }),
            ..Self::default()
        }
    }

    /// Build state from the environment. Returns `None` when no upstream
    /// URL is configured (echo mode).
    pub fn from_env() -> Self {
        match std::env::var(ENV_UPSTREAM_URL) {
            Ok(url) if !url.trim().is_empty() => Self::with_upstream(
                url.trim().trim_end_matches('/'),
                std::env::var(ENV_UPSTREAM_API_KEY)
                    .ok()
                    .filter(|k| !k.is_empty()),
            ),
            _ => Self::default(),
        }
    }
}

/// Build the axum Router.
pub fn router(state: ProxyState) -> Router {
    Router::new()
        .route("/v1/chat/completions", post(chat::chat_completions))
        .route("/v1/models", get(models::list_models))
        .route("/v1/health", get(health))
        .with_state(state)
}

async fn health() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({ "ok": true }))
}

/// Start the proxy server on the given address.
pub async fn serve(state: ProxyState, addr: SocketAddr) -> Result<SocketAddr, std::io::Error> {
    let app = router(state);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let bound = listener.local_addr()?;
    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            tracing::error!("proxy server error: {e}");
        }
    });
    Ok(bound)
}
