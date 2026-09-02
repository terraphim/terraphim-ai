//! `POST /v1/chat/completions` — main OpenAI-compatible endpoint.
//!
//! When an upstream proxy is configured (see [`super::ProxyState::upstream`])
//! the request body is forwarded **verbatim** (raw JSON) so no OpenAI fields
//! are lost — clients sending `tools`, `max_tokens`, `response_format`, etc.
//! get them passed through unchanged. Without an upstream, TinyClaw answers
//! with a hermetic echo stub (stream mode returns 501).

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use chrono::Utc;
use serde_json::{Value, json};

use super::ProxyState;

/// Forward a request body to the upstream proxy and map the response back.
async fn forward_to_upstream(state: &ProxyState, path: &str, body: &Value) -> Response {
    let upstream = match &state.upstream {
        Some(u) => u.clone(),
        None => return StatusCode::BAD_GATEWAY.into_response(),
    };
    let url = format!("{}{}", upstream.base_url, path);
    let mut req = state.client.post(&url).json(body);
    if let Some(key) = &upstream.api_key {
        req = req.bearer_auth(key);
    }
    match req.send().await {
        Ok(resp) => {
            let status = resp.status();
            let content_type = resp
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .cloned();
            let bytes = match resp.bytes().await {
                Ok(b) => b.to_vec(),
                Err(e) => {
                    return (
                        StatusCode::BAD_GATEWAY,
                        Json(json!({
                            "error": { "message": format!("upstream read error: {e}"), "type": "upstream_error" }
                        })),
                    )
                        .into_response();
                }
            };
            let mut builder = axum::http::Response::builder().status(status);
            if let Some(ct) = content_type {
                builder = builder.header("content-type", ct);
            }
            builder
                .body(axum::body::Body::from(bytes))
                .unwrap_or_else(|_| StatusCode::BAD_GATEWAY.into_response())
        }
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(json!({
                "error": { "message": format!("upstream unreachable: {e}"), "type": "upstream_error" }
            })),
        )
            .into_response(),
    }
}

/// `POST /v1/chat/completions`
///
/// With an upstream configured, the raw JSON body is forwarded verbatim and
/// the upstream response (status, body, content-type) is returned unchanged.
/// Without an upstream we echo the last user message back as the assistant
/// response — the hermetic fallback (no model credentials wired).
pub async fn chat_completions(State(state): State<ProxyState>, body: Json<Value>) -> Response {
    if state.upstream.is_some() {
        return forward_to_upstream(&state, "/v1/chat/completions", &body.0).await;
    }

    // Echo path: parse just enough to answer.
    let stream = body["stream"].as_bool().unwrap_or(false);
    if stream {
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

    let model = body["model"].as_str().unwrap_or("tinyclaw-default");
    let last_user = body["messages"]
        .as_array()
        .into_iter()
        .flatten()
        .rev()
        .find(|m| m["role"] == "user")
        .and_then(|m| m["content"].as_str())
        .unwrap_or_default();

    let now = Utc::now().timestamp();
    let id = format!("chatcmpl-{:x}", now);
    let response = json!({
        "id": id,
        "object": "chat.completion",
        "created": now,
        "model": model,
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
