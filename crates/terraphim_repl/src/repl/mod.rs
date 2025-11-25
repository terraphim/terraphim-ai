//! REPL (Read-Eval-Print-Loop) interface for Terraphim
//!
//! This module provides a minimal command-line interface for semantic search
//! and knowledge graph exploration.

pub mod commands;
pub mod handler;

pub use handler::{ReplHandler, run_repl_offline_mode};
