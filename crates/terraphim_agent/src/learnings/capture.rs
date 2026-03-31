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
    #[error("Learning not found: {0}")]
    NotFound(String),
}

/// Source of the learning (project-specific or global).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LearningSource {
    /// Captured from a specific project
    Project,
    /// Captured globally (fallback)
    Global,
}

/// Type of user correction captured during an agent session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CorrectionType {
    /// "use bun instead of npm"
    ToolPreference,
    /// "we use Result<T> not unwrap()"
    CodePattern,
    /// "we call it X not Y"
    Naming,
    /// "always run tests before committing"
    WorkflowStep,
    /// "the endpoint is /api/v2 not /api/v1"
    FactCorrection,
    /// "use British English"
    StylePreference,
    /// Catchall
    Other(String),
}

impl std::fmt::Display for CorrectionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CorrectionType::ToolPreference => write!(f, "tool-preference"),
            CorrectionType::CodePattern => write!(f, "code-pattern"),
            CorrectionType::Naming => write!(f, "naming"),
            CorrectionType::WorkflowStep => write!(f, "workflow-step"),
            CorrectionType::FactCorrection => write!(f, "fact-correction"),
            CorrectionType::StylePreference => write!(f, "style-preference"),
            CorrectionType::Other(s) => write!(f, "other:{}", s),
        }
    }
}

impl std::str::FromStr for CorrectionType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "tool-preference" => Ok(CorrectionType::ToolPreference),
            "code-pattern" => Ok(CorrectionType::CodePattern),
            "naming" => Ok(CorrectionType::Naming),
            "workflow-step" => Ok(CorrectionType::WorkflowStep),
            "fact-correction" => Ok(CorrectionType::FactCorrection),
            "style-preference" => Ok(CorrectionType::StylePreference),
            other => {
                if let Some(suffix) = other.strip_prefix("other:") {
                    Ok(CorrectionType::Other(suffix.to_string()))
                } else {
                    Ok(CorrectionType::Other(other.to_string()))
                }
            }
        }
    }
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

/// A user correction captured during an agent session.
/// Unlike CapturedLearning (which captures failed commands),
/// CorrectionEvent captures any user feedback: preferences,
/// naming conventions, workflow steps, fact corrections.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrectionEvent {
    /// Unique ID (UUID-timestamp, same format as CapturedLearning)
    pub id: String,
    /// Type of correction
    pub correction_type: CorrectionType,
    /// What the agent said/did originally
    pub original: String,
    /// What the user said instead
    pub corrected: String,
    /// Surrounding context (conversation snippet, file path, etc.)
    pub context_description: String,
    /// Source: project or global
    pub source: LearningSource,
    /// Context metadata
    pub context: LearningContext,
    /// Session ID for traceability
    pub session_id: Option<String>,
    /// Tags for categorisation
    pub tags: Vec<String>,
}

impl CorrectionEvent {
    /// Create a new correction event.
    pub fn new(
        correction_type: CorrectionType,
        original: String,
        corrected: String,
        context_description: String,
        source: LearningSource,
    ) -> Self {
        let id = format!("{}-{}", Uuid::new_v4().simple(), timestamp_millis());
        Self {
            id,
            correction_type,
            original,
            corrected,
            context_description,
            source,
            context: LearningContext::default(),
            session_id: None,
            tags: Vec::new(),
        }
    }

    /// Set session ID.
    #[allow(dead_code)]
    pub fn with_session_id(mut self, session_id: String) -> Self {
        self.session_id = Some(session_id);
        self
    }

    /// Add tags.
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Convert to markdown format for storage.
    /// Uses same YAML frontmatter pattern as CapturedLearning.
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();

        // Frontmatter
        md.push_str("---\n");
        md.push_str(&format!("id: {}\n", self.id));
        md.push_str("type: correction\n");
        md.push_str(&format!("correction_type: {}\n", self.correction_type));
        md.push_str(&format!("source: {:?}\n", self.source));
        md.push_str(&format!(
            "captured_at: {}\n",
            self.context.captured_at.to_rfc3339()
        ));
        md.push_str(&format!("working_dir: {}\n", self.context.working_dir));

        if let Some(ref hostname) = self.context.hostname {
            md.push_str(&format!("hostname: {}\n", hostname));
        }

        if let Some(ref session_id) = self.session_id {
            md.push_str(&format!("session_id: {}\n", session_id));
        }

        if !self.tags.is_empty() {
            md.push_str("tags:\n");
            for tag in &self.tags {
                md.push_str(&format!("  - {}\n", tag));
            }
        }

        md.push_str("---\n\n");

        // Body
        md.push_str("## Original\n\n");
        md.push_str(&format!("`{}`\n\n", self.original));

