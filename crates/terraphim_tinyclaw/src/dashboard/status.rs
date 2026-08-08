//! `GET /api/status` — gateway/session summary.
//!
//! Hermes contract (web_server.py:3074-3457): returns counts and enums
//! only — no exception messages, no request paths, no tokens.
//! Public path (no auth required).

use axum::Json;
use serde_json::{Value, json};

use super::DashboardState;

/// `GET /api/status`
pub async fn get_status(state: axum::extract::State<DashboardState>) -> Json<Value> {
    let sessions = state.sessions.lock().await;
    let active_sessions = sessions.list_sessions().map(|v| v.len()).unwrap_or(0);

    let cron_jobs = state
        .cron_store
        .load_all()
        .await
        .map(|v| v.len())
        .unwrap_or(0);

    Json(json!({
        "profiles": ["default"],
        "gateway_mode": "dashboard",
        "gateways": ["dashboard"],
        "components": {
            "sessions": { "active": active_sessions },
            "cron": { "total_jobs": cron_jobs },
            "channels": { "configured": ["cli"] },
            "mcp": { "tools_exposed": 10 },
        }
    }))
}
