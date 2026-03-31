//! Input validation module for RLM security.
//!
//! This module provides security-focused validation functions for:
//! - Snapshot names (path traversal prevention)
//! - Code input size limits (DoS prevention)
//! - Session ID format validation

use crate::error::{RlmError, RlmResult};
use crate::types::SessionId;

/// Maximum code size (1MB = 1,048,576 bytes) to prevent DoS via memory exhaustion.
pub const MAX_CODE_SIZE: usize = 1_048_576;

/// Maximum input size for general inputs (10MB) to prevent memory exhaustion.
pub const MAX_INPUT_SIZE: usize = 10_485_760;

/// Maximum recursion depth for nested operations.
pub const MAX_RECURSION_DEPTH: u32 = 50;

/// Maximum snapshot name length.
pub const MAX_SNAPSHOT_NAME_LENGTH: usize = 256;

/// Validates a snapshot name for security.
///
/// # Security Considerations
///
/// Rejects names that could be used for path traversal attacks:
/// - Contains `..` (parent directory reference)
/// - Contains `/` or `\` (path separators)
/// - Contains null bytes
/// - Empty names
/// - Names exceeding MAX_SNAPSHOT_NAME_LENGTH
///
/// # Arguments
///
/// * `name` - The snapshot name to validate
///
/// # Returns
///
/// * `Ok(())` if the name is valid
/// * `Err(RlmError)` if the name is invalid
///
/// # Examples
///
/// ```
/// use terraphim_rlm::validation::validate_snapshot_name;
///
/// assert!(validate_snapshot_name("valid-snapshot").is_ok());
/// assert!(validate_snapshot_name("snapshot-v1.2.3").is_ok());
/// assert!(validate_snapshot_name("../etc/passwd").is_err()); // Path traversal
/// assert!(validate_snapshot_name("snap/name").is_err()); // Path separator
/// ```
pub fn validate_snapshot_name(name: &str) -> RlmResult<()> {
    // Check for empty name
    if name.is_empty() {
        return Err(RlmError::ConfigError {
            message: "Snapshot name cannot be empty".to_string(),
        });
    }

    // Check maximum length
    if name.len() > MAX_SNAPSHOT_NAME_LENGTH {
        return Err(RlmError::ConfigError {
            message: format!(
                "Snapshot name too long: {} bytes (max {})",
                name.len(),
                MAX_SNAPSHOT_NAME_LENGTH
            ),
        });
    }

    // Check for path traversal patterns
    if name.contains("..") {
        return Err(RlmError::ConfigError {
            message: format!("Snapshot name contains path traversal pattern: {}", name),
        });
    }

    // Check for path separators
    if name.contains('/') || name.contains('\\') {
        return Err(RlmError::ConfigError {
            message: format!("Snapshot name contains path separator: {}", name),
        });
    }

    // Check for null bytes
    if name.contains('\0') {
        return Err(RlmError::ConfigError {
            message: "Snapshot name contains null byte".to_string(),
        });
    }

    Ok(())
}

/// Validates code input size to prevent DoS via memory exhaustion.
///
/// # Security Considerations
///
/// Enforces MAX_CODE_SIZE limit on code inputs to prevent:
/// - Memory exhaustion attacks
/// - Excessive VM startup time due to large code volumes
/// - Storage exhaustion from large snapshots
///
/// # Arguments
///
/// * `code` - The code input to validate
///
/// # Returns
///
/// * `Ok(())` if the code size is within limits
/// * `Err(RlmError)` if the code exceeds MAX_CODE_SIZE
///
/// # Examples
///
/// ```
/// use terraphim_rlm::validation::{validate_code_input, MAX_CODE_SIZE};
///
/// let valid_code = "print('hello')";
/// assert!(validate_code_input(valid_code).is_ok());
///
/// let huge_code = "x".repeat(MAX_CODE_SIZE + 1);
/// assert!(validate_code_input(&huge_code).is_err());
/// ```
pub fn validate_code_input(code: &str) -> RlmResult<()> {
    let size = code.len();
    if size > MAX_CODE_SIZE {
        return Err(RlmError::ConfigError {
            message: format!(
                "Code size {} bytes exceeds maximum of {} bytes",
                size, MAX_CODE_SIZE
            ),
        });
    }
    Ok(())
}

