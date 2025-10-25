//! REPL (Read-Eval-Print-Loop) interface for Terraphim TUI
//!
//! This module provides a command-line interface that matches functionality
//! available in Tauri desktop application, with commands for search, chat,
//! configuration management, and MCP tools integration.

#[cfg(feature = "repl")]
pub mod commands;

#[cfg(feature = "repl")]
pub mod handler;

#[cfg(feature = "repl")]
pub mod web_operations;

#[cfg(feature = "repl-file")]
pub mod file_operations;

#[cfg(feature = "repl-chat")]
pub mod chat;

#[cfg(feature = "repl-mcp")]
pub mod mcp_tools;

// Re-export main run functions
#[cfg(feature = "repl")]
pub use handler::{run_repl_offline_mode, run_repl_server_mode};
