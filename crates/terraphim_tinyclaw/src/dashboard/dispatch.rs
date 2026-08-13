//! Dashboard dispatch endpoint for driving the shared agent loop.

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;
use serde_json::json;

use super::DashboardState;
use crate::agent::entry::dispatch_to_agent_loop;
use crate::bus::InboundMessage;

#[derive(Debug, Deserialize)]
pub struct DispatchRequest {
    #[serde(default = "default_sender")]
    pub sender_id: String,
    #[serde(default = "default_chat")]
    pub chat_id: String,
    pub content: String,
}

fn default_sender() -> String {
    "dashboard".to_string()
}

fn default_chat() -> String {
    "dashboard".to_string()
}

/// `POST /api/agent/messages`
///
/// Hermes contract parity: when `DashboardState::fire_token` is set
/// (typically from `TINYCLAW_FIRE_TOKEN`), the endpoint enforces the
/// same `Authorization: Bearer <token>` gate as
/// `POST /api/cron/fire`. A missing or wrong bearer yields a 401 with
/// `{"error": "invalid auth token"}` and the request is dropped before
/// the agent loop is touched. When the state has no token configured
/// (dev/test), the endpoint is open and the caller is responsible for
/// network-level isolation.
pub async fn post_message(
    State(state): State<DashboardState>,
    headers: axum::http::HeaderMap,
    Json(body): Json<DispatchRequest>,
) -> axum::response::Response {
    if let Some(response) = super::auth::require_fire_token(&state.fire_token, &headers) {
        return response;
    }

    if body.content.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "content is required" })),
        )
            .into_response();
    }

    match dispatch_to_agent_loop(
        &state.bus,
        InboundMessage::new("dashboard", body.sender_id, body.chat_id, body.content),
    )
    .await
    {
        Ok(()) => (StatusCode::ACCEPTED, Json(json!({ "status": "accepted" }))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}
