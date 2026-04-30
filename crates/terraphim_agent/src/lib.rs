//! Terraphim AI agent runtime.
//!
//! Provides the interactive TUI, robot mode CLI, MCP tool index, and
//! forgiving command parser used by the `terraphim-agent` binary.
//!
//! ## Key modules
//!
//! - [`robot`] -- machine-readable (JSON) output for AI agent integration
//! - [`forgiving`] -- typo-tolerant command dispatcher
//! - [`mcp_tool_index`] -- discovery and search of available MCP tools
//! - [`service`] -- search and session management backend
//! - [`onboarding`] -- first-run setup and configuration wizard
//!
//! ## Features
//!
//! - `server` -- enables the HTTP client for remote agent communication
//! - `repl` -- full interactive REPL with command history
//! - `shared-learning` -- cross-session learning store
#[cfg(feature = "server")]
pub mod client;
pub mod onboarding;
pub mod service;
#[cfg(feature = "shared-learning")]
pub mod shared_learning;
pub mod tui_backend;

// Robot mode - always available for AI agent integration
pub mod robot;

// Forgiving CLI - always available for typo-tolerant parsing
pub mod forgiving;

// MCP Tool Index - for discovering and searching MCP tools
pub mod mcp_tool_index;

#[cfg(feature = "repl")]
pub mod repl;

#[cfg(feature = "repl-custom")]
pub mod commands;

#[cfg(feature = "server")]
pub use client::*;

// Re-export robot mode types
pub use robot::{
    BudgetEngine, BudgetError, BudgetedResults, ExitCode, FieldMode, OutputFormat, RobotConfig,
    RobotError, RobotFormatter, RobotResponse, SelfDocumentation,
};

// Re-export forgiving CLI types
pub use forgiving::{AliasRegistry, ForgivingParser, ParseResult};

#[cfg(feature = "repl")]
pub use repl::*;

#[cfg(feature = "repl-custom")]
pub use commands::*;

// Test-specific exports - make modules available in tests with required features
#[cfg(test)]
pub mod test_exports {
    #[cfg(feature = "repl")]
    pub use crate::repl::*;

    #[cfg(feature = "repl")]
    pub use std::str::FromStr;

    #[cfg(feature = "repl-custom")]
    pub use crate::commands::*;

    pub use crate::forgiving::*;
    pub use crate::robot::*;
}
