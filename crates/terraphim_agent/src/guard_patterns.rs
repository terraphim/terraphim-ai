//! Guard patterns for blocking destructive git and filesystem commands.
//!
//! This module defines patterns that should be blocked to prevent accidental
//! destruction of uncommitted work or important files.

use regex::Regex;
use serde::{Deserialize, Serialize};

/// Result of checking a command against guard patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardResult {
    /// The decision: "allow" or "block"
    pub decision: String,
    /// Reason for blocking (only present if blocked)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// The original command that was checked
    pub command: String,
    /// The pattern that matched (only present if blocked)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
}

impl GuardResult {
    /// Create an "allow" result
    pub fn allow(command: String) -> Self {
        Self {
            decision: "allow".to_string(),
            reason: None,
            command,
            pattern: None,
        }
    }

    /// Create a "block" result
    pub fn block(command: String, reason: String, pattern: String) -> Self {
        Self {
            decision: "block".to_string(),
            reason: Some(reason),
            command,
            pattern: Some(pattern),
        }
    }
}

/// A pattern that should be blocked with its reason
struct DestructivePattern {
    regex: Regex,
    reason: &'static str,
    pattern_str: &'static str,
}

impl DestructivePattern {
    fn new(pattern: &'static str, reason: &'static str) -> Self {
        Self {
            regex: Regex::new(pattern).expect("Invalid regex pattern"),
            reason,
            pattern_str: pattern,
        }
    }
}

/// A pattern that is explicitly safe (allowlist)
struct SafePattern {
    regex: Regex,
}

impl SafePattern {
    fn new(pattern: &'static str) -> Self {
        Self {
            regex: Regex::new(pattern).expect("Invalid regex pattern"),
        }
    }
}

/// Guard that checks commands against destructive patterns
pub struct CommandGuard {
    destructive_patterns: Vec<DestructivePattern>,
    safe_patterns: Vec<SafePattern>,
}

