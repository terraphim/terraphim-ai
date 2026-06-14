//! Hooks for RLM event capture by external agents.
//!
//! This module defines structured events that external systems (like
//! terraphim-agent) can subscribe to for learning capture, security auditing,
//! and observability without tight coupling to the execution engine.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use terraphim_rlm::hooks::{ValidationEvent, emit_validation_event};
//!
//! // Events are logged at warn level and can be captured by agent hooks.
//! emit_validation_event(ValidationEvent {
//!     input: "ls -la".to_string(),
//!     unknown_terms: vec!["ls".to_string(), "la".to_string()],
//!     matched_terms: vec!["list".to_string()],
//!     retry_count: 1,
//!     passed: false,
//! });
//! ```

use serde::{Deserialize, Serialize};

/// A validation event emitted when KG validation runs on a command.
///
/// External agents (terraphim-agent, security monitors) can capture these
/// events via log scraping or a registered callback to build learning
/// databases and security audit trails.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationEvent {
    /// The command or code input that was validated.
    pub input: String,
    /// Terms not found in the knowledge graph.
    pub unknown_terms: Vec<String>,
    /// Terms that matched in the knowledge graph.
    pub matched_terms: Vec<String>,
    /// Number of retries for this command (across the session/loop).
    pub retry_count: u32,
    /// Whether the validation passed.
    pub passed: bool,
    /// Validation message explaining the result.
    pub message: String,
}

/// Emit a validation event for external capture.
///
/// Currently logs at `warn` level with structured fields suitable for
/// log-based agents. In future, this could dispatch to a registered callback
/// or event bus.
pub fn emit_validation_event(event: &ValidationEvent) {
    if event.passed {
        log::info!(
            "RLM validation passed: input_len={}, matched={:?}",
            event.input.len(),
            event.matched_terms,
        );
    } else {
        log::warn!(
            "RLM validation FAILED: retry={}, unknown={:?}, matched={:?}, msg={}",
            event.retry_count,
            event.unknown_terms,
            event.matched_terms,
            event.message,
        );
    }
}
