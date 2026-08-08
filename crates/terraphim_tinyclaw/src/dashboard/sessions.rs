//! `GET /api/sessions` — list active messaging sessions.

use axum::Json;
use axum::extract::State;
use serde_json::{Value, json};

use super::DashboardState;

/// `GET /api/sessions`
pub async fn list_sessions(State(state): State<DashboardState>) -> Json<Value> {
    let sessions = state.sessions.lock().await;
    let keys = sessions.list_sessions().unwrap_or_default();
    let mut out = Vec::with_capacity(keys.len());
    for key in keys {
        if let Some(s) = sessions.get(&key) {
            out.push(json!({
                "session_key": key,
                "message_count": s.messages.len(),
                "summary": s.summary,
            }));
        }
    }
    Json(json!({ "count": out.len(), "sessions": out }))
}
