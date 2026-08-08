//! ACP (Agent Communication Protocol) adapter.
//!
//! Wave 5 (Phase C4) of the Hermes parity arc. Exposes a subset of Hermes'
//! ACP protocol surface over JSON-RPC:
//!
//! - `initialize` — protocol handshake (returns protocolVersion, agentInfo,
//!   capabilities)
//! - `new_session` — create a session
//! - `load_session` — load existing session
//! - `list_sessions` — enumerate sessions
//! - `send_message` — append a message to a session
//! - `cancel` — mark a session cancelled
//!
//! Uses stdio JSON-RPC (similar to the MCP server). Hermes' ACP reference
//! lives in `tests/acp/test_server.py`.

pub mod handlers;
pub mod protocol;
pub mod router;

pub use protocol::{AgentCapabilities, AgentInfo, InitializeResult, ProtocolVersion};

use std::sync::Arc;
use tokio::sync::Mutex;

use crate::session::SessionManager;

/// Shared ACP server state.
#[derive(Clone)]
pub struct AcpState {
    pub sessions: Arc<Mutex<SessionManager>>,
}

impl AcpState {
    pub fn new(sessions_dir: std::path::PathBuf) -> Self {
        Self {
            sessions: Arc::new(Mutex::new(SessionManager::new(sessions_dir))),
        }
    }
}