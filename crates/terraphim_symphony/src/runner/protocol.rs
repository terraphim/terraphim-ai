//! Codex app-server JSON-RPC protocol types.
//!
//! Defines the message types for line-delimited JSON communication
//! with the coding-agent app-server over stdio.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A JSON-RPC request (client -> server or server -> client).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    /// Request ID (integer or string).
    pub id: serde_json::Value,
    /// Method name.
    pub method: String,
    /// Parameters.
    #[serde(default)]
    pub params: serde_json::Value,
}

/// A JSON-RPC response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    /// Response ID matching the request.
    pub id: serde_json::Value,
    /// Result (present on success).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Error (present on failure).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

/// A JSON-RPC notification (no `id` field).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcNotification {
    /// Method name.
    pub method: String,
    /// Parameters.
    #[serde(default)]
    pub params: serde_json::Value,
}

/// A JSON-RPC error payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    /// Error code.
    pub code: i64,
    /// Error message.
    pub message: String,
    /// Optional additional data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// Parsed message from the app-server stdout.
#[derive(Debug, Clone)]
pub enum AppServerMessage {
    /// A response to a client request.
    Response(JsonRpcResponse),
    /// A server-initiated notification.
    Notification(JsonRpcNotification),
    /// A server-initiated request (e.g. approval).
    Request(JsonRpcRequest),
    /// A line that could not be parsed as JSON-RPC.
    Malformed(String),
}

impl AppServerMessage {
    /// Try to parse a JSON line into an app-server message.
    pub fn parse_line(line: &str) -> Self {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return AppServerMessage::Malformed(String::new());
        }

        let value: serde_json::Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => return AppServerMessage::Malformed(trimmed.to_string()),
        };

        let obj = match value.as_object() {
            Some(o) => o,
            None => return AppServerMessage::Malformed(trimmed.to_string()),
        };

        // Distinguish by presence of `id` and/or `method`
        let has_id = obj.contains_key("id");
        let has_method = obj.contains_key("method");
        let has_result = obj.contains_key("result");
        let has_error = obj.contains_key("error");

        if has_id && (has_result || has_error) && !has_method {
            // Response
            match serde_json::from_value(value) {
                Ok(resp) => AppServerMessage::Response(resp),
                Err(_) => AppServerMessage::Malformed(trimmed.to_string()),
            }
        } else if has_id && has_method {
            // Server-initiated request
            match serde_json::from_value(value) {
                Ok(req) => AppServerMessage::Request(req),
                Err(_) => AppServerMessage::Malformed(trimmed.to_string()),
            }
        } else if has_method && !has_id {
            // Notification
            match serde_json::from_value(value) {
                Ok(notif) => AppServerMessage::Notification(notif),
                Err(_) => AppServerMessage::Malformed(trimmed.to_string()),
            }
        } else {
            AppServerMessage::Malformed(trimmed.to_string())
        }
    }
}

/// Events emitted by the agent runner to the orchestrator.
#[derive(Debug, Clone)]
pub enum AgentEvent {
    /// Agent session has been established.
    SessionStarted {
        session_id: String,
        thread_id: String,
        turn_id: String,
        pid: Option<u32>,
    },
    /// A turn completed successfully.
    TurnCompleted { turn_id: String, turn_count: u32 },
    /// A turn failed.
    TurnFailed { turn_id: String, reason: String },
    /// An approval was auto-approved.
    ApprovalAutoApproved { approval_type: String },
    /// An unsupported tool call was rejected.
    UnsupportedToolCall { tool_name: String },
    /// A general notification from the agent.
    Notification { message: String },
    /// Token usage update.
    TokenUsage {
        input_tokens: u64,
        output_tokens: u64,
        total_tokens: u64,
    },
    /// Startup failed.
    StartupFailed { reason: String },
}

/// Token usage counters.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenCounts {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
}

/// Aggregate token totals for the service.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenTotals {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    /// Aggregate runtime seconds across all completed sessions.
    pub seconds_running: f64,
}

