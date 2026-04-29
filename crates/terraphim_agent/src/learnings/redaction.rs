//! Secret redaction using regex pattern matching.
//!
//! This module provides regex-based redaction of secrets like AWS keys,
//! API tokens, connection strings, and environment variables from text
//! before storage in the learnings directory.
//!
//! # Usage
//!
//! ```rust
//! use terraphim_agent::learnings::redaction::redact;
//!
//! let input = "AWS_KEY=AKIAIOSFODNN7EXAMPLE connected";
//! let redacted = redact(input);
//! assert!(redacted.contains("[REDACTED:aws-key]"));
//! ```

use std::borrow::Cow;
use std::sync::OnceLock;

use regex::Regex;
use serde::Deserialize;

/// Configuration for secret redaction.
///
/// Loaded from `[learnings.redaction]` section of `.terraphim/learning-capture.toml`.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct RedactionConfig {
    /// Enable redaction (default: true)
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Additional custom regex patterns beyond built-ins
    #[serde(default)]
    pub custom_patterns: Vec<String>,
}

fn default_true() -> bool {
    true
}

/// A compiled redaction pattern with its replacement text.
struct RedactionPattern {
    regex: Regex,
    name: String,
}

/// Built-in secret patterns for redaction.
///
/// Each pattern maps a regex to a short pattern name used in the
/// replacement text `[REDACTED:<pattern-name>]`.
const BUILTIN_PATTERNS: &[(&str, &str)] = &[
    // GCP service account keys (JSON fragment detection) — specific before generic
    (
        r#""private_key"\s*:\s*"-----BEGIN PRIVATE KEY-----[^"]+-----END PRIVATE KEY-----""#,
        "gcp-key",
    ),
    (r#""private_key_id"\s*:\s*"[a-f0-9]{40}""#, "gcp-key-id"),
    // AWS Access Key IDs (AKIA followed by 16 alphanumeric chars)
    (r"AKIA[A-Z0-9]{16}", "aws-key"),
    // AWS Secret Access Keys (40 char base64-ish)
    (r"[A-Za-z0-9/+=]{40}", "aws-secret"),
    (r#""private_key_id"\s*:\s*"[a-f0-9]{40}""#, "gcp-key-id"),
    // Azure keys (various formats)
    (
        r"[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}",
        "azure-guid",
    ),
    (
        r"DefaultEndpointsProtocol=https;AccountName=[^;]+;AccountKey=[^;]+",
        "azure-connection",
    ),
    // Generic API keys with common prefixes
    (r"sk-[A-Za-z0-9-_]{20,}", "api-key-sk"),
    (r"xox[baprs]-[A-Za-z0-9-]+", "slack-token"),
    (r"ghp_[A-Za-z0-9]{36}", "github-token"),
    (r"gho_[A-Za-z0-9]{36}", "github-oauth"),
    // Connection strings with embedded credentials
    (r"postgresql://[^@\s]+:[^@\s]+@", "db-connection"),
    (r"mysql://[^@\s]+:[^@\s]+@", "db-connection"),
    (r"mongodb(\+srv)?://[^@\s]+:[^@\s]+@", "db-connection"),
    (r"redis://[^@\s]+:[^@\s]+@", "db-connection"),
];

/// Environment variable names whose values should be redacted.
const ENV_VAR_NAMES: &[&str] = &[
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

/// Global cache for compiled built-in patterns.
static BUILTIN_REDACTOR: OnceLock<Vec<RedactionPattern>> = OnceLock::new();

/// Compile built-in patterns into a vector of RedactionPattern structs.
fn compile_builtin_patterns() -> &'static Vec<RedactionPattern> {
    BUILTIN_REDACTOR.get_or_init(|| {
        BUILTIN_PATTERNS
            .iter()
            .filter_map(|(pattern, name)| {
                Regex::new(pattern).ok().map(|regex| RedactionPattern {
                    regex,
                    name: (*name).to_string(),
                })
            })
            .collect()
    })
}

/// Redact secrets from text using built-in regex pattern matching.
///
/// This function applies regex patterns to find and replace secret patterns
/// like AWS keys, API tokens, connection strings, and environment variables.
/// Redacted values are replaced with `[REDACTED:<pattern-name>]` so downstream
/// readers can still see that something was there.
///
/// # Arguments
///
/// * `text` - The text to redact
///
/// # Returns
///
/// A `Cow<str>` containing the text with secrets replaced. If no secrets
/// were found, returns `Cow::Borrowed(text)` for zero-allocation.
///
/// # Example
///
/// ```
/// use terraphim_agent::learnings::redaction::redact;
///
/// let input = "AWS_KEY=AKIAIOSFODNN7EXAMPLE connected";
/// let redacted = redact(input);
/// assert!(redacted.contains("[REDACTED:aws-key]"));
/// ```
pub fn redact(text: &str) -> Cow<'_, str> {
    redact_with_config(text, None)
}

/// Redact secrets from text with optional configuration.
///
/// If `config` is `None`, uses built-in patterns only.
/// If `config` is provided and `enabled` is `false`, returns the text unchanged.
/// Custom patterns from config are applied after built-in patterns.
pub fn redact_with_config<'a>(text: &'a str, config: Option<&'a RedactionConfig>) -> Cow<'a, str> {
    // If config says disabled, return as-is
    if let Some(cfg) = config {
        if !cfg.enabled {
            return Cow::Borrowed(text);
        }
    }

    // First, strip environment variable values
    let mut result = strip_env_vars(text);
    let mut changed = result != text;

    // Apply built-in patterns
    let patterns = compile_builtin_patterns();
    for pattern in patterns {
        if pattern.regex.is_match(&result) {
            let replacement = format!("[REDACTED:{}]", pattern.name);
            result = pattern
                .regex
                .replace_all(&result, replacement.as_str())
                .to_string();
            changed = true;
        }
    }

    // Apply custom patterns from config
    if let Some(cfg) = config {
        for custom_pattern in &cfg.custom_patterns {
            if let Ok(re) = Regex::new(custom_pattern) {
                if re.is_match(&result) {
                    result = re.replace_all(&result, "[REDACTED:custom]").to_string();
                    changed = true;
                }
            }
        }
    }

    if changed {
        Cow::Owned(result)
    } else {
        Cow::Borrowed(text)
    }
}

/// Strip environment variable values from text.
///
/// Matches patterns like `VAR=value` or `VAR="value"` and replaces
/// the value with `[REDACTED:env]`.
fn strip_env_vars(text: &str) -> String {
    let mut result = text.to_string();

    for var_name in ENV_VAR_NAMES {
        // Match VAR=value or VAR="value" or VAR='value'
        let patterns = [
            format!(r"{}\s*=\s*[^\s]+", regex::escape(var_name)),
            format!(r#"{}\s*=\s*"[^"]+""#, regex::escape(var_name)),
            format!(r#"{}\s*=\s*'[^']+'"#, regex::escape(var_name)),
        ];

        for pattern in patterns {
            if let Ok(re) = Regex::new(&pattern) {
                let replacement = format!("{}=[REDACTED:env]", var_name);
                result = re.replace_all(&result, replacement.as_str()).to_string();
            }
        }
    }

    result
}

/// Check if text contains potential secrets.
///
/// This is a quick check that can be used before capture to warn users.
pub fn contains_secrets(text: &str) -> bool {
    let patterns = [
        r"AKIA[A-Z0-9]{16}",
        r"sk-[A-Za-z0-9]{20,}",
        r"password\s*=",
        r"secret\s*=",
        r"api_key\s*=",
    ];

    for pattern in patterns {
        if let Ok(re) = Regex::new(pattern) {
            if re.is_match(text) {
                return true;
            }
        }
    }

    false
}

/// Legacy entry point for backward compatibility.
///
/// Delegates to [`redact`]. Prefer `redact` for new code.
pub fn redact_secrets(text: &str) -> String {
    redact(text).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_aws_key() {
        let input = "Using key AKIAIOSFODNN7EXAMPLE to connect";
        let redacted = redact(input);
        assert!(redacted.contains("[REDACTED:aws-key]"));
        assert!(!redacted.contains("AKIAIOSFODNN7EXAMPLE"));
    }

    #[test]
    fn test_redact_aws_secret() {
        let input = "secret: wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
        let redacted = redact(input);
        assert!(redacted.contains("[REDACTED:aws-secret]"));
    }

    #[test]
    fn test_redact_gcp_key() {
        let input = r#"{"private_key": "-----BEGIN PRIVATE KEY-----\nMIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQC5Z5Z5Z5Z5Z5Z5\n-----END PRIVATE KEY-----"}"#;
        let redacted = redact(input);
        assert!(redacted.contains("[REDACTED:gcp-key]"));
    }

    #[test]
    fn test_redact_gcp_key_id() {
        let input = r#"{"private_key_id": "1234567890abcdef1234567890abcdef12345678"}"#;
        let redacted = redact(input);
        assert!(redacted.contains("[REDACTED:gcp-key-id]"));
    }

    #[test]
    fn test_redact_azure_guid() {
        let input = "subscription: 12345678-1234-1234-1234-123456789012";
        let redacted = redact(input);
        assert!(redacted.contains("[REDACTED:azure-guid]"));
    }

    #[test]
    fn test_redact_azure_connection() {
        let input = "DefaultEndpointsProtocol=https;AccountName=myaccount;AccountKey=mykey123==";
        let redacted = redact(input);
        assert!(redacted.contains("[REDACTED:azure-connection]"));
    }

    #[test]
    fn test_redact_api_key_sk() {
        let input = "Key: sk-proj-abcdefghijklmnopqrstuvwxyz";
        let redacted = redact(input);
        assert!(redacted.contains("[REDACTED:api-key-sk]"));
    }

    #[test]
    fn test_redact_slack_token() {
        // Use a fake token that matches the regex but avoids triggering
        // GitHub secret scanning in test code.
        let input = "token: xoxb-test-fake-token-12345";
        let redacted = redact(input);
        assert!(redacted.contains("[REDACTED:slack-token]"));
    }

    #[test]
    fn test_redact_github_token() {
        let input = "ghp_abcdefghijklmnopqrstuvwxyz1234567890ab";
        let redacted = redact(input);
        assert!(redacted.contains("[REDACTED:github-token]"));
    }

    #[test]
    fn test_redact_connection_string_postgresql() {
        let input = "postgresql://user:password@localhost:5432/db";
        let redacted = redact(input);
        assert!(redacted.contains("[REDACTED:db-connection]"));
        assert!(!redacted.contains("password"));
    }

    #[test]
    fn test_redact_connection_string_mysql() {
        let input = "mysql://admin:secret123@db.example.com:3306/app";
        let redacted = redact(input);
        assert!(redacted.contains("[REDACTED:db-connection]"));
    }

    #[test]
    fn test_redact_connection_string_mongodb() {
        let input = "mongodb+srv://user:pass@cluster.mongodb.net/db";
        let redacted = redact(input);
        assert!(redacted.contains("[REDACTED:db-connection]"));
    }

    #[test]
    fn test_redact_connection_string_redis() {
        let input = "redis://user:pass@redis.example.com:6379";
        let redacted = redact(input);
        assert!(redacted.contains("[REDACTED:db-connection]"));
    }

    #[test]
    fn test_strip_env_vars() {
        let input = r#"DATABASE_URL=postgres://user:pass@host API_KEY="secret123""#;
        let redacted = redact(input);
        assert!(redacted.contains("DATABASE_URL=[REDACTED:env]"));
        assert!(redacted.contains("API_KEY=[REDACTED:env]"));
        assert!(!redacted.contains("secret123"));
    }

    #[test]
    fn test_no_secrets_unchanged() {
        let input = "cargo build --release";
        let redacted = redact(input);
        assert!(matches!(redacted, Cow::Borrowed(_)));
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
        let redacted = redact(input);
        assert!(redacted.contains("[REDACTED:aws-key]"));
        assert!(redacted.contains("[REDACTED:api-key-sk]"));
    }

    #[test]
    fn test_custom_pattern_via_config() {
        let config = RedactionConfig {
            enabled: true,
            custom_patterns: vec![r"SECRET_[A-Z0-9]{8}".to_string()],
        };
        let input = "my SECRET_ABCD1234 token";
        let redacted = redact_with_config(input, Some(&config));
        assert!(redacted.contains("[REDACTED:custom]"));
        assert!(!redacted.contains("SECRET_ABCD1234"));
    }

    #[test]
    fn test_config_disabled() {
        let config = RedactionConfig {
            enabled: false,
            custom_patterns: vec![],
        };
        let input = "AKIAIOSFODNN7EXAMPLE";
        let redacted = redact_with_config(input, Some(&config));
        assert!(redacted.contains("AKIAIOSFODNN7EXAMPLE"));
    }

    #[test]
    fn test_redact_secrets_backward_compat() {
        let input = "AKIAIOSFODNN7EXAMPLE";
        let redacted = redact_secrets(input);
        assert!(redacted.contains("[REDACTED:aws-key]"));
    }

    /// Property test: for any input, redacted output length is bounded.
    ///
    /// The redacted text should never be more than 2x the input length
    /// (replacement tokens are shorter than most secrets).
    #[test]
    fn test_redaction_bounded_expansion() {
        // Test with various inputs including edge cases
        let test_cases = [
            "",
            "a",
            "cargo build",
            "AKIAIOSFODNN7EXAMPLE",
            "postgresql://user:password@localhost/db",
            "multiple secrets: AKIAIOSFODNN7EXAMPLE and sk-proj-abc123",
            &"a".repeat(1000),
            &"xoxb-".repeat(100),
        ];

        for input in &test_cases {
            let redacted = redact(input);
            assert!(
                redacted.len() <= input.len() * 2,
                "Redacted text too long for input of length {}: got length {}",
                input.len(),
                redacted.len()
            );
        }
    }
}
