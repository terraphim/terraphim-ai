//! Output capture with @mention detection

use regex::Regex;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{ChildStderr, ChildStdout};
use tokio::sync::{broadcast, mpsc};

use terraphim_types::capability::ProcessId;

/// Events that can be captured from agent output
#[derive(Debug, Clone)]
pub enum OutputEvent {
    /// Standard output line
    Stdout { process_id: ProcessId, line: String },
    /// Standard error line
    Stderr { process_id: ProcessId, line: String },
    /// @mention detected in output
    Mention {
        process_id: ProcessId,
        target: String,
        message: String,
    },
    /// Process completed
    Completed {
        process_id: ProcessId,
        exit_code: Option<i32>,
    },
}

/// Captures output from agent processes with @mention detection
#[derive(Debug)]
pub struct OutputCapture {
    process_id: ProcessId,
    mention_regex: Regex,
    event_sender: mpsc::Sender<OutputEvent>,
    /// Broadcast sender for live output streaming (e.g., WebSocket subscribers)
    broadcast_sender: broadcast::Sender<OutputEvent>,
}

impl OutputCapture {
    /// Create a new output capture
    pub fn new(
        process_id: ProcessId,
        stdout: BufReader<ChildStdout>,
        stderr: BufReader<ChildStderr>,
    ) -> Self {
        let (event_sender, _event_receiver) = mpsc::channel(100);
        let (broadcast_sender, _) = broadcast::channel(256);

        let capture = Self {
            process_id,
            mention_regex: Regex::new(r"@(\w+)").unwrap(),
            event_sender,
            broadcast_sender,
        };

        // Start capturing stdout and stderr
        capture.capture_stdout(stdout);
        capture.capture_stderr(stderr);

        capture
    }

    /// Subscribe to live output events via broadcast channel.
    ///
    /// Returns a receiver that gets a clone of every output event.
    /// Suitable for streaming to WebSocket clients.
    pub fn subscribe(&self) -> broadcast::Receiver<OutputEvent> {
        self.broadcast_sender.subscribe()
    }

    /// Get a reference to the broadcast sender.
    pub fn broadcaster(&self) -> &broadcast::Sender<OutputEvent> {
        &self.broadcast_sender
    }

    /// Start capturing stdout
    fn capture_stdout(&self, mut stdout: BufReader<ChildStdout>) {
        let process_id = self.process_id;
        let mention_regex = self.mention_regex.clone();
        let event_sender = self.event_sender.clone();
        let broadcast_sender = self.broadcast_sender.clone();

        tokio::spawn(async move {
            let mut line = String::new();

            loop {
                line.clear();
                match stdout.read_line(&mut line).await {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        let line = line.trim().to_string();
                        if line.is_empty() {
                            continue;
                        }

                        // Check for @mentions
                        if let Some(captures) = mention_regex.captures(&line) {
                            if let Some(target) = captures.get(1) {
                                let target = target.as_str().to_string();
                                let message = line.clone();

                                let mention_event = OutputEvent::Mention {
                                    process_id,
                                    target,
                                    message,
                                };
                                let _ = event_sender.send(mention_event.clone()).await;
                                let _ = broadcast_sender.send(mention_event);
                            }
                        }

                        // Send stdout event
                        let stdout_event = OutputEvent::Stdout { process_id, line };
                        let _ = event_sender.send(stdout_event.clone()).await;
                        let _ = broadcast_sender.send(stdout_event);
                    }
                    Err(e) => {
                        tracing::error!(process_id = %process_id, error = %e, "Error reading stdout");
                        break;
                    }
                }
            }
        });
    }

    /// Start capturing stderr
    fn capture_stderr(&self, mut stderr: BufReader<ChildStderr>) {
        let process_id = self.process_id;
        let event_sender = self.event_sender.clone();
        let broadcast_sender = self.broadcast_sender.clone();

        tokio::spawn(async move {
            let mut line = String::new();

            loop {
                line.clear();
                match stderr.read_line(&mut line).await {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        let line = line.trim().to_string();
                        if line.is_empty() {
                            continue;
                        }

                        let stderr_event = OutputEvent::Stderr { process_id, line };
                        let _ = event_sender.send(stderr_event.clone()).await;
                        let _ = broadcast_sender.send(stderr_event);
                    }
                    Err(e) => {
                        tracing::error!(process_id = %process_id, error = %e, "Error reading stderr");
                        break;
                    }
                }
            }
        });
    }

    /// Get the event sender (for external use)
    pub fn event_sender(&self) -> mpsc::Sender<OutputEvent> {
        self.event_sender.clone()
    }
}

/// Parsed opencode NDJSON event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenCodeEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub timestamp: Option<u64>,
    #[serde(rename = "sessionID")]
    pub session_id: Option<String>,
    pub part: Option<serde_json::Value>,
}

