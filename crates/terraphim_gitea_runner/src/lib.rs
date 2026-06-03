//! # Terraphim Native Gitea Runner (#1910 Milestone 1)
//!
//! A native Gitea Actions runner that speaks Gitea's `RunnerService` Connect-RPC
//! protocol directly (JSON wire) and executes jobs under a Terraphim
//! [`PolicyPlanner`](policy::PolicyPlanner) which routes each step to the host or
//! through `rch` (sccache-backed). Execution reuses the
//! [`terraphim_github_runner`] session/executor stack (host-only in M1).
//!
//! ## Lanes
//! This runner is the *native* CI lane. During migration the *interim* ADF
//! `build-runner-llm` lane writes `adf/build`; this runner writes
//! `terraphim-native/<job>`. A coexistence guard ([`config::RunnerConfig::active_repos`])
//! ensures exactly one lane runs per repo.
//!
//! ## Modules
//! - [`types`] -- Connect-JSON wire structs for the RunnerService surface.
//! - [`client`] -- [`client::GiteaRunnerClient`] over reqwest.
//! - [`state`] -- persisted runner identity (`.runner`).
//! - [`config`] -- [`config::RunnerConfig`].
//! - [`policy`] -- command allowlist + host/rch routing.
//! - [`build_md`] -- compile a repo `BUILD.md` into a `ParsedWorkflow`.
//! - [`checkout`] -- check out the target repo at the task's sha before build.
//! - [`workflow_payload`] -- decode a Gitea `WorkflowPayload` into a `ParsedWorkflow`.
//! - [`logs`] -- `UpdateLog` batching with monotonic ack.
//! - [`status`] -- single-writer commit-status helper.
//! - [`task_worker`] -- end-to-end task execution.
//! - [`poller`] -- the fetch/dispatch loop.

pub mod build_md;
pub mod checkout;
pub mod client;
pub mod config;
pub mod logs;
pub mod policy;
pub mod poller;
pub mod state;
pub mod status;
pub mod task_worker;
pub mod types;
pub mod workflow_payload;

pub use config::RunnerConfig;
pub use policy::{CommandRoute, DeterministicPlanner, ExecutionPlan, PolicyPlanner, TrustLevel};
pub use state::RunnerState;

/// Errors surfaced by the native runner.
#[derive(Debug, thiserror::Error)]
pub enum RunnerError {
    /// A Gitea RunnerService RPC failed.
    #[error("gitea protocol error: {0}")]
    Protocol(String),
    /// Local runner state could not be read/written.
    #[error("runner state error: {0}")]
    State(String),
    /// A workflow payload could not be compiled into an execution plan.
    #[error("workflow compile error: {0}")]
    Compile(String),
    /// Terraphim policy rejected a command before execution.
    #[error("policy rejected command: {0}")]
    PolicyRejected(String),
    /// Execution via the reused github_runner stack failed.
    #[error("execution error: {0}")]
    Execution(String),
}

/// Convenience result alias.
pub type Result<T> = std::result::Result<T, RunnerError>;
