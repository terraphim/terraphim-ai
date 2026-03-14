//! Claude Code session lifecycle.
//!
//! Manages a single-shot `claude -p` invocation for a coding-agent session.
//! Parses NDJSON stream events from `--output-format stream-json` and maps
//! them to the shared `AgentEvent` / `WorkerOutcome` types.

use crate::config::ServiceConfig;
use crate::error::{Result, SymphonyError};
use crate::runner::protocol::{AgentEvent, TokenCounts};
use crate::runner::session::WorkerOutcome;

use serde::Deserialize;
use std::path::Path;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// A single Claude Code NDJSON event from `--output-format stream-json`.
///
/// Claude Code emits one JSON object per line. The `type` field determines
/// the event kind. We only deserialise the fields we care about; the rest
/// is captured as `serde_json::Value` in the params map.
#[derive(Debug, Clone, Deserialize)]
pub struct ClaudeCodeEvent {
    /// Event type (e.g. "system", "assistant", "result", "error").
    #[serde(rename = "type")]
    pub event_type: String,

    /// Subtype for finer-grained classification (e.g. "init", "text", "tool_use").
    #[serde(default)]
    pub subtype: Option<String>,

    /// Textual content (for assistant text, result summaries, error messages).
    #[serde(default)]
    pub content: Option<String>,

    /// Cost / usage tracking.
    #[serde(default)]
    pub cost_usd: Option<f64>,

    /// Duration in seconds (present on "result" events).
    #[serde(default)]
    pub duration_secs: Option<f64>,

    /// Duration in API seconds (present on "result" events).
    #[serde(default)]
    pub duration_api_secs: Option<f64>,

    /// Number of turns executed.
    #[serde(default)]
    pub num_turns: Option<u32>,

    /// Session ID.
    #[serde(default)]
    pub session_id: Option<String>,

    /// Total input tokens (present on "result").
    #[serde(default)]
    pub total_input_tokens: Option<u64>,

    /// Total output tokens (present on "result").
    #[serde(default)]
    pub total_output_tokens: Option<u64>,

    /// Tool name (for tool_use subtype).
    #[serde(default)]
    pub tool_name: Option<String>,

    /// Catch-all for other fields.
    #[serde(flatten)]
    pub extra: serde_json::Value,
}

impl ClaudeCodeEvent {
    /// Try to parse a single NDJSON line into a Claude Code event.
    pub fn parse_line(line: &str) -> Option<Self> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return None;
        }
        serde_json::from_str(trimmed).ok()
    }
}

/// A live Claude Code session.
///
/// Unlike `CodexSession`, this is a fire-and-forget single invocation of
/// `claude -p "prompt" --output-format stream-json`. There is no handshake,
/// no bidirectional messaging, and no approval flow.
pub struct ClaudeCodeSession {
    child: Child,
    stdout_rx: mpsc::Receiver<ClaudeCodeEvent>,
    tokens: TokenCounts,
    session_id: Option<String>,
    num_turns: u32,
}

impl ClaudeCodeSession {
    /// Launch `claude -p` with the given prompt and parse NDJSON events.
    pub async fn start(
        cwd: &Path,
        config: &ServiceConfig,
        prompt: &str,
        event_tx: mpsc::Sender<AgentEvent>,
    ) -> Result<Self> {
        // Build the claude command
        let mut cmd_parts = vec![
            "claude".to_string(),
            "-p".to_string(),
            prompt.to_string(),
            "--output-format".to_string(),
            "stream-json".to_string(),
            "--verbose".to_string(),
        ];

        // Append max-turns from config
        let max_turns = config.max_turns();
        cmd_parts.push("--max-turns".to_string());
        cmd_parts.push(max_turns.to_string());

        // Append any additional flags from config
        if let Some(flags) = config.claude_flags() {
            for flag in flags.split_whitespace() {
                cmd_parts.push(flag.to_string());
            }
        }

        // Append settings file/JSON if configured (enables hooks parity with interactive mode)
        if let Some(settings) = config.claude_settings() {
            cmd_parts.push("--settings".to_string());
            cmd_parts.push(settings);
        }

        let command_str = shell_escape_command(&cmd_parts);
        info!(command = %command_str, cwd = %cwd.display(), "launching claude code session");

        let mut child = Command::new("bash")
            .arg("-lc")
            .arg(&command_str)
            .current_dir(cwd)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| SymphonyError::Agent {
                reason: format!("failed to spawn claude: {e}"),
            })?;

