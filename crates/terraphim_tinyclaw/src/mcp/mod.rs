//! MCP (Model Context Protocol) client and server for the 9-tool channel bridge.
//!
//! Wave 2 of the Hermes parity arc (epic #3160).
//!
//! - **Server** (`server.rs`): exposes TinyClaw's conversations, messages, and
//!   approval requests as MCP tools over stdio, matching Hermes' `mcp_serve.py`.
//! - **Client** (`client.rs`): connects to external MCP servers via stdio and
//!   exposes their tools to TinyClaw's `ToolRegistry`.
//!
//! **Default behaviour: disabled.** The MCP server is only started when
//! `mcp.enabled = true` in config. The client is only used when
//! `mcp.server_command` is configured.

pub mod client;
pub mod server;
pub mod tools;

pub use client::McpClient;
pub use server::TinyClawMcpServer;
pub use tools::*;

/// Errors the MCP layer can produce.
#[derive(Debug, thiserror::Error)]
pub enum McpError {
    /// MCP server error.
    #[error("MCP server error: {0}")]
    Server(String),

    /// MCP client error.
    #[error("MCP client error: {0}")]
    Client(String),

    /// Session not found.
    #[error("session not found: {0}")]
    SessionNotFound(String),

    /// Conversation not found.
    #[error("conversation not found: {0}")]
    ConversationNotFound(String),

    /// Approval request not found.
    #[error("approval request not found: {0}")]
    ApprovalNotFound(String),

    /// rmcp protocol error.
    #[error(transparent)]
    Rmcp(#[from] rmcp::Error),
}
