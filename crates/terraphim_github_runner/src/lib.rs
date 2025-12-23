//! # Terraphim GitHub Runner
//!
//! GitHub Actions runner with Firecracker sandbox integration for Terraphim AI.
//!
//! This crate provides:
//! - Webhook event handling for GitHub events
//! - LLM-based workflow understanding and translation
//! - Firecracker VM execution with snapshot management
//! - Command history tracking and rollback
//! - Knowledge graph integration for learning from execution patterns
//!
//! ## Feature Flags
//!
//! - `github-runner`: Enables the full runner functionality with multi-agent integration
//!
//! ## Architecture
//!
//! ```text
//! GitHub Event → WorkflowParser (LLM) → SessionManager → WorkflowExecutor
//!                                              ↓
//!                                       Firecracker VM
//!                                              ↓
//!                                    Snapshot on Success
//!                                              ↓
//!                                   LearningCoordinator
//! ```
//!
//! ## Example
//!
//! ```rust,ignore
//! use terraphim_github_runner::{GitHubEvent, WorkflowContext, RunnerConfig};
//!
//! // Create context from GitHub event
//! let context = WorkflowContext::new(event);
//!
//! // Execute workflow (when github-runner feature is enabled)
//! #[cfg(feature = "github-runner")]
//! let result = orchestrator.execute_workflow(context).await?;
//! ```

pub mod error;
pub mod models;

// Submodules (stubs for now, will be implemented in later steps)
pub mod learning;
pub mod session;
pub mod workflow;

// Re-exports for convenient access
pub use error::{GitHubRunnerError, Result};
pub use models::{
    ExecutionStatus, ExecutionStep, GitHubEvent, GitHubEventType, PullRequestInfo, RepositoryInfo,
    RunnerConfig, SessionId, SnapshotId, WorkflowContext, WorkflowResult,
};
pub use workflow::{ParsedWorkflow, WorkflowParser, WorkflowStep};

/// Crate version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Check if the github-runner feature is enabled
pub const fn is_runner_enabled() -> bool {
    cfg!(feature = "github-runner")
}
