//! `POST /v1/chat/completions` — main OpenAI-compatible endpoint.
//!
//! Translates the OpenAI Chat Completions request shape into a TinyClaw
//! agent call and returns an OpenAI-shaped response. Stream mode is not
//! yet implemented.

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use chrono::Utc;
use serde::Deserialize;
use serde_json::json;

use super::ProxyState;

/// OpenAI Chat Completions request body (partial — only fields we use).
#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(default)]
    pub stream: bool,
    #[serde(default)]
    pub temperature: Option<f32>,
}

/// Single message in the conversation.
#[derive(Debug, Clone, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// `POST /v1/chat/completions`
///
/// For Wave 5 we don't actually invoke an LLM (no model credentials wired).
/// Instead we echo the last user message back as the assistant response.
/// This is the minimum to satisfy OpenAI-compatible clients for testing.
pub async fn chat_completions(
    State(_state): State<ProxyState>,
    Json(body): Json<ChatRequest>,
) -> impl IntoResponse {
    if body.stream {
        return (
            StatusCode::NOT_IMPLEMENTED,
            Json(json!({
                "error": {
                    "message": "streaming not yet implemented",
                    "type": "invalid_request_error",
                    "code": "stream_unsupported"
                }
            })),
        )
            .into_response();
    }

    let last_user = body
        .messages
        .iter()
        .rev()
        .find(|m| m.role == "user")
        .map(|m| m.content.clone())
        .unwrap_or_default();

    let now = Utc::now().timestamp();
    let id = format!("chatcmpl-{:x}", now);
    let response = json!({
        "id": id,
        "object": "chat.completion",
        "created": now,
        "model": body.model,
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": format!("[tinyclaw echo] {last_user}"),
            },
            "finish_reason": "stop",
        }],
        "usage": {
            "prompt_tokens": 0,
            "completion_tokens": 0,
            "total_tokens": 0
        }
    });

    (StatusCode::OK, Json(response)).into_response()
}
