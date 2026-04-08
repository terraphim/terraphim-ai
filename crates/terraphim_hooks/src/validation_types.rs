//! Validation types for command and operation validation.
//!
//! Defines core types used across all validation hooks and services.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Outcome of a validation operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationOutcome {
    /// Validation passed, operation may proceed
    Allow,
    /// Validation passed with warning, operation may proceed
    Warn(String),
    /// Validation failed, operation MUST be blocked
    Block(String),
    /// Validation modified the operation, use modified content
    Modify(String),
}

/// Error type for validation failures.
#[derive(Error, Debug, Clone, Serialize, Deserialize)]
#[error("validation error: {message}")]
pub struct ValidationError {
    /// Human readable error message
    pub message: String,
    /// Optional machine readable error code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

/// Result of a validation operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Final outcome of the validation
    pub outcome: ValidationOutcome,
    /// Original input text that was validated
    pub original: String,
    /// Execution time in milliseconds
    pub duration_ms: u64,
    /// Error information if validation failed internally
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ValidationError>,
}

impl ValidationResult {
    /// Create a successful validation result.
    pub fn allow(original: String, duration_ms: u64) -> Self {
        Self {
            outcome: ValidationOutcome::Allow,
            original,
            duration_ms,
            error: None,
        }
    }

    /// Create a validation result with warning.
    pub fn warn(original: String, message: String, duration_ms: u64) -> Self {
        Self {
            outcome: ValidationOutcome::Warn(message),
            original,
            duration_ms,
            error: None,
        }
    }

    /// Create a blocked validation result.
    pub fn block(original: String, reason: String, duration_ms: u64) -> Self {
        Self {
            outcome: ValidationOutcome::Block(reason),
            original,
            duration_ms,
            error: None,
        }
    }

    /// Create a modified validation result.
    pub fn modify(original: String, modified: String, duration_ms: u64) -> Self {
        Self {
            outcome: ValidationOutcome::Modify(modified),
            original,
            duration_ms,
            error: None,
        }
    }

    /// Create a fail-open result when validation fails internally.
    pub fn fail_open(original: String, error: ValidationError, duration_ms: u64) -> Self {
        Self {
            outcome: ValidationOutcome::Allow,
            original,
            duration_ms,
            error: Some(error),
        }
    }

    /// Returns true if this validation passed (Allow or Warn).
    pub fn is_allowed(&self) -> bool {
        !matches!(self.outcome, ValidationOutcome::Block(_))
    }

    /// Returns true if this validation should block execution.
    pub fn should_block(&self) -> bool {
        matches!(self.outcome, ValidationOutcome::Block(_))
    }

    /// Returns true if validation produced a modified result.
    pub fn is_modified(&self) -> bool {
        matches!(self.outcome, ValidationOutcome::Modify(_))
    }

    /// Get the final text after validation.
    pub fn final_text(&self) -> &str {
        match &self.outcome {
            ValidationOutcome::Modify(text) => text,
            _ => &self.original,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_result_allow() {
        let result = ValidationResult::allow("test command".to_string(), 42);
        assert!(result.is_allowed());
        assert!(!result.should_block());
        assert!(!result.is_modified());
        assert_eq!(result.final_text(), "test command");
        assert_eq!(result.duration_ms, 42);
    }

    #[test]
    fn test_validation_result_warn() {
        let result = ValidationResult::warn(
            "test command".to_string(),
            "deprecated command".to_string(),
            42,
        );
        assert!(result.is_allowed());
        assert!(!result.should_block());
        assert!(!result.is_modified());
        if let ValidationOutcome::Warn(msg) = result.outcome {
            assert_eq!(msg, "deprecated command");
        } else {
            panic!("Expected Warn outcome");
        }
    }

    #[test]
    fn test_validation_result_block() {
        let result = ValidationResult::block(
            "dangerous command".to_string(),
            "command is blacklisted".to_string(),
            42,
        );
        assert!(!result.is_allowed());
        assert!(result.should_block());
        assert!(!result.is_modified());
        if let ValidationOutcome::Block(reason) = result.outcome {
            assert_eq!(reason, "command is blacklisted");
        } else {
            panic!("Expected Block outcome");
        }
    }

    #[test]
    fn test_validation_result_modify() {
        let result =
            ValidationResult::modify("npm install".to_string(), "bun install".to_string(), 42);
        assert!(result.is_allowed());
        assert!(!result.should_block());
        assert!(result.is_modified());
        assert_eq!(result.final_text(), "bun install");
        if let ValidationOutcome::Modify(modified) = result.outcome {
            assert_eq!(modified, "bun install");
        } else {
            panic!("Expected Modify outcome");
        }
    }

    #[test]
    fn test_validation_result_fail_open() {
        let error = ValidationError {
            message: "validation service unavailable".to_string(),
            code: Some("SERVICE_UNAVAILABLE".to_string()),
        };
        let result = ValidationResult::fail_open("test command".to_string(), error, 42);
        assert!(result.is_allowed());
        assert!(!result.should_block());
        assert!(result.error.is_some());
    }

    #[test]
    fn test_validation_result_serde_round_trip() {
        let original = "npm install express";
        let result =
            ValidationResult::modify(original.to_string(), "bun install express".to_string(), 123);

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: ValidationResult = serde_json::from_str(&json).unwrap();

        assert!(deserialized.is_modified());
        assert_eq!(deserialized.original, original);
        assert_eq!(deserialized.final_text(), "bun install express");
        assert_eq!(deserialized.duration_ms, 123);
    }

    #[test]
    fn test_validation_outcome_serialization() {
        let outcome = ValidationOutcome::Allow;
        let json = serde_json::to_string(&outcome).unwrap();
        assert_eq!(json, "\"allow\"");

        let outcome = ValidationOutcome::Block("reason".to_string());
        let json = serde_json::to_string(&outcome).unwrap();
        assert_eq!(json, r#"{"block":"reason"}"#);
    }
}
