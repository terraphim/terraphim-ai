//! Canonical machine-readable PR gate result contract.
//!
//! Each PR gate agent (`pr-reviewer`, `pr-validator`, `pr-verifier`) keeps
//! its own discipline-specific human report, but must emit a single
//! `<!-- adf:gate-result { ... } -->` block to its final output. The
//! orchestrator parses this block to post branch-protection commit
//! statuses that reflect the actual current gate outcome.
//!
//! The structural review parser in [`crate::pr_review`] is unchanged and
//! remains the source of truth for the auto-merge engine. The gate result
//! contract here is intentionally narrow: it does not understand the
//! human report and only validates the machine-readable block.

use thiserror::Error;

/// Current schema version. Bump on breaking changes to [`PrGateResult`].
pub const GATE_RESULT_SCHEMA_VERSION: u8 = 1;

const GATE_RESULT_OPEN: &str = "<!-- adf:gate-result";
const GATE_RESULT_CLOSE: &str = "-->";

/// Machine-readable gate result emitted by every PR gate agent.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct PrGateResult {
    pub schema_version: u8,
    pub agent: String,
    pub context: String,
    pub pr_number: u64,
    pub head_sha: String,
    pub status: GateStatus,
    pub confidence: u8,
    pub blocking_findings: u32,
    pub summary: String,
}

/// Status declared by the gate agent. Maps to a commit status with the help
/// of [`status_from_gate_result`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateStatus {
    Pass,
    Concerns,
    Fail,
}

/// Errors produced by the gate result parser and validator. Each variant
/// causes the orchestrator to fail the gate closed.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum PrGateResultError {
    #[error("missing adf:gate-result block")]
    MissingBlock,
    #[error("malformed adf:gate-result JSON: {0}")]
    MalformedJson(String),
    #[error("unsupported schema_version {0}")]
    UnsupportedSchema(u8),
    #[error("result context {actual} does not match expected {expected}")]
    ContextMismatch { actual: String, expected: String },
    #[error("result agent {actual} does not match expected {expected}")]
    AgentMismatch { actual: String, expected: String },
    #[error("result PR #{actual} does not match expected #{expected}")]
    PrMismatch { actual: u64, expected: u64 },
    #[error("result head_sha {actual} does not match expected {expected}")]
    HeadMismatch { actual: String, expected: String },
    #[error("confidence {0} is outside 1..=5")]
    ConfidenceOutOfRange(u8),
}

/// Per-dispatch metadata describing the PR and the context the agent
/// was dispatched for. Used to validate the parsed gate result before
/// promoting it to a commit status.
#[derive(Debug, Clone)]
pub struct PrGateMeta {
    pub pr_number: u64,
    pub project: String,
    pub agent_name: String,
    pub context: String,
    pub head_sha: String,
}

/// Extract a [`PrGateResult`] from the markdown output of a PR gate agent.
///
/// The block is delimited by an HTML comment opened with `<!-- adf:gate-result`
/// and closed with `-->`. Anything between the markers must be a single JSON
/// object. Trailing prose is allowed (it can contain the human-readable
/// report) but the JSON must be the only thing between the markers.
pub fn extract_gate_result(markdown: &str) -> Result<PrGateResult, PrGateResultError> {
    let Some(start) = markdown.find(GATE_RESULT_OPEN) else {
        return Err(PrGateResultError::MissingBlock);
    };
    let after_open = start + GATE_RESULT_OPEN.len();
    let Some(close_rel) = markdown[after_open..].find(GATE_RESULT_CLOSE) else {
        return Err(PrGateResultError::MissingBlock);
    };
    let body = &markdown[after_open..after_open + close_rel];
    let trimmed = body.trim();
    let parsed: PrGateResult = serde_json::from_str(trimmed)
        .map_err(|e| PrGateResultError::MalformedJson(e.to_string()))?;
    if parsed.schema_version != GATE_RESULT_SCHEMA_VERSION {
        return Err(PrGateResultError::UnsupportedSchema(parsed.schema_version));
    }
    if !(1..=5).contains(&parsed.confidence) {
        return Err(PrGateResultError::ConfidenceOutOfRange(parsed.confidence));
    }
    Ok(parsed)
}

