//! Output stream parser for CLI tool JSON output.
//!
//! Parses `opencode run --format json` step_finish events and
//! `claude -p --output-format stream-json` events into CompletionEvents
//! suitable for the TelemetryStore.

use crate::control_plane::telemetry::{CompletionEvent, TokenBreakdown};
use chrono::Utc;

/// Parsed result from a line of CLI output.
#[derive(Debug, Clone, PartialEq)]
pub enum ParsedOutput {
    /// A completion event with token/latency data.
    Completion(CompletionEvent),
    /// A step-start or intermediate event (ignored).
    Ignored,
    /// A line that could not be parsed.
    Unparseable(String),
}

/// Parse a single line from `opencode run --format json` output.
///
/// Relevant events:
/// - `step_finish`: contains tokens and cost
///
/// Example input:
/// ```json
/// {"type":"step_finish","timestamp":1234,"sessionID":"ses_xxx","part":{"type":"step-finish","tokens":{"total":48432,"input":45327,"output":97,"reasoning":0,"cache":{"write":0,"read":3008}},"cost":0}}
/// ```
pub fn parse_opencode_line(
    line: &str,
    session_id: &str,
    model: &str,
    start_timestamp: Option<i64>,
) -> ParsedOutput {
    let line = line.trim();
    if line.is_empty() {
        return ParsedOutput::Ignored;
    }

    let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
        return ParsedOutput::Unparseable(line.to_string());
    };

    let event_type = value.get("type").and_then(|v| v.as_str()).unwrap_or("");

    match event_type {
        "step_finish" => parse_opencode_step_finish(&value, session_id, model, start_timestamp),
        "step_start" | "text" | "tool_use" | "tool_result" => ParsedOutput::Ignored,
        _ => ParsedOutput::Ignored,
    }
}

