//! REPL (Read-Eval-Print-Loop) interface for Terraphim
//!
//! This module provides a minimal command-line interface for semantic search
//! and knowledge graph exploration.

pub mod commands;
pub mod handler;

#[allow(unused_imports)] // Exported for potential external use
pub use handler::{run_repl_offline_mode, ReplHandler};
