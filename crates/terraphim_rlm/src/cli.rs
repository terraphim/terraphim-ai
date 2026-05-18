//! CLI interface for terraphim_rlm.
//!
//! Provides a command-line interface to the RLM orchestration system.
//! Reads JSON arguments from stdin, executes commands, outputs JSON results.

use std::collections::HashMap;

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// CLI arguments for terraphim_rlm.
#[derive(Parser)]
#[command(name = "terraphim_rlm")]
#[command(about = "Terraphim RLM - Recursive Language Model orchestration")]
#[command(version = env!("CARGO_PKG_VERSION"))]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

/// Available commands.
#[derive(Subcommand)]
pub enum Commands {
    /// Create or destroy a session
    Session {
        #[command(subcommand)]
        action: SessionAction,
    },
    /// Execute Python code
    Code {
        /// Session ID
        #[arg(long)]
        session_id: String,
    },
    /// Execute a bash command
    Bash {
        /// Session ID
        #[arg(long)]
        session_id: String,
    },
    /// Query the LLM
    Query {
        /// Session ID
        #[arg(long)]
        session_id: String,
    },
    /// Manage context variables
    Context {
        /// Session ID
        #[arg(long)]
        session_id: String,
    },
    /// Manage snapshots
    Snapshot {
        /// Session ID
        #[arg(long)]
        session_id: String,
    },
    /// Get session status
    Status {
        /// Session ID
        #[arg(long)]
        session_id: String,
    },
}

/// Session management actions.
#[derive(Subcommand)]
pub enum SessionAction {
    /// Create a new session
    Create,
    /// Destroy a session
    Destroy {
        /// Session ID
        #[arg(long)]
        session_id: String,
    },
}

// =============================================================================
// Request types (parsed from stdin JSON)
// =============================================================================

/// Request for code execution.
#[derive(Debug, Deserialize)]
pub struct CodeRequest {
    pub code: String,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

/// Request for bash execution.
#[derive(Debug, Deserialize)]
pub struct BashRequest {
    pub command: String,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    pub working_dir: Option<String>,
}

/// Request for LLM query.
#[derive(Debug, Deserialize)]
pub struct QueryRequest {
    pub prompt: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub temperature: Option<f64>,
    #[serde(default)]
    pub max_tokens: Option<u64>,
}

/// Request for context variable operations.
#[derive(Debug, Deserialize)]
pub struct ContextRequest {
    pub action: String,
    #[serde(default)]
    pub key: Option<String>,
    #[serde(default)]
    pub value: Option<String>,
}

/// Request for snapshot operations.
#[derive(Debug, Deserialize)]
pub struct SnapshotRequest {
    pub action: String,
    #[serde(default)]
    pub snapshot_name: Option<String>,
}

/// Request for status.
#[derive(Debug, Deserialize)]
pub struct StatusRequest {
    #[serde(default)]
    pub include_history: bool,
}

// =============================================================================
// Response types (serialized to stdout JSON)
// =============================================================================

/// Wrapper for all CLI responses.
#[derive(Debug, Serialize)]
pub struct CliResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<CliError>,
}

/// Error response.
#[derive(Debug, Serialize)]
pub struct CliError {
    pub r#type: String,
    pub message: String,
}

impl CliResponse {
    pub fn success<T: Serialize>(data: T) -> Self {
        Self {
            success: true,
            data: Some(serde_json::to_value(data).unwrap_or(Value::Null)),
            error: None,
        }
    }

    pub fn error(error_type: &str, message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(CliError {
                r#type: error_type.to_string(),
                message,
            }),
        }
    }
}

/// Execution result response.
#[derive(Debug, Serialize)]
pub struct ExecutionResponse {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub execution_time_ms: u64,
    pub success: bool,
}

/// Query result response.
#[derive(Debug, Serialize)]
pub struct QueryResponse {
    pub response: String,
    pub tokens_used: u64,
    pub model: String,
}

/// Context result response.
#[derive(Debug, Serialize)]
pub struct ContextResponse {
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<HashMap<String, String>>,
}

/// Snapshot result response.
#[derive(Debug, Serialize)]
pub struct SnapshotResponse {
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshots: Option<Vec<String>>,
}

/// Session creation response.
#[derive(Debug, Serialize)]
pub struct SessionCreateResponse {
    pub session_id: String,
    pub state: String,
    pub created_at: String,
    pub expires_at: String,
}
