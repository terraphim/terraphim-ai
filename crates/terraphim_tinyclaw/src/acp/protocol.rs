//! ACP protocol types — handshake messages, capabilities.

use serde::{Deserialize, Serialize};

/// ACP protocol version we implement.
///
/// Hermes' `test_server.py:121-132` checks the protocol version. ACP v0
/// is the current spec; we ship v0.1 for parity with Hermes' test fixtures.
pub const PROTOCOL_VERSION: &str = "0.1";

/// Agent metadata returned in `initialize`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentInfo {
    pub name: String,
    pub version: String,
}

/// Capabilities advertised during handshake.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct AgentCapabilities {
    /// Can the agent load existing sessions?
    #[serde(default)]
    pub load_session: bool,
    /// Can the agent stream messages?
    #[serde(default)]
    pub streaming: bool,
}

/// Result returned from `initialize`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeResult {
    pub protocol_version: String,
    pub agent_info: AgentInfo,
    pub capabilities: AgentCapabilities,
}

impl InitializeResult {
    pub fn new() -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION.to_string(),
            agent_info: AgentInfo {
                name: "tinyclaw".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            capabilities: AgentCapabilities {
                load_session: true,
                streaming: false,
            },
        }
    }
}

impl Default for InitializeResult {
    fn default() -> Self {
        Self::new()
    }
}