        let pid = child.id();

        let stdout = child.stdout.take().ok_or_else(|| SymphonyError::Agent {
            reason: "failed to capture stdout".into(),
        })?;
        let stderr = child.stderr.take().ok_or_else(|| SymphonyError::Agent {
            reason: "failed to capture stderr".into(),
        })?;

        let (event_out_tx, event_out_rx) = mpsc::channel(512);

        // Spawn stdout NDJSON reader
        tokio::spawn(Self::read_ndjson(BufReader::new(stdout), event_out_tx));
        // Spawn stderr drain
        tokio::spawn(Self::drain_stderr(BufReader::new(stderr)));

        // Emit session started event immediately
        let _ = event_tx
            .send(AgentEvent::SessionStarted {
                session_id: String::new(),
                thread_id: String::new(),
                turn_id: String::new(),
                pid,
            })
            .await;

        Ok(Self {
            child,
            stdout_rx: event_out_rx,
            tokens: TokenCounts::default(),
            session_id: None,
            num_turns: 0,
        })
    }

    /// Run the session to completion, processing NDJSON events until the
    /// process exits. Returns a `WorkerOutcome`.
    pub async fn run_to_completion(
        mut self,
        event_tx: &mpsc::Sender<AgentEvent>,
        timeout: Duration,
    ) -> WorkerOutcome {
        let result = tokio::time::timeout(timeout, self.process_events(event_tx)).await;

        match result {
            Ok(Ok(())) => {
                // Wait for child to exit
                let exit_status = self.child.wait().await;
                let exit_code = exit_status
                    .as_ref()
                    .ok()
                    .and_then(|s| s.code())
                    .unwrap_or(-1);

                if exit_code == 0 {
                    WorkerOutcome::Normal {
                        turn_count: self.num_turns.max(1),
                        tokens: self.tokens,
                    }
                } else {
                    WorkerOutcome::Failed {
                        reason: format!("claude exited with code {exit_code}"),
                        turn_count: self.num_turns.max(1),
                        tokens: self.tokens,
                    }
                }
            }
            Ok(Err(e)) => {
                self.stop().await;
                WorkerOutcome::Failed {
                    reason: e.to_string(),
                    turn_count: self.num_turns.max(1),
                    tokens: self.tokens,
                }
            }
            Err(_) => {
                self.stop().await;
                WorkerOutcome::Failed {
                    reason: format!("session timeout after {}s", timeout.as_secs()),
                    turn_count: self.num_turns.max(1),
                    tokens: self.tokens,
                }
            }
        }
    }

    /// Process NDJSON events until the stdout channel closes.
    async fn process_events(&mut self, event_tx: &mpsc::Sender<AgentEvent>) -> Result<()> {
        while let Some(event) = self.stdout_rx.recv().await {
            self.handle_event(&event, event_tx).await;
        }
        Ok(())
    }

    /// Map a single Claude Code event to an `AgentEvent` and update internal state.
    async fn handle_event(&mut self, event: &ClaudeCodeEvent, event_tx: &mpsc::Sender<AgentEvent>) {
        match event.event_type.as_str() {
            "system" => {
                // System init events may carry session_id
                if let Some(sid) = &event.session_id {
                    self.session_id = Some(sid.clone());
                    let _ = event_tx
                        .send(AgentEvent::SessionStarted {
                            session_id: sid.clone(),
                            thread_id: sid.clone(),
                            turn_id: String::new(),
                            pid: None,
                        })
                        .await;
                }
                debug!(subtype = ?event.subtype, "claude code system event");
            }

            "assistant" => {
                match event.subtype.as_deref() {
                    Some("text") => {
                        // Text output from the model
                        let _ = event_tx
                            .send(AgentEvent::Notification {
                                message: "assistant_text".into(),
                            })
                            .await;
                    }
                    Some("tool_use") => {
                        let tool = event.tool_name.as_deref().unwrap_or("unknown");
                        let _ = event_tx
                            .send(AgentEvent::Notification {
                                message: format!("tool_use:{tool}"),
                            })
                            .await;
                    }
                    _ => {
                        let _ = event_tx
                            .send(AgentEvent::Notification {
                                message: format!(
                                    "assistant:{}",
                                    event.subtype.as_deref().unwrap_or("unknown")
                                ),
                            })
                            .await;
                    }
                }
            }

            "tool" => {
                // Tool execution result
                let _ = event_tx
                    .send(AgentEvent::Notification {
                        message: format!(
                            "tool_result:{}",
                            event.tool_name.as_deref().unwrap_or("unknown")
                        ),
                    })
                    .await;
            }

            "result" => {
                // Final result with token totals
                let input = event.total_input_tokens.unwrap_or(0);
                let output = event.total_output_tokens.unwrap_or(0);
                self.tokens = TokenCounts {
                    input_tokens: input,
                    output_tokens: output,
                    total_tokens: input + output,
                };

                if let Some(turns) = event.num_turns {
                    self.num_turns = turns;
                }

                let _ = event_tx
                    .send(AgentEvent::TokenUsage {
                        input_tokens: input,
                        output_tokens: output,
                        total_tokens: input + output,
                    })
                    .await;

                let _ = event_tx
                    .send(AgentEvent::TurnCompleted {
                        turn_id: self.session_id.clone().unwrap_or_default(),
                        turn_count: self.num_turns,
                    })
                    .await;

                info!(
                    input_tokens = input,
                    output_tokens = output,
                    num_turns = self.num_turns,
                    cost_usd = ?event.cost_usd,
                    duration_secs = ?event.duration_secs,
                    "claude code session completed"
                );
            }

            "error" => {
                let msg = event.content.as_deref().unwrap_or("unknown error");
                warn!(error = msg, "claude code error event");
                let _ = event_tx
                    .send(AgentEvent::TurnFailed {
                        turn_id: self.session_id.clone().unwrap_or_default(),
                        reason: msg.to_string(),
                    })
                    .await;
            }

            other => {
                debug!(event_type = other, "unhandled claude code event type");
            }
        }
    }

    /// Stop the claude process.
    pub async fn stop(&mut self) {
        if let Err(e) = self.child.kill().await {
            debug!("failed to kill claude process: {e}");
        }
        let _ = self.child.wait().await;
    }

    /// Read NDJSON lines from stdout and send parsed events.
    async fn read_ndjson(
        mut reader: BufReader<tokio::process::ChildStdout>,
        tx: mpsc::Sender<ClaudeCodeEvent>,
    ) {
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line).await {
                Ok(0) => break, // EOF
                Ok(_) => {
                    if let Some(event) = ClaudeCodeEvent::parse_line(&line) {
                        if tx.send(event).await.is_err() {
                            break;
                        }
                    }
                }
                Err(e) => {
                    error!("claude stdout read error: {e}");
                    break;
                }
            }
        }
    }

    /// Drain stderr to prevent buffer blocking.
    async fn drain_stderr(mut reader: BufReader<tokio::process::ChildStderr>) {
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line).await {
                Ok(0) => break,
                Ok(_) => {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        debug!(target: "symphony::claude_code::stderr", "{trimmed}");
                    }
                }
                Err(_) => break,
            }
        }
    }
}

