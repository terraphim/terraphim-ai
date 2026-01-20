/// Input Validation and Sanitization Module
///
/// Provides comprehensive input validation, sanitization, and security controls
/// for user-facing components, implementing OWASP-recommended security patterns
/// to prevent injection attacks and ensure data integrity.
use std::collections::HashSet;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ValidationError {
    #[error("Input cannot be empty")]
    EmptyInput,
    #[error("Input exceeds maximum length of {max} characters")]
    TooLong { max: usize },
    #[error("Input contains prohibited characters: {chars}")]
    ProhibitedCharacters { chars: String },
    #[error("Input contains potentially dangerous patterns")]
    DangerousPatterns,
    #[error("Search query too complex - exceeds query limits")]
    QueryTooComplex,
    #[error("Invalid file path format")]
    InvalidPath,
    #[error("Input contains control characters")]
    ControlCharacters,
}

/// Maximum lengths for different input types
pub mod limits {
    pub const MAX_SEARCH_QUERY: usize = 1000;
    pub const MAX_USERNAME: usize = 100;
    pub const MAX_FILENAME: usize = 255;
    pub const MAX_CHAT_MESSAGE: usize = 10000;
    pub const MAX_CONFIG_VALUE: usize = 5000;
    pub const MAX_URL_LENGTH: usize = 2048;
}

/// Sets of prohibited characters for different contexts
pub mod prohibited_chars {
    pub const CONTROL_CHARS: &str = "\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0a\x0b\x0c\x0d\x0e\x0f\x10\x11\x12\x13\x14\x15\x16\x17\x18\x19\x1a\x1b\x1c\x1d\x1e\x1f\x7f";

    // Characters dangerous for file operations
    pub const FILE_PATH: &str = "<>:\"|?*\\";

    // Characters dangerous for shell operations
    pub const SHELL: &str = ";&`$(){}[]|<>";

    // Characters dangerous for SQL (though we use parameterized queries)
    pub const SQL: &str = "'\"\\;--/*";

    // Characters dangerous for HTML injection
    pub const HTML: &str = "<>&\"'";
}

/// Dangerous patterns that could indicate injection attempts
pub const DANGEROUS_PATTERNS: &[&str] = &[
    // Script injection patterns
    "javascript:",
    "vbscript:",
    "data:text/html",
    "data:application",
    // Command injection patterns
    "rm -rf",
    "sudo ",
    "passwd",
    "curl ",
    "wget ",
    "nc ",
    "netcat ",
    // Path traversal patterns
    "../",
    "..\\",
    "/etc/",
    "/proc/",
    "/sys/",
    // Code execution patterns
    "eval(",
    "exec(",
    "system(",
    "shell_exec(",
    // File inclusion patterns
    "include(",
    "require(",
    "file_get_contents(",
];

/// Validates and sanitizes search query inputs
pub fn validate_search_query(input: &str) -> Result<String, ValidationError> {
    // Basic validation
    if input.is_empty() {
        return Err(ValidationError::EmptyInput);
    }

    if input.len() > limits::MAX_SEARCH_QUERY {
        return Err(ValidationError::TooLong {
            max: limits::MAX_SEARCH_QUERY,
        });
    }

    // Check for control characters
    if input
        .chars()
        .any(|c| c.is_control() && c != '\t' && c != '\n' && c != '\r')
    {
        return Err(ValidationError::ControlCharacters);
    }

    // Check for dangerous patterns
    let lowercase_input = input.to_lowercase();
    for pattern in DANGEROUS_PATTERNS {
        if lowercase_input.contains(pattern) {
            return Err(ValidationError::DangerousPatterns);
        }
    }

    // Sanitize by removing potentially harmful characters while preserving legitimate content
    let sanitized = input
        .chars()
        .filter(|c| {
            // Allow printable characters, spaces, tabs, newlines
            c.is_alphanumeric()
                || c.is_whitespace()
                || "!@#$%^&*()_+-=[]{}|;':\",./<>?".contains(*c)
        })
        .collect::<String>()
        .trim()
        .to_string();

    if sanitized.is_empty() {
        return Err(ValidationError::EmptyInput);
    }

    // Check query complexity (prevent DoS via complex regex-like patterns)
    if count_special_chars(&sanitized) > (sanitized.len() / 2) {
        return Err(ValidationError::QueryTooComplex);
    }

    Ok(sanitized)
}

/// Validates and sanitizes file path inputs
pub fn validate_file_path(input: &str) -> Result<String, ValidationError> {
    if input.is_empty() {
        return Err(ValidationError::EmptyInput);
    }

    if input.len() > limits::MAX_FILENAME {
        return Err(ValidationError::TooLong {
            max: limits::MAX_FILENAME,
        });
    }

    // Check for path traversal attempts
    if input.contains("../") || input.contains("..\\") {
        return Err(ValidationError::InvalidPath);
    }

    // Check for prohibited file path characters
    let prohibited_set: HashSet<char> = prohibited_chars::FILE_PATH.chars().collect();
    let found_prohibited: String = input
        .chars()
        .filter(|c| prohibited_set.contains(c))
        .collect();

    if !found_prohibited.is_empty() {
        return Err(ValidationError::ProhibitedCharacters {
            chars: found_prohibited,
        });
    }

    // Sanitize by removing prohibited characters
    let sanitized: String = input
        .chars()
        .filter(|c| !prohibited_set.contains(c))
        .collect();

    if sanitized.is_empty() {
        return Err(ValidationError::EmptyInput);
    }

    Ok(sanitized)
}

