//! Validation results and status tracking
//!
//! This module defines the data structures used to track validation results,
//! status, and reporting throughout the validation process.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Validation status for individual checks
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ValidationStatus {
    Pending,
    InProgress,
    Passed,
    Failed,
    Skipped,
    Error,
}

impl std::fmt::Display for ValidationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationStatus::Pending => write!(f, "Pending"),
            ValidationStatus::InProgress => write!(f, "InProgress"),
            ValidationStatus::Passed => write!(f, "Passed"),
            ValidationStatus::Failed => write!(f, "Failed"),
            ValidationStatus::Skipped => write!(f, "Skipped"),
            ValidationStatus::Error => write!(f, "Error"),
        }
    }
}

impl ValidationStatus {
    /// Check if the status represents a successful validation
    pub fn is_success(&self) -> bool {
        matches!(self, ValidationStatus::Passed)
    }

    /// Check if the status represents a failure
    pub fn is_failure(&self) -> bool {
        matches!(self, ValidationStatus::Failed | ValidationStatus::Error)
    }

    /// Check if the status is final (no longer pending)
    pub fn is_final(&self) -> bool {
        matches!(
            self,
            ValidationStatus::Passed
                | ValidationStatus::Failed
                | ValidationStatus::Skipped
                | ValidationStatus::Error
        )
    }
}

/// Severity level for validation issues
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Info,
    Warning,
    Error,
    Critical,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Info => write!(f, "Info"),
            Severity::Warning => write!(f, "Warning"),
            Severity::Error => write!(f, "Error"),
            Severity::Critical => write!(f, "Critical"),
        }
    }
}

/// Individual validation issue or finding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub id: Uuid,
    pub severity: Severity,
    pub category: String,
    pub title: String,
    pub description: String,
    pub recommendation: Option<String>,
    pub artifact_id: Option<Uuid>,
    pub timestamp: DateTime<Utc>,
}

impl ValidationIssue {
    /// Create a new validation issue
    pub fn new(severity: Severity, category: String, title: String, description: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            severity,
            category,
            title,
            description,
            recommendation: None,
            artifact_id: None,
            timestamp: Utc::now(),
        }
    }

    /// Add a recommendation to the issue
    pub fn with_recommendation(mut self, recommendation: String) -> Self {
        self.recommendation = Some(recommendation);
        self
    }

    /// Associate the issue with an artifact
    pub fn with_artifact(mut self, artifact_id: Uuid) -> Self {
        self.artifact_id = Some(artifact_id);
        self
    }
}

/// Result of a single validation check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub id: Uuid,
    pub name: String,
    pub category: String,
    pub status: ValidationStatus,
    pub duration_ms: u64,
    pub issues: Vec<ValidationIssue>,
    pub metadata: HashMap<String, String>,
    pub timestamp: DateTime<Utc>,
}

impl ValidationResult {
    /// Create a new validation result
    pub fn new(name: String, category: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            category,
            status: ValidationStatus::Pending,
            duration_ms: 0,
            issues: Vec::new(),
            metadata: HashMap::new(),
            timestamp: Utc::now(),
        }
    }

    /// Mark the validation as started
    pub fn start(&mut self) {
        self.status = ValidationStatus::InProgress;
        self.timestamp = Utc::now();
    }

    /// Mark the validation as passed
    pub fn pass(&mut self, duration_ms: u64) {
        self.status = ValidationStatus::Passed;
        self.duration_ms = duration_ms;
    }

    /// Mark the validation as failed
    pub fn fail(&mut self, duration_ms: u64, issues: Vec<ValidationIssue>) {
        self.status = ValidationStatus::Failed;
        self.duration_ms = duration_ms;
        self.issues = issues;
    }

    /// Mark the validation as having an error
    pub fn error(&mut self, duration_ms: u64, error_message: String) {
        self.status = ValidationStatus::Error;
        self.duration_ms = duration_ms;
        let issue = ValidationIssue::new(
            Severity::Critical,
            "system".to_string(),
            "Validation Error".to_string(),
            error_message,
        );
        self.issues.push(issue);
    }

    /// Skip the validation
    pub fn skip(&mut self, reason: String) {
        self.status = ValidationStatus::Skipped;
        let issue = ValidationIssue::new(
            Severity::Info,
            "system".to_string(),
            "Validation Skipped".to_string(),
            reason,
        );
        self.issues.push(issue);
    }

    /// Add metadata to the result
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }

    /// Get issues by severity
    pub fn get_issues_by_severity(&self, severity: Severity) -> Vec<&ValidationIssue> {
        self.issues
            .iter()
            .filter(|issue| issue.severity == severity)
            .collect()
    }

    /// Check if the validation has critical issues
    pub fn has_critical_issues(&self) -> bool {
        self.issues
            .iter()
            .any(|issue| issue.severity == Severity::Critical)
    }
}