fn parse_opencode_step_finish(
    value: &serde_json::Value,
    session_id: &str,
    model: &str,
    start_timestamp: Option<i64>,
) -> ParsedOutput {
    let part = match value.get("part") {
        Some(p) => p,
        None => return ParsedOutput::Unparseable(value.to_string()),
    };

    let tokens = part
        .get("tokens")
        .map(|t| TokenBreakdown {
            total: t.get("total").and_then(|v| v.as_u64()).unwrap_or(0),
            input: t.get("input").and_then(|v| v.as_u64()).unwrap_or(0),
            output: t.get("output").and_then(|v| v.as_u64()).unwrap_or(0),
            reasoning: t.get("reasoning").and_then(|v| v.as_u64()).unwrap_or(0),
            cache_read: t
                .get("cache")
                .and_then(|c| c.get("read"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
            cache_write: t
                .get("cache")
                .and_then(|c| c.get("write"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
        })
        .unwrap_or_default();

    let cost_usd = value.get("cost").and_then(|v| v.as_f64()).unwrap_or(0.0);

    let finish_timestamp = value
        .get("timestamp")
        .and_then(|v| v.as_i64())
        .unwrap_or_else(|| Utc::now().timestamp_millis());

    let latency_ms = start_timestamp
        .map(|start| {
            let diff = finish_timestamp - start;
            if diff > 0 {
                diff as u64
            } else {
                0
            }
        })
        .unwrap_or(0);

    let reason = part
        .get("reason")
        .and_then(|v| v.as_str())
        .unwrap_or("stop");

    let success = reason == "stop";
    let error = if success {
        None
    } else {
        Some(format!("completion ended with reason: {}", reason))
    };

    ParsedOutput::Completion(CompletionEvent {
        model: model.to_string(),
        session_id: session_id.to_string(),
        completed_at: Utc::now(),
        latency_ms,
        success,
        tokens,
        cost_usd,
        error,
    })
}

/// Parse a single line from `claude -p --output-format stream-json` output.
///
/// Claude stream-json wraps events differently. The relevant event types
/// contain usage information in the final message.
pub fn parse_claude_line(line: &str, session_id: &str, model: &str) -> ParsedOutput {
    let line = line.trim();
    if line.is_empty() {
        return ParsedOutput::Ignored;
    }

    let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
        return ParsedOutput::Unparseable(line.to_string());
    };

    let event_type = value.get("type").and_then(|v| v.as_str()).unwrap_or("");

    match event_type {
        "result" => parse_claude_result(&value, session_id, model),
        "assistant" | "user" | "system" => ParsedOutput::Ignored,
        _ => ParsedOutput::Ignored,
    }
}

fn parse_claude_result(value: &serde_json::Value, session_id: &str, model: &str) -> ParsedOutput {
    let usage = value.get("usage");

    let tokens = usage
        .map(|u| TokenBreakdown {
            total: u.get("total_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
            input: u.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
            output: u.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
            reasoning: 0,
            cache_read: u
                .get("cache_read_input_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
            cache_write: u
                .get("cache_creation_input_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
        })
        .unwrap_or_default();

    let cost_usd = value
        .get("cost_usd")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    let duration_ms = value
        .get("duration_ms")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    let is_error = value
        .get("is_error")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let error = if is_error {
        value
            .get("error")
            .and_then(|v| v.as_str())
            .map(String::from)
    } else {
        None
    };

    ParsedOutput::Completion(CompletionEvent {
        model: model.to_string(),
        session_id: session_id.to_string(),
        completed_at: Utc::now(),
        latency_ms: duration_ms,
        success: !is_error,
        tokens,
        cost_usd,
        error,
    })
}

/// Parse stderr output for error patterns (subscription limits, rate limits).
///
/// Returns Some(error_message) if a subscription-limit error is detected.
pub fn parse_stderr_for_limit_errors(stderr: &str) -> Option<String> {
    for line in stderr.lines() {
        let lower = line.to_lowercase();
        if lower.contains("weekly session limit")
            || lower.contains("monthly limit")
            || lower.contains("rate limit")
            || lower.contains("quota exceeded")
            || lower.contains("429")
            || lower.contains("capacity limit")
            || lower.contains("spending limit")
            || lower.contains("subscription limit")
            || lower.contains("usage limit")
            || lower.contains("hit your limit")
            || lower.contains("you've hit your limit")
            || lower.contains("plan limit")
            || lower.contains("tier limit")
            || lower.contains("usage cap")
            || lower.contains("daily limit")
            || lower.contains("hourly limit")
            || lower.contains("out of quota")
            || lower.contains("quota exhausted")
            || lower.contains("subscription quota")
            || lower.contains("insufficient balance")
            || lower.contains("insufficient_quota")
        {
            return Some(line.to_string());
        }
    }
    None
}

/// Parse an entire output stream (stdout) line by line, returning completion events.
pub fn parse_opencode_output(stdout: &str, session_id: &str, model: &str) -> Vec<CompletionEvent> {
    let mut start_ts: Option<i64> = None;
    let mut events = Vec::new();

    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
            if value.get("type").and_then(|v| v.as_str()) == Some("step_start") {
                start_ts = value.get("timestamp").and_then(|v| v.as_i64());
                continue;
            }
        }

        if let ParsedOutput::Completion(event) =
            parse_opencode_line(line, session_id, model, start_ts)
        {
            events.push(event);
        }
    }

    events
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_opencode_step_finish() {
        let line = r#"{"type":"step_finish","timestamp":1775767451123,"sessionID":"ses_test","part":{"id":"prt_1","reason":"stop","snapshot":"abc","messageID":"msg_1","sessionID":"ses_test","type":"step-finish","tokens":{"total":48432,"input":45327,"output":97,"reasoning":0,"cache":{"write":0,"read":3008}},"cost":0}}"#;

        let result = parse_opencode_line(
            line,
            "sess-1",
            "zai-coding-plan/glm-5.1",
            Some(1775767450000i64),
        );
        match result {
            ParsedOutput::Completion(event) => {
                assert_eq!(event.model, "zai-coding-plan/glm-5.1");
                assert_eq!(event.session_id, "sess-1");
                assert!(event.success);
                assert_eq!(event.tokens.total, 48432);
                assert_eq!(event.tokens.input, 45327);
                assert_eq!(event.tokens.output, 97);
                assert_eq!(event.tokens.cache_read, 3008);
                assert_eq!(event.tokens.cache_write, 0);
                assert_eq!(event.latency_ms, 1123);
            }
            _ => panic!("Expected Completion, got {:?}", result),
        }
    }

    #[test]
    fn test_parse_opencode_ignored_events() {
        assert_eq!(
            parse_opencode_line(
                r#"{"type":"step_start","timestamp":1234}"#,
                "sess-1",
                "model-a",
                None
            ),
            ParsedOutput::Ignored
        );
        assert_eq!(
            parse_opencode_line(
                r#"{"type":"text","timestamp":1234}"#,
                "sess-1",
                "model-a",
                None
            ),
            ParsedOutput::Ignored
        );
    }

    #[test]
    fn test_parse_opencode_unparseable() {
        let result = parse_opencode_line("not json at all", "sess-1", "model-a", None);
        assert!(matches!(result, ParsedOutput::Unparseable(_)));
    }

    #[test]
    fn test_parse_opencode_non_stop_reason() {
        let line = r#"{"type":"step_finish","timestamp":1775767451123,"sessionID":"ses_test","part":{"id":"prt_1","reason":"error","type":"step-finish","tokens":{"total":100,"input":50,"output":50,"reasoning":0,"cache":{"write":0,"read":0}},"cost":0}}"#;

        let result = parse_opencode_line(line, "sess-1", "model-a", Some(1775767450000i64));
        match result {
            ParsedOutput::Completion(event) => {
                assert!(!event.success);
                assert!(event.error.is_some());
                assert!(event.error.unwrap().contains("error"));
            }
            _ => panic!("Expected Completion"),
        }
    }

    #[test]
    fn test_parse_full_output() {
        let stdout = r#"
{"type":"step_start","timestamp":1000,"sessionID":"ses_test","part":{"type":"step-start"}}
{"type":"text","timestamp":1001,"sessionID":"ses_test","part":{"type":"text","text":"Hello"}}
{"type":"step_finish","timestamp":2000,"sessionID":"ses_test","part":{"type":"step-finish","tokens":{"total":500,"input":400,"output":100,"reasoning":0,"cache":{"write":0,"read":0}},"cost":0.01}}
"#;

        let events = parse_opencode_output(stdout, "sess-1", "model-a");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].tokens.total, 500);
        assert_eq!(events[0].latency_ms, 1000);
    }

    #[test]
    fn test_parse_stderr_limit_error() {
        let stderr = "Error: weekly session limit reached\nPlease try again later";
        assert!(parse_stderr_for_limit_errors(stderr).is_some());

        let stderr = "connection refused\nno route to host";
        assert!(parse_stderr_for_limit_errors(stderr).is_none());
    }
}
