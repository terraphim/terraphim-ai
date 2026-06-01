#![allow(dead_code)]

//! Graduated Guard for command execution safety.
//!
//! Provides a three-tier execution model:
//! - **Allow**: Known-safe commands (e.g., `ls`, `cat`, `echo`)
//! - **Sandbox**: Unknown commands executed in a restricted environment
//! - **Deny**: Known-dangerous patterns (e.g., `rm -rf /`, `mkfs`, `dd if=/dev/zero`)
//!
//! The guard integrates with the learning system to elevate caution for commands
//! that have previously failed.
//!
//! Wiring this guard into the REPL command execution path is tracked by #1953.
#![allow(dead_code)]

use std::fmt;

/// Three-tier execution safety model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionTier {
    /// Command is known-safe; execute directly.
    Allow,
    /// Command is unknown; execute in sandboxed environment.
    Sandbox,
    /// Command matches dangerous pattern; block execution.
    Deny,
}

impl fmt::Display for ExecutionTier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecutionTier::Allow => write!(f, "ALLOW"),
            ExecutionTier::Sandbox => write!(f, "SANDBOX"),
            ExecutionTier::Deny => write!(f, "DENY"),
        }
    }
}

/// Decision produced by the guard for a given command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardDecision {
    /// Execution tier assigned.
    pub tier: ExecutionTier,
    /// Human-readable reason for the decision.
    pub reason: String,
    /// Whether this command has previously failed (from learning feedback).
    pub previously_failed: bool,
}

impl GuardDecision {
    /// Create a new guard decision.
    pub fn new(tier: ExecutionTier, reason: impl Into<String>) -> Self {
        Self {
            tier,
            reason: reason.into(),
            previously_failed: false,
        }
    }

    /// Mark this command as having previously failed.
    pub fn with_previous_failure(mut self) -> Self {
        self.previously_failed = true;
        if self.tier == ExecutionTier::Allow {
            self.tier = ExecutionTier::Sandbox;
            self.reason = format!(
                "{} (elevated to sandbox due to previous failure)",
                self.reason
            );
        }
        self
    }

    /// Check if execution is permitted (Allow or Sandbox).
    pub fn is_permitted(&self) -> bool {
        matches!(self.tier, ExecutionTier::Allow | ExecutionTier::Sandbox)
    }

    /// Check if execution is denied.
    pub fn is_denied(&self) -> bool {
        self.tier == ExecutionTier::Deny
    }
}

/// Known-safe command patterns (exact matches or prefixes).
const ALLOW_PATTERNS: &[&str] = &[
    "ls",
    "ll",
    "dir",
    "cat",
    "head",
    "tail",
    "less",
    "more",
    "echo",
    "printf",
    "pwd",
    "whoami",
    "uname",
    "date",
    "uptime",
    "grep",
    "rg",
    "find",
    "fd",
    "git status",
    "git log",
    "git diff",
    "git show",
    "git branch",
    "git push",
    "git pull",
    "git fetch",
    "git clone",
    "git checkout",
    "git add",
    "git commit",
    "git merge",
    "git rebase",
    "git cherry-pick",
    "cargo check",
    "cargo clippy",
    "cargo fmt",
    "cargo doc",
    "terraform plan",
    "terraform validate",
    "terraform show",
    "docker ps",
    "docker images",
    "docker logs",
    "kubectl get",
    "kubectl describe",
    "kubectl logs",
    "tar -tvf",
    "tar --list",
    "zipinfo",
    "unzip -l",
    "md5sum",
    "sha256sum",
    "sha1sum",
    "wc",
    "sort",
    "uniq",
    "cut",
    "awk",
    "sed",
    "tr",
    "which",
    "whereis",
    "type",
    "env",
    "printenv",
    "history",
    "man",
    "help",
    "info",
    "clear",
    "reset",
    "true",
    "false",
    "exit",
];

/// Known-dangerous command patterns (substring matches).
const DENY_PATTERNS: &[&str] = &[
    "rm -rf /",
    "rm -rf /*",
    "rm -rf ~",
    "rm -rf $HOME",
    "mkfs",
    "mkfs.ext",
    "mkfs.xfs",
    "mkfs.btrfs",
    "dd if=/dev/zero",
    "dd if=/dev/random",
    "dd if=/dev/urandom",
    ":(){ :|:& };:",
    "fork bomb",
    "> /dev/sda",
    "> /dev/hda",
    "> /dev/nvme",
    "chmod -R 777 /",
    "chmod -R 000 /",
    "mv / /dev/null",
    "rmdir /",
    "rmdir /*",
    "truncate -s 0 /",
    "shred -n 0 -z /",
    "kill -9 -1",
    "kill -9 1",
    "reboot",
    "shutdown",
    "poweroff",
    "halt",
    "init 0",
    "init 6",
    "systemctl stop",
    "echo .* > /etc/passwd",
    "echo .* > /etc/shadow",
];

