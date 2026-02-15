//! Guard patterns for blocking destructive git and filesystem commands.
//!
//! This module uses terraphim's Aho-Corasick thesaurus matching to detect
//! destructive commands. Patterns are defined in JSON thesaurus files where
//! command variants (synonyms) map to concept categories via `nterm`, and
//! the `url` field carries the human-readable block reason.

use serde::{Deserialize, Serialize};
use terraphim_automata::{find_matches, load_thesaurus_from_json};
use terraphim_types::Thesaurus;

/// Default destructive patterns thesaurus (embedded at compile time)
const DEFAULT_DESTRUCTIVE_JSON: &str = include_str!("../data/guard_destructive.json");

/// Default allowlist thesaurus (embedded at compile time)
const DEFAULT_ALLOWLIST_JSON: &str = include_str!("../data/guard_allowlist.json");

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

/// Guard that checks commands against destructive patterns using terraphim
/// thesaurus-driven Aho-Corasick matching.
pub struct CommandGuard {
    destructive_thesaurus: Thesaurus,
    allowlist_thesaurus: Thesaurus,
}

impl Default for CommandGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandGuard {
    /// Create a new command guard with default embedded thesauruses
    pub fn new() -> Self {
        let destructive_thesaurus = load_thesaurus_from_json(DEFAULT_DESTRUCTIVE_JSON)
            .expect("Failed to load embedded guard_destructive.json");
        let allowlist_thesaurus = load_thesaurus_from_json(DEFAULT_ALLOWLIST_JSON)
            .expect("Failed to load embedded guard_allowlist.json");

        Self {
            destructive_thesaurus,
            allowlist_thesaurus,
        }
    }

