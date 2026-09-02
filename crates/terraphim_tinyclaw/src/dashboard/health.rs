//! `GET /api/health` — process liveness.
//!
//! Hermes contract (web_server.py:3064-3072): returns
//! `{"ok": true, "version": ..., "auth_required": bool}`.

use axum::Json;
use serde_json::{Value, json};

use super::DashboardState;

/// Version string baked at compile time.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// `GET /api/health`
pub async fn get_health(state: axum::extract::State<DashboardState>) -> Json<Value> {
    Json(json!({
        "ok": true,
        "version": VERSION,
        "auth_required": state.auth_required,
    }))
}
