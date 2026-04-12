//! Procedure capture types for the learning system.
//!
//! This module provides types for capturing successful command sequences
//! (procedures) that can be replayed and refined over time.
//!
//! # Example
//!
//! ```
//! use terraphim_types::procedure::{CapturedProcedure, ProcedureStep, ProcedureConfidence};
//!
//! let mut procedure = CapturedProcedure::new(
//!     "install-rust".to_string(),
//!     "Install Rust toolchain".to_string(),
//!     "Steps to install Rust using rustup".to_string(),
//! );
//!
//! procedure.add_step(ProcedureStep {
//!     ordinal: 1,
//!     command: "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh".to_string(),
//!     precondition: Some("curl is installed".to_string()),
//!     postcondition: Some("rustup is installed".to_string()),
//!     working_dir: None,
//!     privileged: false,
//!     tags: vec!["install".to_string()],
//! });
//! ```

use serde::{Deserialize, Serialize};

/// A single step in a captured procedure.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProcedureStep {
    /// Step number (1-indexed)
    pub ordinal: u32,
    /// The command to execute
    pub command: String,
    /// Precondition that must be true before executing
    pub precondition: Option<String>,
    /// Postcondition that should be true after executing
    pub postcondition: Option<String>,
    /// Working directory for this step (optional)
    pub working_dir: Option<String>,
    /// Whether this step requires elevated privileges
    pub privileged: bool,
    /// Tags for categorization
    pub tags: Vec<String>,
}

/// Confidence metrics for a procedure based on execution history.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProcedureConfidence {
    /// Number of successful executions
    pub success_count: u32,
    /// Number of failed executions
    pub failure_count: u32,
    /// Computed confidence score (0.0 - 1.0)
    pub score: f64,
}

impl ProcedureConfidence {
    /// Create a new confidence tracker with zero counts.
    pub fn new() -> Self {
        Self {
            success_count: 0,
            failure_count: 0,
            score: 0.0,
        }
    }

    /// Record a successful execution.
    pub fn record_success(&mut self) {
        self.success_count += 1;
        self.recalculate_score();
    }

    /// Record a failed execution.
    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.recalculate_score();
    }

    /// Recalculate the confidence score.
    ///
    /// Score = success_count / (success_count + failure_count)
    /// Returns 0.0 if total count is 0.
    fn recalculate_score(&mut self) {
        let total = self.success_count + self.failure_count;
        if total == 0 {
            self.score = 0.0;
        } else {
            self.score = self.success_count as f64 / total as f64;
        }
    }

    /// Get the total number of executions.
    pub fn total_executions(&self) -> u32 {
        self.success_count + self.failure_count
    }

    /// Check if this procedure has high confidence (> 0.8).
    pub fn is_high_confidence(&self) -> bool {
        self.score > 0.8
    }
}

impl Default for ProcedureConfidence {
    fn default() -> Self {
        Self::new()
    }
}

/// A captured procedure with ordered steps and execution history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapturedProcedure {
    /// Unique identifier (UUID)
    pub id: String,
    /// Human-readable title
    pub title: String,
    /// Description of what this procedure does
    pub description: String,
    /// Ordered steps to execute
    pub steps: Vec<ProcedureStep>,
    /// Confidence metrics
    pub confidence: ProcedureConfidence,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Creation timestamp (ISO 8601)
    pub created_at: String,
    /// Last update timestamp (ISO 8601)
    pub updated_at: String,
    /// Source session ID if captured from a session
    pub source_session: Option<String>,
    /// Whether this procedure has been disabled (e.g., by auto-healing)
    #[serde(default)]
    pub disabled: bool,
}