/// Validates general input size.
///
/// Use this for non-code inputs that still need size limits.
///
/// # Arguments
///
/// * `input` - The input to validate
///
/// # Returns
///
/// * `Ok(())` if the input size is within limits
/// * `Err(RlmError)` if the input exceeds MAX_INPUT_SIZE
pub fn validate_input_size(input: &str) -> RlmResult<()> {
    let size = input.len();
    if size > MAX_INPUT_SIZE {
        return Err(RlmError::ConfigError {
            message: format!(
                "Input size {} bytes exceeds maximum of {} bytes",
                size, MAX_INPUT_SIZE
            ),
        });
    }
    Ok(())
}

/// Validates a session ID string format.
///
/// # Security Considerations
///
/// Ensures session IDs are valid UUIDs to prevent:
/// - Session fixation attacks with malformed IDs
/// - Injection of special characters into storage systems
/// - Information disclosure via error messages
///
/// # Arguments
///
/// * `session_id` - The session ID string to validate
///
/// # Returns
///
/// * `Ok(SessionId)` if the ID is a valid UUID
/// * `Err(RlmError)` if the ID format is invalid
///
/// # Examples
///
/// ```
/// use terraphim_rlm::validation::validate_session_id;
///
/// // Valid ULID (26 characters)
/// let result = validate_session_id("01ARZ3NDEKTSV4RRFFQ69G5FAV");
/// assert!(result.is_ok());
///
/// // Invalid formats
/// assert!(validate_session_id("not-a-ulid").is_err());
/// assert!(validate_session_id("").is_err());
/// assert!(validate_session_id("../etc/passwd").is_err());
/// ```
pub fn validate_session_id(session_id: &str) -> RlmResult<SessionId> {
    SessionId::from_string(session_id).map_err(|_| RlmError::InvalidSessionToken {
        token: session_id.to_string(),
    })
}

/// Validates recursion depth to prevent stack overflow.
///
/// # Arguments
///
/// * `depth` - Current recursion depth
///
/// # Returns
///
/// * `Ok(())` if depth is within limits
/// * `Err(RlmError)` if depth exceeds MAX_RECURSION_DEPTH
pub fn validate_recursion_depth(depth: u32) -> RlmResult<()> {
    if depth > MAX_RECURSION_DEPTH {
        return Err(RlmError::RecursionDepthExceeded {
            depth,
            max_depth: MAX_RECURSION_DEPTH,
        });
    }
    Ok(())
}

