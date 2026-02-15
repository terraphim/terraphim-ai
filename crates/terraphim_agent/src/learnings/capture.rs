//! Learning capture logic for failed commands.
//!
//! This module provides the core functionality to capture failed commands
//! as learning documents, including auto-suggesting corrections from the
//! knowledge graph.

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::learnings::LearningCaptureConfig;
use crate::learnings::redaction::redact_secrets;

/// Errors that can occur during learning capture.
#[derive(Error, Debug)]
pub enum LearningError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Storage directory not accessible: {0}")]
    StorageError(String),
    #[error("Command ignored due to pattern match: {0}")]
    Ignored(String),
}

/// Source of the learning (project-specific or global).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LearningSource {
    /// Captured from a specific project
    Project,
    /// Captured globally (fallback)
    Global,
}

/// Context information for a captured learning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningContext {
    /// Working directory where command was executed
    pub working_dir: String,
    /// Timestamp of capture
    pub captured_at: DateTime<Utc>,
    /// Hostname (optional)
    pub hostname: Option<String>,
    /// User who executed the command
    pub user: Option<String>,
}

impl Default for LearningContext {
    fn default() -> Self {
        Self {
            working_dir: std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| "unknown".to_string()),
            captured_at: Utc::now(),
            hostname: std::env::var("HOSTNAME").ok(),
            user: std::env::var("USER").ok(),
        }
    }
}

/// A captured learning from a failed command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapturedLearning {
    /// Unique ID (UUID)
    pub id: String,
    /// Original command that failed
    pub command: String,
    /// For chained commands, the specific failing sub-command
    pub failing_subcommand: Option<String>,
    /// Full command chain (if chained)
    pub full_chain: Option<String>,
    /// Error output (redacted)
    pub error_output: String,
    /// Exit code
    pub exit_code: i32,
    /// Suggested correction (if auto-suggested from KG)
    pub correction: Option<String>,
    /// Source: project or global
    pub source: LearningSource,
    /// Context
    pub context: LearningContext,
    /// Tags for categorization
    pub tags: Vec<String>,
}

impl CapturedLearning {
    /// Create a new captured learning.
    pub fn new(
        command: String,
        error_output: String,
        exit_code: i32,
        source: LearningSource,
    ) -> Self {
        let id = format!("{}-{}", Uuid::new_v4().simple(), timestamp_millis());

        Self {
            id,
            command,
            failing_subcommand: None,
            full_chain: None,
            error_output,
            exit_code,
            correction: None,
            source,
            context: LearningContext::default(),
            tags: Vec::new(),
        }
    }

    /// Set the failing subcommand for chained commands.
    pub fn with_failing_subcommand(mut self, subcommand: String, full_chain: String) -> Self {
        self.failing_subcommand = Some(subcommand);
        self.full_chain = Some(full_chain);
        self
    }

    /// Set a suggested correction.
    #[allow(dead_code)]
    pub fn with_correction(mut self, correction: String) -> Self {
        self.correction = Some(correction);
        self
    }

    /// Add tags.
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Convert to markdown format for storage.
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();

        // Frontmatter
        md.push_str("---\n");
        md.push_str(&format!("id: {}\n", self.id));
        md.push_str(&format!("command: {}\n", self.command));
        md.push_str(&format!("exit_code: {}\n", self.exit_code));
        md.push_str(&format!("source: {:?}\n", self.source));
        md.push_str(&format!(
            "captured_at: {}\n",
            self.context.captured_at.to_rfc3339()
        ));
        md.push_str(&format!("working_dir: {}\n", self.context.working_dir));

        if let Some(ref hostname) = self.context.hostname {
            md.push_str(&format!("hostname: {}\n", hostname));
        }

        if let Some(ref subcommand) = self.failing_subcommand {
            md.push_str(&format!("failing_subcommand: {}\n", subcommand));
        }

        if let Some(ref correction) = self.correction {
            md.push_str(&format!("correction: {}\n", correction));
        }

        if !self.tags.is_empty() {
            md.push_str("tags:\n");
            for tag in &self.tags {
                md.push_str(&format!("  - {}\n", tag));
            }
        }

        md.push_str("---\n\n");

        // Body
        md.push_str(&format!("## Command\n\n`{}`\n\n", self.command));

        if let Some(ref full_chain) = self.full_chain {
            md.push_str(&format!("### Full Chain\n\n`{}`\n\n", full_chain));
        }

        md.push_str("## Error Output\n\n");
        md.push_str("```\n");
        md.push_str(&self.error_output);
        md.push_str("\n```\n\n");

        if let Some(ref correction) = self.correction {
            md.push_str("## Suggested Correction\n\n");
            md.push_str(&format!("`{}`\n\n", correction));
        }