/// Severity of a review finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FindingSeverity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

/// Category of a review finding (maps to the 6 review groups).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingCategory {
    Security,
    Architecture,
    Performance,
    Quality,
    Domain,
    DesignQuality,
}

/// A single structured finding from a review agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewFinding {
    pub file: String,
    #[serde(default)]
    pub line: u32,
    pub severity: FindingSeverity,
    pub category: FindingCategory,
    pub finding: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
    #[serde(default = "default_confidence")]
    pub confidence: f64,
}

fn default_confidence() -> f64 {
    0.5
}

/// Output schema for a single review agent's results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewAgentOutput {
    pub agent: String,
    pub findings: Vec<ReviewFinding>,
    pub summary: String,
    pub pass: bool,
}

/// ADF envelope message types for swarm orchestration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "envelope_type", rename_all = "snake_case")]
pub enum AdfEnvelope {
    ReviewCommand {
        correlation_id: Uuid,
        agent_name: String,
        group: FindingCategory,
        git_ref: String,
        worktree_path: String,
        changed_files: Vec<String>,
        dispatched_at: DateTime<Utc>,
    },
    ReviewResponse {
        correlation_id: Uuid,
        output: ReviewAgentOutput,
        duration_ms: u64,
        completed_at: DateTime<Utc>,
    },
    ReviewError {
        correlation_id: Uuid,
        agent_name: String,
        reason: String,
        failed_at: DateTime<Utc>,
    },
    ReviewCancel {
        correlation_id: Uuid,
        reason: String,
    },
}

impl AdfEnvelope {
    pub fn correlation_id(&self) -> Uuid {
        match self {
            AdfEnvelope::ReviewCommand { correlation_id, .. }
            | AdfEnvelope::ReviewResponse { correlation_id, .. }
            | AdfEnvelope::ReviewError { correlation_id, .. }
            | AdfEnvelope::ReviewCancel { correlation_id, .. } => *correlation_id,
        }
    }

