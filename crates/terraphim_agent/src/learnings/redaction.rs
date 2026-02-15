//! Secret redaction using regex pattern matching.
//!
//! This module provides regex-based redaction of secrets like AWS keys,
//! API tokens, and connection strings from text before storage.

/// Standard secret patterns for redaction.
/// Patterns are matched using regex.
const SECRET_PATTERNS: &[(&str, &str)] = &[
    // AWS Access Key IDs (AKIA followed by 16 alphanumeric chars)
    (r"AKIA[A-Z0-9]{16}", "[AWS_KEY_REDACTED]"),
    // AWS Secret Access Keys (40 char base64-ish)
    (r"[A-Za-z0-9/+=]{40}", "[AWS_SECRET_REDACTED]"),
    // Generic API keys with common prefixes
    (r"sk-[A-Za-z0-9-_]{20,}", "[OPENAI_KEY_REDACTED]"),
    (r"xox[baprs]-[A-Za-z0-9-]+", "[SLACK_TOKEN_REDACTED]"),
    (r"ghp_[A-Za-z0-9]{36}", "[GITHUB_TOKEN_REDACTED]"),
    (r"gho_[A-Za-z0-9]{36}", "[GITHUB_TOKEN_REDACTED]"),
    // Connection strings
    (r"postgresql://[^@\s]+:[^@\s]+@", "postgresql://[REDACTED]@"),
    (r"mysql://[^@\s]+:[^@\s]+@", "mysql://[REDACTED]@"),
    (
        r"mongodb(\+srv)?://[^@\s]+:[^@\s]+@",
        "mongodb://[REDACTED]@",
    ),
    (r"redis://[^@\s]+:[^@\s]+@", "redis://[REDACTED]@"),
];

/// Environment variable patterns to strip entirely
const ENV_VAR_PATTERNS: &[&str] = &[
    "AWS_ACCESS_KEY_ID",
    "AWS_SECRET_ACCESS_KEY",
    "AWS_SESSION_TOKEN",
    "DATABASE_URL",
    "API_KEY",
    "SECRET_KEY",
    "PASSWORD",
    "TOKEN",
    "AUTH",
    "CREDENTIAL",
];

/// Redact secrets from text using regex pattern matching.
///
/// This function applies regex patterns to find and replace secret patterns
/// like AWS keys, API tokens, and connection strings.
///
/// # Arguments
///
/// * `text` - The text to redact
///
/// # Returns
///
/// The text with secrets replaced by `[REDACTED]` placeholders.
///
/// # Example
///
/// ```
/// use terraphim_agent::learnings::redact_secrets;
///
/// let input = "AWS_KEY=AKIAIOSFODNN7EXAMPLE connected";
/// let redacted = redact_secrets(input);
/// assert!(redacted.contains("[AWS_KEY_REDACTED]"));
/// ```
pub fn redact_secrets(text: &str) -> String {
    // First, strip environment variable values
    let mut result = strip_env_vars(text);

    // Apply regex-based redaction patterns
    for (pattern, replacement) in SECRET_PATTERNS {
        if let Ok(re) = regex::Regex::new(pattern) {
            result = re.replace_all(&result, *replacement).to_string();
        }
    }

    result
}

/// Strip environment variable values from text.
///
/// Matches patterns like `VAR=value` or `VAR="value"` and replaces
/// the value with `[ENV_REDACTED]`.
fn strip_env_vars(text: &str) -> String {
    let mut result = text.to_string();

    for var_name in ENV_VAR_PATTERNS {
        // Match VAR=value or VAR="value" or VAR='value'
        let pattern_unquoted = format!("{0}\\s*=\\s*[^\\s]+", var_name);
        let pattern_double = format!("{0}\\s*=\\s*\"[^\"]+\"", var_name);
        let pattern_single = format!("{0}\\s*=\\s*'[^']+'", var_name);
        let patterns = [pattern_unquoted, pattern_double, pattern_single];

        for pattern in patterns {
            if let Ok(re) = regex::Regex::new(&pattern) {
                let replacement = format!("{}=[ENV_REDACTED]", var_name);
                result = re.replace_all(&result, replacement.as_str()).to_string();
            }
        }
    }

    result
}

/// Check if text contains potential secrets.
///
/// This is a quick check that can be used before capture to warn users.
#[allow(dead_code)]
pub fn contains_secrets(text: &str) -> bool {
    // Check for common secret patterns
    let patterns = [
        r"AKIA[A-Z0-9]{16}",
        r"sk-[A-Za-z0-9]{20,}",
        r"password\s*=",
        r"secret\s*=",
        r"api_key\s*=",
    ];

    for pattern in patterns {
        if let Ok(re) = regex::Regex::new(pattern) {
            if re.is_match(text) {
                return true;
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_aws_key() {
        let input = "Using key AKIAIOSFODNN7EXAMPLE to connect";
        let redacted = redact_secrets(input);
        assert!(redacted.contains("[AWS_KEY_REDACTED]"));
        assert!(!redacted.contains("AKIAIOSFODNN7EXAMPLE"));
    }

    #[test]
    fn test_redact_connection_string() {
        let input = "postgresql://user:password@localhost:5432/db";
        let redacted = redact_secrets(input);
        assert!(redacted.contains("[REDACTED]"));
        assert!(!redacted.contains("password"));
    }

    #[test]
    fn test_strip_env_vars() {
        let input = r#"DATABASE_URL=postgres://user:pass@host API_KEY="secret123""#;
        let stripped = strip_env_vars(input);
        assert!(stripped.contains("DATABASE_URL=[ENV_REDACTED]"));
        assert!(stripped.contains("API_KEY=[ENV_REDACTED]"));
        assert!(!stripped.contains("secret123"));
    }

    #[test]
    fn test_no_secrets_unchanged() {
        let input = "cargo build --release";
        let redacted = redact_secrets(input);
        assert_eq!(redacted, input);
    }

    #[test]
    fn test_contains_secrets() {
        assert!(contains_secrets("AKIAIOSFODNN7EXAMPLE"));
        assert!(contains_secrets("password=secret"));
        assert!(contains_secrets("api_key=abc123"));
        assert!(!contains_secrets("cargo build"));
        assert!(!contains_secrets("npm install"));
    }

    #[test]
    fn test_redact_multiple_secrets() {
        let input = "Key: AKIAIOSFODNN7EXAMPLE and sk-proj-abcdefghijklmnopqrst";
        let redacted = redact_secrets(input);
        assert!(redacted.contains("[AWS_KEY_REDACTED]"));
        assert!(redacted.contains("[OPENAI_KEY_REDACTED]"));
    }
}
