//! Validation utilities for RLM.
//!
//! This module provides validation functions for user inputs,
//! snapshot names, and other data that needs sanitization.

use crate::error::RlmError;

/// Validates a snapshot name to prevent path traversal attacks.
///
/// # Arguments
///
/// * `name` - The snapshot name to validate
///
/// # Returns
///
/// Ok(()) if valid, Err otherwise
pub fn validate_snapshot_name(name: &str) -> Result<(), RlmError> {
    // Check for path traversal attempts
    if name.contains("..") || name.contains('/') || name.contains('\\') {
        return Err(RlmError::ConfigError {
            message: format!(
                "Invalid snapshot name '{}': contains path traversal characters",
                name
            ),
        });
    }

    // Check length
    if name.is_empty() {
        return Err(RlmError::ConfigError {
            message: "Snapshot name cannot be empty".to_string(),
        });
    }

    if name.len() > 256 {
        return Err(RlmError::ConfigError {
            message: format!(
                "Snapshot name too long: {} characters (max 256)",
                name.len()
            ),
        });
    }

    Ok(())
}

/// Validates code input for basic safety.
///
/// # Arguments
///
/// * `code` - The code to validate
///
/// # Returns
///
/// Ok(()) if valid, Err otherwise
pub fn validate_code_input(code: &str) -> Result<(), RlmError> {
    // Basic length validation
    if code.is_empty() {
        return Err(RlmError::CommandParseFailed {
            message: "Code input cannot be empty".to_string(),
        });
    }

    // Check for extremely large inputs (100MB limit)
    if code.len() > 100_000_000 {
        return Err(RlmError::CommandParseFailed {
            message: format!("Code input too large: {} bytes (max 100MB)", code.len()),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_snapshot_name_valid() {
        assert!(validate_snapshot_name("valid-name").is_ok());
        assert!(validate_snapshot_name("snapshot_123").is_ok());
        assert!(validate_snapshot_name("test.snapshot").is_ok());
    }

    #[test]
    fn test_validate_snapshot_name_path_traversal() {
        assert!(validate_snapshot_name("../etc/passwd").is_err());
        assert!(validate_snapshot_name("path/to/snapshot").is_err());
        assert!(validate_snapshot_name("snapshot\\windows").is_err());
    }

    #[test]
    fn test_validate_snapshot_name_empty() {
        assert!(validate_snapshot_name("").is_err());
    }

    #[test]
    fn test_validate_code_input() {
        assert!(validate_code_input("print('hello')").is_ok());
        assert!(validate_code_input("").is_err());
    }
}
