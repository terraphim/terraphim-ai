// Phase H: Graduated Guard is implemented but not yet wired into main.rs.
// These dead_code warnings are expected and will be resolved when the guard
// is integrated into the execution path (issue #704).
#![allow(dead_code)]

//! Graduated Guard: three-tier command execution safety evaluation.
//!
//! Classifies shell commands into one of three tiers before execution:
//!
//! - **Allow**: read-only or known-safe operations; execute directly
//! - **Sandbox**: unknown or unrecognised commands; route through Firecracker
//! - **Deny**: known-dangerous destructive patterns; block entirely
//!
//! # Example
//!
//! ```rust
//! use terraphim_agent::learnings::guard::{evaluate_command, ExecutionTier};
//!
//! let decision = evaluate_command("cargo test --lib", &[]);
//! assert_eq!(decision.tier, ExecutionTier::Allow);
//!
//! let decision = evaluate_command("rm -rf /", &[]);
//! assert_eq!(decision.tier, ExecutionTier::Deny);
//! ```

/// The three execution tiers used by the graduated guard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionTier {
    /// Command is known-safe; execute directly without sandboxing.
    Allow,
    /// Command is unknown or previously failed; route through Firecracker sandbox.
    Sandbox,
    /// Command matches a known-dangerous pattern; block execution entirely.
    Deny,
}

/// The outcome of a guard evaluation.
#[derive(Debug, Clone)]
pub struct GuardDecision {
    /// The tier assigned to this command.
    pub tier: ExecutionTier,
    /// Human-readable explanation for the decision.
    pub reason: String,
    /// The pattern that triggered this decision, if any.
    pub matched_pattern: Option<String>,
}

impl GuardDecision {
    #[allow(dead_code)]
    fn allow(reason: impl Into<String>) -> Self {
        Self {
            tier: ExecutionTier::Allow,
            reason: reason.into(),
            matched_pattern: None,
        }
    }

    fn allow_pattern(reason: impl Into<String>, pattern: impl Into<String>) -> Self {
        Self {
            tier: ExecutionTier::Allow,
            reason: reason.into(),
            matched_pattern: Some(pattern.into()),
        }
    }

    fn sandbox(reason: impl Into<String>) -> Self {
        Self {
            tier: ExecutionTier::Sandbox,
            reason: reason.into(),
            matched_pattern: None,
        }
    }

    fn sandbox_pattern(reason: impl Into<String>, pattern: impl Into<String>) -> Self {
        Self {
            tier: ExecutionTier::Sandbox,
            reason: reason.into(),
            matched_pattern: Some(pattern.into()),
        }
    }

    fn deny(reason: impl Into<String>, pattern: impl Into<String>) -> Self {
        Self {
            tier: ExecutionTier::Deny,
            reason: reason.into(),
            matched_pattern: Some(pattern.into()),
        }
    }
}

/// Prefixes of known-dangerous commands that should be denied outright.
///
/// Each entry is a prefix match against the trimmed, lowercased command.
const DENY_PREFIXES: &[&str] = &[
    "rm -rf /",
    "rm -rf ~",
    "rm -rf $home",
    ":(){:|:&};:",     // fork bomb
    "dd if=",
    "mkfs",
    "mkfs.",
    "fdisk",
    "parted",
    "> /dev/",
    "shred ",
    "wipefs",
];

/// Substrings that, if found anywhere in the command, trigger a Deny.
const DENY_SUBSTRINGS: &[&str] = &[
    "drop table",
    "drop database",
    "drop schema",
    "truncate table",
    "--force-with-lease",
    "git push --force",
    "git push -f ",
    "git reset --hard",
    "> /etc/",
    ">/etc/",
    "> /boot/",
    ">/boot/",
    "chmod -R 777 /",
    "chmod -R 777 ~",
    "sudo rm",
];

/// Prefixes of known-safe, read-only commands.
///
/// Commands matching these prefixes are allowed directly.
const ALLOW_PREFIXES: &[&str] = &[
    "cargo check",
    "cargo test",
    "cargo clippy",
    "cargo fmt",
    "cargo doc",
    "cargo build",
    "cargo run",
    "cargo bench",
    "cat ",
    "cat\t",
    "echo ",
    "echo\t",
    "ls ",
    "ls\t",
    "ls",
    "pwd",
    "which ",
    "type ",
    "file ",
    "head ",
    "tail ",
    "wc ",
    "diff ",
    "grep ",
    "rg ",
    "rg\t",
    "find ",
    "fd ",
    "bat ",
    "git log",
    "git diff",
    "git status",
    "git show",
    "git branch",
    "git remote",
    "git fetch",
    "git stash list",
    "git tag",
    "git rev-parse",
    "git describe",
    "git shortlog",
    "git blame",
    "rustc --version",
    "rustup",
    "cargo --version",
    "rustfmt",
    "date ",
    "date\n",
    "date",
    "env",
    "printenv",
    "uname",
    "hostname",
    "whoami",
    "id ",
    "id\n",
    "id",
    "ps ",
    "top ",
    "df ",
    "du ",
    "free ",
    "uptime",
    "lsof ",
    "curl ",
    "wget ",
    "http ",
    "httpie",
    "jq ",
    "yq ",
    "python3 -c",
    "python -c",
    "node -e",
];