/// Shell-escape a command and its arguments for `bash -lc`.
fn shell_escape_command(parts: &[String]) -> String {
    parts
        .iter()
        .map(|p| {
            if p.contains(' ') || p.contains('\'') || p.contains('"') || p.contains('\\') {
                // Single-quote the argument, escaping embedded single quotes
                format!("'{}'", p.replace('\'', "'\\''"))
            } else {
                p.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_system_event() {
        let line = r#"{"type":"system","subtype":"init","session_id":"abc-123","content":"Claude Code v2.1"}"#;
        let event = ClaudeCodeEvent::parse_line(line).unwrap();
        assert_eq!(event.event_type, "system");
        assert_eq!(event.subtype.as_deref(), Some("init"));
        assert_eq!(event.session_id.as_deref(), Some("abc-123"));
    }

    #[test]
    fn parse_assistant_text_event() {
        let line = r#"{"type":"assistant","subtype":"text","content":"Hello world"}"#;
        let event = ClaudeCodeEvent::parse_line(line).unwrap();
        assert_eq!(event.event_type, "assistant");
        assert_eq!(event.subtype.as_deref(), Some("text"));
        assert_eq!(event.content.as_deref(), Some("Hello world"));
    }

    #[test]
    fn parse_tool_use_event() {
        let line = r#"{"type":"assistant","subtype":"tool_use","tool_name":"Read"}"#;
        let event = ClaudeCodeEvent::parse_line(line).unwrap();
        assert_eq!(event.event_type, "assistant");
        assert_eq!(event.subtype.as_deref(), Some("tool_use"));
        assert_eq!(event.tool_name.as_deref(), Some("Read"));
    }

    #[test]
    fn parse_result_event() {
        let line = r#"{"type":"result","subtype":"success","cost_usd":0.05,"duration_secs":42.3,"duration_api_secs":15.1,"num_turns":3,"session_id":"sess-1","total_input_tokens":5000,"total_output_tokens":2000}"#;
        let event = ClaudeCodeEvent::parse_line(line).unwrap();
        assert_eq!(event.event_type, "result");
        assert_eq!(event.total_input_tokens, Some(5000));
        assert_eq!(event.total_output_tokens, Some(2000));
        assert_eq!(event.num_turns, Some(3));
        assert!((event.cost_usd.unwrap() - 0.05).abs() < f64::EPSILON);
    }

    #[test]
    fn parse_error_event() {
        let line = r#"{"type":"error","content":"Rate limit exceeded"}"#;
        let event = ClaudeCodeEvent::parse_line(line).unwrap();
        assert_eq!(event.event_type, "error");
        assert_eq!(event.content.as_deref(), Some("Rate limit exceeded"));
    }

    #[test]
    fn parse_empty_line_returns_none() {
        assert!(ClaudeCodeEvent::parse_line("").is_none());
        assert!(ClaudeCodeEvent::parse_line("   ").is_none());
    }

    #[test]
    fn parse_malformed_returns_none() {
        assert!(ClaudeCodeEvent::parse_line("not json").is_none());
        assert!(ClaudeCodeEvent::parse_line("[1,2,3]").is_none());
    }

    #[test]
    fn parse_unknown_fields_captured() {
        let line = r#"{"type":"assistant","subtype":"text","content":"hi","custom_field":"value"}"#;
        let event = ClaudeCodeEvent::parse_line(line).unwrap();
        assert_eq!(event.event_type, "assistant");
        // custom_field should be in the extra map
        assert_eq!(
            event.extra.get("custom_field").and_then(|v| v.as_str()),
            Some("value")
        );
    }

    #[test]
    fn shell_escape_simple() {
        let parts = vec!["claude".into(), "-p".into(), "hello world".into()];
        let escaped = shell_escape_command(&parts);
        assert_eq!(escaped, "claude -p 'hello world'");
    }

    #[test]
    fn shell_escape_with_quotes() {
        let parts = vec!["echo".into(), "it's a test".into()];
        let escaped = shell_escape_command(&parts);
        assert_eq!(escaped, "echo 'it'\\''s a test'");
    }

    #[test]
    fn shell_escape_no_special_chars() {
        let parts = vec![
            "claude".into(),
            "-p".into(),
            "--max-turns".into(),
            "10".into(),
        ];
        let escaped = shell_escape_command(&parts);
        assert_eq!(escaped, "claude -p --max-turns 10");
    }
}