/// Validates username or identifier inputs
pub fn validate_username(input: &str) -> Result<String, ValidationError> {
    if input.is_empty() {
        return Err(ValidationError::EmptyInput);
    }

    if input.len() > limits::MAX_USERNAME {
        return Err(ValidationError::TooLong {
            max: limits::MAX_USERNAME,
        });
    }

    // Only allow alphanumeric characters, underscores, and hyphens
    if !input
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
        return Err(ValidationError::ProhibitedCharacters {
            chars: "Only alphanumeric, underscore, and hyphen allowed".to_string(),
        });
    }

    Ok(input.trim().to_string())
}

/// Validates chat message inputs
pub fn validate_chat_message(input: &str) -> Result<String, ValidationError> {
    if input.is_empty() {
        return Err(ValidationError::EmptyInput);
    }

    if input.len() > limits::MAX_CHAT_MESSAGE {
        return Err(ValidationError::TooLong {
            max: limits::MAX_CHAT_MESSAGE,
        });
    }

    // Remove control characters except common whitespace
    let sanitized = input
        .chars()
        .filter(|c| !c.is_control() || " \t\n\r".contains(*c))
        .collect();

    Ok(sanitized)
}

/// Validates configuration value inputs
pub fn validate_config_value(input: &str) -> Result<String, ValidationError> {
    if input.is_empty() {
        return Err(ValidationError::EmptyInput);
    }

    if input.len() > limits::MAX_CONFIG_VALUE {
        return Err(ValidationError::TooLong {
            max: limits::MAX_CONFIG_VALUE,
        });
    }

    // Check for JSON injection patterns
    if input.contains("</script>") || input.contains("javascript:") {
        return Err(ValidationError::DangerousPatterns);
    }

    Ok(input.trim().to_string())
}

/// Helper function to count special characters (non-alphanumeric)
fn count_special_chars(input: &str) -> usize {
    input.chars().filter(|c| !c.is_alphanumeric()).count()
}

/// Security information disclosure prevention for error messages
pub fn sanitize_error_message(internal_error: &str, user_context: bool) -> String {
    if !user_context {
        // For non-user contexts, return sanitized version
        return "An internal error occurred. Please try again.".to_string();
    }

    // For user contexts, sanitize the error message
    let sanitized = internal_error
        .replace("\\", "\\\\")
        .replace("\"", "\\\"")
        .replace("\n", "\\n")
        .replace("\r", "\\r")
        .replace("\t", "\\t");

    // Limit error message length to prevent information disclosure
    if sanitized.len() > 200 {
        format!("{}...", &sanitized[..197])
    } else {
        sanitized
    }
}

/// Comprehensive input validation for all user inputs
pub enum InputType {
    SearchQuery,
    FilePath,
    Username,
    ChatMessage,
    ConfigValue,
}

pub fn validate_input(input: &str, input_type: InputType) -> Result<String, ValidationError> {
    match input_type {
        InputType::SearchQuery => validate_search_query(input),
        InputType::FilePath => validate_file_path(input),
        InputType::Username => validate_username(input),
        InputType::ChatMessage => validate_chat_message(input),
        InputType::ConfigValue => validate_config_value(input),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_query_validation() {
        // Valid queries
        assert!(validate_search_query("test search").is_ok());
        assert!(validate_search_query("test+search").is_ok());
        assert!(validate_search_query("test \"quoted\"").is_ok());

        // Invalid queries
        assert!(validate_search_query("").is_err()); // Empty
        assert!(validate_search_query("javascript:alert(1)").is_err()); // Dangerous pattern
        assert!(validate_search_query("../etc/passwd").is_err()); // Path traversal

        // Very long query
        let long_query = "a".repeat(limits::MAX_SEARCH_QUERY + 1);
        assert!(validate_search_query(&long_query).is_err());
    }

    #[test]
    fn test_file_path_validation() {
        // Valid paths
        assert!(validate_file_path("document.txt").is_ok());
        assert!(validate_file_path("my-file_name.txt").is_ok());

        // Invalid paths
        assert!(validate_file_path("").is_err()); // Empty
        assert!(validate_file_path("../../../etc/passwd").is_err()); // Path traversal
        assert!(validate_file_path("file<name>.txt").is_err()); // Prohibited chars
    }

    #[test]
    fn test_username_validation() {
        // Valid usernames
        assert!(validate_username("user123").is_ok());
        assert!(validate_username("user_name").is_ok());
        assert!(validate_username("user-name").is_ok());

        // Invalid usernames
        assert!(validate_username("").is_err()); // Empty
        assert!(validate_username("user@name").is_err()); // Invalid char
        assert!(validate_username("user name").is_err()); // Space
    }

    #[test]
    fn test_error_sanitization() {
        let internal_error = "Database connection failed: Connection refused to localhost:5432";
        let sanitized = sanitize_error_message(internal_error, true);
        assert!(!sanitized.contains("localhost"));
        assert!(!sanitized.contains("5432"));
    }
}
