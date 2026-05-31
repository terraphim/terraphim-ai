//! Redaction utilities for agent output.
//!
//! Provides pattern-based scrubbing of sensitive content (API keys, tokens,
//! secrets, passwords) from agent stdout/stderr before storage or remote
//! posting.

use regex::Regex;

/// Default patterns that indicate sensitive content in agent output.
///
/// Each pattern must capture the secret value as its **last capture group**.
/// Prefix groups (if any) are preserved; the last group is replaced with
/// `***REDACTED***`.
pub const DEFAULT_REDACTION_PATTERNS: &[&str] = &[
    // API keys with various separators (prefix group 1, value group 2)
    r"(?i)(api[_-]?key\s*[:=]\s*)([^\s]+)",
    // Tokens with various separators (prefix group 1, value group 2)
    r"(?i)(token\s*[:=]\s*)([^\s]+)",
    // Secrets with various separators (prefix group 1, value group 2)
    r"(?i)(secret\s*[:=]\s*)([^\s]+)",
    // Passwords with various separators (prefix group 1, value group 2)
    r"(?i)(password\s*[:=]\s*)([^\s]+)",
    // OpenAI-style API keys (value group 1, no prefix)
    r"(?i)(sk-[a-zA-Z0-9]{20,})",
    // GitHub personal access tokens (value group 1, no prefix)
    r"(?i)(ghp_[a-zA-Z0-9]{36})",
    // Generic bearer tokens (prefix group 1, value group 2)
    r"(?i)(bearer\s+)([a-zA-Z0-9_\-]{20,})",
];

/// Redact sensitive patterns from a string, replacing matches with `***REDACTED***`.
///
/// Preserves prefix capture groups (if any) and replaces only the last
/// capture group (the secret value).
pub fn redact(input: &str) -> String {
    let mut result = input.to_string();

    for pattern in DEFAULT_REDACTION_PATTERNS {
        if let Ok(re) = Regex::new(pattern) {
            result = re
                .replace_all(&result, |caps: &regex::Captures| {
                    let group_count = caps.len();

                    // No capture groups beyond the full match -> redact entire match
                    if group_count <= 2 {
                        return "***REDACTED***".to_string();
                    }

                    // Preserve all capture groups except the last one
                    let mut replacement = String::new();
                    for i in 1..group_count - 1 {
                        if let Some(m) = caps.get(i) {
                            replacement.push_str(m.as_str());
                        }
                    }
                    replacement.push_str("***REDACTED***");
                    replacement
                })
                .to_string();
        }
    }

    result
}

/// Verify that a string contains no apparent secrets.
///
/// Returns `true` if no redaction patterns match (i.e. the string is clean).
/// Strings that have already been redacted (contain `***REDACTED***`) are
/// considered clean.
pub fn verify_redacted(input: &str) -> bool {
    // Already-redacted strings are clean
    if input.contains("***REDACTED***") {
        return true;
    }

    for pattern in DEFAULT_REDACTION_PATTERNS {
        if let Ok(re) = Regex::new(pattern) {
            if re.is_match(input) {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_api_key() {
        let input = "api_key=sk-abc123xyz789";
        let result = redact(input);
        assert!(result.contains("api_key="));
        assert!(result.contains("***REDACTED***"));
        assert!(!result.contains("sk-abc123xyz789"));
    }

    #[test]
    fn test_redact_token() {
        let input = "Authorization: token ghp_abcdef1234567890abcdef1234567890abcd";
        let result = redact(input);
        assert!(result.contains("Authorization: token "));
        assert!(result.contains("***REDACTED***"));
        assert!(!result.contains("ghp_abcdef"));
    }

    #[test]
    fn test_redact_secret() {
        let input = "secret=my-hidden-value";
        let result = redact(input);
        assert!(result.contains("secret="));
        assert!(result.contains("***REDACTED***"));
        assert!(!result.contains("my-hidden-value"));
    }

    #[test]
    fn test_redact_password() {
        let input = "password=SuperSecret123!";
        let result = redact(input);
        assert!(result.contains("password="));
        assert!(result.contains("***REDACTED***"));
        assert!(!result.contains("SuperSecret123!"));
    }

    #[test]
    fn test_redact_sk_key() {
        let input = "The key is sk-abcdefghijklmnopqrstuvwxyz123456";
        let result = redact(input);
        assert!(result.contains("The key is "));
        assert!(result.contains("***REDACTED***"));
        assert!(!result.contains("sk-abcdefghijklmnopqrstuvwxyz123456"));
    }

    #[test]
    fn test_redact_bearer_token() {
        let input = "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9";
        let result = redact(input);
        assert!(result.contains("Authorization: Bearer "));
        assert!(result.contains("***REDACTED***"));
        assert!(!result.contains("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9"));
    }

    #[test]
    fn test_no_false_positives_on_safe_text() {
        let input = "This is normal output with no secrets. The api_key is not set.";
        let result = redact(input);
        // Should not contain REDACTED since there's no value after api_key=
        assert!(!result.contains("***REDACTED***"));
    }

    #[test]
    fn test_verify_redacted_detects_leak() {
        let clean = "This is safe output";
        let dirty = "api_key=secret123";

        assert!(verify_redacted(clean));
        assert!(!verify_redacted(dirty));
    }

    #[test]
    fn test_verify_redacted_after_redaction() {
        let dirty = "api_key=secret123\ntoken=abc";
        let redacted = redact(dirty);
        // After redaction, the string should be considered clean
        assert!(verify_redacted(&redacted));
        // No raw secrets should remain
        assert!(!redacted.contains("secret123"));
        assert!(!redacted.contains("=abc"));
    }

    #[test]
    fn test_redact_preserves_structure() {
        let input = "Config:\napi_key=secret\ntimeout=30s\ntoken=abc";
        let result = redact(input);
        assert!(result.contains("Config:"));
        assert!(result.contains("timeout=30s"));
        assert!(result.contains("api_key="));
        assert!(result.contains("token="));
    }

    #[test]
    fn test_redact_multiple_secrets() {
        let input = "api_key=first_secret token=second_secret";
        let result = redact(input);
        // Both secrets should be redacted
        assert!(!result.contains("first_secret"));
        assert!(!result.contains("second_secret"));
        assert_eq!(result.matches("***REDACTED***").count(), 2);
    }
}