impl OpenCodeEvent {
    /// Extract text content from a text event
    pub fn text_content(&self) -> Option<&str> {
        if self.event_type == "text" {
            self.part.as_ref()?.get("text")?.as_str()
        } else {
            None
        }
    }

    /// Check if this is a result (final) event
    pub fn is_result(&self) -> bool {
        self.event_type == "result"
    }

    /// Check if this is a step finish event
    pub fn is_step_finish(&self) -> bool {
        self.event_type == "step_finish"
    }

    /// Extract total token count from step_finish or result events
    pub fn total_tokens(&self) -> Option<u64> {
        self.part
            .as_ref()?
            .get("tokens")?
            .get("total")?
            .as_u64()
    }

    /// Parse a single NDJSON line into an OpenCodeEvent
    pub fn parse_line(line: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(line.trim())
    }

    /// Parse multiple NDJSON lines (newline-delimited JSON)
    pub fn parse_lines(lines: &str) -> Vec<Result<Self, serde_json::Error>> {
        lines
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(Self::parse_line)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mention_regex() {
        let regex = Regex::new(r"@(\w+)").unwrap();

        let text = "Hello @kimiko, can you help?";
        let captures = regex.captures(text).unwrap();
        assert_eq!(captures.get(1).unwrap().as_str(), "kimiko");

        let text = "No mentions here";
        assert!(regex.captures(text).is_none());
    }

    // OpenCodeEvent NDJSON parsing tests

    #[test]
    fn test_parse_step_start_event() {
        let json = r#"{"type":"step_start","timestamp":1234567890,"sessionID":"sess-123","part":{"step":1}}"#;
        let event = OpenCodeEvent::parse_line(json).unwrap();

        assert_eq!(event.event_type, "step_start");
        assert_eq!(event.timestamp, Some(1234567890));
        assert_eq!(event.session_id, Some("sess-123".to_string()));
        assert!(event.part.is_some());
    }

    #[test]
    fn test_parse_text_event() {
        let json = r#"{"type":"text","timestamp":1234567891,"sessionID":"sess-123","part":{"text":"Hello, world!"}}"#;
        let event = OpenCodeEvent::parse_line(json).unwrap();

        assert_eq!(event.event_type, "text");
        assert_eq!(event.text_content(), Some("Hello, world!"));
    }

    #[test]
    fn test_parse_tool_use_event() {
        let json = r#"{"type":"tool_use","timestamp":1234567892,"sessionID":"sess-123","part":{"tool":"Read","args":{"path":"/tmp/file.txt"}}}"#;
        let event = OpenCodeEvent::parse_line(json).unwrap();

        assert_eq!(event.event_type, "tool_use");
        assert!(event.part.is_some());
        assert!(event.text_content().is_none());
        assert!(!event.is_result());
        assert!(!event.is_step_finish());
    }

    #[test]
    fn test_parse_step_finish_event() {
        let json = r#"{"type":"step_finish","timestamp":1234567893,"sessionID":"sess-123","part":{"step":1,"tokens":{"total":150,"prompt":100,"completion":50}}}"#;
        let event = OpenCodeEvent::parse_line(json).unwrap();

        assert_eq!(event.event_type, "step_finish");
        assert!(event.is_step_finish());
        assert!(!event.is_result());
        assert_eq!(event.total_tokens(), Some(150));
    }

    #[test]
    fn test_parse_result_event() {
        let json = r#"{"type":"result","timestamp":1234567894,"sessionID":"sess-123","part":{"success":true,"cost":0.002,"tokens":{"total":500,"prompt":300,"completion":200}}}"#;
        let event = OpenCodeEvent::parse_line(json).unwrap();

        assert_eq!(event.event_type, "result");
        assert!(event.is_result());
        assert!(!event.is_step_finish());
        assert_eq!(event.total_tokens(), Some(500));
    }

    #[test]
    fn test_text_content_extraction() {
        let text_event = OpenCodeEvent {
            event_type: "text".to_string(),
            timestamp: Some(1234567890),
            session_id: None,
            part: Some(serde_json::json!({"text": "Some content here"})),
        };
        assert_eq!(text_event.text_content(), Some("Some content here"));

        let non_text_event = OpenCodeEvent {
            event_type: "step_start".to_string(),
            timestamp: None,
            session_id: None,
            part: Some(serde_json::json!({"step": 1})),
        };
        assert!(non_text_event.text_content().is_none());

        let event_no_part = OpenCodeEvent {
            event_type: "text".to_string(),
            timestamp: None,
            session_id: None,
            part: None,
        };
        assert!(event_no_part.text_content().is_none());

        let event_no_text_field = OpenCodeEvent {
            event_type: "text".to_string(),
            timestamp: None,
            session_id: None,
            part: Some(serde_json::json!({"other": "value"})),
        };
        assert!(event_no_text_field.text_content().is_none());
    }

    #[test]
    fn test_is_result_detection() {
        let result_event = OpenCodeEvent {
            event_type: "result".to_string(),
            timestamp: None,
            session_id: None,
            part: None,
        };
        assert!(result_event.is_result());

        let other_event = OpenCodeEvent {
            event_type: "step_start".to_string(),
            timestamp: None,
            session_id: None,
            part: None,
        };
        assert!(!other_event.is_result());
    }

    #[test]
    fn test_is_step_finish_detection() {
        let finish_event = OpenCodeEvent {
            event_type: "step_finish".to_string(),
            timestamp: None,
            session_id: None,
            part: None,
        };
        assert!(finish_event.is_step_finish());

        let other_event = OpenCodeEvent {
            event_type: "text".to_string(),
            timestamp: None,
            session_id: None,
            part: None,
        };
        assert!(!other_event.is_step_finish());
    }

    #[test]
    fn test_total_tokens_extraction() {
        let event_with_tokens = OpenCodeEvent {
            event_type: "step_finish".to_string(),
            timestamp: None,
            session_id: None,
            part: Some(serde_json::json!({"tokens": {"total": 1234, "prompt": 500}})),
        };
        assert_eq!(event_with_tokens.total_tokens(), Some(1234));

        let event_no_tokens = OpenCodeEvent {
            event_type: "text".to_string(),
            timestamp: None,
            session_id: None,
            part: Some(serde_json::json!({"text": "hello"})),
        };
        assert!(event_no_tokens.total_tokens().is_none());

        let event_no_part = OpenCodeEvent {
            event_type: "step_finish".to_string(),
            timestamp: None,
            session_id: None,
            part: None,
        };
        assert!(event_no_part.total_tokens().is_none());
    }

    #[test]
    fn test_parse_ndjson_sequence() {
        let ndjson = r#"{"type":"step_start","timestamp":1,"sessionID":"s1","part":{"step":1}}
{"type":"text","timestamp":2,"sessionID":"s1","part":{"text":"Processing..."}}
{"type":"tool_use","timestamp":3,"sessionID":"s1","part":{"tool":"Read"}}
{"type":"step_finish","timestamp":4,"sessionID":"s1","part":{"step":1,"tokens":{"total":100}}}
{"type":"result","timestamp":5,"sessionID":"s1","part":{"success":true,"tokens":{"total":100}}}"#;

        let events: Vec<_> = OpenCodeEvent::parse_lines(ndjson)
            .into_iter()
            .filter_map(|r| r.ok())
            .collect();

        assert_eq!(events.len(), 5);
        assert_eq!(events[0].event_type, "step_start");
        assert_eq!(events[1].event_type, "text");
        assert_eq!(events[1].text_content(), Some("Processing..."));
        assert_eq!(events[2].event_type, "tool_use");
        assert_eq!(events[3].event_type, "step_finish");
        assert!(events[3].is_step_finish());
        assert_eq!(events[3].total_tokens(), Some(100));
        assert_eq!(events[4].event_type, "result");
        assert!(events[4].is_result());
        assert_eq!(events[4].total_tokens(), Some(100));
    }

    #[test]
    fn test_parse_empty_and_whitespace_lines() {
        let ndjson = r#"
{"type":"text","timestamp":1,"part":{"text":"First"}}

{"type":"text","timestamp":2,"part":{"text":"Second"}}

"#;

        let events: Vec<_> = OpenCodeEvent::parse_lines(ndjson)
            .into_iter()
            .filter_map(|r| r.ok())
            .collect();

        assert_eq!(events.len(), 2);
        assert_eq!(events[0].text_content(), Some("First"));
        assert_eq!(events[1].text_content(), Some("Second"));
    }

    #[test]
    fn test_parse_invalid_json() {
        let invalid = r#"{"type":"text","part":{"text":}"#;
        let result = OpenCodeEvent::parse_line(invalid);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_mixed_valid_invalid() {
        let ndjson = r#"{"type":"text","timestamp":1,"part":{"text":"Valid"}}
not valid json here
{"type":"result","timestamp":2,"part":{}}"#;

        let results: Vec<_> = OpenCodeEvent::parse_lines(ndjson);
        assert_eq!(results.len(), 3);
        assert!(results[0].is_ok());
        assert!(results[1].is_err());
        assert!(results[2].is_ok());
    }

    #[test]
    fn test_event_without_optional_fields() {
        let json = r#"{"type":"text"}"#;
        let event = OpenCodeEvent::parse_line(json).unwrap();

        assert_eq!(event.event_type, "text");
        assert!(event.timestamp.is_none());
        assert!(event.session_id.is_none());
        assert!(event.part.is_none());
        assert!(event.text_content().is_none());
    }
}