impl CapturedProcedure {
    /// Create a new captured procedure.
    pub fn new(id: String, title: String, description: String) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id,
            title,
            description,
            steps: Vec::new(),
            confidence: ProcedureConfidence::new(),
            tags: Vec::new(),
            created_at: now.clone(),
            updated_at: now,
            source_session: None,
            disabled: false,
        }
    }

    /// Add a step to the procedure.
    pub fn add_step(&mut self, step: ProcedureStep) {
        self.steps.push(step);
        self.touch();
    }

    /// Add multiple steps to the procedure.
    pub fn add_steps(&mut self, steps: Vec<ProcedureStep>) {
        self.steps.extend(steps);
        self.touch();
    }

    /// Set the source session ID.
    pub fn with_source_session(mut self, session_id: String) -> Self {
        self.source_session = Some(session_id);
        self
    }

    /// Add tags.
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Set the confidence metrics.
    pub fn with_confidence(mut self, confidence: ProcedureConfidence) -> Self {
        self.confidence = confidence;
        self
    }

    /// Update the timestamp to now.
    fn touch(&mut self) {
        self.updated_at = chrono::Utc::now().to_rfc3339();
    }

    /// Record a successful execution.
    pub fn record_success(&mut self) {
        self.confidence.record_success();
        self.touch();
    }

    /// Record a failed execution.
    pub fn record_failure(&mut self) {
        self.confidence.record_failure();
        self.touch();
    }

    /// Get the number of steps.
    pub fn step_count(&self) -> usize {
        self.steps.len()
    }

    /// Check if this procedure has any steps.
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// Merge steps from another procedure into this one.
    ///
    /// This is used for deduplication - when a similar procedure is found,
    /// we can merge the steps to consolidate knowledge.
    pub fn merge_steps(&mut self, other: &CapturedProcedure) {
        // Only merge if both have steps
        if other.steps.is_empty() {
            return;
        }

        // Simple merge: add steps that don't already exist
        for other_step in &other.steps {
            let exists = self.steps.iter().any(|s| s.command == other_step.command);
            if !exists {
                let mut new_step = other_step.clone();
                new_step.ordinal = self.steps.len() as u32 + 1;
                self.steps.push(new_step);
            }
        }

        // Merge tags
        for tag in &other.tags {
            if !self.tags.contains(tag) {
                self.tags.push(tag.clone());
            }
        }

        self.touch();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_procedure_step_roundtrip() {
        let step = ProcedureStep {
            ordinal: 1,
            command: "git status".to_string(),
            precondition: Some("git is installed".to_string()),
            postcondition: Some("status is displayed".to_string()),
            working_dir: Some("/tmp".to_string()),
            privileged: false,
            tags: vec!["git".to_string(), "status".to_string()],
        };

        let json = serde_json::to_string(&step).unwrap();
        let deserialized: ProcedureStep = serde_json::from_str(&json).unwrap();

        assert_eq!(step, deserialized);
    }

    #[test]
    fn test_confidence_new_is_zero() {
        let confidence = ProcedureConfidence::new();
        assert_eq!(confidence.success_count, 0);
        assert_eq!(confidence.failure_count, 0);
        assert_eq!(confidence.score, 0.0);
    }

    #[test]
    fn test_confidence_record_success() {
        let mut confidence = ProcedureConfidence::new();
        confidence.record_success();

        assert_eq!(confidence.success_count, 1);
        assert_eq!(confidence.failure_count, 0);
        assert_eq!(confidence.score, 1.0);
    }

    #[test]
    fn test_confidence_record_failure() {
        let mut confidence = ProcedureConfidence::new();
        confidence.record_failure();

        assert_eq!(confidence.success_count, 0);
        assert_eq!(confidence.failure_count, 1);
        assert_eq!(confidence.score, 0.0);
    }

    #[test]
    fn test_confidence_mixed_scoring() {
        let mut confidence = ProcedureConfidence::new();

        // 3 successes, 1 failure = 0.75
        confidence.record_success();
        confidence.record_success();
        confidence.record_success();
        confidence.record_failure();

        assert_eq!(confidence.success_count, 3);
        assert_eq!(confidence.failure_count, 1);
        assert_eq!(confidence.score, 0.75);
        assert!(!confidence.is_high_confidence());

        // One more success = 4/5 = 0.8
        confidence.record_success();
        assert_eq!(confidence.score, 0.8);
        assert!(!confidence.is_high_confidence()); // strictly > 0.8

        // One more success = 5/6 = ~0.833
        confidence.record_success();
        assert!(confidence.score > 0.8);
        assert!(confidence.is_high_confidence());
    }

    #[test]
    fn test_captured_procedure_json_roundtrip() {
        let mut procedure = CapturedProcedure::new(
            "test-id".to_string(),
            "Test Procedure".to_string(),
            "A test procedure".to_string(),
        );

        procedure.add_step(ProcedureStep {
            ordinal: 1,
            command: "echo hello".to_string(),
            precondition: None,
            postcondition: Some("hello is printed".to_string()),
            working_dir: None,
            privileged: false,
            tags: vec!["test".to_string()],
        });

        let json = serde_json::to_string(&procedure).unwrap();
        let deserialized: CapturedProcedure = serde_json::from_str(&json).unwrap();

        assert_eq!(procedure.id, deserialized.id);
        assert_eq!(procedure.title, deserialized.title);
        assert_eq!(procedure.description, deserialized.description);
        assert_eq!(procedure.steps.len(), deserialized.steps.len());
        assert_eq!(procedure.steps[0].command, deserialized.steps[0].command);
    }

    #[test]
    fn test_captured_procedure_add_step() {
        let mut procedure = CapturedProcedure::new(
            "test-id".to_string(),
            "Test".to_string(),
            "Test desc".to_string(),
        );

        assert_eq!(procedure.step_count(), 0);

        procedure.add_step(ProcedureStep {
            ordinal: 1,
            command: "cmd1".to_string(),
            precondition: None,
            postcondition: None,
            working_dir: None,
            privileged: false,
            tags: vec![],
        });

        assert_eq!(procedure.step_count(), 1);
    }

    #[test]
    fn test_captured_procedure_record_execution() {
        let mut procedure = CapturedProcedure::new(
            "test-id".to_string(),
            "Test".to_string(),
            "Test desc".to_string(),
        );

        let original_updated_at = procedure.updated_at.clone();

        procedure.record_success();
        assert_eq!(procedure.confidence.success_count, 1);

        procedure.record_failure();
        assert_eq!(procedure.confidence.failure_count, 1);

        // updated_at should have changed
        assert_ne!(procedure.updated_at, original_updated_at);
    }

    #[test]
    fn test_captured_procedure_merge_steps() {
        let mut proc1 = CapturedProcedure::new(
            "proc1".to_string(),
            "Procedure 1".to_string(),
            "First procedure".to_string(),
        );

        proc1.add_step(ProcedureStep {
            ordinal: 1,
            command: "cmd1".to_string(),
            precondition: None,
            postcondition: None,
            working_dir: None,
            privileged: false,
            tags: vec!["tag1".to_string()],
        });

        let mut proc2 = CapturedProcedure::new(
            "proc2".to_string(),
            "Procedure 2".to_string(),
            "Second procedure".to_string(),
        );

        proc2.add_step(ProcedureStep {
            ordinal: 1,
            command: "cmd1".to_string(), // Same command
            precondition: None,
            postcondition: None,
            working_dir: None,
            privileged: false,
            tags: vec!["tag2".to_string()],
        });

        proc2.add_step(ProcedureStep {
            ordinal: 2,
            command: "cmd2".to_string(), // New command
            precondition: None,
            postcondition: None,
            working_dir: None,
            privileged: false,
            tags: vec!["tag3".to_string()],
        });

        proc1.merge_steps(&proc2);

        // Should have 2 steps (cmd1 only once, plus cmd2)
        assert_eq!(proc1.step_count(), 2);

        // proc2 has empty procedure-level tags, so no tags should be merged
        // (step-level tags are not merged, only procedure-level tags)
        assert!(proc1.tags.is_empty());
    }
}