/// Collection of validation results for a complete validation run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationSummary {
    pub id: Uuid,
    pub version: String,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub total_duration_ms: u64,
    pub results: HashMap<Uuid, ValidationResult>,
    pub overall_status: ValidationStatus,
}

impl ValidationSummary {
    /// Create a new validation summary
    pub fn new(version: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            version,
            start_time: Utc::now(),
            end_time: None,
            total_duration_ms: 0,
            results: HashMap::new(),
            overall_status: ValidationStatus::Pending,
        }
    }

    /// Add a validation result
    pub fn add_result(&mut self, result: ValidationResult) {
        self.results.insert(result.id, result);
        self.update_overall_status();
    }

    /// Update the overall validation status
    fn update_overall_status(&mut self) {
        if self.results.is_empty() {
            self.overall_status = ValidationStatus::Pending;
            return;
        }

        let statuses: Vec<_> = self.results.values().map(|result| &result.status).collect();

        // If any validation failed or had an error, the overall status is failed
        if statuses
            .iter()
            .any(|status| matches!(status, ValidationStatus::Failed | ValidationStatus::Error))
        {
            self.overall_status = ValidationStatus::Failed;
        }
        // If any validation is still in progress, the overall status is in progress
        else if statuses
            .iter()
            .any(|status| matches!(status, ValidationStatus::InProgress))
        {
            self.overall_status = ValidationStatus::InProgress;
        }
        // If all validations passed, the overall status is passed
        else if statuses
            .iter()
            .all(|status| matches!(status, ValidationStatus::Passed | ValidationStatus::Skipped))
        {
            self.overall_status = ValidationStatus::Passed;
        }
        // Otherwise, the status is pending
        else {
            self.overall_status = ValidationStatus::Pending;
        }
    }

    /// Complete the validation run
    pub fn complete(&mut self) {
        self.end_time = Some(Utc::now());
        if let Some(end_time) = self.end_time {
            self.total_duration_ms = (end_time - self.start_time).num_milliseconds() as u64;
        }
        self.update_overall_status();
    }

    /// Get validation statistics
    pub fn get_statistics(&self) -> ValidationStatistics {
        let total_validations = self.results.len();
        let passed_validations = self
            .results
            .values()
            .filter(|result| result.status.is_success())
            .count();
        let failed_validations = self
            .results
            .values()
            .filter(|result| result.status.is_failure())
            .count();
        let skipped_validations = self
            .results
            .values()
            .filter(|result| matches!(result.status, ValidationStatus::Skipped))
            .count();

        let total_issues: usize = self
            .results
            .values()
            .map(|result| result.issues.len())
            .sum();

        let critical_issues: usize = self
            .results
            .values()
            .map(|result| result.get_issues_by_severity(Severity::Critical).len())
            .sum();

        ValidationStatistics {
            total_validations,
            passed_validations,
            failed_validations,
            skipped_validations,
            total_issues,
            critical_issues,
        }
    }
}

/// Validation statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationStatistics {
    pub total_validations: usize,
    pub passed_validations: usize,
    pub failed_validations: usize,
    pub skipped_validations: usize,
    pub total_issues: usize,
    pub critical_issues: usize,
}

impl ValidationStatistics {
    /// Calculate success rate
    pub fn success_rate(&self) -> f64 {
        if self.total_validations == 0 {
            0.0
        } else {
            self.passed_validations as f64 / self.total_validations as f64
        }
    }

    /// Calculate failure rate
    pub fn failure_rate(&self) -> f64 {
        if self.total_validations == 0 {
            0.0
        } else {
            self.failed_validations as f64 / self.total_validations as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_status() {
        assert!(ValidationStatus::Passed.is_success());
        assert!(ValidationStatus::Failed.is_failure());
        assert!(ValidationStatus::Error.is_failure());
        assert!(!ValidationStatus::Pending.is_final());
        assert!(ValidationStatus::Passed.is_final());
    }

    #[test]
    fn test_validation_result() {
        let mut result = ValidationResult::new("test".to_string(), "unit".to_string());

        result.start();
        assert_eq!(result.status, ValidationStatus::InProgress);

        result.pass(100);
        assert_eq!(result.status, ValidationStatus::Passed);
        assert_eq!(result.duration_ms, 100);
    }

    #[test]
    fn test_validation_summary() {
        let mut summary = ValidationSummary::new("1.0.0".to_string());

        let mut result = ValidationResult::new("test".to_string(), "unit".to_string());
        result.pass(50);

        summary.add_result(result);
        assert_eq!(summary.overall_status, ValidationStatus::Passed);

        let stats = summary.get_statistics();
        assert_eq!(stats.total_validations, 1);
        assert_eq!(stats.passed_validations, 1);
        assert_eq!(stats.success_rate(), 1.0);
    }
}