/// Evaluate a command and return the appropriate guard decision.
///
/// # Examples
///
/// ```
/// use terraphim_agent::learnings::guard::{evaluate_command, ExecutionTier};
///
/// let decision = evaluate_command("ls -la");
/// assert_eq!(decision.tier, ExecutionTier::Allow);
///
/// let decision = evaluate_command("rm -rf /");
/// assert_eq!(decision.tier, ExecutionTier::Deny);
/// ```
pub fn evaluate_command(command: &str) -> GuardDecision {
    let trimmed = command.trim();

    // Check deny patterns first (highest priority)
    for pattern in DENY_PATTERNS {
        if trimmed.contains(pattern) {
            return GuardDecision::new(
                ExecutionTier::Deny,
                format!("Matches dangerous pattern: '{}'", pattern),
            );
        }
    }

    // Check allow patterns
    for pattern in ALLOW_PATTERNS {
        if trimmed.starts_with(pattern) {
            return GuardDecision::new(
                ExecutionTier::Allow,
                format!("Known-safe command: '{}'", pattern),
            );
        }
    }

    // Unknown command -> sandbox
    GuardDecision::new(
        ExecutionTier::Sandbox,
        "Unknown command; execute in sandbox",
    )
}

/// Evaluate a command with learning feedback integration.
///
/// If the command has previously failed (recorded in learnings), it will be
/// elevated to Sandbox tier even if it would normally be Allowed.
///
/// # Examples
///
/// ```
/// use terraphim_agent::learnings::guard::{evaluate_command_with_learning, ExecutionTier};
///
/// // Command that would normally be allowed, but has failed before
/// let decision = evaluate_command_with_learning("git push", true);
/// assert_eq!(decision.tier, ExecutionTier::Sandbox);
/// assert!(decision.previously_failed);
/// ```
pub fn evaluate_command_with_learning(command: &str, has_failed_before: bool) -> GuardDecision {
    let decision = evaluate_command(command);
    if has_failed_before {
        decision.with_previous_failure()
    } else {
        decision
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allow_basic_commands() {
        assert_eq!(evaluate_command("ls -la").tier, ExecutionTier::Allow);
        assert_eq!(evaluate_command("cat file.txt").tier, ExecutionTier::Allow);
        assert_eq!(evaluate_command("echo hello").tier, ExecutionTier::Allow);
        assert_eq!(evaluate_command("git status").tier, ExecutionTier::Allow);
        assert_eq!(evaluate_command("cargo check").tier, ExecutionTier::Allow);
    }

    #[test]
    fn test_deny_dangerous_commands() {
        assert_eq!(evaluate_command("rm -rf /").tier, ExecutionTier::Deny);
        assert_eq!(
            evaluate_command("dd if=/dev/zero of=/dev/sda").tier,
            ExecutionTier::Deny
        );
        assert_eq!(
            evaluate_command("mkfs.ext4 /dev/sda1").tier,
            ExecutionTier::Deny
        );
        assert_eq!(evaluate_command("chmod -R 777 /").tier, ExecutionTier::Deny);
    }

    #[test]
    fn test_sandbox_unknown_commands() {
        assert_eq!(
            evaluate_command("some-unknown-tool --flag").tier,
            ExecutionTier::Sandbox
        );
        assert_eq!(
            evaluate_command("./random-script.sh").tier,
            ExecutionTier::Sandbox
        );
    }

    #[test]
    fn test_learning_elevation() {
        let decision = evaluate_command_with_learning("git push", true);
        assert_eq!(decision.tier, ExecutionTier::Sandbox);
        assert!(decision.previously_failed);
        assert!(decision.reason.contains("elevated to sandbox"));
    }

    #[test]
    fn test_no_elevation_without_failure() {
        let decision = evaluate_command_with_learning("git push", false);
        assert_eq!(decision.tier, ExecutionTier::Allow);
        assert!(!decision.previously_failed);
    }

    #[test]
    fn test_deny_not_elevated() {
        // Deny commands stay denied even with previous failure
        let decision = evaluate_command_with_learning("rm -rf /", true);
        assert_eq!(decision.tier, ExecutionTier::Deny);
    }

    #[test]
    fn test_sandbox_stays_sandbox() {
        // Unknown commands stay sandbox even with previous failure
        let decision = evaluate_command_with_learning("unknown-cmd", true);
        assert_eq!(decision.tier, ExecutionTier::Sandbox);
    }

    #[test]
    fn test_decision_permitted() {
        assert!(evaluate_command("ls").is_permitted());
        assert!(evaluate_command("unknown").is_permitted());
        assert!(!evaluate_command("rm -rf /").is_permitted());
    }

    #[test]
    fn test_display_tier() {
        assert_eq!(format!("{}", ExecutionTier::Allow), "ALLOW");
        assert_eq!(format!("{}", ExecutionTier::Sandbox), "SANDBOX");
        assert_eq!(format!("{}", ExecutionTier::Deny), "DENY");
    }
}