        md.push_str("## Corrected\n\n");
        md.push_str(&format!("`{}`\n\n", self.corrected));

        if !self.context_description.is_empty() {
            md.push_str("## Context\n\n");
            md.push_str(&self.context_description);
            md.push_str("\n\n");
        }

        md
    }

    /// Parse from markdown file content.
    pub fn from_markdown(content: &str) -> Option<Self> {
        let parts: Vec<&str> = content.splitn(3, "---").collect();
        if parts.len() < 3 {
            return None;
        }

        let frontmatter = parts[1].trim();
        let body = parts[2].trim();

        let mut id = String::new();
        let mut correction_type = CorrectionType::Other("unknown".to_string());
        let mut source = LearningSource::Project;
        let mut captured_at = Utc::now();
        let mut working_dir = String::new();
        let mut hostname = None;
        let mut session_id = None;
        let mut file_type = String::new();
        let tags = Vec::new();

        for line in frontmatter.lines() {
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim();
                let value = value.trim();
                match key {
                    "id" => id = value.to_string(),
                    "type" => file_type = value.to_string(),
                    "correction_type" => {
                        correction_type = value
                            .parse()
                            .unwrap_or(CorrectionType::Other("unknown".to_string()));
                    }
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
                    "session_id" => session_id = Some(value.to_string()),
                    _ => {}
                }
            }
        }

        // Must be a correction file
        if file_type != "correction" {
            return None;
        }

        // Extract original and corrected from body
        let original = extract_code_after_heading(body, "## Original");
        let corrected = extract_code_after_heading(body, "## Corrected");
        let context_description = extract_section_text(body, "## Context");

        Some(Self {
            id,
            correction_type,
            original: original.unwrap_or_default(),
            corrected: corrected.unwrap_or_default(),
            context_description: context_description.unwrap_or_default(),
            source,
            context: LearningContext {
                working_dir,
                captured_at,
                hostname,
                user: None,
            },
            session_id,
            tags,
        })
    }
}

/// Extract inline code after a markdown heading.
fn extract_code_after_heading(body: &str, heading: &str) -> Option<String> {
    let idx = body.find(heading)?;
    let after = &body[idx + heading.len()..];
    // Find the first backtick-delimited code
    let start = after.find('`')? + 1;
    let rest = &after[start..];
    let end = rest.find('`')?;
    Some(rest[..end].to_string())
}

/// Extract plain text section after a heading (up to next heading or EOF).
fn extract_section_text(body: &str, heading: &str) -> Option<String> {
    let idx = body.find(heading)?;
    let after = &body[idx + heading.len()..].trim_start();
    // Find next heading or end
    let end = after.find("\n## ").unwrap_or(after.len());
    let text = after[..end].trim().to_string();
    if text.is_empty() { None } else { Some(text) }
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

/// Capture a user correction as a learning document.
///
/// # Arguments
///
/// * `correction_type` - Type of correction
/// * `original` - What the agent said/did
/// * `corrected` - What the user said instead
/// * `context_description` - Surrounding context
/// * `config` - Learning capture configuration
///
/// # Returns
///
/// Path to the saved correction file.
pub fn capture_correction(
    correction_type: CorrectionType,
    original: &str,
    corrected: &str,
    context_description: &str,
    config: &LearningCaptureConfig,
) -> Result<PathBuf, LearningError> {
    if !config.enabled {
        return Err(LearningError::Ignored("Capture disabled".to_string()));
    }

    // Redact secrets from all text fields
    let redacted_original = redact_secrets(original);
    let redacted_corrected = redact_secrets(corrected);
    let redacted_context = redact_secrets(context_description);

    let storage_dir = config.storage_location();
    fs::create_dir_all(&storage_dir)
        .map_err(|e| LearningError::StorageError(format!("Cannot create storage dir: {}", e)))?;

    let source = if storage_dir == config.project_dir {
        LearningSource::Project
    } else {
        LearningSource::Global
    };

    let correction = CorrectionEvent::new(
        correction_type.clone(),
        redacted_original,
        redacted_corrected,
        redacted_context,
        source,
    )
    .with_tags(vec![
        "correction".to_string(),
        format!("type:{}", correction_type),
    ]);

    let filename = format!("correction-{}.md", correction.id);
    let filepath = storage_dir.join(&filename);
    fs::write(&filepath, correction.to_markdown())?;

    log::info!("Captured correction: {}", filepath.display());
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
#[allow(dead_code)]
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

#[allow(dead_code)]
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

/// Add a correction to an existing learning document.
///
/// Finds the learning by exact ID or prefix match, updates the correction
/// field, and overwrites the markdown file.
pub fn correct_learning(
    storage_dir: &PathBuf,
    id: &str,
    correction: &str,
) -> Result<PathBuf, LearningError> {
    if !storage_dir.exists() {
        return Err(LearningError::NotFound(id.to_string()));
    }

    let entries = fs::read_dir(storage_dir)?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map(|e| e == "md").unwrap_or(false) {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Some(mut learning) = CapturedLearning::from_markdown(&content) {
                    if learning.id == id || learning.id.starts_with(id) {
                        learning.correction = Some(correction.to_string());
                        fs::write(&path, learning.to_markdown())?;
                        return Ok(path);
                    }
                }
            }
        }
    }

    Err(LearningError::NotFound(id.to_string()))
}