        md
    }

    /// Parse from markdown file content.
    pub fn from_markdown(content: &str) -> Option<Self> {
        // Simple YAML frontmatter parsing
        let parts: Vec<&str> = content.splitn(3, "---").collect();
        if parts.len() < 3 {
            return None;
        }

        let frontmatter = parts[1].trim();
        let body = parts[2].trim();

        // Parse frontmatter (simple key: value extraction)
        let mut id = String::new();
        let mut command = String::new();
        let mut exit_code = 1i32;
        let mut source = LearningSource::Project;
        let mut captured_at = Utc::now();
        let mut working_dir = String::new();
        let mut hostname = None;
        let mut failing_subcommand = None;
        let full_chain = None;
        let mut correction = None;
        let mut error_output = String::new();
        let tags = Vec::new();

        for line in frontmatter.lines() {
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim();
                let value = value.trim();

                match key {
                    "id" => id = value.to_string(),
                    "command" => command = value.to_string(),
                    "exit_code" => exit_code = value.parse().unwrap_or(1),
                    "source" => {
                        source = if value == "Project" {
                            LearningSource::Project
                        } else {
                            LearningSource::Global
                        }
                    }
                    "captured_at" => {
                        captured_at = DateTime::parse_from_rfc3339(value)
                            .map(|d| d.with_timezone(&Utc))
                            .unwrap_or_else(|_| Utc::now())
                    }
                    "working_dir" => working_dir = value.to_string(),
                    "hostname" => hostname = Some(value.to_string()),
                    "failing_subcommand" => failing_subcommand = Some(value.to_string()),
                    "correction" => correction = Some(value.to_string()),
                    _ => {}
                }
            }
        }

        // Extract error output from code block
        if let Some(start) = body.find("```\n") {
            let after_start = &body[start + 4..];
            if let Some(end) = after_start.find("\n```") {
                error_output = after_start[..end].to_string();
            }
        }

        Some(Self {
            id,
            command,
            failing_subcommand,
            full_chain,
            error_output,
            exit_code,
            correction,
            source,
            context: LearningContext {
                working_dir,
                captured_at,
                hostname,
                user: None,
            },
            tags,
        })
    }
}

/// Capture a failed command as a learning document.
///
/// This function:
/// 1. Checks if the command should be ignored
/// 2. Redacts secrets from error output
/// 3. Auto-suggests correction from existing learnings (optional)
/// 4. Writes to storage location
///
/// # Arguments
///
/// * `command` - The command that failed
/// * `error_output` - The error output (stderr)
/// * `exit_code` - The exit code
/// * `config` - Learning capture configuration
///
/// # Returns
///
/// Path to the saved learning file, or error if capture failed.
pub fn capture_failed_command(
    command: &str,
    error_output: &str,
    exit_code: i32,
    config: &LearningCaptureConfig,
) -> Result<PathBuf, LearningError> {
    // Check if capture is enabled
    if !config.enabled {
        return Err(LearningError::Ignored("Capture disabled".to_string()));
    }

    // Check if command should be ignored
    if config.should_ignore(command) {
        return Err(LearningError::Ignored(command.to_string()));
    }

    // Parse chained commands
    let (actual_command, full_chain) = parse_chained_command(command, exit_code);

    // Redact secrets
    let redacted_error = redact_secrets(error_output);

    // Determine storage location
    let storage_dir = config.storage_location();

    // Create storage directory if it doesn't exist
    fs::create_dir_all(&storage_dir)
        .map_err(|e| LearningError::StorageError(format!("Cannot create storage dir: {}", e)))?;

    // Determine source
    let source = if storage_dir == config.project_dir {
        LearningSource::Project
    } else {
        LearningSource::Global
    };

    // Create learning
    let mut learning =
        CapturedLearning::new(actual_command.clone(), redacted_error, exit_code, source);

    // Set full chain if different from actual command
    if let Some(ref chain) = full_chain {
        if *chain != actual_command {
            learning = learning.with_failing_subcommand(actual_command, chain.clone());
        }
    }

    // Auto-suggest correction (future: query existing learnings)
    // For now, this is a placeholder for the auto-suggest feature
    // TODO: Implement auto-suggest using terraphim_automata::find_matches

    // Add default tags
    let tags = vec!["learning".to_string(), format!("exit-{}", exit_code)];
    learning = learning.with_tags(tags);

    // Generate filename
    let filename = format!("learning-{}.md", learning.id);
    let filepath = storage_dir.join(&filename);

    // Write to file
    fs::write(&filepath, learning.to_markdown())?;

    log::info!("Captured learning: {} ({})", filepath.display(), command);

    Ok(filepath)
}

