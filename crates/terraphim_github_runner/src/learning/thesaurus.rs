//! Command thesaurus for knowledge graph integration
//!
//! Provides:
//! - Atomic counter for node ID generation
//! - Pre-seeded command thesaurus with common CI/CD commands

use std::sync::atomic::{AtomicU64, Ordering};
use terraphim_types::{NormalizedTerm, NormalizedTermValue, Thesaurus};

/// Global atomic counter for generating unique command node IDs
static COMMAND_ID_SEQ: AtomicU64 = AtomicU64::new(1);

/// Generate a unique command node ID using atomic counter
pub fn get_command_id() -> u64 {
    COMMAND_ID_SEQ.fetch_add(1, Ordering::SeqCst)
}

/// Build a command thesaurus pre-seeded with common CI/CD commands
///
/// This thesaurus contains common build, test, and deployment commands
/// that are frequently encountered in GitHub workflows.
pub fn build_command_thesaurus() -> Thesaurus {
    let mut thesaurus = Thesaurus::new("github-commands".to_string());

    // Common cargo commands
    let cargo_commands = [
        "cargo build",
        "cargo test",
        "cargo clippy",
        "cargo fmt",
        "cargo check",
        "cargo run",
        "cargo doc",
        "cargo publish",
        "cargo bench",
        "cargo clean",
    ];

    // Common git commands
    let git_commands = [
        "git clone",
        "git checkout",
        "git pull",
        "git push",
        "git commit",
        "git merge",
        "git rebase",
        "git fetch",
        "git status",
        "git diff",
    ];

    // Common npm/yarn commands
    let node_commands = [
        "npm install",
        "npm run",
        "npm test",
        "npm build",
        "yarn install",
        "yarn run",
        "yarn test",
        "yarn build",
    ];

    // Common system commands
    let system_commands = [
        "apt-get install",
        "apt-get update",
        "pip install",
        "python",
        "make",
        "cmake",
        "docker build",
        "docker run",
        "docker push",
    ];

    // Add all commands to thesaurus
    for cmd in cargo_commands
        .iter()
        .chain(git_commands.iter())
        .chain(node_commands.iter())
        .chain(system_commands.iter())
    {
        let normalized = normalize_command(cmd);
        let id = get_command_id();
        let value = NormalizedTermValue::new(normalized.clone());
        let term = NormalizedTerm::new(id, value.clone());
        thesaurus.insert(value, term);
    }

    thesaurus
}

/// Normalize a command string for consistent matching
///
/// - Converts to lowercase
/// - Trims whitespace
/// - Extracts base command (first two words for compound commands)
pub fn normalize_command(command: &str) -> String {
    let trimmed = command.trim().to_lowercase();

    // Extract base command (e.g., "cargo build --release" -> "cargo build")
    let parts: Vec<&str> = trimmed.split_whitespace().collect();

    match parts.len() {
        0 => String::new(),
        1 => parts[0].to_string(),
        _ => format!("{} {}", parts[0], parts[1]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_command_id_increments() {
        // Don't reset - tests run in parallel, just verify incrementing behavior
        let id1 = get_command_id();
        let id2 = get_command_id();
        let id3 = get_command_id();

        // IDs should be unique and incrementing
        assert!(id2 > id1);
        assert!(id3 > id2);
        assert_ne!(id1, id2);
        assert_ne!(id2, id3);
    }

    #[test]
    fn test_normalize_command() {
        assert_eq!(normalize_command("cargo build"), "cargo build");
        assert_eq!(normalize_command("cargo build --release"), "cargo build");
        assert_eq!(normalize_command("  CARGO BUILD  "), "cargo build");
        assert_eq!(normalize_command("git"), "git");
        assert_eq!(normalize_command(""), "");
    }

    #[test]
    fn test_build_command_thesaurus() {
        // Don't reset counter - tests run in parallel
        let thesaurus = build_command_thesaurus();

        // Should have all pre-seeded commands
        assert!(thesaurus.len() > 30);

        // Check some specific commands exist using get with NormalizedTermValue
        assert!(
            thesaurus
                .get(&NormalizedTermValue::new("cargo build".to_string()))
                .is_some()
        );
        assert!(
            thesaurus
                .get(&NormalizedTermValue::new("git clone".to_string()))
                .is_some()
        );
        assert!(
            thesaurus
                .get(&NormalizedTermValue::new("npm install".to_string()))
                .is_some()
        );
        assert!(
            thesaurus
                .get(&NormalizedTermValue::new("docker build".to_string()))
                .is_some()
        );
    }
}