/// Unified learning entry for display (learning or correction).
#[derive(Debug, Clone)]
pub enum LearningEntry {
    Learning(CapturedLearning),
    Correction(CorrectionEvent),
}

impl LearningEntry {
    pub fn source(&self) -> &LearningSource {
        match self {
            LearningEntry::Learning(l) => &l.source,
            LearningEntry::Correction(c) => &c.source,
        }
    }

    #[allow(dead_code)]
    pub fn id(&self) -> &str {
        match self {
            LearningEntry::Learning(l) => &l.id,
            LearningEntry::Correction(c) => &c.id,
        }
    }

    pub fn captured_at(&self) -> chrono::DateTime<chrono::Utc> {
        match self {
            LearningEntry::Learning(l) => l.context.captured_at,
            LearningEntry::Correction(c) => c.context.captured_at,
        }
    }

    /// Summary line for display.
    pub fn summary(&self) -> String {
        match self {
            LearningEntry::Learning(l) => {
                format!("[cmd] {} (exit: {})", l.command, l.exit_code)
            }
            LearningEntry::Correction(c) => {
                format!("[{}] {} -> {}", c.correction_type, c.original, c.corrected)
            }
        }
    }

    /// Correction text if any.
    pub fn correction_text(&self) -> Option<&str> {
        match self {
            LearningEntry::Learning(l) => l.correction.as_deref(),
            LearningEntry::Correction(c) => Some(&c.corrected),
        }
    }
}

/// List all entries (learnings + corrections) from storage.
pub fn list_all_entries(
    storage_dir: &PathBuf,
    limit: usize,
) -> Result<Vec<LearningEntry>, LearningError> {
    let mut entries = Vec::new();

    if !storage_dir.exists() {
        return Ok(entries);
    }

    for entry in fs::read_dir(storage_dir)?.flatten() {
        let path = entry.path();
        if path.extension().map(|e| e == "md").unwrap_or(false) {
            if let Ok(content) = fs::read_to_string(&path) {
                let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if filename.starts_with("correction-") {
                    if let Some(correction) = CorrectionEvent::from_markdown(&content) {
                        entries.push(LearningEntry::Correction(correction));
                    }
                } else if let Some(learning) = CapturedLearning::from_markdown(&content) {
                    entries.push(LearningEntry::Learning(learning));
                }
            }
        }
    }

    entries.sort_by_key(|b| std::cmp::Reverse(b.captured_at()));
    if entries.len() > limit {
        entries.truncate(limit);
    }

    Ok(entries)
}

/// Query all entries (learnings + corrections) by pattern.
pub fn query_all_entries(
    storage_dir: &PathBuf,
    pattern: &str,
    exact: bool,
) -> Result<Vec<LearningEntry>, LearningError> {
    let all = list_all_entries(storage_dir, usize::MAX)?;
    let pattern_lower = pattern.to_lowercase();

    let filtered: Vec<_> = all
        .into_iter()
        .filter(|entry| {
            let text = match entry {
                LearningEntry::Learning(l) => {
                    format!("{} {}", l.command, l.error_output)
                }
                LearningEntry::Correction(c) => {
                    format!("{} {} {}", c.original, c.corrected, c.context_description)
                }
            };
            if exact {
                text.contains(pattern)
            } else {
                text.to_lowercase().contains(&pattern_lower)
            }
        })
        .collect();

    Ok(filtered)
}

/// Score entry relevance based on keyword matching.
/// Returns a score based on the number of matching keywords between
/// the context and the learning content.
#[allow(dead_code)]
fn score_entry_relevance(entry: &LearningEntry, context_keywords: &[String]) -> usize {
    let text = match entry {
        LearningEntry::Learning(l) => {
            format!("{} {} {:?}", l.command, l.error_output, l.tags)
        }
        LearningEntry::Correction(c) => {
            format!("{} {} {}", c.original, c.corrected, c.context_description)
        }
    }
    .to_lowercase();

    context_keywords
        .iter()
        .filter(|keyword| text.contains(*keyword))
        .count()
}

