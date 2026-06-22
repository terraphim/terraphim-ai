//! Terraphim execution policy: command allowlist + host/rch routing.
//!
//! The planner is **authoritative** -- repo-authored workflow steps never bypass
//! it. Cargo-heavy commands are rewritten to run through `rch` (sccache-backed);
//! everything else on the allowlist runs on the host. Disallowed commands fail
//! compilation before execution.

use crate::Result;
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

/// Whether `name` looks like a POSIX shell variable identifier.
pub(crate) fn is_env_name(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Bytes consumed for one assignment value (the part after `=`).
pub(crate) fn consume_assignment_value(s: &str) -> usize {
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
pub(crate) fn strip_env_assignments(cmd: &str) -> &str {
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
pub(crate) fn program(cmd: &str) -> &str {
    strip_env_assignments(cmd)
        .split_whitespace()
        .next()
        .unwrap_or("")
}
