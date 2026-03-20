//! Integration tests for Claude Code session NDJSON parsing
//!
//! These tests validate the parsing of Claude Code's `--output-format stream-json`
//! NDJSON event stream without requiring a real Claude binary.

use std::time::Duration;
use tokio::time::timeout;

/// Claude Code NDJSON event from `--output-format stream-json`
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ClaudeCodeEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(default)]
    pub subtype: Option<String>,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub cost_usd: Option<f64>,
    #[serde(default)]
    pub duration_secs: Option<f64>,
    #[serde(default)]
    pub num_turns: Option<u32>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub total_input_tokens: Option<u64>,
    #[serde(default)]
    pub total_output_tokens: Option<u64>,
    #[serde(default)]
    pub tool_name: Option<String>,
    #[serde(flatten)]
    pub extra: serde_json::Value,
}

impl ClaudeCodeEvent {
    /// Parse a single NDJSON line
    pub fn parse_line(line: &str) -> Option<Self> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return None;
        }
        serde_json::from_str(trimmed).ok()
    }

    /// Extract text content from assistant text events
    pub fn text_content(&self) -> Option<&str> {
        if self.event_type == "assistant" && self.subtype.as_deref() == Some("text") {
            self.content.as_deref()
        } else {
            None
        }
    }

    /// Check if this is a result (final) event
    pub fn is_result(&self) -> bool {
        self.event_type == "result"
    }

    /// Check if this is a system init event
    pub fn is_init(&self) -> bool {
        self.event_type == "system" && self.subtype.as_deref() == Some("init")
    }

    /// Get session ID from event
    pub fn get_session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }
}

/// Mock Claude Code session for testing
pub struct MockClaudeCodeSession {
    events: Vec<ClaudeCodeEvent>,
    session_id: Option<String>,
}

impl MockClaudeCodeSession {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            session_id: None,
        }
    }

    /// Parse NDJSON stream and store events
    pub fn parse_stream(&mut self, ndjson: &str) -> Vec<Result<ClaudeCodeEvent, String>> {
        let mut results = Vec::new();
        
        for line in ndjson.lines() {
            match ClaudeCodeEvent::parse_line(line) {
                Some(event) => {
                    // Extract session ID from init event
                    if event.is_init() && event.session_id.is_some() {
                        self.session_id = event.session_id.clone();
                    }
                    self.events.push(event.clone());
                    results.push(Ok(event));
                }
                None if line.trim().is_empty() => {
                    // Skip empty lines gracefully
                }
                None => {
                    results.push(Err(format!("Failed to parse: {}", line)));
                }
            }
        }
        
        results
    }

    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    pub fn events(&self) -> &[ClaudeCodeEvent] {
        &self.events
    }

    /// Simulate processing with timeout
    pub async fn process_with_timeout(
        &self,
        _duration: Duration,
    ) -> Result<Vec<&ClaudeCodeEvent>, &'static str> {
        // In a real scenario, this would process async events
        // For mock, we just return what we have
        if self.events.is_empty() {
            return Err("No events to process");
        }
        Ok(self.events.iter().collect())
    }
}

impl Default for MockClaudeCodeSession {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Test 1: Parse mock Claude Code NDJSON stream
    // =========================================================================

    #[test]
    fn test_parse_init_event() {
        let json = r#"{"type":"system","subtype":"init","session_id":"sess-abc-123","content":"Claude Code v2.1"}"#;
        let event = ClaudeCodeEvent::parse_line(json).expect("Should parse init event");
        
        assert_eq!(event.event_type, "system");
        assert_eq!(event.subtype.as_deref(), Some("init"));
        assert_eq!(event.session_id.as_deref(), Some("sess-abc-123"));
        assert!(event.is_init());
    }

    #[test]
    fn test_parse_assistant_text_event() {
        let json = r#"{"type":"assistant","subtype":"text","content":"I'll help you with that task."}"#;
        let event = ClaudeCodeEvent::parse_line(json).expect("Should parse assistant text");
        
        assert_eq!(event.event_type, "assistant");
        assert_eq!(event.subtype.as_deref(), Some("text"));
        assert_eq!(event.text_content(), Some("I'll help you with that task."));
    }