/// A scored learning entry with its relevance score.
#[derive(Debug, Clone)]
pub struct ScoredEntry {
    /// The learning entry
    pub entry: LearningEntry,
    /// Relevance score (higher is better)
    pub score: usize,
}

impl ScoredEntry {
    /// Format as a suggestion line for display.
    #[allow(dead_code)]
    pub fn format_suggestion(&self) -> String {
        match &self.entry {
            LearningEntry::Learning(l) => {
                format!("[cmd] {} (exit: {}) - {}", l.command, l.exit_code, l.id)
            }
            LearningEntry::Correction(c) => {
                format!(
                    "[{}] {} -> {} - {}",
                    c.correction_type, c.original, c.corrected, c.id
                )
            }
        }
    }
}

/// JSONL transcript entry types for auto-extraction.
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct TranscriptEntry {
    #[serde(default)]
    pub r#type: Option<String>,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub tool_name: Option<String>,
    #[serde(default)]
    pub tool_input: Option<serde_json::Value>,
    #[serde(default)]
    pub tool_result: Option<serde_json::Value>,
    #[serde(default)]
    pub exit_code: Option<i32>,
    #[serde(default)]
    pub error: Option<String>,
}

/// Check if content contains explicit correction phrases.
#[allow(dead_code)]
fn contains_correction_phrase(content: &str) -> Option<(String, String)> {
    let lower = content.to_lowercase();

    // Pattern: "instead use X" or "use X instead"
    if let Some(idx) = lower.find("instead use") {
        let after = &content[idx + 11..];
        return Some((content.to_string(), after.trim().to_string()));
    }
    if let Some(idx) = lower.find("use ") {
        let rest = &lower[idx + 4..];
        if rest.contains("instead") {
            let end = rest.find("instead").unwrap_or(rest.len());
            let tool = &content[idx + 4..idx + 4 + end].trim();
            return Some((content.to_string(), tool.to_string()));
        }
    }

    // Pattern: "should be"
    if let Some(idx) = lower.find("should be") {
        let after = &content[idx + 9..];
        return Some((content.to_string(), after.trim().to_string()));
    }

    // Pattern: "correct way"
    if let Some(idx) = lower.find("correct way") {
        let after = &content[idx + 11..];
        // Look for "is to" or "to"
        if after.contains("is to") {
            let start = after.find("is to").unwrap_or(0) + 5;
            return Some((content.to_string(), after[start..].trim().to_string()));
        }
        return Some((content.to_string(), after.trim().to_string()));
    }

    // Pattern: "use X not Y" or "use X, not Y"
    if let Some(idx) = lower.find("use ") {
        let rest = &content[idx + 4..];
        let lower_rest = rest.to_lowercase();
        if let Some(not_idx) = lower_rest.find(" not ") {
            let tool = rest[..not_idx].trim();
            // Find the end of the old tool (rest of string or next word boundary)
            let old_tool_rest = &rest[not_idx + 5..];
            let old_tool = old_tool_rest
                .split_whitespace()
                .next()
                .unwrap_or(old_tool_rest)
                .trim();
            return Some((old_tool.to_string(), tool.to_string()));
        }
    }

    None
}