/// Validate that a parsed [`PrGateResult`] matches the dispatch metadata.
/// Fails closed on context, agent, PR, or head SHA mismatch.
pub fn validate_gate_result(
    result: &PrGateResult,
    meta: &PrGateMeta,
) -> Result<(), PrGateResultError> {
    if result.context != meta.context {
        return Err(PrGateResultError::ContextMismatch {
            actual: result.context.clone(),
            expected: meta.context.clone(),
        });
    }
    if result.agent != meta.agent_name {
        return Err(PrGateResultError::AgentMismatch {
            actual: result.agent.clone(),
            expected: meta.agent_name.clone(),
        });
    }
    if result.pr_number != meta.pr_number {
        return Err(PrGateResultError::PrMismatch {
            actual: result.pr_number,
            expected: meta.pr_number,
        });
    }
    if result.head_sha != meta.head_sha {
        return Err(PrGateResultError::HeadMismatch {
            actual: result.head_sha.clone(),
            expected: meta.head_sha.clone(),
        });
    }
    Ok(())
}

/// Map a validated [`PrGateResult`] to a Gitea commit status.
///
/// `blocking_findings` is treated as the deciding signal: even an agent that
/// self-reports `pass` with explicit blocking findings is mapped to
/// failure. `concerns` with no blocking findings is a non-blocking success
/// for `adf/validation` and `adf/verification`; an explicit `fail` is
/// always failure.
pub fn status_from_gate_result(result: &PrGateResult) -> (terraphim_tracker::StatusState, String) {
    if result.blocking_findings > 0 {
        return (
            terraphim_tracker::StatusState::Failure,
            format!(
                "{}: {} blocking finding(s) reported",
                result.context, result.blocking_findings
            ),
        );
    }
    match result.status {
        GateStatus::Pass => (
            terraphim_tracker::StatusState::Success,
            format!("{} pass ({}/5)", result.context, result.confidence),
        ),
        GateStatus::Concerns => (
            terraphim_tracker::StatusState::Success,
            format!(
                "{} pass with concerns ({}/5)",
                result.context, result.confidence
            ),
        ),
        GateStatus::Fail => (
            terraphim_tracker::StatusState::Failure,
            format!("{} failed", result.context),
        ),
    }
}

