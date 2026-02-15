//! Learning Capture System for Terraphim AI
//!
//! This module provides automatic capture and indexing of failed commands
//! as learning documents that can be queried via the knowledge graph.
//!
//! # Architecture
//!
//! ```text
//! Failed Command → Secret Redaction → Auto-Suggest → Store as Markdown
//!                                      ↓
//!                              RoleGraph Index
//!                                      ↓
//!                              Query with KG Expansion
//! ```
//!
//! # Usage
//!
//! ```rust,ignore
//! use terraphim_agent::learnings::{capture_failed_command, LearningCaptureConfig};
//!
//! let config = LearningCaptureConfig::default();
//! let path = capture_failed_command("git push -f", "remote: rejected", 1, &config)?;
//! println!("Captured learning: {:?}", path);
//! ```

mod capture;
mod redaction;

pub use capture::{capture_failed_command, list_learnings, query_learnings, LearningSource};

// Re-export for testing - not used by CLI yet
#[allow(unused_imports)]
pub use capture::{CapturedLearning, LearningContext, LearningError};

#[allow(unused_imports)]
pub use redaction::redact_secrets;

use std::path::PathBuf;

/// Configuration for learning capture.
#[derive(Debug, Clone)]
pub struct LearningCaptureConfig {
    /// Project-specific learnings directory
    pub project_dir: PathBuf,
    /// Global fallback directory
    pub global_dir: PathBuf,
    /// Enable capture
    pub enabled: bool,
    /// Patterns to ignore (glob-style)
    pub ignore_patterns: Vec<String>,
}

impl Default for LearningCaptureConfig {
    fn default() -> Self {
        let project_dir = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(".terraphim")
            .join("learnings");

        let global_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("~/.local/share"))
            .join("terraphim")
            .join("learnings");

        Self {
            project_dir,
            global_dir,
            enabled: true,
            ignore_patterns: vec![
                "cargo test*".to_string(),
                "npm test*".to_string(),
                "pytest*".to_string(),
                "yarn test*".to_string(),
            ],
        }
    }
}

impl LearningCaptureConfig {
    /// Create config with custom directories
    #[allow(dead_code)]
    pub fn new(project_dir: PathBuf, global_dir: PathBuf) -> Self {
        Self {
            project_dir,
            global_dir,
            ..Default::default()
        }
    }

    /// Determine storage location based on availability
    pub fn storage_location(&self) -> PathBuf {
        if self.project_dir.exists()
            || self
                .project_dir
                .parent()
                .map(|p| p.exists())
                .unwrap_or(false)
        {
            self.project_dir.clone()
        } else {
            self.global_dir.clone()
        }
    }

    /// Check if a command should be ignored based on patterns
    pub fn should_ignore(&self, command: &str) -> bool {
        for pattern in &self.ignore_patterns {
            if let Ok(glob) = glob::Pattern::new(pattern) {
                if glob.matches(command) {
                    return true;
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = LearningCaptureConfig::default();
        assert!(config.enabled);
        assert!(!config.ignore_patterns.is_empty());
    }

    #[test]
    fn test_should_ignore_test_commands() {
        let config = LearningCaptureConfig::default();
        assert!(config.should_ignore("cargo test"));
        assert!(config.should_ignore("cargo test --lib"));
        assert!(config.should_ignore("npm test"));
        assert!(config.should_ignore("pytest tests/"));
        assert!(!config.should_ignore("cargo build"));
        assert!(!config.should_ignore("git push"));
    }

    #[test]
    fn test_storage_location_prefers_project() {
        let config = LearningCaptureConfig::default();
        // If project dir's parent exists, prefer project
        let location = config.storage_location();
        assert!(location.ends_with("learnings"));
    }
}
