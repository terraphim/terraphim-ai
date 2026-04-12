//! Procedure storage for captured successful procedures.
//!
//! This module provides persistent storage for CapturedProcedure instances,
//! with Aho-Corasick-based deduplication support.
//!
//! # Example
//!
//! ```
//! use std::path::PathBuf;
//! use terraphim_agent::learnings::procedure::ProcedureStore;
//! use terraphim_types::procedure::{CapturedProcedure, ProcedureStep};
//!
//! # fn example() -> std::io::Result<()> {
//! let store = ProcedureStore::new(PathBuf::from("~/.config/terraphim/learnings/procedures.jsonl"));
//!
//! let mut procedure = CapturedProcedure::new(
//!     "install-rust".to_string(),
//!     "Install Rust".to_string(),
//!     "Install Rust toolchain".to_string(),
//! );
//!
//! procedure.add_step(ProcedureStep {
//!     ordinal: 1,
//!     command: "curl https://sh.rustup.rs | sh".to_string(),
//!     precondition: None,
//!     postcondition: None,
//!     working_dir: None,
//!     privileged: false,
//!     tags: vec![],
//! });
//!
//! store.save(&procedure)?;
//! # Ok(())
//! # }
//! ```

use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::PathBuf;

use terraphim_automata::matcher::find_matches;
#[cfg(test)]
use terraphim_types::procedure::ProcedureConfidence;
use terraphim_types::{
    NormalizedTerm, NormalizedTermValue, Thesaurus, procedure::CapturedProcedure,
};

/// Health status of a procedure based on its confidence metrics.
#[derive(Debug, Clone, PartialEq)]
pub enum HealthStatus {
    /// Procedure is performing well (score >= 0.7)
    Healthy,
    /// Procedure is showing signs of degradation (score 0.3..0.7)
    Degraded,
    /// Procedure is critically failing (score < 0.3)
    Critical,
    /// Not enough executions to determine health (< 3 total)
    Insufficient,
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthStatus::Healthy => write!(f, "Healthy"),
            HealthStatus::Degraded => write!(f, "Degraded"),
            HealthStatus::Critical => write!(f, "Critical"),
            HealthStatus::Insufficient => write!(f, "Insufficient"),
        }
    }
}

/// Health report for a single procedure.
#[derive(Debug, Clone)]
pub struct ProcedureHealthReport {
    /// Procedure ID
    pub id: String,
    /// Current health status
    pub status: HealthStatus,
    /// Success rate as a fraction (0.0 - 1.0)
    pub success_rate: f64,
    /// Total number of executions
    pub total_executions: u32,
    /// Whether the procedure was auto-disabled
    pub auto_disabled: bool,
}

/// Storage for captured procedures with deduplication support.
pub struct ProcedureStore {
    /// Path to the JSONL storage file
    store_path: PathBuf,
}

impl ProcedureStore {
    /// Create a new ProcedureStore with the given path.
    ///
    /// The path should be a JSONL file (e.g., `procedures.jsonl`).
    /// Parent directories will be created automatically when saving.
    pub fn new(store_path: PathBuf) -> Self {
        Self { store_path }
    }

