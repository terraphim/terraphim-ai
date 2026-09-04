//! Failure vocabulary for the Terraphim platform runtime.

use jiff::Timestamp;
use serde_json::Value;

/// Classification of a failure observed during execution or validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FailureKind {
    /// Failure originating in the Ultimate Bug Scanner pipeline.
    Ubs,
    /// Failure originating in the remote compilation helper (`rch`).
    Rch,
    /// Failure originating in the Terraphim Kache caching layer.
    Kache,
    /// Failure in the runner backend itself.
    Runner,
    /// Failure in workflow orchestration.
    Workflow,
    /// Failure caused by a policy violation or planner error.
    Policy,
}

/// Recommended next step after a failure is detected.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SuggestedAction {
    /// Edit the offending code and rerun.
    FixCode,
    /// Acknowledge the failure with a documented justification.
    Suppress {
        /// Human-readable reason for suppression.
        justification: String,
    },
    /// Promote this outcome to a known-good golden reference.
    PromoteToGolden,
    /// Send the work to a different execution route.
    Reroute,
    /// Hand the failure off to an LLM for diagnosis.
    EscalateToLlm,
    /// Leave the failure for later triage.
    Defer,
}

/// A single failure event, serialisable for logs and reports.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FailureEvent {
    /// Failure classification.
    pub kind: FailureKind,
    /// Stable signature for deduplication and grouping.
    pub signature: String,
    /// Recommended action.
    pub action: SuggestedAction,
    /// Free-form structured context.
    pub context: Value,
    /// Identifier of the workspace where the failure occurred.
    pub workspace: String,
    /// Timestamp when the failure was recorded.
    pub timestamp: Timestamp,
}