    pub fn to_jsonl(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    pub fn from_jsonl(line: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(line.trim())
    }
}

/// Deduplicate findings by (file, line, category).
/// When duplicates exist, keep the highest-severity finding.
pub fn deduplicate_findings(findings: Vec<ReviewFinding>) -> Vec<ReviewFinding> {
    use std::collections::HashMap;
    let mut best: HashMap<(String, u32, FindingCategory), ReviewFinding> = HashMap::new();
    for finding in findings {
        let key = (finding.file.clone(), finding.line, finding.category);
        best.entry(key)
            .and_modify(|existing| {
                if finding.severity > existing.severity {
                    *existing = finding.clone();
                }
            })
            .or_insert(finding);
    }
    let mut result: Vec<ReviewFinding> = best.into_values().collect();
    #[allow(clippy::unnecessary_sort_by)]
    result.sort_by(|a, b| {
        b.severity
            .cmp(&a.severity)
            .then_with(|| a.file.cmp(&b.file))
            .then_with(|| a.line.cmp(&b.line))
    });
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_response() {
        let line = r#"{"id":1,"result":{"thread":{"id":"t1"}}}"#;
        match AppServerMessage::parse_line(line) {
            AppServerMessage::Response(r) => {
                assert_eq!(r.id, serde_json::json!(1));
                assert!(r.result.is_some());
                assert!(r.error.is_none());
            }
            other => panic!("expected Response, got {other:?}"),
        }
    }

    #[test]
    fn parse_notification() {
        let line = r#"{"method":"turn/completed","params":{"turnId":"t1"}}"#;
        match AppServerMessage::parse_line(line) {
            AppServerMessage::Notification(n) => {
                assert_eq!(n.method, "turn/completed");
            }
            other => panic!("expected Notification, got {other:?}"),
        }
    }

    #[test]
    fn parse_server_request() {
        let line = r#"{"id":"req-1","method":"item/commandExecution/requestApproval","params":{"command":"ls"}}"#;
        match AppServerMessage::parse_line(line) {
            AppServerMessage::Request(r) => {
                assert_eq!(r.method, "item/commandExecution/requestApproval");
            }
            other => panic!("expected Request, got {other:?}"),
        }
    }

    #[test]
    fn parse_error_response() {
        let line = r#"{"id":2,"error":{"code":-1,"message":"failed"}}"#;
        match AppServerMessage::parse_line(line) {
            AppServerMessage::Response(r) => {
                assert!(r.error.is_some());
                assert_eq!(r.error.unwrap().message, "failed");
            }
            other => panic!("expected Response, got {other:?}"),
        }
    }

    #[test]
    fn parse_malformed() {
        assert!(matches!(
            AppServerMessage::parse_line("not json"),
            AppServerMessage::Malformed(_)
        ));
    }

    #[test]
    fn parse_empty_line() {
        assert!(matches!(
            AppServerMessage::parse_line(""),
            AppServerMessage::Malformed(_)
        ));
    }

    #[test]
    fn parse_non_object_json() {
        assert!(matches!(
            AppServerMessage::parse_line("[1,2,3]"),
            AppServerMessage::Malformed(_)
        ));
    }

    #[test]
    fn test_adf_envelope_review_command_roundtrip() {
        let cmd = AdfEnvelope::ReviewCommand {
            correlation_id: Uuid::new_v4(),
            agent_name: "security-agent".to_string(),
            group: FindingCategory::Security,
            git_ref: "abc123".to_string(),
            worktree_path: "/tmp/worktree".to_string(),
            changed_files: vec!["src/main.rs".to_string()],
            dispatched_at: Utc::now(),
        };
        let jsonl = cmd.to_jsonl().unwrap();
        let parsed = AdfEnvelope::from_jsonl(&jsonl).unwrap();
        assert_eq!(cmd.correlation_id(), parsed.correlation_id());
    }

    #[test]
    fn test_adf_envelope_review_response_roundtrip() {
        let response = AdfEnvelope::ReviewResponse {
            correlation_id: Uuid::new_v4(),
            output: ReviewAgentOutput {
                agent: "test-agent".to_string(),
                findings: vec![],
                summary: "All good".to_string(),
                pass: true,
            },
            duration_ms: 1000,
            completed_at: Utc::now(),
        };
        let jsonl = response.to_jsonl().unwrap();
        let parsed = AdfEnvelope::from_jsonl(&jsonl).unwrap();
        assert_eq!(response.correlation_id(), parsed.correlation_id());
    }

    #[test]
    fn test_adf_envelope_review_error_roundtrip() {
        let err = AdfEnvelope::ReviewError {
            correlation_id: Uuid::new_v4(),
            agent_name: "failing-agent".to_string(),
            reason: "Network timeout".to_string(),
            failed_at: Utc::now(),
        };
        let jsonl = err.to_jsonl().unwrap();
        let parsed = AdfEnvelope::from_jsonl(&jsonl).unwrap();
        assert_eq!(err.correlation_id(), parsed.correlation_id());
    }

    #[test]
    fn test_adf_envelope_review_cancel_roundtrip() {
        let cancel = AdfEnvelope::ReviewCancel {
            correlation_id: Uuid::new_v4(),
            reason: "Timeout exceeded".to_string(),
        };
        let jsonl = cancel.to_jsonl().unwrap();
        let parsed = AdfEnvelope::from_jsonl(&jsonl).unwrap();
        assert_eq!(cancel.correlation_id(), parsed.correlation_id());
    }

    #[test]
    fn test_adf_envelope_correlation_id() {
        let id = Uuid::new_v4();
        let cmd = AdfEnvelope::ReviewCommand {
            correlation_id: id,
            agent_name: "test".to_string(),
            group: FindingCategory::Quality,
            git_ref: "main".to_string(),
            worktree_path: "/tmp".to_string(),
            changed_files: vec![],
            dispatched_at: Utc::now(),
        };
        assert_eq!(cmd.correlation_id(), id);
    }

    #[test]
    fn test_finding_severity_ordering() {
        assert!(FindingSeverity::Info < FindingSeverity::Low);
        assert!(FindingSeverity::Low < FindingSeverity::Medium);
        assert!(FindingSeverity::Medium < FindingSeverity::High);
        assert!(FindingSeverity::High < FindingSeverity::Critical);
    }

    #[test]
    fn test_review_agent_output_json_schema() {
        let output = ReviewAgentOutput {
            agent: "test-agent".to_string(),
            findings: vec![ReviewFinding {
                file: "src/lib.rs".to_string(),
                line: 42,
                severity: FindingSeverity::High,
                category: FindingCategory::Security,
                finding: "Potential SQL injection".to_string(),
                suggestion: Some("Use prepared statements".to_string()),
                confidence: 0.95,
            }],
            summary: "Found 1 issue".to_string(),
            pass: false,
        };
        let json = serde_json::to_string_pretty(&output).unwrap();
        assert!(json.contains("test-agent"));
        assert!(json.contains("Potential SQL injection"));
    }

    #[test]
    fn test_deduplicate_same_file_line_category() {
        let findings = vec![
            ReviewFinding {
                file: "src/lib.rs".to_string(),
                line: 42,
                severity: FindingSeverity::Low,
                category: FindingCategory::Security,
                finding: "Low severity issue".to_string(),
                suggestion: None,
                confidence: 0.5,
            },
            ReviewFinding {
                file: "src/lib.rs".to_string(),
                line: 42,
                severity: FindingSeverity::High,
                category: FindingCategory::Security,
                finding: "High severity issue".to_string(),
                suggestion: None,
                confidence: 0.5,
            },
        ];
        let deduped = deduplicate_findings(findings);
        assert_eq!(deduped.len(), 1);
        assert_eq!(deduped[0].severity, FindingSeverity::High);
    }

    #[test]
    fn test_deduplicate_different_locations_preserved() {
        let findings = vec![
            ReviewFinding {
                file: "src/a.rs".to_string(),
                line: 1,
                severity: FindingSeverity::High,
                category: FindingCategory::Security,
                finding: "Issue A".to_string(),
                suggestion: None,
                confidence: 0.5,
            },
            ReviewFinding {
                file: "src/b.rs".to_string(),
                line: 1,
                severity: FindingSeverity::High,
                category: FindingCategory::Security,
                finding: "Issue B".to_string(),
                suggestion: None,
                confidence: 0.5,
            },
        ];
        let deduped = deduplicate_findings(findings);
        assert_eq!(deduped.len(), 2);
    }

    #[test]
    fn test_deduplicate_empty_input() {
        let deduped = deduplicate_findings(vec![]);
        assert!(deduped.is_empty());
    }

    #[test]
    fn test_deduplicate_sort_order() {
        let findings = vec![
            ReviewFinding {
                file: "src/b.rs".to_string(),
                line: 10,
                severity: FindingSeverity::Medium,
                category: FindingCategory::Quality,
                finding: "Medium B".to_string(),
                suggestion: None,
                confidence: 0.5,
            },
            ReviewFinding {
                file: "src/a.rs".to_string(),
                line: 5,
                severity: FindingSeverity::High,
                category: FindingCategory::Security,
                finding: "High A".to_string(),
                suggestion: None,
                confidence: 0.5,
            },
            ReviewFinding {
                file: "src/a.rs".to_string(),
                line: 3,
                severity: FindingSeverity::High,
                category: FindingCategory::Security,
                finding: "High A earlier".to_string(),
                suggestion: None,
                confidence: 0.5,
            },
        ];
        let deduped = deduplicate_findings(findings);
        assert_eq!(deduped.len(), 3);
        // Highest severity first
        assert_eq!(deduped[0].severity, FindingSeverity::High);
        assert_eq!(deduped[0].file, "src/a.rs");
        assert_eq!(deduped[0].line, 3); // Earlier line within same severity
        // Then medium severity
        assert_eq!(deduped[2].severity, FindingSeverity::Medium);
    }
}
