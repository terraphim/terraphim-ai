//! OpenAI-compatible HTTP proxy.
//!
//! Wave 5 (Phase C2) of the Hermes parity arc. Translates OpenAI Chat
//! Completions API requests into TinyClaw agent invocations and returns
//! OpenAI-shaped responses. Allows any OpenAI-compatible client (Cursor,
//! Continue.dev, Aider, etc.) to use TinyClaw as a backend.
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

/// Shared proxy state.
#[derive(Clone)]
pub struct ProxyState {
    /// Map of model name to agent identifier.
    pub models: Arc<Mutex<Vec<ModelInfo>>>,
    pub sessions: Arc<Mutex<SessionManager>>,
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
