//! Unified hooks infrastructure for Terraphim AI.
//!
//! This crate provides shared functionality for Claude Code hooks and Git hooks,
//! including text replacement via knowledge graphs and binary discovery utilities.

mod discovery;
mod replacement;

pub use discovery::{BinaryLocation, discover_binary};
pub use replacement::{HookResult, LinkType, ReplacementService};

/// Re-export key types from terraphim_automata for convenience.
pub use terraphim_automata::Matched;
pub use terraphim_types::Thesaurus;