/// Extract command from Bash tool input.
#[allow(dead_code)]
fn extract_command_from_input(input: &serde_json::Value) -> Option<String> {
    input
        .get("command")
        .or_else(|| input.get("cmd"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// Auto-extract corrections from a JSONL session transcript.
///
/// Scans the transcript line by line and identifies:
/// 1. Failed Bash commands (exit code != 0) followed by successful variants
/// 2. Explicit correction phrases like "instead use", "should be", etc.
///
/// # Arguments
///
/// * `transcript_path` - Path to the JSONL transcript file
///
/// # Returns
///
/// Vector of extracted CorrectionEvent objects.
#[allow(dead_code)]
pub fn auto_extract_corrections(
    transcript_path: &std::path::Path,
) -> Result<Vec<CorrectionEvent>, LearningError> {
    use std::io::BufRead;

    let file = fs::File::open(transcript_path)?;
    let reader = std::io::BufReader::new(file);

    let mut corrections = Vec::new();
    let mut last_failed_command: Option<(String, i32, String)> = None; // (command, exit_code, error)

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let entry: TranscriptEntry = match serde_json::from_str(&line) {
            Ok(e) => e,
            Err(_) => continue, // Skip malformed lines
        };

        // Check for Bash tool results with exit codes
        if entry.tool_name.as_deref() == Some("Bash")
            || entry.r#type.as_deref() == Some("tool_result")
        {
            // Check if this is a failed Bash command
            if let Some(exit_code) = entry.exit_code {
                if exit_code != 0 {
                    // Extract the command from tool_input in previous context or from error
                    if let Some(ref tool_input) = entry.tool_input {
                        if let Some(cmd) = extract_command_from_input(tool_input) {
                            let error = entry
                                .error
                                .clone()
                                .or_else(|| entry.content.clone())
                                .unwrap_or_default();
                            last_failed_command = Some((cmd, exit_code, error));
                        }
                    }
                } else if exit_code == 0 {
                    // Successful command - check if we had a previous failure
                    if let Some((failed_cmd, failed_exit, failed_error)) =
                        last_failed_command.take()
                    {
                        // Extract the successful command
                        if let Some(ref tool_input) = entry.tool_input {
                            if let Some(success_cmd) = extract_command_from_input(tool_input) {
                                // Only create correction if commands are different
                                if failed_cmd != success_cmd {
                                    let context = format!(
                                        "Auto-extracted from session transcript. Failed with exit {}: {}",
                                        failed_exit, failed_error
                                    );
                                    let correction = CorrectionEvent::new(
                                        CorrectionType::ToolPreference,
                                        failed_cmd,
                                        success_cmd,
                                        context,
                                        LearningSource::Project,
                                    )
                                    .with_tags(vec![
                                        "auto-extracted".to_string(),
                                        "transcript".to_string(),
                                    ]);
                                    corrections.push(correction);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Check for explicit correction phrases in content
        if let Some(ref content) = entry.content {
            if let Some((original, corrected)) = contains_correction_phrase(content) {
                let context = format!(
                    "Auto-extracted from session transcript content: {}",
                    content.chars().take(100).collect::<String>()
                );
                let correction = CorrectionEvent::new(
                    CorrectionType::Other("phrase-detected".to_string()),
                    original,
                    corrected,
                    context,
                    LearningSource::Project,
                )
                .with_tags(vec!["auto-extracted".to_string(), "phrase".to_string()]);
                corrections.push(correction);
            }
        }

        // Also check in tool_result if it's a string
        if let Some(ref tool_result) = entry.tool_result {
            if let Some(content) = tool_result.as_str() {
                if let Some((original, corrected)) = contains_correction_phrase(content) {
                    let context = format!(
                        "Auto-extracted from tool result: {}",
                        content.chars().take(100).collect::<String>()
                    );
                    let correction = CorrectionEvent::new(
                        CorrectionType::Other("phrase-detected".to_string()),
                        original,
                        corrected,
                        context,
                        LearningSource::Project,
                    )
                    .with_tags(vec![
                        "auto-extracted".to_string(),
                        "tool-result".to_string(),
                    ]);
                    corrections.push(correction);
                }
            }
        }
    }

    Ok(corrections)
}

/// Suggest learnings based on context relevance.
///
/// Takes a context string (e.g., current working directory or task description),
/// extracts keywords from it, and scores all learnings by keyword frequency.
/// Returns the top-N most relevant learnings.
///
/// # Arguments
///
/// * `storage_dir` - Directory containing learning markdown files
/// * `context` - Context string to match against (e.g., "rust project with cargo build")
/// * `limit` - Maximum number of suggestions to return
///
/// # Returns
///
/// List of scored entries sorted by relevance (highest first).
#[allow(dead_code)]
pub fn suggest_learnings(
    storage_dir: &PathBuf,
    context: &str,
    limit: usize,
) -> Result<Vec<ScoredEntry>, LearningError> {
    let all_entries = list_all_entries(storage_dir, usize::MAX)?;

    if all_entries.is_empty() {
        return Ok(Vec::new());
    }

    // Extract keywords from context (simple word tokenization)
    let context_keywords: Vec<String> = context
        .split_whitespace()
        .map(|w| {
            w.to_lowercase()
                .trim_matches(|c: char| !c.is_alphanumeric())
                .to_string()
        })
        .filter(|w| !w.is_empty() && w.len() > 2) // Filter out short words
        .collect();

    if context_keywords.is_empty() {
        // Fallback: return most recent entries if no keywords extracted
        let recent: Vec<ScoredEntry> = all_entries
            .into_iter()
            .take(limit)
            .map(|entry| ScoredEntry { entry, score: 0 })
            .collect();
        return Ok(recent);
    }

    // Score all entries
    let mut scored: Vec<ScoredEntry> = all_entries
        .into_iter()
        .map(|entry| {
            let score = score_entry_relevance(&entry, &context_keywords);
            ScoredEntry { entry, score }
        })
        .filter(|se| se.score > 0) // Only include entries with at least one match
        .collect();

    // Sort by score descending
    scored.sort_by(|a, b| b.score.cmp(&a.score));

    // Limit results
    if scored.len() > limit {
        scored.truncate(limit);
    }

    Ok(scored)
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
    fn test_correct_learning() {
        let temp_dir = TempDir::new().unwrap();
        let storage = temp_dir.path().join("learnings");
        let config = LearningCaptureConfig::new(storage.clone(), temp_dir.path().join("global"));

        // Capture a learning
        let path =
            capture_failed_command("bad-cmd --test", "command not found", 127, &config).unwrap();
        assert!(path.exists());

        // Read back to get the ID
        let content = fs::read_to_string(&path).unwrap();
        let learning = CapturedLearning::from_markdown(&content).unwrap();
        let id = learning.id.clone();
        assert!(learning.correction.is_none());

        // Add correction
        let result = correct_learning(&storage, &id, "Use 'good-cmd --test' instead");
        assert!(result.is_ok());

        // Verify correction persisted
        let updated_content = fs::read_to_string(&path).unwrap();
        let updated = CapturedLearning::from_markdown(&updated_content).unwrap();
        assert_eq!(
            updated.correction.as_deref(),
            Some("Use 'good-cmd --test' instead")
        );
        assert_eq!(updated.command, "bad-cmd --test");
        assert_eq!(updated.exit_code, 127);
    }

    #[test]
    fn test_correct_learning_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let storage = temp_dir.path().join("learnings");
        fs::create_dir(&storage).unwrap();

        let result = correct_learning(&storage, "nonexistent-id", "some correction");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), LearningError::NotFound(_)));
    }

    #[test]
    fn test_correct_learning_prefix_match() {
        let temp_dir = TempDir::new().unwrap();
        let storage = temp_dir.path().join("learnings");
        let config = LearningCaptureConfig::new(storage.clone(), temp_dir.path().join("global"));

        // Capture a learning
        let path = capture_failed_command("git push -f", "remote: rejected", 1, &config).unwrap();

        // Get the full ID
        let content = fs::read_to_string(&path).unwrap();
        let learning = CapturedLearning::from_markdown(&content).unwrap();
        let full_id = learning.id.clone();

        // Use only the first 8 characters as prefix
        let prefix = &full_id[..8];
        let result = correct_learning(&storage, prefix, "git push origin main");
        assert!(result.is_ok());

        // Verify
        let updated_content = fs::read_to_string(&path).unwrap();
        let updated = CapturedLearning::from_markdown(&updated_content).unwrap();
        assert_eq!(updated.correction.as_deref(), Some("git push origin main"));
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

    #[test]
    fn test_correction_event_to_markdown() {
        let correction = CorrectionEvent::new(
            CorrectionType::ToolPreference,
            "npm install".to_string(),
            "bun add".to_string(),
            "User prefers bun over npm".to_string(),
            LearningSource::Project,
        )
        .with_session_id("session-123".to_string())
        .with_tags(vec!["tool".to_string(), "preference".to_string()]);

        let md = correction.to_markdown();
        assert!(md.contains("type: correction"));
        assert!(md.contains("correction_type: tool-preference"));
        assert!(md.contains("`npm install`"));
        assert!(md.contains("`bun add`"));
        assert!(md.contains("session_id: session-123"));
        assert!(md.contains("User prefers bun over npm"));
    }

    #[test]
    fn test_correction_event_roundtrip() {
        let original = CorrectionEvent::new(
            CorrectionType::CodePattern,
            "use unwrap()".to_string(),
            "use Result<T>".to_string(),
            "Better error handling".to_string(),
            LearningSource::Global,
        );

        let md = original.to_markdown();
        let parsed = CorrectionEvent::from_markdown(&md).unwrap();

        assert_eq!(parsed.correction_type, original.correction_type);
        assert_eq!(parsed.original, original.original);
        assert_eq!(parsed.corrected, original.corrected);
        assert_eq!(parsed.context_description, original.context_description);
    }

    #[test]
    fn test_capture_correction() {
        let temp_dir = TempDir::new().unwrap();
        let config = LearningCaptureConfig::new(
            temp_dir.path().join("learnings"),
            temp_dir.path().join("global"),
        );

        let result = capture_correction(
            CorrectionType::Naming,
            "variable_name",
            "variableName",
            "Use camelCase for variables",
            &config,
        );

        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.exists());

        // Verify content
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("type: correction"));
        assert!(content.contains("correction_type: naming"));
        assert!(content.contains("`variable_name`"));
        assert!(content.contains("`variableName`"));
    }

    #[test]
    fn test_correction_secret_redaction() {
        let temp_dir = TempDir::new().unwrap();
        let config = LearningCaptureConfig::new(
            temp_dir.path().join("learnings"),
            temp_dir.path().join("global"),
        );

        let result = capture_correction(
            CorrectionType::FactCorrection,
            "AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE",
            "Use environment variables instead",
            "Never hardcode AWS keys",
            &config,
        );

        assert!(result.is_ok());
        let path = result.unwrap();
        let content = fs::read_to_string(&path).unwrap();

        // Secret should be redacted
        assert!(!content.contains("AKIAIOSFODNN7EXAMPLE"));
        assert!(content.contains("[AWS_KEY_REDACTED]") || content.contains("[ENV_REDACTED]"));
    }

    #[test]
    fn test_list_all_entries_mixed() {
        let temp_dir = TempDir::new().unwrap();
        let storage = temp_dir.path().join("learnings");
        fs::create_dir(&storage).unwrap();

        // Create 2 learnings
        let learning1 = CapturedLearning::new(
            "cmd1".to_string(),
            "error1".to_string(),
            1,
            LearningSource::Project,
        );
        let learning2 = CapturedLearning::new(
            "cmd2".to_string(),
            "error2".to_string(),
            2,
            LearningSource::Project,
        );

        // Create 2 corrections
        let correction1 = CorrectionEvent::new(
            CorrectionType::ToolPreference,
            "old".to_string(),
            "new".to_string(),
            "context".to_string(),
            LearningSource::Project,
        );
        let correction2 = CorrectionEvent::new(
            CorrectionType::Naming,
            "foo".to_string(),
            "bar".to_string(),
            "desc".to_string(),
            LearningSource::Project,
        );

        // Write files
        fs::write(storage.join("learning-1.md"), learning1.to_markdown()).unwrap();
        fs::write(storage.join("learning-2.md"), learning2.to_markdown()).unwrap();
        fs::write(storage.join("correction-1.md"), correction1.to_markdown()).unwrap();
        fs::write(storage.join("correction-2.md"), correction2.to_markdown()).unwrap();

        let entries = list_all_entries(&storage, 10).unwrap();
        assert_eq!(entries.len(), 4);
    }

    #[test]
    fn test_query_all_entries_finds_corrections() {
        let temp_dir = TempDir::new().unwrap();
        let storage = temp_dir.path().join("learnings");
        fs::create_dir(&storage).unwrap();

        // Create a learning
        let learning = CapturedLearning::new(
            "git status".to_string(),
            "error".to_string(),
            1,
            LearningSource::Project,
        );

        // Create a correction with "bun" in the text
        let correction = CorrectionEvent::new(
            CorrectionType::ToolPreference,
            "npm install".to_string(),
            "bun add".to_string(),
            "Use bun instead of npm".to_string(),
            LearningSource::Project,
        );

        fs::write(storage.join("learning-test.md"), learning.to_markdown()).unwrap();
        fs::write(storage.join("correction-test.md"), correction.to_markdown()).unwrap();

        let results = query_all_entries(&storage, "bun", false).unwrap();
        assert_eq!(results.len(), 1);
        match &results[0] {
            LearningEntry::Correction(c) => {
                assert_eq!(c.original, "npm install");
            }
            _ => panic!("Expected correction entry"),
        }
    }

    #[test]
    fn test_correction_type_roundtrip() {
        let variants = vec![
            CorrectionType::ToolPreference,
            CorrectionType::CodePattern,
            CorrectionType::Naming,
            CorrectionType::WorkflowStep,
            CorrectionType::FactCorrection,
            CorrectionType::StylePreference,
            CorrectionType::Other("custom".to_string()),
        ];

        for variant in variants {
            let display = format!("{}", variant);
            let parsed: CorrectionType = display.parse().unwrap();
            assert_eq!(variant, parsed);
        }
    }

    #[test]
    fn test_learning_entry_summary() {
        let learning = CapturedLearning::new(
            "git push".to_string(),
            "error".to_string(),
            1,
            LearningSource::Project,
        );
        let learning_entry = LearningEntry::Learning(learning);
        assert!(learning_entry.summary().contains("[cmd]"));
        assert!(learning_entry.summary().contains("git push"));

        let correction = CorrectionEvent::new(
            CorrectionType::ToolPreference,
            "npm".to_string(),
            "bun".to_string(),
            "context".to_string(),
            LearningSource::Global,
        );
        let correction_entry = LearningEntry::Correction(correction);
        assert!(correction_entry.summary().contains("tool-preference"));
        assert!(correction_entry.summary().contains("npm"));
        assert!(correction_entry.summary().contains("bun"));
    }

    #[test]
    fn test_contains_correction_phrase_instead_use() {
        let content = "You should instead use cargo build";
        let result = contains_correction_phrase(content);
        assert!(result.is_some());
        let (original, _corrected) = result.unwrap();
        assert!(original.contains("You should"));
    }

    #[test]
    fn test_contains_correction_phrase_use_instead() {
        let content = "Use bun instead of npm for faster installs";
        let result = contains_correction_phrase(content);
        assert!(result.is_some());
        let (original, _corrected) = result.unwrap();
        assert!(original.contains("Use bun"));
    }

    #[test]
    fn test_contains_correction_phrase_should_be() {
        let content = "The variable name should be user_count";
        let result = contains_correction_phrase(content);
        assert!(result.is_some());
        let (original, _corrected) = result.unwrap();
        assert!(original.contains("variable name"));
    }

    #[test]
    fn test_contains_correction_phrase_correct_way() {
        let content = "The correct way is to use cargo check first";
        let result = contains_correction_phrase(content);
        assert!(result.is_some());
        let (original, _corrected) = result.unwrap();
        assert!(original.contains("The correct way"));
    }

    #[test]
    fn test_contains_correction_phrase_use_not() {
        let content = "Use yarn not npm for this project";
        let result = contains_correction_phrase(content);
        assert!(result.is_some());
        let (original, corrected) = result.unwrap();
        assert_eq!(original, "npm");
        assert_eq!(corrected, "yarn");
    }

    #[test]
    fn test_contains_correction_phrase_no_match() {
        let content = "This is just a normal sentence without corrections";
        let result = contains_correction_phrase(content);
        assert!(result.is_none());
    }

    #[test]
    fn test_auto_extract_corrections_from_transcript() {
        use std::io::Write;

        let temp_dir = TempDir::new().unwrap();
        let storage = temp_dir.path().join("learnings");
        fs::create_dir(&storage).unwrap();

        // Create a mock transcript with failed then successful commands
        let transcript_path = temp_dir.path().join("session.jsonl");
        let transcript_content = r#"
{"type": "tool_use", "tool_name": "Bash", "tool_input": {"command": "git push -f"}}
{"type": "tool_result", "tool_name": "Bash", "exit_code": 1, "error": "remote: rejected", "tool_input": {"command": "git push -f"}}
{"type": "tool_use", "tool_name": "Bash", "tool_input": {"command": "git push origin main"}}
{"type": "tool_result", "tool_name": "Bash", "exit_code": 0, "tool_input": {"command": "git push origin main"}}
{"content": "You should instead use cargo check before building"}
"#;
        let mut file = fs::File::create(&transcript_path).unwrap();
        file.write_all(transcript_content.as_bytes()).unwrap();

        let corrections = auto_extract_corrections(&transcript_path).unwrap();

        // Should find at least 2 corrections: the command fix + the phrase
        assert!(
            corrections.len() >= 2,
            "Expected at least 2 corrections, got {}",
            corrections.len()
        );

        // Check for the command correction
        let cmd_correction = corrections
            .iter()
            .find(|c| c.original == "git push -f" && c.corrected == "git push origin main");
        assert!(
            cmd_correction.is_some(),
            "Should find command correction: git push -f -> git push origin main"
        );

        // Check for the phrase correction
        let phrase_correction = corrections
            .iter()
            .find(|c| c.corrected.contains("cargo check"));
        assert!(
            phrase_correction.is_some(),
            "Should find phrase correction containing 'cargo check'"
        );
    }

    #[test]
    fn test_auto_extract_corrections_empty_transcript() {
        let temp_dir = TempDir::new().unwrap();

        // Create an empty transcript
        let transcript_path = temp_dir.path().join("empty.jsonl");
        fs::write(&transcript_path, "").unwrap();

        let corrections = auto_extract_corrections(&transcript_path).unwrap();
        assert!(corrections.is_empty());
    }

    #[test]
    fn test_auto_extract_corrections_no_failures() {
        use std::io::Write;

        let temp_dir = TempDir::new().unwrap();

        // Create a transcript with only successful commands
        let transcript_path = temp_dir.path().join("success.jsonl");
        let transcript_content = r#"
{"type": "tool_use", "tool_name": "Bash", "tool_input": {"command": "git status"}}
{"type": "tool_result", "tool_name": "Bash", "exit_code": 0, "tool_input": {"command": "git status"}}
{"type": "tool_use", "tool_name": "Bash", "tool_input": {"command": "git log"}}
{"type": "tool_result", "tool_name": "Bash", "exit_code": 0, "tool_input": {"command": "git log"}}
"#;
        let mut file = fs::File::create(&transcript_path).unwrap();
        file.write_all(transcript_content.as_bytes()).unwrap();

        let corrections = auto_extract_corrections(&transcript_path).unwrap();
        // No corrections since all commands succeeded
        assert!(corrections.is_empty());
    }
}