/// Best-effort extraction of the final assistant-visible text from CLI
/// drain output. The CLI tool name (e.g. `claude`, `opencode`, `pi-rust`)
/// determines the streaming JSON shape; unknown CLIs fall back to plain
/// text. Used by the orchestrator when consuming drain logs to find the
/// canonical `adf:gate-result` block.
pub fn extract_assistant_text(lines: &[String], cli_tool: &str) -> String {
    let cli = cli_tool.rsplit('/').next().unwrap_or(cli_tool);
    let extracted = if cli.contains("claude") {
        collect_json_text(lines, |value| {
            if value.get("type").and_then(serde_json::Value::as_str) == Some("content_block_delta")
            {
                value
                    .get("delta")
                    .and_then(|delta| delta.get("text"))
                    .and_then(serde_json::Value::as_str)
            } else {
                None
            }
        })
    } else if cli.contains("opencode") {
        collect_json_text(lines, |value| {
            value
                .get("text")
                .and_then(serde_json::Value::as_str)
                .or_else(|| {
                    value
                        .get("part")
                        .and_then(|part| part.get("text"))
                        .and_then(serde_json::Value::as_str)
                })
        })
    } else {
        collect_json_text(lines, |value| {
            value
                .get("text")
                .and_then(serde_json::Value::as_str)
                .or_else(|| {
                    value
                        .get("message")
                        .and_then(|message| message.get("content"))
                        .and_then(serde_json::Value::as_str)
                        .or_else(|| {
                            value
                                .get("message")
                                .and_then(|message| message.get("content"))
                                .and_then(serde_json::Value::as_array)
                                .and_then(|parts| {
                                    parts.iter().find_map(|p| {
                                        p.get("text").and_then(serde_json::Value::as_str)
                                    })
                                })
                        })
                })
        })
    };

    let trimmed = extracted.trim();
    if !trimmed.is_empty() {
        return trimmed.to_string();
    }

    lines
        .iter()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('{') || trimmed.starts_with('#') {
                None
            } else {
                Some(trimmed)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn collect_json_text<F>(lines: &[String], mut pick: F) -> String
where
    F: FnMut(&serde_json::Value) -> Option<&str>,
{
    let mut chunks: Vec<String> = Vec::new();
    for line in lines {
        let trimmed = line.trim();
        let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) else {
            continue;
        };
        if let Some(text) = pick(&value) {
            chunks.push(text.to_string());
        }
    }
    chunks.join("")
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample_block() -> &'static str {
        r#"Some human report...

<!-- adf:gate-result
{
  "schema_version": 1,
  "agent": "pr-validator",
  "context": "adf/validation",
  "pr_number": 2268,
  "head_sha": "deadbeefcafebabe",
  "status": "concerns",
  "confidence": 4,
  "blocking_findings": 0,
  "summary": "Validation passed with minor non-blocking concerns"
}
-->

Trailing prose."#
    }

    fn sample_meta() -> PrGateMeta {
        PrGateMeta {
            pr_number: 2268,
            project: "terraphim-ai".to_string(),
            agent_name: "pr-validator".to_string(),
            context: "adf/validation".to_string(),
            head_sha: "deadbeefcafebabe".to_string(),
        }
    }

    #[test]
    fn extract_gate_result_parses_valid_block() {
        let result = extract_gate_result(sample_block()).expect("valid block must parse");
        assert_eq!(result.schema_version, 1);
        assert_eq!(result.agent, "pr-validator");
        assert_eq!(result.context, "adf/validation");
        assert_eq!(result.pr_number, 2268);
        assert_eq!(result.head_sha, "deadbeefcafebabe");
        assert_eq!(result.status, GateStatus::Concerns);
        assert_eq!(result.confidence, 4);
        assert_eq!(result.blocking_findings, 0);
        assert_eq!(
            result.summary,
            "Validation passed with minor non-blocking concerns"
        );
    }

    #[test]
    fn extract_gate_result_rejects_missing_block() {
        let err = extract_gate_result("no block here").unwrap_err();
        assert_eq!(err, PrGateResultError::MissingBlock);
    }

    #[test]
    fn extract_gate_result_rejects_malformed_json() {
        let body = "<!-- adf:gate-result\n{ not json }\n-->";
        let err = extract_gate_result(body).unwrap_err();
        assert!(matches!(err, PrGateResultError::MalformedJson(_)));
    }

    #[test]
    fn extract_gate_result_rejects_unsupported_schema_version() {
        let body = r#"<!-- adf:gate-result
{
  "schema_version": 99,
  "agent": "x",
  "context": "y",
  "pr_number": 1,
  "head_sha": "a",
  "status": "pass",
  "confidence": 3,
  "blocking_findings": 0,
  "summary": "s"
}
-->"#;
        let err = extract_gate_result(body).unwrap_err();
        assert_eq!(err, PrGateResultError::UnsupportedSchema(99));
    }

    #[test]
    fn extract_gate_result_rejects_out_of_range_confidence() {
        let body = r#"<!-- adf:gate-result
{
  "schema_version": 1,
  "agent": "x",
  "context": "y",
  "pr_number": 1,
  "head_sha": "a",
  "status": "pass",
  "confidence": 9,
  "blocking_findings": 0,
  "summary": "s"
}
-->"#;
        let err = extract_gate_result(body).unwrap_err();
        assert_eq!(err, PrGateResultError::ConfidenceOutOfRange(9));
    }

    #[test]
    fn extract_gate_result_rejects_missing_close_marker() {
        let body = "<!-- adf:gate-result\n{ \"schema_version\": 1 }";
        let err = extract_gate_result(body).unwrap_err();
        assert_eq!(err, PrGateResultError::MissingBlock);
    }

    #[test]
    fn validate_gate_result_rejects_stale_head() {
        let result = extract_gate_result(sample_block()).unwrap();
        let mut meta = sample_meta();
        meta.head_sha = "newsha".to_string();
        let err = validate_gate_result(&result, &meta).unwrap_err();
        assert_eq!(
            err,
            PrGateResultError::HeadMismatch {
                actual: "deadbeefcafebabe".to_string(),
                expected: "newsha".to_string(),
            }
        );
    }

    #[test]
    fn validate_gate_result_rejects_wrong_context() {
        let result = extract_gate_result(sample_block()).unwrap();
        let mut meta = sample_meta();
        meta.context = "adf/verification".to_string();
        let err = validate_gate_result(&result, &meta).unwrap_err();
        assert_eq!(
            err,
            PrGateResultError::ContextMismatch {
                actual: "adf/validation".to_string(),
                expected: "adf/verification".to_string(),
            }
        );
    }

    #[test]
    fn validate_gate_result_rejects_wrong_agent() {
        let result = extract_gate_result(sample_block()).unwrap();
        let mut meta = sample_meta();
        meta.agent_name = "pr-reviewer".to_string();
        let err = validate_gate_result(&result, &meta).unwrap_err();
        assert!(matches!(err, PrGateResultError::AgentMismatch { .. }));
    }

    #[test]
    fn validate_gate_result_rejects_wrong_pr() {
        let result = extract_gate_result(sample_block()).unwrap();
        let mut meta = sample_meta();
        meta.pr_number = 9999;
        let err = validate_gate_result(&result, &meta).unwrap_err();
        assert_eq!(
            err,
            PrGateResultError::PrMismatch {
                actual: 2268,
                expected: 9999,
            }
        );
    }

    #[test]
    fn validate_gate_result_accepts_matching_metadata() {
        let result = extract_gate_result(sample_block()).unwrap();
        validate_gate_result(&result, &sample_meta()).expect("matching metadata should validate");
    }

    fn result_with(status: GateStatus, blocking_findings: u32) -> PrGateResult {
        PrGateResult {
            schema_version: 1,
            agent: "pr-validator".to_string(),
            context: "adf/validation".to_string(),
            pr_number: 1,
            head_sha: "a".to_string(),
            status,
            confidence: 4,
            blocking_findings,
            summary: "s".to_string(),
        }
    }

    #[test]
    fn status_policy_fails_when_blocking_findings_present() {
        let (state, desc) = status_from_gate_result(&result_with(GateStatus::Pass, 1));
        assert_eq!(state, terraphim_tracker::StatusState::Failure);
        assert!(desc.contains("1 blocking finding"));
    }

    #[test]
    fn status_policy_maps_pass_to_success() {
        let (state, desc) = status_from_gate_result(&result_with(GateStatus::Pass, 0));
        assert_eq!(state, terraphim_tracker::StatusState::Success);
        assert!(desc.contains("pass"));
        assert!(!desc.contains("concerns"));
    }

    #[test]
    fn status_policy_maps_concerns_to_success_without_blocking() {
        let (state, desc) = status_from_gate_result(&result_with(GateStatus::Concerns, 0));
        assert_eq!(state, terraphim_tracker::StatusState::Success);
        assert!(desc.contains("concerns"));
    }

    #[test]
    fn status_policy_maps_fail_to_failure() {
        let (state, _desc) = status_from_gate_result(&result_with(GateStatus::Fail, 0));
        assert_eq!(state, terraphim_tracker::StatusState::Failure);
    }

    #[test]
    fn extract_assistant_text_parses_claude_deltas() {
        let lines = vec![
            r#"{"type":"content_block_delta","delta":{"text":"first "}}"#.to_string(),
            r#"{"type":"content_block_delta","delta":{"text":"second"}}"#.to_string(),
        ];
        let text = extract_assistant_text(&lines, "/usr/local/bin/claude");
        assert_eq!(text, "first second");
    }

    #[test]
    fn extract_assistant_text_parses_opencode_shapes() {
        let lines = vec![
            r#"{"type":"text","text":"hello "}"#.to_string(),
            r#"{"part":{"text":"world"}}"#.to_string(),
        ];
        let text = extract_assistant_text(&lines, "/usr/local/bin/opencode");
        assert_eq!(text, "hello world");
    }

    #[test]
    fn extract_assistant_text_falls_back_to_plain_text() {
        let lines = vec![
            "# header".to_string(),
            String::new(),
            "report line 1".to_string(),
            "report line 2".to_string(),
        ];
        let text = extract_assistant_text(&lines, "unknown-cli");
        assert_eq!(text, "report line 1\nreport line 2");
    }
}