/// Parse a chained command to find the failing subcommand.
///
/// For commands like `cmd1 && cmd2 || cmd3`, attempts to determine
/// which subcommand failed based on the chain structure.
///
/// Returns (actual_command, full_chain_option)
fn parse_chained_command(command: &str, _exit_code: i32) -> (String, Option<String>) {
    // Check for simple chains
    let chain_operators = [" && ", " || ", "; "];

    for op in &chain_operators {
        if command.contains(op) {
            // Split by the operator
            let parts: Vec<&str> = command.split(op).collect();
            if parts.len() > 1 {
                // For now, return the first part as the failing command
                // In a more sophisticated implementation, we would track
                // which command actually failed based on execution order
                return (parts[0].trim().to_string(), Some(command.to_string()));
            }
        }
    }

    // No chain detected
    (command.trim().to_string(), None)
}

/// Get current timestamp in milliseconds.
fn timestamp_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// List recent learnings from storage.
pub fn list_learnings(
    storage_dir: &PathBuf,
    limit: usize,
) -> Result<Vec<CapturedLearning>, LearningError> {
    let mut learnings = Vec::new();

    if !storage_dir.exists() {
        return Ok(learnings);
    }

    let entries = fs::read_dir(storage_dir)?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map(|e| e == "md").unwrap_or(false) {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Some(learning) = CapturedLearning::from_markdown(&content) {
                    learnings.push(learning);
                }
            }
        }
    }

    // Sort by captured_at descending (most recent first)
    learnings.sort_by(|a, b| b.context.captured_at.cmp(&a.context.captured_at));

    // Limit results
    if learnings.len() > limit {
        learnings.truncate(limit);
    }

    Ok(learnings)
}

/// Query learnings by pattern (simple text search).
pub fn query_learnings(
    storage_dir: &PathBuf,
    pattern: &str,
    exact: bool,
) -> Result<Vec<CapturedLearning>, LearningError> {
    let all = list_learnings(storage_dir, usize::MAX)?;

    let filtered: Vec<_> = all
        .into_iter()
        .filter(|l| {
            if exact {
                l.command == pattern || l.error_output.contains(pattern)
            } else {
                l.command.to_lowercase().contains(&pattern.to_lowercase())
                    || l.error_output
                        .to_lowercase()
                        .contains(&pattern.to_lowercase())
            }
        })
        .collect();

    Ok(filtered)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_captured_learning_to_markdown() {
        let learning = CapturedLearning::new(
            "git push -f".to_string(),
            "remote: rejected".to_string(),
            1,
            LearningSource::Project,
        )
        .with_correction("git push".to_string());

        let md = learning.to_markdown();
        assert!(md.contains("git push -f"));
        assert!(md.contains("remote: rejected"));
        assert!(md.contains("correction:"));
        assert!(md.contains("git push"));
        assert!(md.contains("id:")); // Check that ID field is present
    }

    #[test]
    fn test_captured_learning_roundtrip() {
        let original = CapturedLearning::new(
            "npm install".to_string(),
            "EACCES: permission denied".to_string(),
            1,
            LearningSource::Global,
        );

        let md = original.to_markdown();
        let parsed = CapturedLearning::from_markdown(&md).unwrap();

        assert_eq!(parsed.command, original.command);
        assert_eq!(parsed.exit_code, original.exit_code);
        assert_eq!(parsed.error_output.trim(), original.error_output);
    }

    #[test]
    fn test_capture_failed_command() {
        let temp_dir = TempDir::new().unwrap();
        let config = LearningCaptureConfig::new(
            temp_dir.path().join("learnings"),
            temp_dir.path().join("global"),
        );

        let result =
            capture_failed_command("git status", "fatal: not a git repository", 128, &config);

        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.exists());

        // Verify content
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("git status"));
        assert!(content.contains("not a git repository"));
    }

    #[test]
    fn test_capture_ignores_test_commands() {
        let temp_dir = TempDir::new().unwrap();
        let config = LearningCaptureConfig::new(
            temp_dir.path().join("learnings"),
            temp_dir.path().join("global"),
        );

        let result = capture_failed_command("cargo test", "test failed", 1, &config);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), LearningError::Ignored(_)));
    }

    #[test]
    fn test_parse_chained_command() {
        let (cmd, chain) = parse_chained_command("docker build . && docker run", 1);
        assert_eq!(cmd, "docker build .");
        assert_eq!(chain, Some("docker build . && docker run".to_string()));

        let (cmd2, chain2) = parse_chained_command("git status", 0);
        assert_eq!(cmd2, "git status");
        assert_eq!(chain2, None);
    }

    #[test]
    fn test_list_learnings() {
        let temp_dir = TempDir::new().unwrap();
        let storage = temp_dir.path().join("learnings");
        fs::create_dir(&storage).unwrap();

        // Create a test learning file
        let learning = CapturedLearning::new(
            "test cmd".to_string(),
            "error".to_string(),
            1,
            LearningSource::Project,
        );
        fs::write(storage.join("test.md"), learning.to_markdown()).unwrap();

        let result = list_learnings(&storage, 10).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].command, "test cmd");
    }
}