    /// Get the default store path in the user's config directory.
    ///
    /// Returns `~/.config/terraphim/learnings/procedures.jsonl` on Unix-like systems,
    /// or the equivalent config directory on other platforms.
    ///
    /// Note: This function is not used internally but is provided as a convenience
    /// for external callers who want a sensible default path.
    #[allow(dead_code)]
    pub fn default_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("terraphim")
            .join("learnings")
            .join("procedures.jsonl")
    }

    /// Ensure the parent directory exists.
    fn ensure_dir_exists(&self) -> io::Result<()> {
        if let Some(parent) = self.store_path.parent() {
            fs::create_dir_all(parent)?;
        }
        Ok(())
    }

    /// Save a procedure to storage.
    ///
    /// If a procedure with the same ID already exists, it will be updated.
    /// This operation performs deduplication checks before saving.
    pub fn save(&self, procedure: &CapturedProcedure) -> io::Result<()> {
        self.ensure_dir_exists()?;

        // Load existing procedures
        let mut procedures = self.load_all()?;

        // Check for existing procedure with same ID
        let existing_index = procedures.iter().position(|p| p.id == procedure.id);

        if let Some(index) = existing_index {
            // Update existing procedure
            procedures[index] = procedure.clone();
        } else {
            // Add new procedure
            procedures.push(procedure.clone());
        }

        // Write all procedures back to file
        self.write_all(&procedures)
    }

    /// Save a procedure with deduplication check.
    ///
    /// If a similar procedure (matching title via Aho-Corasick) with high confidence
    /// (> 0.8) exists, merge the steps instead of creating a duplicate.
    ///
    /// Returns the saved (or merged) procedure.
    pub fn save_with_dedup(
        &self,
        mut procedure: CapturedProcedure,
    ) -> io::Result<CapturedProcedure> {
        self.ensure_dir_exists()?;

        // Load existing procedures for dedup check
        let existing_procedures = self.load_all()?;

        // Build thesaurus from existing procedure titles for deduplication
        let mut thesaurus = Thesaurus::new("procedure_titles".to_string());
        for (idx, existing) in existing_procedures.iter().enumerate() {
            let normalized_title = existing.title.to_lowercase();
            let term = NormalizedTerm::new(idx as u64, NormalizedTermValue::from(normalized_title));
            thesaurus.insert(
                NormalizedTermValue::from(existing.title.to_lowercase()),
                term,
            );
        }

        // Check for matching titles using Aho-Corasick
        let matches = find_matches(&procedure.title.to_lowercase(), thesaurus, false)
            .map_err(io::Error::other)?;

        let mut merged = false;
        let mut merged_procedure_id = None;

        for matched in matches {
            // Find the matching procedure
            if let Some(existing) = existing_procedures
                .iter()
                .find(|p| p.title.to_lowercase() == matched.term.to_lowercase())
            {
                // Check if it has high confidence
                if existing.confidence.is_high_confidence() {
                    log::info!(
                        "Found similar procedure '{}' with high confidence ({}), merging steps",
                        existing.title,
                        existing.confidence.score
                    );

                    // Merge steps into the new procedure
                    procedure.merge_steps(existing);
                    merged = true;
                    merged_procedure_id = Some(existing.id.clone());
                    break;
                }
            }
        }

        if merged {
            // If we merged with an existing procedure, update the ID to match
            if let Some(existing_id) = merged_procedure_id {
                procedure.id = existing_id;
            }
        }

        // Save the (possibly merged) procedure
        self.save(&procedure)?;

        Ok(procedure)
    }

    /// Load all procedures from storage.
    pub fn load_all(&self) -> io::Result<Vec<CapturedProcedure>> {
        if !self.store_path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&self.store_path)?;
        let reader = BufReader::new(file);
        let mut procedures = Vec::new();

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }

            match serde_json::from_str::<CapturedProcedure>(&line) {
                Ok(procedure) => procedures.push(procedure),
                Err(e) => {
                    log::warn!("Failed to parse procedure from JSONL: {}", e);
                    continue;
                }
            }
        }

        Ok(procedures)
    }

    /// Write all procedures to storage (internal helper).
    fn write_all(&self, procedures: &[CapturedProcedure]) -> io::Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.store_path)?;

        for procedure in procedures {
            let json = serde_json::to_string(procedure)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            writeln!(file, "{}", json)?;
        }

        file.flush()?;
        Ok(())
    }

    /// Find procedures by title (case-insensitive substring search).
    #[cfg(test)]
    pub fn find_by_title(&self, query: &str) -> io::Result<Vec<CapturedProcedure>> {
        let all = self.load_all()?;
        let query_lower = query.to_lowercase();

        let filtered: Vec<_> = all
            .into_iter()
            .filter(|p| {
                p.title.to_lowercase().contains(&query_lower)
                    || p.description.to_lowercase().contains(&query_lower)
            })
            .collect();

        Ok(filtered)
    }

    /// Find a procedure by its exact ID.
    pub fn find_by_id(&self, id: &str) -> io::Result<Option<CapturedProcedure>> {
        let all = self.load_all()?;
        Ok(all.into_iter().find(|p| p.id == id))
    }

    /// Update the confidence metrics for a procedure.
    ///
    /// Records a success or failure and updates the score.
    pub fn update_confidence(&self, id: &str, success: bool) -> io::Result<()> {
        let mut procedures = self.load_all()?;

        if let Some(procedure) = procedures.iter_mut().find(|p| p.id == id) {
            if success {
                procedure.record_success();
            } else {
                procedure.record_failure();
            }
            self.write_all(&procedures)?;
        } else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Procedure with ID '{}' not found", id),
            ));
        }

        Ok(())
    }

    /// Run a health check on all stored procedures.
    ///
    /// Returns a report for each procedure with its health status.
    /// Procedures with score < 0.3 AND >= 5 total executions are
    /// automatically disabled (auto-healing).
    pub fn health_check(&self) -> io::Result<Vec<ProcedureHealthReport>> {
        let mut procedures = self.load_all()?;
        let mut reports = Vec::new();
        let mut needs_save = false;

        for procedure in &mut procedures {
            let total = procedure.confidence.total_executions();
            let score = procedure.confidence.score;

            let status = if total < 3 {
                HealthStatus::Insufficient
            } else if score >= 0.7 {
                HealthStatus::Healthy
            } else if score >= 0.3 {
                HealthStatus::Degraded
            } else {
                HealthStatus::Critical
            };

            // Auto-disable critically failing procedures with enough data
            let auto_disabled = status == HealthStatus::Critical && total >= 5;
            if auto_disabled && !procedure.disabled {
                procedure.disabled = true;
                needs_save = true;
            }

            reports.push(ProcedureHealthReport {
                id: procedure.id.clone(),
                status,
                success_rate: score,
                total_executions: total,
                auto_disabled,
            });
        }

        if needs_save {
            self.write_all(&procedures)?;
        }

        Ok(reports)
    }

    /// Set the disabled flag on a procedure.
    pub fn set_disabled(&self, id: &str, disabled: bool) -> io::Result<()> {
        let mut procedures = self.load_all()?;

        if let Some(procedure) = procedures.iter_mut().find(|p| p.id == id) {
            procedure.disabled = disabled;
            self.write_all(&procedures)?;
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Procedure with ID '{}' not found", id),
            ))
        }
    }

    /// Delete a procedure by ID.
    #[cfg(test)]
    pub fn delete(&self, id: &str) -> io::Result<bool> {
        let mut procedures = self.load_all()?;
        let original_len = procedures.len();

        procedures.retain(|p| p.id != id);

        if procedures.len() != original_len {
            self.write_all(&procedures)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

/// Commands considered trivial and excluded from session-to-procedure extraction.
///
/// These are navigational, informational, or read-only commands that do not
/// contribute meaningful steps to a procedure.
pub const TRIVIAL_COMMANDS: &[&str] = &[
    "cd ", "ls", "pwd", "echo ", "cat ", "head ", "tail ", "wc ", "which ", "type ", "date",
    "whoami",
];

/// Check whether a command is trivial (should be excluded from procedure extraction).
fn is_trivial_command(command: &str) -> bool {
    let trimmed = command.trim();
    TRIVIAL_COMMANDS
        .iter()
        .any(|prefix| trimmed == prefix.trim() || trimmed.starts_with(prefix))
}

/// Create a `CapturedProcedure` from a list of (command, exit_code) pairs.
///
/// This is the core extraction logic for session-based auto-capture.
/// It filters out trivial commands (cd, ls, pwd, etc.) and commands that
/// failed (exit_code != 0), then maps the remaining commands to ordered
/// `ProcedureStep` entries.
///
/// # Arguments
///
/// * `commands` - A list of `(command_string, exit_code)` tuples extracted from a session.
/// * `title` - Optional title. If `None`, auto-generates from the first meaningful command.
///
/// # Returns
///
/// A `CapturedProcedure` with steps derived from the successful, non-trivial commands.
pub fn from_session_commands(
    commands: Vec<(String, i32)>,
    title: Option<String>,
) -> CapturedProcedure {
    use terraphim_types::procedure::ProcedureStep;
    use uuid::Uuid;

    // Filter: only successful, non-trivial commands
    let meaningful: Vec<&str> = commands
        .iter()
        .filter(|(_, exit_code)| *exit_code == 0)
        .map(|(cmd, _)| cmd.as_str())
        .filter(|cmd| !is_trivial_command(cmd))
        .collect();

    // Auto-generate title from first meaningful command if not provided
    let generated_title = title.unwrap_or_else(|| {
        if let Some(first) = meaningful.first() {
            let truncated = if first.len() > 60 {
                format!("{}...", &first[..60])
            } else {
                (*first).to_string()
            };
            format!("Session: {}", truncated)
        } else {
            "Session procedure (empty)".to_string()
        }
    });

    let description = format!(
        "Auto-captured from session ({} steps from {} commands)",
        meaningful.len(),
        commands.len()
    );

    let id = Uuid::new_v4().to_string();
    let mut procedure = CapturedProcedure::new(id, generated_title, description);

    for (ordinal, cmd) in meaningful.iter().enumerate() {
        procedure.add_step(ProcedureStep {
            ordinal: (ordinal + 1) as u32,
            command: cmd.to_string(),
            precondition: None,
            postcondition: None,
            working_dir: None,
            privileged: false,
            tags: vec![],
        });
    }

    procedure
}

/// Extract Bash commands with success/failure status from a session.
///
/// Pairs `ToolUse` blocks (where `name == "Bash"`) with their corresponding
/// `ToolResult` blocks using the `tool_use_id` linkage. Returns a list of
/// `(command, exit_code)` pairs where exit_code is 0 for success and 1 for error.
#[cfg(feature = "repl-sessions")]
pub fn extract_bash_commands_from_session(
    session: &terraphim_sessions::Session,
) -> Vec<(String, i32)> {
    use terraphim_sessions::ContentBlock;

    // Collect all ToolResult blocks indexed by tool_use_id
    let mut results: std::collections::HashMap<&str, bool> = std::collections::HashMap::new();
    for msg in &session.messages {
        for block in &msg.blocks {
            if let ContentBlock::ToolResult {
                tool_use_id,
                is_error,
                ..
            } = block
            {
                results.insert(tool_use_id.as_str(), *is_error);
            }
        }
    }

    // Collect Bash ToolUse blocks and match with results
    let mut commands = Vec::new();
    for msg in &session.messages {
        for block in &msg.blocks {
            if let ContentBlock::ToolUse { id, name, input } = block {
                if name == "Bash" {
                    if let Some(cmd) = input.get("command").and_then(|v| v.as_str()) {
                        let is_error = results.get(id.as_str()).copied().unwrap_or(false);
                        let exit_code = if is_error { 1 } else { 0 };
                        commands.push((cmd.to_string(), exit_code));
                    }
                }
            }
        }
    }

    commands
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use terraphim_types::procedure::ProcedureStep;

    fn create_test_store() -> (TempDir, ProcedureStore) {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("procedures.jsonl");
        let store = ProcedureStore::new(store_path);
        (temp_dir, store)
    }

    fn create_test_procedure(id: &str, title: &str) -> CapturedProcedure {
        let mut procedure = CapturedProcedure::new(
            id.to_string(),
            title.to_string(),
            format!("Description for {}", title),
        );

        procedure.add_step(ProcedureStep {
            ordinal: 1,
            command: "echo test".to_string(),
            precondition: None,
            postcondition: None,
            working_dir: None,
            privileged: false,
            tags: vec!["test".to_string()],
        });

        procedure
    }

    #[test]
    fn test_procedure_store_save_and_load() {
        let (_temp_dir, store) = create_test_store();

        let procedure = create_test_procedure("test-1", "Test Procedure");
        store.save(&procedure).unwrap();

        let loaded = store.load_all().unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, "test-1");
        assert_eq!(loaded[0].title, "Test Procedure");
    }

    #[test]
    fn test_procedure_store_find_by_title() {
        let (_temp_dir, store) = create_test_store();

        let proc1 = create_test_procedure("test-1", "Install Rust");
        let proc2 = create_test_procedure("test-2", "Install Node.js");
        let proc3 = create_test_procedure("test-3", "Deploy Application");

        store.save(&proc1).unwrap();
        store.save(&proc2).unwrap();
        store.save(&proc3).unwrap();

        let results = store.find_by_title("Install").unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|p| p.title == "Install Rust"));
        assert!(results.iter().any(|p| p.title == "Install Node.js"));
    }

    #[test]
    fn test_procedure_store_update_confidence() {
        let (_temp_dir, store) = create_test_store();

        let mut procedure = create_test_procedure("test-1", "Test Procedure");
        procedure.confidence = ProcedureConfidence::new();
        store.save(&procedure).unwrap();

        // Record some successes
        store.update_confidence("test-1", true).unwrap();
        store.update_confidence("test-1", true).unwrap();
        store.update_confidence("test-1", false).unwrap();

        let loaded = store.load_all().unwrap();
        assert_eq!(loaded[0].confidence.success_count, 2);
        assert_eq!(loaded[0].confidence.failure_count, 1);
        assert_eq!(loaded[0].confidence.score, 2.0 / 3.0);
    }

    #[test]
    fn test_procedure_store_update_confidence_not_found() {
        let (_temp_dir, store) = create_test_store();

        let result = store.update_confidence("nonexistent", true);
        assert!(result.is_err());
        assert!(result.unwrap_err().kind() == io::ErrorKind::NotFound);
    }

    #[test]
    fn test_dedup_matching_titles() {
        let (_temp_dir, store) = create_test_store();

        // Create a procedure with high confidence
        let mut existing_proc = create_test_procedure("existing-id", "Rust Install");
        // Use record_success to properly set the score
        for _ in 0..10 {
            existing_proc.record_success();
        }
        existing_proc.record_failure();
        // Score should be ~0.909, high confidence
        assert!(existing_proc.confidence.is_high_confidence());

        existing_proc.add_step(ProcedureStep {
            ordinal: 2,
            command: "rustc --version".to_string(),
            precondition: None,
            postcondition: None,
            working_dir: None,
            privileged: false,
            tags: vec![],
        });
        store.save(&existing_proc).unwrap();

        // Create a new procedure with title that contains the pattern "rust install"
        let mut new_proc = create_test_procedure("new-id", "Rust Install Guide");
        new_proc.add_step(ProcedureStep {
            ordinal: 1,
            command: "curl https://sh.rustup.rs | sh".to_string(),
            precondition: None,
            postcondition: None,
            working_dir: None,
            privileged: false,
            tags: vec![],
        });

        // Save with deduplication - should merge with existing
        let saved = store.save_with_dedup(new_proc).unwrap();

        // Should have merged steps (echo test from both, plus rustc and curl)
        // new_proc has: echo test, curl
        // existing has: echo test, rustc
        // After merge: echo test, curl, rustc = 3 steps
        assert_eq!(
            saved.step_count(),
            3,
            "Expected 3 steps after merge: echo test, curl, rustc"
        );

        // Verify the merged procedure is saved (should replace existing)
        let all = store.load_all().unwrap();
        assert_eq!(all.len(), 1, "Should have only 1 procedure after merge");
        assert_eq!(
            all[0].step_count(),
            3,
            "Saved procedure should have 3 steps"
        );
    }

    #[test]
    fn test_dedup_no_match_for_different_titles() {
        let (_temp_dir, store) = create_test_store();

        // Create a procedure with high confidence
        let mut existing_proc = create_test_procedure("existing-id", "Install Rust");
        existing_proc.confidence.success_count = 10;
        existing_proc.confidence.failure_count = 0;
        existing_proc.confidence.score = 1.0;
        store.save(&existing_proc).unwrap();

        // Create a new procedure with different title
        let new_proc = create_test_procedure("new-id", "Deploy to Kubernetes");

        // Save with deduplication - should create new
        let saved = store.save_with_dedup(new_proc).unwrap();

        // Should be a new procedure
        assert_eq!(saved.id, "new-id");

        // Verify both procedures exist
        let all = store.load_all().unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_procedure_store_delete() {
        let (_temp_dir, store) = create_test_store();

        let proc1 = create_test_procedure("test-1", "Procedure 1");
        let proc2 = create_test_procedure("test-2", "Procedure 2");

        store.save(&proc1).unwrap();
        store.save(&proc2).unwrap();

        let deleted = store.delete("test-1").unwrap();
        assert!(deleted);

        let loaded = store.load_all().unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, "test-2");

        // Deleting non-existent should return false
        let deleted_again = store.delete("test-1").unwrap();
        assert!(!deleted_again);
    }

    #[test]
    fn test_health_check_insufficient_data() {
        let (_temp_dir, store) = create_test_store();

        let mut proc = create_test_procedure("test-1", "Low Data Procedure");
        // Only 2 executions -- below the 3 threshold
        proc.confidence = ProcedureConfidence::new();
        proc.record_success();
        proc.record_success();
        store.save(&proc).unwrap();

        let reports = store.health_check().unwrap();
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].status, HealthStatus::Insufficient);
        assert!(!reports[0].auto_disabled);
    }

    #[test]
    fn test_health_check_healthy() {
        let (_temp_dir, store) = create_test_store();

        let mut proc = create_test_procedure("test-1", "Healthy Procedure");
        proc.confidence = ProcedureConfidence::new();
        for _ in 0..9 {
            proc.record_success();
        }
        proc.record_failure(); // 90% success rate
        store.save(&proc).unwrap();

        let reports = store.health_check().unwrap();
        assert_eq!(reports[0].status, HealthStatus::Healthy);
        assert!(!reports[0].auto_disabled);
    }

    #[test]
    fn test_health_check_degraded() {
        let (_temp_dir, store) = create_test_store();

        let mut proc = create_test_procedure("test-1", "Degraded Procedure");
        proc.confidence = ProcedureConfidence::new();
        // 2 successes, 2 failures = 50% (between 0.3 and 0.7)
        proc.record_success();
        proc.record_success();
        proc.record_failure();
        proc.record_failure();
        store.save(&proc).unwrap();

        let reports = store.health_check().unwrap();
        assert_eq!(reports[0].status, HealthStatus::Degraded);
        assert!(!reports[0].auto_disabled);
    }

    #[test]
    fn test_health_check_critical_auto_disable() {
        let (_temp_dir, store) = create_test_store();

        let mut proc = create_test_procedure("test-1", "Critical Procedure");
        proc.confidence = ProcedureConfidence::new();
        // 1 success, 4 failures = 20% with 5 total executions
        proc.record_success();
        for _ in 0..4 {
            proc.record_failure();
        }
        store.save(&proc).unwrap();

        let reports = store.health_check().unwrap();
        assert_eq!(reports[0].status, HealthStatus::Critical);
        assert!(reports[0].auto_disabled);

        // Verify the procedure was actually saved as disabled
        let loaded = store.find_by_id("test-1").unwrap().unwrap();
        assert!(loaded.disabled);
    }

    #[test]
    fn test_health_check_critical_below_threshold_not_auto_disabled() {
        let (_temp_dir, store) = create_test_store();

        let mut proc = create_test_procedure("test-1", "Failing but new");
        proc.confidence = ProcedureConfidence::new();
        // 0 successes, 3 failures = 0% but only 3 total (below 5 threshold)
        for _ in 0..3 {
            proc.record_failure();
        }
        store.save(&proc).unwrap();

        let reports = store.health_check().unwrap();
        assert_eq!(reports[0].status, HealthStatus::Critical);
        assert!(!reports[0].auto_disabled); // Not enough executions for auto-disable

        let loaded = store.find_by_id("test-1").unwrap().unwrap();
        assert!(!loaded.disabled);
    }

    #[test]
    fn test_set_disabled() {
        let (_temp_dir, store) = create_test_store();

        let proc = create_test_procedure("test-1", "Procedure to disable");
        store.save(&proc).unwrap();

        // Disable
        store.set_disabled("test-1", true).unwrap();
        let loaded = store.find_by_id("test-1").unwrap().unwrap();
        assert!(loaded.disabled);

        // Re-enable
        store.set_disabled("test-1", false).unwrap();
        let loaded = store.find_by_id("test-1").unwrap().unwrap();
        assert!(!loaded.disabled);
    }

    #[test]
    fn test_set_disabled_not_found() {
        let (_temp_dir, store) = create_test_store();

        let result = store.set_disabled("nonexistent", true);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn test_procedure_store_find_by_id() {
        let (_temp_dir, store) = create_test_store();

        let proc1 = create_test_procedure("test-1", "Procedure 1");
        store.save(&proc1).unwrap();

        let found = store.find_by_id("test-1").unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().title, "Procedure 1");

        let not_found = store.find_by_id("nonexistent").unwrap();
        assert!(not_found.is_none());
    }

    #[test]
    fn test_from_session_commands_basic() {
        use super::from_session_commands;

        let commands = vec![
            ("cargo build".to_string(), 0),
            ("cargo test".to_string(), 0),
            ("cargo clippy".to_string(), 0),
        ];

        let proc = from_session_commands(commands, Some("Build and test".to_string()));

        assert_eq!(proc.title, "Build and test");
        assert_eq!(proc.steps.len(), 3);
        assert_eq!(proc.steps[0].ordinal, 1);
        assert_eq!(proc.steps[0].command, "cargo build");
        assert_eq!(proc.steps[1].ordinal, 2);
        assert_eq!(proc.steps[1].command, "cargo test");
        assert_eq!(proc.steps[2].ordinal, 3);
        assert_eq!(proc.steps[2].command, "cargo clippy");
        assert!(proc.description.contains("3 steps"));
    }

    #[test]
    fn test_from_session_commands_filters_trivial() {
        use super::from_session_commands;

        let commands = vec![
            ("cd /tmp".to_string(), 0),
            ("ls".to_string(), 0),
            ("cargo build".to_string(), 0),
            ("pwd".to_string(), 0),
            ("echo hello".to_string(), 0),
            ("cargo test".to_string(), 0),
            ("cat README.md".to_string(), 0),
            ("head -5 file.txt".to_string(), 0),
            ("tail -10 log.txt".to_string(), 0),
            ("wc -l file.txt".to_string(), 0),
            ("which cargo".to_string(), 0),
            ("type cargo".to_string(), 0),
            ("date".to_string(), 0),
            ("whoami".to_string(), 0),
        ];

        let proc = from_session_commands(commands, Some("Filtered".to_string()));

        // Only cargo build and cargo test should remain
        assert_eq!(proc.steps.len(), 2);
        assert_eq!(proc.steps[0].command, "cargo build");
        assert_eq!(proc.steps[1].command, "cargo test");
    }

    #[test]
    fn test_from_session_commands_filters_failures() {
        use super::from_session_commands;

        let commands = vec![
            ("cargo build".to_string(), 0),
            ("cargo test".to_string(), 1), // failed
            ("cargo clippy".to_string(), 0),
        ];

        let proc = from_session_commands(commands, Some("With failures".to_string()));

        assert_eq!(proc.steps.len(), 2);
        assert_eq!(proc.steps[0].command, "cargo build");
        assert_eq!(proc.steps[1].command, "cargo clippy");
    }

    #[test]
    fn test_from_session_commands_auto_title() {
        use super::from_session_commands;

        let commands = vec![
            ("ls".to_string(), 0),          // trivial, skipped
            ("cargo build".to_string(), 0), // first meaningful
            ("cargo test".to_string(), 0),
        ];

        let proc = from_session_commands(commands, None);

        assert_eq!(proc.title, "Session: cargo build");
    }

    #[test]
    fn test_from_session_commands_auto_title_long_command() {
        use super::from_session_commands;

        let long_cmd = "a".repeat(100);
        let commands = vec![(long_cmd.clone(), 0)];

        let proc = from_session_commands(commands, None);

        // Title should be truncated to 60 chars + "..."
        assert!(proc.title.starts_with("Session: "));
        assert!(proc.title.ends_with("..."));
        assert!(proc.title.len() < 80);
    }

    #[test]
    fn test_from_session_commands_empty() {
        use super::from_session_commands;

        let commands: Vec<(String, i32)> = vec![];
        let proc = from_session_commands(commands, None);

        assert_eq!(proc.title, "Session procedure (empty)");
        assert_eq!(proc.steps.len(), 0);
    }

    #[test]
    fn test_from_session_commands_all_trivial() {
        use super::from_session_commands;

        let commands = vec![
            ("ls".to_string(), 0),
            ("pwd".to_string(), 0),
            ("cd /tmp".to_string(), 0),
        ];

        let proc = from_session_commands(commands, None);

        assert_eq!(proc.title, "Session procedure (empty)");
        assert_eq!(proc.steps.len(), 0);
    }

    #[test]
    fn test_is_trivial_command() {
        use super::is_trivial_command;

        assert!(is_trivial_command("ls"));
        assert!(is_trivial_command("ls -la"));
        assert!(is_trivial_command("cd /tmp"));
        assert!(is_trivial_command("pwd"));
        assert!(is_trivial_command("echo hello"));
        assert!(is_trivial_command("cat file.txt"));
        assert!(is_trivial_command("date"));
        assert!(is_trivial_command("whoami"));

        assert!(!is_trivial_command("cargo build"));
        assert!(!is_trivial_command("git status"));
        assert!(!is_trivial_command("make install"));
        assert!(!is_trivial_command("docker compose up"));
    }

    #[cfg(feature = "repl-sessions")]
    #[test]
    fn test_extract_bash_commands_from_session() {
        use super::extract_bash_commands_from_session;
        use std::path::PathBuf;
        use terraphim_sessions::{ContentBlock, Message, MessageRole, Session, SessionMetadata};

        let mut msg1 = Message::text(0, MessageRole::Assistant, "building");
        msg1.blocks.push(ContentBlock::ToolUse {
            id: "tu1".to_string(),
            name: "Bash".to_string(),
            input: serde_json::json!({"command": "cargo build"}),
        });

        let mut msg2 = Message::text(1, MessageRole::Tool, "result1");
        msg2.blocks.push(ContentBlock::ToolResult {
            tool_use_id: "tu1".to_string(),
            content: "success".to_string(),
            is_error: false,
        });

        let mut msg3 = Message::text(2, MessageRole::Assistant, "testing");
        msg3.blocks.push(ContentBlock::ToolUse {
            id: "tu2".to_string(),
            name: "Bash".to_string(),
            input: serde_json::json!({"command": "cargo test"}),
        });

        let mut msg4 = Message::text(3, MessageRole::Tool, "result2");
        msg4.blocks.push(ContentBlock::ToolResult {
            tool_use_id: "tu2".to_string(),
            content: "error".to_string(),
            is_error: true,
        });

        // Non-Bash tool use should be ignored
        let mut msg5 = Message::text(4, MessageRole::Assistant, "reading");
        msg5.blocks.push(ContentBlock::ToolUse {
            id: "tu3".to_string(),
            name: "Read".to_string(),
            input: serde_json::json!({"file_path": "/some/file.rs"}),
        });

        let session = Session {
            id: "test-session".to_string(),
            source: "test".to_string(),
            external_id: "test".to_string(),
            title: Some("Test session".to_string()),
            source_path: PathBuf::from("."),
            started_at: None,
            ended_at: None,
            messages: vec![msg1, msg2, msg3, msg4, msg5],
            metadata: SessionMetadata::default(),
        };

        let commands = extract_bash_commands_from_session(&session);

        assert_eq!(commands.len(), 2);
        assert_eq!(commands[0], ("cargo build".to_string(), 0));
        assert_eq!(commands[1], ("cargo test".to_string(), 1));
    }

    #[cfg(feature = "repl-sessions")]
    #[test]
    fn test_extract_and_convert_session_to_procedure() {
        use super::{extract_bash_commands_from_session, from_session_commands};
        use std::path::PathBuf;
        use terraphim_sessions::{ContentBlock, Message, MessageRole, Session, SessionMetadata};

        // Build a session with mixed commands
        let mut messages = Vec::new();

        let bash_cmds = vec![
            ("tu1", "ls -la", false),
            ("tu2", "cargo build --release", false),
            ("tu3", "cargo test", true), // failed
            ("tu4", "cd /tmp", false),   // trivial
            ("tu5", "cargo clippy", false),
        ];

        for (id, cmd, is_error) in &bash_cmds {
            let mut tool_msg = Message::text(messages.len(), MessageRole::Assistant, "cmd");
            tool_msg.blocks.push(ContentBlock::ToolUse {
                id: id.to_string(),
                name: "Bash".to_string(),
                input: serde_json::json!({"command": cmd}),
            });
            messages.push(tool_msg);

            let mut result_msg = Message::text(messages.len(), MessageRole::Tool, "result");
            result_msg.blocks.push(ContentBlock::ToolResult {
                tool_use_id: id.to_string(),
                content: "output".to_string(),
                is_error: *is_error,
            });
            messages.push(result_msg);
        }

        let session = Session {
            id: "integration-test".to_string(),
            source: "test".to_string(),
            external_id: "test".to_string(),
            title: Some("Integration test".to_string()),
            source_path: PathBuf::from("."),
            started_at: None,
            ended_at: None,
            messages,
            metadata: SessionMetadata::default(),
        };

        let commands = extract_bash_commands_from_session(&session);
        assert_eq!(commands.len(), 5); // All 5 Bash commands extracted

        let proc = from_session_commands(commands, None);
        // ls (trivial), cargo build (ok), cargo test (failed), cd (trivial), cargo clippy (ok)
        // Only cargo build and cargo clippy should remain
        assert_eq!(proc.steps.len(), 2);
        assert_eq!(proc.steps[0].command, "cargo build --release");
        assert_eq!(proc.steps[1].command, "cargo clippy");
        assert!(proc.title.starts_with("Session: cargo build --release"));
    }
}
