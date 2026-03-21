//! Symphony orchestration service.
//!
//! A long-running service that continuously reads work from issue trackers,
//! creates isolated per-issue workspaces, and runs coding agent sessions
//! against each issue.

#[cfg(feature = "api")]
pub mod api;
pub mod config;
pub mod error;
pub mod orchestrator;
pub mod runner;
pub mod tracker;
pub mod workspace;

pub use error::{Result, SymphonyError};
pub use orchestrator::{OrchestratorRuntimeState, StateSnapshot, SymphonyOrchestrator};
pub use runner::{
    AdfEnvelope, AgentEvent, CodexSession, FindingCategory, FindingSeverity, ReviewAgentOutput,
    ReviewFinding, TokenCounts, TokenTotals, WorkerOutcome, deduplicate_findings,
};
pub use tracker::{Issue, IssueTracker};
pub use workspace::WorkspaceManager;
