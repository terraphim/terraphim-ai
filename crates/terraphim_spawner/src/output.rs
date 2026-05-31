//! Output capture with @mention detection and redaction
//!
//! Captures stdout/stderr from agent processes, detects @mentions, and
//! stores a bounded buffer of redacted events for timeout reporting.

use regex::Regex;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{ChildStderr, ChildStdout};
use tokio::sync::{broadcast, mpsc};

use crate::redaction;
use terraphim_types::capability::ProcessId;

/// Maximum number of captured events to retain per agent.
const MAX_CAPTURED_EVENTS: usize = 4096;

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

impl OutputEvent {
    /// Return a redacted copy of this event with secrets scrubbed.
    fn redacted(&self) -> Self {
        match self {
            Self::Stdout { process_id, line } => Self::Stdout {
                process_id: *process_id,
                line: redaction::redact(line),
            },
            Self::Stderr { process_id, line } => Self::Stderr {
                process_id: *process_id,
                line: redaction::redact(line),
            },
            Self::Mention {
                process_id,
                target,
                message,
            } => Self::Mention {
                process_id: *process_id,
                target: target.clone(),
                message: redaction::redact(message),
            },
            Self::Completed {
                process_id,
                exit_code,
            } => Self::Completed {
                process_id: *process_id,
                exit_code: *exit_code,
            },
        }
    }
}

/// Captures output from agent processes with @mention detection
#[derive(Debug)]
pub struct OutputCapture {
    process_id: ProcessId,
    mention_regex: Regex,
    event_sender: mpsc::Sender<OutputEvent>,
    /// Broadcast sender for live output streaming (e.g. WebSocket subscribers)
    broadcast_sender: broadcast::Sender<OutputEvent>,
    /// Bounded buffer of redacted events for timeout reporting.
    captured_events: Arc<Mutex<VecDeque<OutputEvent>>>,
}

impl OutputCapture {
    /// Create a new output capture
    pub fn new(
        process_id: ProcessId,
        stdout: BufReader<ChildStdout>,
        stderr: BufReader<ChildStderr>,
    ) -> Self {
        let (event_sender, _event_receiver) = mpsc::channel::<OutputEvent>(100);
        let (broadcast_sender, _) = broadcast::channel(256);

        let capture = Self {
            process_id,
            mention_regex: Regex::new(r"@(\w+)").unwrap(),
            event_sender,
            broadcast_sender,
            captured_events: Arc::new(Mutex::new(VecDeque::new())),
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

    /// Return a snapshot of captured redacted output events.
    pub fn captured_events(&self) -> Vec<OutputEvent> {
        self.captured_events
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .cloned()
            .collect()
    }

    /// Record a redacted event into the bounded buffer.
    fn record_event(captured_events: &Arc<Mutex<VecDeque<OutputEvent>>>, event: &OutputEvent) {
        let mut events = captured_events.lock().unwrap_or_else(|e| e.into_inner());
        if events.len() >= MAX_CAPTURED_EVENTS {
            events.pop_front();
        }
        events.push_back(event.redacted());
    }

    /// Start capturing stdout
    fn capture_stdout(&self, mut stdout: BufReader<ChildStdout>) {
        let process_id = self.process_id;
        let mention_regex = self.mention_regex.clone();
        let event_sender = self.event_sender.clone();
        let broadcast_sender = self.broadcast_sender.clone();
        let captured_events = self.captured_events.clone();

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
                                Self::record_event(&captured_events, &mention_event);
                                let _ = event_sender.send(mention_event.clone()).await;
                                let _ = broadcast_sender.send(mention_event);
                            }
                        }

                        // Send stdout event
                        let stdout_event = OutputEvent::Stdout {
                            process_id,
                            line: line.clone(),
                        };
                        Self::record_event(&captured_events, &stdout_event);
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
        let captured_events = self.captured_events.clone();

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

                        let stderr_event = OutputEvent::Stderr {
                            process_id,
                            line: line.clone(),
                        };
                        Self::record_event(&captured_events, &stderr_event);
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

    #[test]
    fn test_output_event_redacted_scrubs_secrets() {
        let event = OutputEvent::Stdout {
            process_id: ProcessId::new(),
            line: "api_key=secret123".to_string(),
        };
        let redacted = event.redacted();
        match redacted {
            OutputEvent::Stdout { line, .. } => {
                assert!(line.contains("***REDACTED***"));
                assert!(!line.contains("secret123"));
            }
            _ => panic!("Expected Stdout event"),
        }
    }

    #[test]
    fn test_output_event_redacted_preserves_structure() {
        let event = OutputEvent::Stderr {
            process_id: ProcessId::new(),
            line: "Error: timeout after 30s".to_string(),
        };
        let redacted = event.redacted();
        match redacted {
            OutputEvent::Stderr { line, .. } => {
                assert_eq!(line, "Error: timeout after 30s");
            }
            _ => panic!("Expected Stderr event"),
        }
    }

    #[test]
    fn test_captured_events_bounded() {
        let (event_sender, _event_receiver) = mpsc::channel::<OutputEvent>(100);
        let (broadcast_sender, _) = broadcast::channel::<OutputEvent>(256);
        let captured = Arc::new(Mutex::new(VecDeque::new()));

        // Simulate recording MAX_CAPTURED_EVENTS + 10 events
        for i in 0..MAX_CAPTURED_EVENTS + 10 {
            let event = OutputEvent::Stdout {
                process_id: ProcessId::new(),
                line: format!("line {}", i),
            };
            OutputCapture::record_event(&captured, &event);
        }

        let events = captured.lock().unwrap();
        assert_eq!(events.len(), MAX_CAPTURED_EVENTS);
        // The oldest events should have been evicted
        assert!(!events
            .iter()
            .any(|e| matches!(e, OutputEvent::Stdout { line, .. } if line == "line 0")));
        assert!(events
            .iter()
            .any(|e| matches!(e, OutputEvent::Stdout { line, .. } if line == "line 10")));
    }

    #[test]
    fn test_captured_events_redacts_before_storage() {
        let (event_sender, _event_receiver) = mpsc::channel::<OutputEvent>(100);
        let (broadcast_sender, _) = broadcast::channel::<OutputEvent>(256);
        let captured = Arc::new(Mutex::new(VecDeque::new()));

        let event = OutputEvent::Stdout {
            process_id: ProcessId::new(),
            line: "api_key=secret123".to_string(),
        };
        OutputCapture::record_event(&captured, &event);

        let events = captured.lock().unwrap();
        assert_eq!(events.len(), 1);
        match &events[0] {
            OutputEvent::Stdout { line, .. } => {
                assert!(line.contains("***REDACTED***"));
                assert!(!line.contains("secret123"));
            }
            _ => panic!("Expected Stdout event"),
        }
    }
}
