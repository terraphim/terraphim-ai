//! `GET /v1/models` — list available models (OpenAI compatibility).

use axum::Json;
use axum::extract::State;
use serde_json::{Value, json};

use super::ProxyState;

/// `GET /v1/models`
pub async fn list_models(State(state): State<ProxyState>) -> Json<Value> {
    let models = state.models.lock().await;
    Json(json!({
        "object": "list",
        "data": models.iter().collect::<Vec<_>>(),
    }))
}