    /// Get the default embedded destructive patterns JSON string
    pub fn default_destructive_json() -> &'static str {
        DEFAULT_DESTRUCTIVE_JSON
    }

    /// Get the default embedded allowlist JSON string
    pub fn default_allowlist_json() -> &'static str {
        DEFAULT_ALLOWLIST_JSON
    }

    /// Create a command guard with custom thesaurus JSON strings
    pub fn from_json(destructive_json: &str, allowlist_json: &str) -> Result<Self, String> {
        let destructive_thesaurus =
            load_thesaurus_from_json(destructive_json).map_err(|e| e.to_string())?;
        let allowlist_thesaurus =
            load_thesaurus_from_json(allowlist_json).map_err(|e| e.to_string())?;

        Ok(Self {
            destructive_thesaurus,
            allowlist_thesaurus,
        })
    }

    /// Check a command against guard patterns
    ///
    /// Returns a GuardResult indicating whether the command should be allowed or blocked.
    /// Priority: allowlist first, then destructive check, then default allow.
    pub fn check(&self, command: &str) -> GuardResult {
        // Check allowlist first -- if any safe pattern matches, allow immediately
        match find_matches(command, self.allowlist_thesaurus.clone(), false) {
            Ok(matches) if !matches.is_empty() => {
                return GuardResult::allow(command.to_string());
            }
            Ok(_) => {}  // no allowlist match, continue
            Err(_) => {} // fail open on error
        }

        // Check destructive patterns
        match find_matches(command, self.destructive_thesaurus.clone(), false) {
            Ok(matches) if !matches.is_empty() => {
                // Use the first match (LeftmostLongest gives the best match)
                let first_match = &matches[0];
                let reason = first_match.normalized_term.url.clone().unwrap_or_else(|| {
                    format!(
                        "Blocked: matched destructive pattern '{}'",
                        first_match.term
                    )
                });
                let pattern = first_match.term.clone();
                return GuardResult::block(command.to_string(), reason, pattern);
            }
            Ok(_) => {}  // no destructive match
            Err(_) => {} // fail open on error
        }

        // No match -- allow
        GuardResult::allow(command.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // === Existing tests (must all pass) ===

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

    // === New tests for newly covered commands ===

    #[test]
    fn test_rmdir_blocked() {
        let guard = CommandGuard::new();
        let result = guard.check("rmdir /Users/alex/important-dir");
        assert_eq!(result.decision, "block");
        assert!(result.reason.is_some());
    }

    #[test]
    fn test_chmod_blocked() {
        let guard = CommandGuard::new();
        let result = guard.check("chmod +x /usr/local/bin/script.sh");
        assert_eq!(result.decision, "block");
    }

    #[test]
    fn test_chown_blocked() {
        let guard = CommandGuard::new();
        let result = guard.check("chown root:root /etc/passwd");
        assert_eq!(result.decision, "block");
    }

    #[test]
    fn test_git_commit_no_verify_blocked() {
        let guard = CommandGuard::new();
        let result = guard.check("git commit --no-verify -m 'skip hooks'");
        assert_eq!(result.decision, "block");
    }

    #[test]
    fn test_git_push_no_verify_blocked() {
        let guard = CommandGuard::new();
        let result = guard.check("git push --no-verify origin main");
        assert_eq!(result.decision, "block");
    }

    #[test]
    fn test_shred_blocked() {
        let guard = CommandGuard::new();
        let result = guard.check("shred -vfz /home/user/secret.txt");
        assert_eq!(result.decision, "block");
    }

    #[test]
    fn test_truncate_blocked() {
        let guard = CommandGuard::new();
        let result = guard.check("truncate -s 0 /var/log/syslog");
        assert_eq!(result.decision, "block");
    }

    #[test]
    fn test_dd_blocked() {
        let guard = CommandGuard::new();
        let result = guard.check("dd if=/dev/zero of=/dev/sda bs=1M");
        assert_eq!(result.decision, "block");
    }

    #[test]
    fn test_mkfs_blocked() {
        let guard = CommandGuard::new();
        let result = guard.check("mkfs.ext4 /dev/sda1");
        assert_eq!(result.decision, "block");
    }

    #[test]
    fn test_rm_fr_blocked() {
        let guard = CommandGuard::new();
        let result = guard.check("rm -fr /home/user/project");
        assert_eq!(result.decision, "block");
    }

    #[test]
    fn test_git_stash_clear_blocked() {
        let guard = CommandGuard::new();
        let result = guard.check("git stash clear");
        assert_eq!(result.decision, "block");
    }

    #[test]
    fn test_git_reset_merge_blocked() {
        let guard = CommandGuard::new();
        let result = guard.check("git reset --merge");
        assert_eq!(result.decision, "block");
    }

    #[test]
    fn test_git_restore_worktree_blocked() {
        let guard = CommandGuard::new();
        let result = guard.check("git restore --worktree file.txt");
        assert_eq!(result.decision, "block");
    }

    #[test]
    fn test_git_checkout_orphan_allowed() {
        let guard = CommandGuard::new();
        let result = guard.check("git checkout --orphan new-root");
        assert_eq!(result.decision, "allow");
    }

    #[test]
    fn test_git_clean_dry_run_long_allowed() {
        let guard = CommandGuard::new();
        let result = guard.check("git clean --dry-run");
        assert_eq!(result.decision, "allow");
    }

    #[test]
    fn test_fdisk_blocked() {
        let guard = CommandGuard::new();
        let result = guard.check("fdisk /dev/sda");
        assert_eq!(result.decision, "block");
    }

    #[test]
    fn test_git_branch_force_delete_blocked() {
        let guard = CommandGuard::new();
        let result = guard.check("git branch -D old-branch");
        assert_eq!(result.decision, "block");
    }

    // === Structural tests ===

    #[test]
    fn test_custom_thesaurus() {
        let destructive = r#"{
            "name": "custom_destructive",
            "data": {
                "dangerous-cmd": {
                    "id": 1,
                    "nterm": "test_dangerous",
                    "url": "This is a test block reason"
                }
            }
        }"#;
        let allowlist = r#"{
            "name": "custom_allowlist",
            "data": {
                "safe-cmd": {
                    "id": 1,
                    "nterm": "test_safe",
                    "url": "This is safe"
                }
            }
        }"#;

        let guard = CommandGuard::from_json(destructive, allowlist).unwrap();

        let result = guard.check("run dangerous-cmd now");
        assert_eq!(result.decision, "block");
        assert_eq!(result.reason.unwrap(), "This is a test block reason");

        let result = guard.check("run safe-cmd now");
        assert_eq!(result.decision, "allow");

        let result = guard.check("run normal-cmd");
        assert_eq!(result.decision, "allow");
    }

    #[test]
    fn test_guard_json_output_format() {
        let guard = CommandGuard::new();
        let result = guard.check("git reset --hard HEAD");
        let json = serde_json::to_string(&result).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["decision"], "block");
        assert!(parsed["reason"].is_string());
        assert_eq!(parsed["command"], "git reset --hard HEAD");
        assert!(parsed["pattern"].is_string());
    }

    #[test]
    fn test_allow_result_json_format() {
        let guard = CommandGuard::new();
        let result = guard.check("git status");
        let json = serde_json::to_string(&result).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["decision"], "allow");
        // reason and pattern should not be present (skip_serializing_if)
        assert!(parsed.get("reason").is_none());
        assert!(parsed.get("pattern").is_none());
    }

    #[test]
    fn test_thesaurus_load_from_embedded() {
        // Verify the embedded JSON files parse without error
        let _guard = CommandGuard::new();
    }

    #[test]
    fn test_rm_rf_var_tmp_allowed() {
        let guard = CommandGuard::new();
        let result = guard.check("rm -rf /var/tmp/build-cache");
        assert_eq!(result.decision, "allow");
    }

    #[test]
    fn test_rm_fr_tmp_allowed() {
        let guard = CommandGuard::new();
        let result = guard.check("rm -fr /tmp/test-output");
        assert_eq!(result.decision, "allow");
    }
}