    #[test]
    fn test_parse_tool_use_event() {
        let json = r#"{"type":"assistant","subtype":"tool_use","tool_name":"Read","content":"Reading file..."}"#;
        let event = ClaudeCodeEvent::parse_line(json).expect("Should parse tool use");
        
        assert_eq!(event.event_type, "assistant");
        assert_eq!(event.subtype.as_deref(), Some("tool_use"));
        assert_eq!(event.tool_name.as_deref(), Some("Read"));
        assert!(event.text_content().is_none()); // Not a text subtype
    }

    #[test]
    fn test_parse_result_event() {
        let json = r#"{"type":"result","cost_usd":0.05,"duration_secs":42.3,"num_turns":5,"session_id":"sess-abc-123","total_input_tokens":5000,"total_output_tokens":2000}"#;
        let event = ClaudeCodeEvent::parse_line(json).expect("Should parse result");
        
        assert_eq!(event.event_type, "result");
        assert!(event.is_result());
        assert_eq!(event.total_input_tokens, Some(5000));
        assert_eq!(event.total_output_tokens, Some(2000));
        assert_eq!(event.num_turns, Some(5));
        assert!((event.cost_usd.unwrap() - 0.05).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_error_event() {
        let json = r#"{"type":"error","content":"Rate limit exceeded - please try again later"}"#;
        let event = ClaudeCodeEvent::parse_line(json).expect("Should parse error");
        
        assert_eq!(event.event_type, "error");
        assert_eq!(event.content.as_deref(), Some("Rate limit exceeded - please try again later"));
    }

    // =========================================================================
    // Test 2: Extract text content from assistant messages
    // =========================================================================

    #[test]
    fn test_extract_text_content_variations() {
        let test_cases = vec![
            (r#"{"type":"assistant","subtype":"text","content":"Simple message"}"#, Some("Simple message")),
            (r#"{"type":"assistant","subtype":"text","content":""}"#, Some("")),
            (r#"{"type":"assistant","subtype":"tool_use","content":"Not text"}"#, None),
            (r#"{"type":"system","subtype":"init","content":"Not assistant"}"#, None),
            (r#"{"type":"assistant","subtype":"text"}"#, None),
        ];

        for (json, expected) in test_cases {
            let event = ClaudeCodeEvent::parse_line(json).expect("Should parse");
            assert_eq!(event.text_content(), expected, "Failed for: {}", json);
        }
    }

    #[test]
    fn test_extract_multiline_text_content() {
        let content = "Line 1\nLine 2\nLine 3";
        let json = format!(
            r#"{{"type":"assistant","subtype":"text","content":"{}"}}"#,
            content.replace('\n', "\\n")
        );
        let event = ClaudeCodeEvent::parse_line(&json).expect("Should parse");
        assert_eq!(event.text_content(), Some(content));
    }

    // =========================================================================
    // Test 3: Handle malformed NDJSON lines gracefully
    // =========================================================================

    #[test]
    fn test_parse_empty_line_returns_none() {
        assert!(ClaudeCodeEvent::parse_line("").is_none());
        assert!(ClaudeCodeEvent::parse_line("   ").is_none());
        assert!(ClaudeCodeEvent::parse_line("\t\n").is_none());
    }

    #[test]
    fn test_parse_malformed_json_returns_none() {
        let malformed = vec![
            "not json at all",
            "{broken json",
            "}",
            "[1,2,3]", // Valid JSON but not an object
            r#"{"type":}"#, // Invalid syntax
            "",
        ];

        for input in malformed {
            let result = ClaudeCodeEvent::parse_line(input);
            assert!(
                result.is_none() || input.is_empty(),
                "Should return None for: {}",
                input
            );
        }
    }

    #[test]
    fn test_parse_mixed_valid_invalid_lines() {
        let ndjson = r#"{"type":"system","subtype":"init","session_id":"s1"}
this is not valid json
{"type":"assistant","subtype":"text","content":"Hello"}
{"broken":}
{"type":"result","num_turns":3}"#;

        let mut session = MockClaudeCodeSession::new();
        let results = session.parse_stream(ndjson);

        // Should have 5 results (3 valid, 2 errors)
        assert_eq!(results.len(), 5);
        
        // Check valid events were parsed
        assert!(results[0].is_ok());
        assert!(results[1].is_err());
        assert!(results[2].is_ok());
        assert!(results[3].is_err());
        assert!(results[4].is_ok());

        // Check session has only valid events
        assert_eq!(session.events().len(), 3);
    }

    #[test]
    fn test_parse_partial_json() {
        let partial = r#"{"type":"assistant","subtype":"text","content":"Incomplete"#;
        let result = ClaudeCodeEvent::parse_line(partial);
        assert!(result.is_none(), "Partial JSON should not parse");
    }

    // =========================================================================
    // Test 4: Verify session ID extraction from init event
    // =========================================================================

    #[test]
    fn test_session_id_extraction() {
        let ndjson = r#"{"type":"system","subtype":"init","session_id":"test-session-001","content":"Init"}
{"type":"assistant","subtype":"text","content":"Hello"}
{"type":"result","session_id":"test-session-001"}"#;

        let mut session = MockClaudeCodeSession::new();
        session.parse_stream(ndjson);

        assert_eq!(session.session_id(), Some("test-session-001"));
    }

    #[test]
    fn test_session_id_from_result_if_no_init() {
        let ndjson = r#"{"type":"assistant","subtype":"text","content":"Hello"}
{"type":"result","session_id":"result-session-002"}"#;

        let mut session = MockClaudeCodeSession::new();
        session.parse_stream(ndjson);

        // Session ID should not be captured from result (only from init)
        assert_eq!(session.session_id(), None);
    }

    #[test]
    fn test_session_id_persistence_across_events() {
        let ndjson = r#"{"type":"system","subtype":"init","session_id":"persistent-123"}
{"type":"assistant","subtype":"text","content":"First"}
{"type":"assistant","subtype":"tool_use","tool_name":"Read"}
{"type":"result","session_id":"persistent-123"}"#;

        let mut session = MockClaudeCodeSession::new();
        session.parse_stream(ndjson);

        assert_eq!(session.session_id(), Some("persistent-123"));
        
        // Verify all events were captured
        assert_eq!(session.events().len(), 4);
    }

    #[test]
    fn test_get_session_id_method() {
        let json = r#"{"type":"system","subtype":"init","session_id":"abc-def-123"}"#;
        let event = ClaudeCodeEvent::parse_line(json).unwrap();
        
        assert_eq!(event.get_session_id(), Some("abc-def-123"));
        
        let no_id = r#"{"type":"assistant","subtype":"text","content":"No ID"}"#;
        let event_no_id = ClaudeCodeEvent::parse_line(no_id).unwrap();
        assert_eq!(event_no_id.get_session_id(), None);
    }

    // =========================================================================
    // Test 5: Timeout handling (mock slow response)
    // =========================================================================

    #[tokio::test]
    async fn test_timeout_handling_success() {
        let ndjson = r#"{"type":"system","subtype":"init","session_id":"timeout-test"}
{"type":"assistant","subtype":"text","content":"Quick response"}
{"type":"result"}"#;

        let mut session = MockClaudeCodeSession::new();
        session.parse_stream(ndjson);

        let result = timeout(
            Duration::from_secs(1),
            session.process_with_timeout(Duration::from_millis(100))
        ).await;

        assert!(result.is_ok(), "Should complete within timeout");
        let events = result.unwrap().expect("Should process successfully");
        assert_eq!(events.len(), 3);
    }

    #[tokio::test]
    async fn test_timeout_handling_empty_stream() {
        let session = MockClaudeCodeSession::new();

        let result = timeout(
            Duration::from_millis(100),
            session.process_with_timeout(Duration::from_millis(50))
        ).await;

        assert!(result.is_ok());
        // Empty stream returns error from process_with_timeout
        assert!(result.unwrap().is_err());
    }

    #[tokio::test]
    async fn test_simulated_slow_stream() {
        // Simulate a stream that takes time to produce events
        let ndjson = r#"{"type":"system","subtype":"init","session_id":"slow-test"}"#;
        
        let mut session = MockClaudeCodeSession::new();
        session.parse_stream(ndjson);

        // Should complete quickly with short timeout
        let result = timeout(
            Duration::from_millis(50),
            session.process_with_timeout(Duration::from_millis(10))
        ).await;

        assert!(result.is_ok(), "Should handle quick timeout");
    }

    // =========================================================================
    // Integration test: Full session lifecycle
    // =========================================================================

    #[test]
    fn test_full_session_lifecycle() {
        let ndjson = r#"{"type":"system","subtype":"init","session_id":"full-lifecycle-001","content":"Claude Code v2.1"}
{"type":"assistant","subtype":"text","content":"I'll analyze the code for you."}
{"type":"assistant","subtype":"tool_use","tool_name":"Read","content":"Reading src/lib.rs"}
{"type":"assistant","subtype":"text","content":"I found an issue in the error handling."}
{"type":"assistant","subtype":"tool_use","tool_name":"Edit","content":"Fixing the error handling"}
{"type":"assistant","subtype":"text","content":"Done! I've fixed the error handling."}
{"type":"result","cost_usd":0.0234,"duration_secs":15.5,"num_turns":3,"session_id":"full-lifecycle-001","total_input_tokens":2500,"total_output_tokens":800}"#;

        let mut session = MockClaudeCodeSession::new();
        let results = session.parse_stream(ndjson);

        // All lines should parse successfully
        assert_eq!(results.len(), 7);
        assert!(results.iter().all(|r| r.is_ok()));

        // Verify session ID
        assert_eq!(session.session_id(), Some("full-lifecycle-001"));

        // Count event types
        let events = session.events();
        let init_count = events.iter().filter(|e| e.is_init()).count();
        let text_count = events.iter().filter(|e| e.text_content().is_some()).count();
        let tool_count = events.iter().filter(|e| e.subtype.as_deref() == Some("tool_use")).count();
        let result_count = events.iter().filter(|e| e.is_result()).count();

        assert_eq!(init_count, 1);
        assert_eq!(text_count, 3);
        assert_eq!(tool_count, 2);
        assert_eq!(result_count, 1);

        // Verify result event details
        let result_event = events.last().unwrap();
        assert_eq!(result_event.total_input_tokens, Some(2500));
        assert_eq!(result_event.total_output_tokens, Some(800));
        assert_eq!(result_event.num_turns, Some(3));
    }

    #[test]
    fn test_unicode_content() {
        let unicode_content = "Hello 世界 🌍 émojis work!";
        let json = format!(
            r#"{{"type":"assistant","subtype":"text","content":"{}"}}"#,
            unicode_content
        );
        
        let event = ClaudeCodeEvent::parse_line(&json).expect("Should parse unicode");
        assert_eq!(event.text_content(), Some(unicode_content));
    }

    #[test]
    fn test_large_content() {
        let large_content = "x".repeat(10000);
        let json = format!(
            r#"{{"type":"assistant","subtype":"text","content":"{}"}}"#,
            large_content
        );
        
        let event = ClaudeCodeEvent::parse_line(&json).expect("Should parse large content");
        assert_eq!(event.text_content(), Some(large_content.as_str()));
    }

    #[test]
    fn test_special_characters_in_content() {
        let special_content = r#"Special chars: "quotes", \backslash\, 
newline, 	tab"#;
        let json = format!(
            r#"{{"type":"assistant","subtype":"text","content":"{}"}}"#,
            special_content.replace('\\', "\\\\")
                .replace('"', "\\\"")
                .replace('\n', "\\n")
                .replace('\t', "\\t")
        );
        
        let event = ClaudeCodeEvent::parse_line(&json).expect("Should parse special chars");
        assert_eq!(event.text_content(), Some(special_content));
    }
}

// ============================================================================
// Integration Tests with Real CLI (behind feature flag)
// ============================================================================

#[cfg(feature = "integration")]
mod integration_tests {
    use super::*;
    use std::process::Stdio;
    use tokio::io::{AsyncBufReadExt, BufReader};
    use tokio::process::Command;

    /// Check if claude binary exists in PATH
    fn claude_binary_exists() -> bool {
        Command::new("which")
            .arg("claude")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    #[tokio::test]
    #[ignore = "Requires Claude Code CLI to be installed"]
    async fn test_real_claude_session() {
        if !claude_binary_exists() {
            eprintln!("Skipping integration test: claude binary not found in PATH");
            return;
        }

        let mut child = Command::new("claude")
            .args(&["-p", "Say 'test complete'", "--output-format", "stream-json", "--verbose", "--max-turns", "1"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to spawn claude");

        let stdout = child.stdout.take().expect("Failed to get stdout");
        let mut reader = BufReader::new(stdout).lines();
        let mut events = Vec::new();

        // Read with timeout
        let read_result = timeout(Duration::from_secs(30), async {
            while let Ok(Some(line)) = reader.next_line().await {
                if let Some(event) = ClaudeCodeEvent::parse_line(&line) {
                    events.push(event);
                }
            }
        }).await;

        assert!(read_result.is_ok(), "Should read events within timeout");

        // Cleanup
        let _ = child.kill().await;

        // Verify we got some events
        assert!(!events.is_empty(), "Should have received at least one event");
        
        // Check for expected event types
        let has_system = events.iter().any(|e| e.event_type == "system");
        let has_assistant = events.iter().any(|e| e.event_type == "assistant");
        let has_result = events.iter().any(|e| e.is_result());

        assert!(has_system, "Should have system event");
        assert!(has_assistant, "Should have assistant event");
        assert!(has_result, "Should have result event");
    }
}

// Additional tests for edge cases and error scenarios
#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_missing_optional_fields() {
        // Events with minimal fields
        let minimal = r#"{"type":"text"}"#; // Missing required fields, but valid JSON
        let event = ClaudeCodeEvent::parse_line(minimal);
        // This should parse since all fields have defaults
        assert!(event.is_some());
        let e = event.unwrap();
        assert_eq!(e.event_type, "text");
        assert!(e.subtype.is_none());
        assert!(e.content.is_none());
    }

    #[test]
    fn test_numeric_session_id() {
        // Session ID as number (edge case)
        let numeric_id = r#"{"type":"system","subtype":"init","session_id":12345}"#;
        // This should fail because session_id is typed as Option<String>
        let result = ClaudeCodeEvent::parse_line(numeric_id);
        assert!(result.is_none(), "Numeric session_id should not parse as string");
    }

    #[test]
    fn test_extra_fields_preserved() {
        let with_extra = r#"{"type":"assistant","subtype":"text","content":"Hello","custom_field":"custom_value","nested":{"key":"value"}}"#;
        let event = ClaudeCodeEvent::parse_line(with_extra).expect("Should parse");
        
        // Extra fields go into the 'extra' map via #[serde(flatten)]
        assert_eq!(event.extra.get("custom_field").and_then(|v| v.as_str()), Some("custom_value"));
        assert!(event.extra.get("nested").is_some());
    }

    #[test]
    fn test_whitespace_variations() {
        let variations = vec![
            r#"  {"type":"text"}  "#,         // Leading/trailing spaces
            r#"{"type":"text"}
"#,              // Trailing newline
            r#"	{"type":"text"}	"#,         // Tabs
            "{\"type\":\"text\"}",            // Escaped (would need double parsing)
        ];

        for json in variations {
            let result = ClaudeCodeEvent::parse_line(json);
            assert!(result.is_some(), "Should parse: {:?}", json);
        }
    }

    #[test]
    fn test_concurrent_session_ids() {
        // Simulate multiple sessions in one stream (shouldn't happen but test anyway)
        let ndjson = r#"{"type":"system","subtype":"init","session_id":"session-1"}
{"type":"system","subtype":"init","session_id":"session-2"}
{"type":"result","session_id":"session-2"}"#;

        let mut session = MockClaudeCodeSession::new();
        session.parse_stream(ndjson);

        // Should use the last init session ID encountered
        assert_eq!(session.session_id(), Some("session-2"));
    }

    #[test]
    fn test_empty_stream() {
        let session = MockClaudeCodeSession::new();
        assert!(session.events().is_empty());
        assert!(session.session_id().is_none());
    }

    #[test]
    fn test_only_whitespace_stream() {
        let ndjson = "   \n\t\n  \n";
        let mut session = MockClaudeCodeSession::new();
        let results = session.parse_stream(ndjson);
        
        assert!(results.is_empty());
        assert!(session.events().is_empty());
    }

    #[test]
    fn test_result_without_tokens() {
        let json = r#"{"type":"result","cost_usd":0.01}"#;
        let event = ClaudeCodeEvent::parse_line(json).expect("Should parse");
        
        assert!(event.is_result());
        assert!(event.total_input_tokens.is_none());
        assert!(event.total_output_tokens.is_none());
    }
}
