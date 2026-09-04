//! L1 vocabulary for the Terraphim agentic execution platform.
//!
//! This crate defines the shared types and traits that orchestrate the
//! knowledge-graph-driven platform architecture. It contains no business logic,
//! no LLM dependencies, and no TACP dependencies; downstream crates provide the
//! concrete implementations.
//!
//! The vocabulary is intentionally small and stable so that runners, validators,
//! policy planners, and backends can communicate without coupling to each other.

pub mod artifact;
pub mod backend;
pub mod cache;
pub mod error;
pub mod failure;
pub mod policy;
pub mod route;
pub mod runner;
pub mod trust;
pub mod validator;
pub mod workspace;

pub use artifact::{Artifact, ArtifactKind};
pub use backend::{ExecutionResult, ExecutorBackend};
pub use cache::CacheKind;
pub use error::{ExecutionError, PolicyError, ValidatorError};
pub use failure::{FailureEvent, FailureKind, SuggestedAction};
pub use policy::{PolicyDecision, PolicyPlanner};
pub use route::Route;
pub use runner::RunnerKind;
pub use trust::TrustTier;
pub use validator::ValidatorHandle;
pub use workspace::{Workspace, WorkspaceMode};

#[cfg(test)]
mod tests {
    mod smoke;
}