/// Evaluate a shell command and return a [`GuardDecision`].
///
/// # Arguments
///
/// * `command` - The raw command string to evaluate.
/// * `previously_failed` - Slice of commands that previously failed in the learning
///   store. If `command` matches any entry here (prefix or substring), the tier is
///   elevated to [`ExecutionTier::Sandbox`] even if it would otherwise be allowed.
pub fn evaluate_command(command: &str, previously_failed: &[&str]) -> GuardDecision {
    let trimmed = command.trim();
    let lower = trimmed.to_lowercase();

    // --- Tier 3: Deny ---
    for &prefix in DENY_PREFIXES {
        if lower.starts_with(prefix) {
            return GuardDecision::deny(
                format!("matches known-dangerous prefix: '{prefix}'"),
                prefix,
            );
        }
    }
    for &substr in DENY_SUBSTRINGS {
        if lower.contains(substr) {
            return GuardDecision::deny(
                format!("contains known-dangerous pattern: '{substr}'"),
                substr,
            );
        }
    }

    // --- Learning feedback elevation ---
    // If this command previously failed, escalate to Sandbox even if it would
    // otherwise be Allowed. Uses prefix matching for slightly different invocations.
    for &failed in previously_failed {
        let failed_lower = failed.to_lowercase();
        if lower.starts_with(&failed_lower) || failed_lower.starts_with(&lower) {
            return GuardDecision::sandbox_pattern(
                "command matches a previously-failed entry in the learning store",
                failed,
            );
        }
    }

    // --- Tier 1: Allow ---
    for &prefix in ALLOW_PREFIXES {
        if lower.starts_with(prefix) {
            return GuardDecision::allow_pattern("matches known-safe prefix", prefix);
        }
    }

    // --- Tier 2: Sandbox (default for unknowns) ---
    GuardDecision::sandbox("command is not in the known-safe list; routing to sandbox")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allow_cargo_check() {
        let d = evaluate_command("cargo check --workspace", &[]);
        assert_eq!(d.tier, ExecutionTier::Allow);
    }

    #[test]
    fn test_allow_git_log() {
        let d = evaluate_command("git log --oneline -10", &[]);
        assert_eq!(d.tier, ExecutionTier::Allow);
    }

    #[test]
    fn test_allow_cat() {
        let d = evaluate_command("cat /etc/hosts", &[]);
        assert_eq!(d.tier, ExecutionTier::Allow);
    }

    #[test]
    fn test_deny_rm_rf_root() {
        let d = evaluate_command("rm -rf /", &[]);
        assert_eq!(d.tier, ExecutionTier::Deny);
        assert!(d.matched_pattern.is_some());
    }

    #[test]
    fn test_deny_git_force_push() {
        let d = evaluate_command("git push --force origin main", &[]);
        assert_eq!(d.tier, ExecutionTier::Deny);
    }

    #[test]
    fn test_deny_drop_table() {
        let d = evaluate_command("psql -c 'DROP TABLE users'", &[]);
        assert_eq!(d.tier, ExecutionTier::Deny);
    }

    #[test]
    fn test_deny_git_reset_hard() {
        let d = evaluate_command("git reset --hard HEAD~5", &[]);
        assert_eq!(d.tier, ExecutionTier::Deny);
    }

    #[test]
    fn test_sandbox_unknown_command() {
        let d = evaluate_command("my-custom-script --deploy prod", &[]);
        assert_eq!(d.tier, ExecutionTier::Sandbox);
    }

    #[test]
    fn test_sandbox_via_learning_feedback() {
        let d = evaluate_command("cargo build", &["cargo build"]);
        assert_eq!(d.tier, ExecutionTier::Sandbox);
        assert!(d.reason.contains("previously-failed"));
    }

    #[test]
    fn test_learning_feedback_prefix_match() {
        // "cargo build --release" should be sandboxed if "cargo build" previously failed.
        let d = evaluate_command("cargo build --release", &["cargo build"]);
        assert_eq!(d.tier, ExecutionTier::Sandbox);
    }

    #[test]
    fn test_deny_takes_priority_over_learning_feedback() {
        // Even if "rm -rf /" is in the learning store, it must still be Denied.
        let d = evaluate_command("rm -rf /", &["rm -rf /"]);
        assert_eq!(d.tier, ExecutionTier::Deny);
    }

    #[test]
    fn test_empty_command_sandboxed() {
        let d = evaluate_command("", &[]);
        assert_eq!(d.tier, ExecutionTier::Sandbox);
    }

    #[test]
    fn test_case_insensitive_deny() {
        let d = evaluate_command("GIT PUSH --FORCE origin main", &[]);
        assert_eq!(d.tier, ExecutionTier::Deny);
    }

    #[test]
    fn test_allow_ls_no_args() {
        let d = evaluate_command("ls", &[]);
        assert_eq!(d.tier, ExecutionTier::Allow);
    }
}