impl Default for CommandGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandGuard {
    /// Create a new command guard with default patterns
    pub fn new() -> Self {
        let destructive_patterns = vec![
            // Git commands that discard uncommitted changes
            DestructivePattern::new(
                r"git\s+checkout\s+--\s+",
                "git checkout -- discards uncommitted changes permanently. Use 'git stash' first.",
            ),
            // git checkout <ref> -- <path> - safe patterns (checkout -b, --orphan) handled in allowlist
            DestructivePattern::new(
                r"git\s+checkout\s+[^\s]+\s+--\s+",
                "git checkout <ref> -- <path> overwrites working tree. Use 'git stash' first.",
            ),
            // git restore without --staged - safe pattern handled in allowlist
            DestructivePattern::new(
                r"git\s+restore\s+[^\s]",
                "git restore discards uncommitted changes. Use 'git stash' or 'git diff' first.",
            ),
            DestructivePattern::new(
                r"git\s+restore\s+--worktree",
                "git restore --worktree discards uncommitted changes permanently.",
            ),
            // Git reset variants
            DestructivePattern::new(
                r"git\s+reset\s+--hard",
                "git reset --hard destroys uncommitted changes. Use 'git stash' first.",
            ),
            DestructivePattern::new(
                r"git\s+reset\s+--merge",
                "git reset --merge can lose uncommitted changes.",
            ),
            // Git clean
            DestructivePattern::new(
                r"git\s+clean\s+-[a-z]*f",
                "git clean -f removes untracked files permanently. Review with 'git clean -n' first.",
            ),
            // Force operations - safe pattern (--force-with-lease) handled in allowlist
            DestructivePattern::new(
                r"git\s+push\s+.*--force",
                "Force push can destroy remote history. Use --force-with-lease if necessary.",
            ),
            DestructivePattern::new(
                r"git\s+push\s+-f\b",
                "Force push (-f) can destroy remote history. Use --force-with-lease if necessary.",
            ),
            DestructivePattern::new(
                r"git\s+branch\s+-D\b",
                "git branch -D force-deletes without merge check. Use -d for safety.",
            ),
            // Destructive filesystem commands
            DestructivePattern::new(
                r"rm\s+-[a-z]*r[a-z]*f|rm\s+-[a-z]*f[a-z]*r",
                "rm -rf is destructive. List files first, then delete individually with permission.",
            ),
            // Git stash drop/clear
            DestructivePattern::new(
                r"git\s+stash\s+drop",
                "git stash drop permanently deletes stashed changes. List stashes first.",
            ),
            DestructivePattern::new(
                r"git\s+stash\s+clear",
                "git stash clear permanently deletes ALL stashed changes.",
            ),
        ];

        let safe_patterns = vec![
            // Git checkout variants that are safe
            SafePattern::new(r"git\s+checkout\s+-b\s+"),
            SafePattern::new(r"git\s+checkout\s+--orphan\s+"),
            // Git restore --staged is safe (only unstages)
            SafePattern::new(r"git\s+restore\s+--staged"),
            // Git clean dry run is safe
            SafePattern::new(r"git\s+clean\s+-n"),
            SafePattern::new(r"git\s+clean\s+--dry-run"),
            // --force-with-lease is safer than --force
            SafePattern::new(r"git\s+push\s+.*--force-with-lease"),
            // rm -rf on temp directories is safe
            SafePattern::new(r"rm\s+-[a-z]*r[a-z]*f[a-z]*\s+/tmp/"),
            SafePattern::new(r"rm\s+-[a-z]*r[a-z]*f[a-z]*\s+/var/tmp/"),
            SafePattern::new(r#"rm\s+-[a-z]*r[a-z]*f[a-z]*\s+\$TMPDIR/"#),
            SafePattern::new(r#"rm\s+-[a-z]*r[a-z]*f[a-z]*\s+\$\{TMPDIR"#),
            SafePattern::new(r#"rm\s+-[a-z]*r[a-z]*f[a-z]*\s+"\$TMPDIR/"#),
            SafePattern::new(r#"rm\s+-[a-z]*r[a-z]*f[a-z]*\s+"\$\{TMPDIR"#),
        ];

        Self {
            destructive_patterns,
            safe_patterns,
        }
    }

    /// Check if a command matches any safe pattern (allowlist)
    fn is_safe(&self, command: &str) -> bool {
        self.safe_patterns.iter().any(|p| p.regex.is_match(command))
    }

    /// Check a command against guard patterns
    ///
    /// Returns a GuardResult indicating whether the command should be allowed or blocked.
    pub fn check(&self, command: &str) -> GuardResult {
        // Check safe patterns first (allowlist)
        if self.is_safe(command) {
            return GuardResult::allow(command.to_string());
        }

        // Check destructive patterns
        for pattern in &self.destructive_patterns {
            if pattern.regex.is_match(command) {
                return GuardResult::block(
                    command.to_string(),
                    pattern.reason.to_string(),
                    pattern.pattern_str.to_string(),
                );
            }
        }

        // No match - allow
        GuardResult::allow(command.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_checkout_double_dash_blocked() {
        let guard = CommandGuard::new();
        let result = guard.check("git checkout -- file.txt");
        assert_eq!(result.decision, "block");
        assert!(result.reason.is_some());
    }

    #[test]
    fn test_git_checkout_branch_allowed() {
        let guard = CommandGuard::new();
        let result = guard.check("git checkout -b new-feature");
        assert_eq!(result.decision, "allow");
        assert!(result.reason.is_none());
    }

    #[test]
    fn test_git_reset_hard_blocked() {
        let guard = CommandGuard::new();
        let result = guard.check("git reset --hard HEAD~1");
        assert_eq!(result.decision, "block");
    }

    #[test]
    fn test_git_restore_staged_allowed() {
        let guard = CommandGuard::new();
        let result = guard.check("git restore --staged file.txt");
        assert_eq!(result.decision, "allow");
    }

    #[test]
    fn test_rm_rf_blocked() {
        let guard = CommandGuard::new();
        let result = guard.check("rm -rf /home/user/project");
        assert_eq!(result.decision, "block");
    }

    #[test]
    fn test_rm_rf_tmp_allowed() {
        let guard = CommandGuard::new();
        let result = guard.check("rm -rf /tmp/test-dir");
        assert_eq!(result.decision, "allow");
    }

    #[test]
    fn test_git_push_force_blocked() {
        let guard = CommandGuard::new();
        let result = guard.check("git push --force origin main");
        assert_eq!(result.decision, "block");
    }

    #[test]
    fn test_git_push_force_with_lease_allowed() {
        let guard = CommandGuard::new();
        let result = guard.check("git push --force-with-lease origin main");
        assert_eq!(result.decision, "allow");
    }

    #[test]
    fn test_git_clean_blocked() {
        let guard = CommandGuard::new();
        let result = guard.check("git clean -fd");
        assert_eq!(result.decision, "block");
    }

    #[test]
    fn test_git_clean_dry_run_allowed() {
        let guard = CommandGuard::new();
        let result = guard.check("git clean -n");
        assert_eq!(result.decision, "allow");
    }

    #[test]
    fn test_git_stash_drop_blocked() {
        let guard = CommandGuard::new();
        let result = guard.check("git stash drop stash@{0}");
        assert_eq!(result.decision, "block");
    }

    #[test]
    fn test_git_status_allowed() {
        let guard = CommandGuard::new();
        let result = guard.check("git status");
        assert_eq!(result.decision, "allow");
    }

    #[test]
    fn test_normal_command_allowed() {
        let guard = CommandGuard::new();
        let result = guard.check("cargo build --release");
        assert_eq!(result.decision, "allow");
    }
}
