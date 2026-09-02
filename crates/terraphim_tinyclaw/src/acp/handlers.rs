//! ACP handlers — pure-function request handlers (testable without stdio).

use serde::{Deserialize, Serialize};

use super::AcpState;
use super::protocol::{InitializeResult, PROTOCOL_VERSION};

/// Request for `initialize`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InitializeRequest {
    /// Client's protocol version preference.
    #[serde(default)]
    pub protocol_version: Option<String>,
}

/// Response for `new_session` / `load_session`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionResult {
    pub session_id: String,
}

/// Request for `send_message`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageRequest {
    pub session_id: String,
    pub role: String,
    pub content: String,
}

/// Response for `send_message`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageResult {
    pub session_id: String,
    pub message_index: usize,
}

/// Request for `cancel`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelRequest {
    pub session_id: String,
}

/// Response for `list_sessions`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListSessionsResult {
    pub sessions: Vec<String>,
}

/// ACP error code (Hermes-compatible).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpError {
    pub code: i32,
    pub message: String,
}

impl AcpError {
    pub fn session_not_found(id: &str) -> Self {
        Self {
            code: -32004,
            message: format!("Session not found: {id}"),
        }
    }
}

// --- handlers (no I/O, no async for ease of testing) -----------------------

/// Handle `initialize` — return protocol version + agent info.
pub fn handle_initialize(_state: &AcpState, _req: InitializeRequest) -> InitializeResult {
    InitializeResult::new()
}

/// Handle `new_session` — create or retrieve a session.
pub async fn handle_new_session(
    state: &AcpState,
    session_id: String,
) -> Result<SessionResult, AcpError> {
    let mut manager = state.sessions.lock().await;
    let id = {
        let session = manager.get_or_create(&session_id);
        session.key.clone()
    };
    let session_ref = manager.get(&id).ok_or_else(|| AcpError {
        code: -32004,
        message: format!("Session not found after create: {id}"),
    })?;
    manager.save(session_ref).map_err(|e| AcpError {
        code: -32603,
        message: format!("save failed: {e}"),
    })?;
    Ok(SessionResult { session_id: id })
}

/// Handle `load_session` — load existing session.
pub async fn handle_load_session(
    state: &AcpState,
    session_id: String,
) -> Result<SessionResult, AcpError> {
    let manager = state.sessions.lock().await;
    match manager.get(&session_id) {
        Some(s) => Ok(SessionResult {
            session_id: s.key.clone(),
        }),
        None => Err(AcpError::session_not_found(&session_id)),
    }
}

/// Handle `list_sessions`.
pub async fn handle_list_sessions(state: &AcpState) -> Result<ListSessionsResult, AcpError> {
    let manager = state.sessions.lock().await;
    let sessions = manager.list_sessions().unwrap_or_default();
    Ok(ListSessionsResult { sessions })
}

/// Handle `send_message` — append a message to a session.
pub async fn handle_send_message(
    state: &AcpState,
    req: SendMessageRequest,
) -> Result<SendMessageResult, AcpError> {
    let msg = match req.role.as_str() {
        "user" => crate::session::ChatMessage::user(req.content, "acp"),
        "assistant" => crate::session::ChatMessage::assistant(req.content),
        "tool" => crate::session::ChatMessage::tool(req.content, "acp-tool"),
        _ => {
            return Err(AcpError {
                code: -32602,
                message: format!("invalid role: {}", req.role),
            });
        }
    };

    let mut manager = state.sessions.lock().await;
    let session_id = req.session_id.clone();

    // Check session exists (returns same error code as load_session).
    if manager.get(&session_id).is_none() {
        return Err(AcpError::session_not_found(&session_id));
    }
    let message_count = manager.get(&session_id).unwrap().message_count();

    // Append + persist.
    let session = manager.get_or_create(&session_id);
    session.add_message(msg);
    let session_ref = manager.get(&session_id).unwrap();
    manager.save(session_ref).map_err(|e| AcpError {
        code: -32603,
        message: format!("save failed: {e}"),
    })?;

    Ok(SendMessageResult {
        session_id,
        message_index: message_count,
    })
}

/// Handle `cancel` — mark session cancelled.
pub async fn handle_cancel(state: &AcpState, req: CancelRequest) -> Result<AcpError, AcpError> {
    let manager = state.sessions.lock().await;
    if manager.get(&req.session_id).is_none() {
        return Err(AcpError::session_not_found(&req.session_id));
    }
    // TinyClaw doesn't track a "cancelled" flag on sessions yet; cancel is
    // a no-op acknowledgement in this Wave 5 cut. A future Wave 6+ work
    // item can add a `cancelled_at` field to Session.
    let _ = PROTOCOL_VERSION;
    Ok(AcpError {
        code: 0,
        message: "ok".into(),
    })
}