/// Combined validation for code execution requests.
///
/// Validates both the session ID and code input in one call.
///
/// # Arguments
///
/// * `session_id` - The session ID string
/// * `code` - The code to execute
///
/// # Returns
///
/// * `Ok((SessionId, &str))` if both are valid
/// * `Err(RlmError)` if either validation fails
pub fn validate_execution_request<'a>(
    session_id: &str,
    code: &'a str,
) -> RlmResult<(SessionId, &'a str)> {
    let sid = validate_session_id(session_id)?;
    validate_code_input(code)?;
    Ok((sid, code))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_snapshot_name_valid() {
        assert!(validate_snapshot_name("valid-snapshot").is_ok());
        assert!(validate_snapshot_name("snapshot-v1.2.3").is_ok());
        assert!(validate_snapshot_name("base").is_ok());
        assert!(validate_snapshot_name("a").is_ok());
        assert!(validate_snapshot_name("snapshot_with_underscores").is_ok());
        assert!(validate_snapshot_name("snapshot-with-dashes").is_ok());
        assert!(validate_snapshot_name("123numeric-start").is_ok());
    }

    #[test]
    fn test_validate_snapshot_name_path_traversal() {
        assert!(validate_snapshot_name("../etc/passwd").is_err());
        assert!(validate_snapshot_name("..\\windows\\system32").is_err());
        assert!(validate_snapshot_name("snapshot/../../../etc/passwd").is_err());
        assert!(validate_snapshot_name("..").is_err());
        assert!(validate_snapshot_name("...").is_err());
    }

    #[test]
    fn test_validate_snapshot_name_path_separators() {
        assert!(validate_snapshot_name("snap/name").is_err());
        assert!(validate_snapshot_name("snap\\name").is_err());
        assert!(validate_snapshot_name("/etc/passwd").is_err());
        assert!(validate_snapshot_name("C:\\Windows").is_err());
    }

    #[test]
    fn test_validate_snapshot_name_null_bytes() {
        assert!(validate_snapshot_name("snap\0name").is_err());
        assert!(validate_snapshot_name("\0").is_err());
        assert!(validate_snapshot_name("snapshot\0\0").is_err());
    }

    #[test]
    fn test_validate_snapshot_name_empty() {
        assert!(validate_snapshot_name("").is_err());
    }

    #[test]
    fn test_validate_snapshot_name_too_long() {
        let long_name = "a".repeat(MAX_SNAPSHOT_NAME_LENGTH + 1);
        assert!(validate_snapshot_name(&long_name).is_err());
    }

    #[test]
    fn test_validate_snapshot_name_max_length() {
        let max_name = "a".repeat(MAX_SNAPSHOT_NAME_LENGTH);
        assert!(validate_snapshot_name(&max_name).is_ok());
    }

    #[test]
    fn test_validate_code_input_valid() {
        assert!(validate_code_input("print('hello')").is_ok());
        assert!(validate_code_input("").is_ok());
        assert!(validate_code_input(&"x".repeat(MAX_CODE_SIZE)).is_ok());
    }

    #[test]
    fn test_validate_code_input_too_large() {
        let huge_code = "x".repeat(MAX_CODE_SIZE + 1);
        assert!(validate_code_input(&huge_code).is_err());
    }

    #[test]
    fn test_validate_input_size_valid() {
        assert!(validate_input_size("small input").is_ok());
        assert!(validate_input_size(&"x".repeat(MAX_INPUT_SIZE)).is_ok());
    }

    #[test]
    fn test_validate_input_size_too_large() {
        let huge_input = "x".repeat(MAX_INPUT_SIZE + 1);
        assert!(validate_input_size(&huge_input).is_err());
    }

    #[test]
    fn test_validate_session_id_valid() {
        // Valid ULID format (26 characters, Crockford base32)
        let valid_ulid = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
        assert!(validate_session_id(valid_ulid).is_ok());
    }

    #[test]
    fn test_validate_session_id_invalid() {
        assert!(validate_session_id("not-a-ulid").is_err());
        assert!(validate_session_id("").is_err());
        assert!(validate_session_id("../etc/passwd").is_err());
        assert!(validate_session_id("short").is_err());
        assert!(validate_session_id("550e8400-e29b-41d4-a716-446655440000").is_err()); // UUID format
        assert!(validate_session_id("01ARZ3NDEKTSV4RRFFQ69G5FA").is_err()); // Too short (25)
        assert!(validate_session_id("01ARZ3NDEKTSV4RRFFQ69G5FAVV").is_err()); // Too long (27)
    }

    #[test]
    fn test_validate_recursion_depth_valid() {
        assert!(validate_recursion_depth(0).is_ok());
        assert!(validate_recursion_depth(1).is_ok());
        assert!(validate_recursion_depth(MAX_RECURSION_DEPTH).is_ok());
    }

    #[test]
    fn test_validate_recursion_depth_exceeded() {
        assert!(validate_recursion_depth(MAX_RECURSION_DEPTH + 1).is_err());
        assert!(validate_recursion_depth(u32::MAX).is_err());
    }

    #[test]
    fn test_validate_execution_request_valid() {
        let session_id = "01ARZ3NDEKTSV4RRFFQ69G5FAV"; // Valid ULID
        let code = "print('hello')";
        let result = validate_execution_request(session_id, code);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_execution_request_invalid_session() {
        let session_id = "invalid-session";
        let code = "print('hello')";
        let result = validate_execution_request(session_id, code);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_execution_request_invalid_code() {
        let session_id = "01ARZ3NDEKTSV4RRFFQ69G5FAV"; // Valid ULID
        let code = "x".repeat(MAX_CODE_SIZE + 1);
        let result = validate_execution_request(session_id, &code);
        assert!(result.is_err());
    }
}
