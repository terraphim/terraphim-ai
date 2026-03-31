use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

const MAX_OUTPUT_BYTES: usize = 512 * 1024; // 512 KB

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepEnvelope {
    pub step_name: String,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    #[serde(default)]
    pub cost_usd: Option<f64>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub input_tokens: Option<u64>,
    #[serde(default)]
    pub output_tokens: Option<u64>,
    /// Path to temp file containing stdout (for downstream action steps).
    #[serde(default)]
    pub stdout_file: Option<String>,
}

impl StepEnvelope {
    /// Truncate stdout if it exceeds MAX_OUTPUT_BYTES.
    pub fn truncate_stdout(&mut self) {
        if self.stdout.len() > MAX_OUTPUT_BYTES {
            self.stdout.truncate(MAX_OUTPUT_BYTES);
            self.stdout.push_str("\n... [truncated at 512KB]");
        }
    }

    /// Truncate stderr if it exceeds MAX_OUTPUT_BYTES.
    pub fn truncate_stderr(&mut self) {
        if self.stderr.len() > MAX_OUTPUT_BYTES {
            self.stderr.truncate(MAX_OUTPUT_BYTES);
            self.stderr.push_str("\n... [truncated at 512KB]");
        }
    }

    /// Truncate both stdout and stderr.
    pub fn truncate_output(&mut self) {
        self.truncate_stdout();
        self.truncate_stderr();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_envelope() -> StepEnvelope {
        StepEnvelope {
            step_name: "test-step".to_string(),
            started_at: Utc::now(),
            finished_at: Utc::now(),
            exit_code: 0,
            stdout: "test output".to_string(),
            stderr: "".to_string(),
            cost_usd: Some(0.05),
            session_id: Some("sess-123".to_string()),
            input_tokens: Some(100),
            output_tokens: Some(200),
            stdout_file: Some("/tmp/stdout-123.txt".to_string()),
        }
    }

    #[test]
    fn test_envelope_serde_roundtrip() {
        let envelope = create_test_envelope();

        // Serialize to JSON
        let json = serde_json::to_string(&envelope).unwrap();

        // Deserialize back
        let deserialized: StepEnvelope = serde_json::from_str(&json).unwrap();

        // Verify all fields match
        assert_eq!(deserialized.step_name, envelope.step_name);
        assert_eq!(deserialized.exit_code, envelope.exit_code);
        assert_eq!(deserialized.stdout, envelope.stdout);
        assert_eq!(deserialized.stderr, envelope.stderr);
        assert_eq!(deserialized.cost_usd, envelope.cost_usd);
        assert_eq!(deserialized.session_id, envelope.session_id);
        assert_eq!(deserialized.input_tokens, envelope.input_tokens);
        assert_eq!(deserialized.output_tokens, envelope.output_tokens);
        assert_eq!(deserialized.stdout_file, envelope.stdout_file);
    }

    #[test]
    fn test_envelope_stdout_truncation() {
        let mut envelope = StepEnvelope {
            step_name: "big-output".to_string(),
            started_at: Utc::now(),
            finished_at: Utc::now(),
            exit_code: 0,
            stdout: "x".repeat(MAX_OUTPUT_BYTES + 1000),
            stderr: "".to_string(),
            cost_usd: None,
            session_id: None,
            input_tokens: None,
            output_tokens: None,
            stdout_file: None,
        };

        assert_eq!(envelope.stdout.len(), MAX_OUTPUT_BYTES + 1000);

        // Truncate stdout
        envelope.truncate_stdout();

        // Verify it was truncated to MAX_OUTPUT_BYTES plus the truncation message
        assert_eq!(
            envelope.stdout.len(),
            MAX_OUTPUT_BYTES + "\n... [truncated at 512KB]".len()
        );
        assert!(envelope.stdout.ends_with("\n... [truncated at 512KB]"));
    }

    #[test]
    fn test_envelope_stderr_truncation() {
        let mut envelope = StepEnvelope {
            step_name: "stderr-test".to_string(),
            started_at: Utc::now(),
            finished_at: Utc::now(),
            exit_code: 0,
            stdout: String::new(),
            stderr: "x".repeat(MAX_OUTPUT_BYTES + 1000),
            cost_usd: None,
            session_id: None,
            input_tokens: None,
            output_tokens: None,
            stdout_file: None,
        };

        assert_eq!(envelope.stderr.len(), MAX_OUTPUT_BYTES + 1000);

        // Truncate stderr
        envelope.truncate_stderr();

        // Verify it was truncated to MAX_OUTPUT_BYTES plus the truncation message
        assert_eq!(
            envelope.stderr.len(),
            MAX_OUTPUT_BYTES + "\n... [truncated at 512KB]".len()
        );
        assert!(envelope.stderr.ends_with("\n... [truncated at 512KB]"));
    }

    #[test]
    fn test_envelope_no_truncation_when_small() {
        let mut envelope = StepEnvelope {
            step_name: "small-output".to_string(),
            started_at: Utc::now(),
            finished_at: Utc::now(),
            exit_code: 0,
            stdout: "small output".to_string(),
            stderr: "".to_string(),
            cost_usd: None,
            session_id: None,
            input_tokens: None,
            output_tokens: None,
            stdout_file: None,
        };

        let original_len = envelope.stdout.len();
        envelope.truncate_stdout();

        // Should not be truncated
        assert_eq!(envelope.stdout.len(), original_len);
        assert_eq!(envelope.stdout, "small output");
    }

    #[test]
    fn test_envelope_exactly_at_limit() {
        let mut envelope = StepEnvelope {
            step_name: "exact-limit".to_string(),
            started_at: Utc::now(),
            finished_at: Utc::now(),
            exit_code: 0,
            stdout: "x".repeat(MAX_OUTPUT_BYTES),
            stderr: "".to_string(),
            cost_usd: None,
            session_id: None,
            input_tokens: None,
            output_tokens: None,
            stdout_file: None,
        };

        let original_len = envelope.stdout.len();
        envelope.truncate_stdout();

        // Exactly at limit should NOT be truncated
        assert_eq!(envelope.stdout.len(), original_len);
    }

    #[test]
    fn test_envelope_optional_fields() {
        // Test with all optional fields as None
        let envelope = StepEnvelope {
            step_name: "minimal".to_string(),
            started_at: Utc::now(),
            finished_at: Utc::now(),
            exit_code: 0,
            stdout: "output".to_string(),
            stderr: "error".to_string(),
            cost_usd: None,
            session_id: None,
            input_tokens: None,
            output_tokens: None,
            stdout_file: None,
        };

        let json = serde_json::to_string(&envelope).unwrap();
        let deserialized: StepEnvelope = serde_json::from_str(&json).unwrap();

        assert!(deserialized.cost_usd.is_none());
        assert!(deserialized.session_id.is_none());
        assert!(deserialized.input_tokens.is_none());
        assert!(deserialized.output_tokens.is_none());
        assert!(deserialized.stdout_file.is_none());
    }

    #[test]
    fn test_envelope_default_fields_in_json() {
        // Test that optional fields with defaults deserialize correctly
        let json = r#"{
            "step_name": "test",
            "started_at": "2024-01-15T10:30:00Z",
            "finished_at": "2024-01-15T10:31:00Z",
            "exit_code": 0,
            "stdout": "output",
            "stderr": ""
        }"#;

        let envelope: StepEnvelope = serde_json::from_str(json).unwrap();

        assert_eq!(envelope.step_name, "test");
        assert_eq!(envelope.exit_code, 0);
        assert!(envelope.cost_usd.is_none());
        assert!(envelope.session_id.is_none());
        assert!(envelope.input_tokens.is_none());
        assert!(envelope.output_tokens.is_none());
        assert!(envelope.stdout_file.is_none());
    }
}
