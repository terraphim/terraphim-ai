//! Dashboard dispatch endpoint for driving the shared agent loop.

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
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

pub async fn post_message(
    State(state): State<DashboardState>,
    Json(body): Json<DispatchRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    if body.content.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "content is required" })),
        );
    }

    match dispatch_to_agent_loop(
        &state.bus,
        InboundMessage::new("dashboard", body.sender_id, body.chat_id, body.content),
    )
    .await
    {
        Ok(()) => (StatusCode::ACCEPTED, Json(json!({ "status": "accepted" }))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        ),
    }
}
