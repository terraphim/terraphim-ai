//! Codex app-server session lifecycle.
//!
//! Manages the handshake, turn streaming, approval handling, and
//! multi-turn continuation for a coding-agent session.

use crate::config::ServiceConfig;
use crate::error::{Result, SymphonyError};
use crate::runner::protocol::{AgentEvent, AppServerMessage, JsonRpcRequest, TokenCounts};
use crate::tracker::Issue;

use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// Outcome of a single worker run (may span multiple turns).
#[derive(Debug)]
pub enum WorkerOutcome {
    /// All turns completed normally.
    Normal {
        turn_count: u32,
        tokens: TokenCounts,
    },
    /// A turn or session failed.
    Failed {
        reason: String,
        turn_count: u32,
        tokens: TokenCounts,
    },
}

/// A live coding-agent session.
pub struct CodexSession {
    child: Child,
    stdin: BufWriter<tokio::process::ChildStdin>,
    message_rx: mpsc::Receiver<AppServerMessage>,
    next_id: u64,
    pending: HashMap<u64, tokio::sync::oneshot::Sender<serde_json::Value>>,
    thread_id: Option<String>,
    config: ServiceConfig,
    tokens: TokenCounts,
    last_reported_tokens: TokenCounts,
}

impl CodexSession {
    /// Launch the coding-agent app-server and perform the initialise handshake.
    pub async fn start(
        cwd: &Path,
        config: &ServiceConfig,
        event_tx: mpsc::Sender<AgentEvent>,
    ) -> Result<Self> {
        let command_str = config.codex_command();
        info!(command = %command_str, cwd = %cwd.display(), "launching agent");

        let mut child = Command::new("bash")
            .arg("-lc")
            .arg(&command_str)
            .current_dir(cwd)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| SymphonyError::Agent {
                reason: format!("failed to spawn '{command_str}': {e}"),
            })?;

        let pid = child.id();
        let stdin = child.stdin.take().ok_or_else(|| SymphonyError::Agent {
            reason: "failed to capture stdin".into(),
        })?;
        let stdout = child.stdout.take().ok_or_else(|| SymphonyError::Agent {
            reason: "failed to capture stdout".into(),
        })?;
        let stderr = child.stderr.take().ok_or_else(|| SymphonyError::Agent {
            reason: "failed to capture stderr".into(),
        })?;

        let (msg_tx, msg_rx) = mpsc::channel(512);

        // Spawn stdout reader (JSONL protocol messages)
        tokio::spawn(Self::read_stdout(BufReader::new(stdout), msg_tx));
        // Spawn stderr drain (diagnostic only)
        tokio::spawn(Self::drain_stderr(BufReader::new(stderr)));

        let mut session = Self {
            child,
            stdin: BufWriter::new(stdin),
            message_rx: msg_rx,
            next_id: 1,
            pending: HashMap::new(),
            thread_id: None,
            config: config.clone(),
            tokens: TokenCounts::default(),
            last_reported_tokens: TokenCounts::default(),
        };

        // Handshake step 1: initialize
        let read_timeout = Duration::from_millis(config.codex_read_timeout_ms());
        let init_result = session
            .send_request(
                "initialize",
                serde_json::json!({
                    "clientInfo": {
                        "name": "symphony",
                        "version": env!("CARGO_PKG_VERSION"),
                    },
                    "capabilities": {}
                }),
                read_timeout,
            )
            .await;

        if let Err(e) = init_result {
            let _ = event_tx
                .send(AgentEvent::StartupFailed {
                    reason: e.to_string(),
                })
                .await;
            return Err(e);
        }

        // Handshake step 2: initialized notification
        session
            .send_notification("initialized", serde_json::json!({}))
            .await?;

        // Handshake step 3: thread/start
        let thread_params = {
            let mut p = serde_json::json!({
                "cwd": cwd.to_string_lossy(),
            });
            if let Some(policy) = config.codex_approval_policy() {
                p["approvalPolicy"] = serde_json::Value::String(policy);
            }
            if let Some(sandbox) = config.codex_thread_sandbox() {
                p["sandbox"] = serde_json::Value::String(sandbox);
            }
            p
        };

        let thread_result = session
            .send_request("thread/start", thread_params, read_timeout)
            .await?;

        // Extract thread ID
        let thread_id = thread_result
            .pointer("/thread/id")
            .or_else(|| thread_result.pointer("/id"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| SymphonyError::AgentProtocol {
                method: "thread/start".into(),
                message: "missing thread id in response".into(),
            })?;

        session.thread_id = Some(thread_id.clone());

        let _ = event_tx
            .send(AgentEvent::SessionStarted {
                session_id: thread_id,
                thread_id: session.thread_id.clone().unwrap_or_default(),
                turn_id: String::new(),
                pid,
            })
            .await;

        Ok(session)
    }

    /// Run a full worker lifecycle: multiple turns until completion or limit.
    pub async fn run_worker(
        mut self,
        issue: &Issue,
        prompt: &str,
        attempt: Option<u32>,
        event_tx: mpsc::Sender<AgentEvent>,
        tracker: &dyn crate::tracker::IssueTracker,
    ) -> WorkerOutcome {
        let max_turns = self.config.max_turns();
        let mut turn_count = 0u32;
        let turn_timeout = Duration::from_millis(self.config.codex_turn_timeout_ms());

        // First turn uses the full rendered prompt.
        // If this is a retry attempt, prepend retry context.
        let mut current_prompt = if let Some(n) = attempt {
            format!(
                "[Retry attempt {n}] {prompt}"
            )
        } else {
            prompt.to_string()
        };

        loop {
            turn_count += 1;
            let turn_result = self
                .run_turn(&current_prompt, issue, &event_tx, turn_timeout)
                .await;

            match turn_result {
                Ok(turn_id) => {
                    let _ = event_tx
                        .send(AgentEvent::TurnCompleted {
                            turn_id,
                            turn_count,
                        })
                        .await;
                }
                Err(e) => {
                    let _ = event_tx
                        .send(AgentEvent::TurnFailed {
                            turn_id: String::new(),
                            reason: e.to_string(),
                        })
                        .await;
                    self.stop().await;
                    return WorkerOutcome::Failed {
                        reason: e.to_string(),
                        turn_count,
                        tokens: self.tokens.clone(),
                    };
                }
            }

            // Check if we should continue
            if turn_count >= max_turns {
                debug!(turn_count, max_turns, "reached max turns");
                break;
            }

            // Re-check tracker state
            let active_states = self.config.active_states();
            match tracker
                .fetch_issue_states_by_ids(std::slice::from_ref(&issue.id))
                .await
            {
                Ok(refreshed) => {
                    if let Some(updated) = refreshed.first() {
                        let is_active = active_states
                            .iter()
                            .any(|s| s.eq_ignore_ascii_case(&updated.state));
                        if !is_active {
                            info!(
                                issue_identifier = issue.identifier,
                                state = updated.state,
                                "issue no longer active, stopping turns"
                            );
                            break;
                        }
                    } else {
                        warn!(
                            issue_identifier = issue.identifier,
                            "issue not found during state refresh"
                        );
                        break;
                    }
                }
                Err(e) => {
                    warn!(
                        issue_identifier = issue.identifier,
                        "state refresh failed: {e}, stopping turns"
                    );
                    self.stop().await;
                    return WorkerOutcome::Failed {
                        reason: format!("issue state refresh error: {e}"),
                        turn_count,
                        tokens: self.tokens.clone(),
                    };
                }
            }

            // Continuation turns use lighter guidance
            current_prompt = format!(
                "Continue working on issue {}. This is turn {} of {}. \
                 Review what was accomplished in the previous turn and continue.",
                issue.identifier, turn_count + 1, max_turns
            );
        }

        self.stop().await;
        WorkerOutcome::Normal {
            turn_count,
            tokens: self.tokens.clone(),
        }
    }

    /// Run a single turn (public API for the orchestrator worker task).
    ///
    /// This is a simplified version that runs a single turn with the
    /// configured turn timeout.
    pub async fn run_turn_simple(
        &mut self,
        prompt: &str,
        issue: &Issue,
        event_tx: &mpsc::Sender<AgentEvent>,
    ) -> Result<String> {
        let timeout = Duration::from_millis(self.config.codex_turn_timeout_ms());
        self.run_turn(prompt, issue, event_tx, timeout).await
    }

    /// Run a single turn and process messages until completion.
    async fn run_turn(
        &mut self,
        prompt: &str,
        issue: &Issue,
        event_tx: &mpsc::Sender<AgentEvent>,
        timeout: Duration,
    ) -> Result<String> {
        let thread_id = self.thread_id.clone().ok_or_else(|| SymphonyError::AgentProtocol {
            method: "turn/start".into(),
            message: "no thread_id available".into(),
        })?;

        let mut turn_params = serde_json::json!({
            "threadId": thread_id,
            "input": [{"type": "text", "text": prompt}],
            "title": format!("{}: {}", issue.identifier, issue.title),
        });

        if let Some(policy) = self.config.codex_approval_policy() {
            turn_params["approvalPolicy"] = serde_json::Value::String(policy);
        }
        if let Some(sandbox) = self.config.codex_turn_sandbox_policy() {
            turn_params["sandboxPolicy"] = serde_json::json!({"type": sandbox});
        }

        let read_timeout = Duration::from_millis(self.config.codex_read_timeout_ms());
        let turn_result = self
            .send_request("turn/start", turn_params, read_timeout)
            .await?;

        let turn_id = turn_result
            .pointer("/turn/id")
            .or_else(|| turn_result.pointer("/id"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        // Process messages until turn completes or times out
        let result = tokio::time::timeout(timeout, self.process_turn_messages(event_tx)).await;

        match result {
            Ok(Ok(())) => Ok(turn_id),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(SymphonyError::AgentTimeout {
                duration_secs: timeout.as_secs(),
            }),
        }
    }

    /// Process messages from the agent until a turn-ending event.
    async fn process_turn_messages(
        &mut self,
        event_tx: &mpsc::Sender<AgentEvent>,
    ) -> Result<()> {
        loop {
            let msg = self
                .message_rx
                .recv()
                .await
                .ok_or_else(|| SymphonyError::Agent {
                    reason: "agent process connection closed".into(),
                })?;

            match msg {
                AppServerMessage::Notification(n) => {
                    match n.method.as_str() {
                        "turn/completed" => return Ok(()),
                        "turn/failed" => {
                            let reason = n
                                .params
                                .get("error")
                                .and_then(|e| e.as_str())
                                .unwrap_or("unknown failure")
                                .to_string();
                            return Err(SymphonyError::AgentProtocol {
                                method: "turn/failed".into(),
                                message: reason,
                            });
                        }
                        "turn/cancelled" => {
                            return Err(SymphonyError::AgentProtocol {
                                method: "turn/cancelled".into(),
                                message: "turn was cancelled".into(),
                            });
                        }
                        _ => {
                            // Extract token usage if present
                            self.extract_token_usage(&n.params, event_tx).await;

                            let _ = event_tx
                                .send(AgentEvent::Notification {
                                    message: n.method.clone(),
                                })
                                .await;
                        }
                    }
                }
                AppServerMessage::Request(req) => {
                    self.handle_server_request(req, event_tx).await?;
                }
                AppServerMessage::Response(_) => {
                    // Late response to a pending request - dispatch it
                    // (handled by the pending map in send_request)
                }
                AppServerMessage::Malformed(line) => {
                    if !line.is_empty() {
                        debug!("malformed agent message: {}", &line[..line.len().min(200)]);
                    }
                }
            }
        }
    }

    /// Handle a server-initiated request (approval, tool call, user input).
    async fn handle_server_request(
        &mut self,
        req: JsonRpcRequest,
        event_tx: &mpsc::Sender<AgentEvent>,
    ) -> Result<()> {
        match req.method.as_str() {
            // Auto-approve command execution and file changes
            "item/commandExecution/requestApproval"
            | "item/fileChange/requestApproval" => {
                let _ = event_tx
                    .send(AgentEvent::ApprovalAutoApproved {
                        approval_type: req.method.clone(),
                    })
                    .await;
                self.send_response(&req.id, serde_json::json!({"approved": true}))
                    .await?;
            }
            // User input required: hard failure
            "item/tool/requestUserInput" => {
                return Err(SymphonyError::AgentUserInputRequired);
            }
            // Unsupported tool calls
            m if m.starts_with("item/tool/") => {
                let tool_name = req
                    .params
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let _ = event_tx
                    .send(AgentEvent::UnsupportedToolCall {
                        tool_name: tool_name.clone(),
                    })
                    .await;
                self.send_response(
                    &req.id,
                    serde_json::json!({"success": false, "error": "unsupported_tool_call"}),
                )
                .await?;
            }
            _ => {
                debug!(method = req.method, "unhandled server request");
            }
        }
        Ok(())
    }

    /// Extract token usage from a notification payload.
    async fn extract_token_usage(
        &mut self,
        params: &serde_json::Value,
        event_tx: &mpsc::Sender<AgentEvent>,
    ) {
        // Look for absolute totals in common locations
        let usage = params
            .get("total_token_usage")
            .or_else(|| params.get("usage"))
            .or_else(|| params.get("tokenUsage"));

        if let Some(u) = usage {
            let input = u
                .get("input_tokens")
                .or_else(|| u.get("inputTokens"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let output = u
                .get("output_tokens")
                .or_else(|| u.get("outputTokens"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let total = u
                .get("total_tokens")
                .or_else(|| u.get("totalTokens"))
                .and_then(|v| v.as_u64())
                .unwrap_or(input + output);

            // Track deltas relative to last reported totals
            if total > self.last_reported_tokens.total_tokens {
                self.tokens.input_tokens += input.saturating_sub(self.last_reported_tokens.input_tokens);
                self.tokens.output_tokens += output.saturating_sub(self.last_reported_tokens.output_tokens);
                self.tokens.total_tokens += total.saturating_sub(self.last_reported_tokens.total_tokens);

                self.last_reported_tokens = TokenCounts {
                    input_tokens: input,
                    output_tokens: output,
                    total_tokens: total,
                };

                let _ = event_tx
                    .send(AgentEvent::TokenUsage {
                        input_tokens: self.tokens.input_tokens,
                        output_tokens: self.tokens.output_tokens,
                        total_tokens: self.tokens.total_tokens,
                    })
                    .await;
            }
        }
    }

    /// Send a JSON-RPC request and wait for the response.
    async fn send_request(
        &mut self,
        method: &str,
        params: serde_json::Value,
        timeout: Duration,
    ) -> Result<serde_json::Value> {
        let id = self.next_id;
        self.next_id += 1;

        let request = serde_json::json!({
            "id": id,
            "method": method,
            "params": params,
        });

        let mut line = serde_json::to_string(&request).map_err(|e| SymphonyError::AgentProtocol {
            method: method.into(),
            message: format!("serialisation error: {e}"),
        })?;
        line.push('\n');

        self.stdin
            .write_all(line.as_bytes())
            .await
            .map_err(|e| SymphonyError::AgentProtocol {
                method: method.into(),
                message: format!("write error: {e}"),
            })?;
        self.stdin.flush().await.map_err(|e| SymphonyError::AgentProtocol {
            method: method.into(),
            message: format!("flush error: {e}"),
        })?;

        // Wait for response with matching ID
        let result = tokio::time::timeout(timeout, self.wait_for_response(id)).await;

        match result {
            Ok(Ok(value)) => Ok(value),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(SymphonyError::AgentProtocol {
                method: method.into(),
                message: format!("response timeout after {}ms", timeout.as_millis()),
            }),
        }
    }

    /// Wait for a response with the given ID, processing other messages.
    async fn wait_for_response(&mut self, expected_id: u64) -> Result<serde_json::Value> {
        loop {
            let msg = self
                .message_rx
                .recv()
                .await
                .ok_or_else(|| SymphonyError::Agent {
                    reason: "agent process closed during request".into(),
                })?;

            match msg {
                AppServerMessage::Response(r) => {
                    if r.id == serde_json::json!(expected_id) {
                        if let Some(err) = r.error {
                            return Err(SymphonyError::AgentProtocol {
                                method: format!("response({})", expected_id),
                                message: err.message,
                            });
                        }
                        return Ok(r.result.unwrap_or(serde_json::Value::Null));
                    }
                    // Response for a different ID - dispatch to pending waiter
                    if let Some(id_num) = r.id.as_u64() {
                        if let Some(sender) = self.pending.remove(&id_num) {
                            let value = r.result.unwrap_or(serde_json::Value::Null);
                            let _ = sender.send(value);
                        }
                    }
                }
                AppServerMessage::Notification(_) | AppServerMessage::Request(_) => {
                    // During handshake, ignore notifications/requests
                }
                AppServerMessage::Malformed(_) => {}
            }
        }
    }

    /// Send a JSON-RPC notification (no response expected).
    async fn send_notification(
        &mut self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<()> {
        let notification = serde_json::json!({
            "method": method,
            "params": params,
        });

        let mut line =
            serde_json::to_string(&notification).map_err(|e| SymphonyError::AgentProtocol {
                method: method.into(),
                message: format!("serialisation error: {e}"),
            })?;
        line.push('\n');

        self.stdin
            .write_all(line.as_bytes())
            .await
            .map_err(|e| SymphonyError::AgentProtocol {
                method: method.into(),
                message: format!("write error: {e}"),
            })?;
        self.stdin.flush().await.map_err(|e| SymphonyError::AgentProtocol {
            method: method.into(),
            message: format!("flush error: {e}"),
        })?;

        Ok(())
    }

    /// Send a JSON-RPC response to a server-initiated request.
    async fn send_response(
        &mut self,
        id: &serde_json::Value,
        result: serde_json::Value,
    ) -> Result<()> {
        let response = serde_json::json!({
            "id": id,
            "result": result,
        });

        let mut line =
            serde_json::to_string(&response).map_err(|e| SymphonyError::AgentProtocol {
                method: "response".into(),
                message: format!("serialisation error: {e}"),
            })?;
        line.push('\n');

        self.stdin
            .write_all(line.as_bytes())
            .await
            .map_err(|e| SymphonyError::AgentProtocol {
                method: "response".into(),
                message: format!("write error: {e}"),
            })?;
        self.stdin.flush().await.map_err(|e| SymphonyError::AgentProtocol {
            method: "response".into(),
            message: format!("flush error: {e}"),
        })?;

        Ok(())
    }

    /// Get the accumulated token counts for this session.
    pub fn accumulated_tokens(&self) -> TokenCounts {
        self.tokens.clone()
    }

    /// Stop the agent subprocess gracefully.
    pub async fn stop(&mut self) {
        // Try to kill the process
        if let Err(e) = self.child.kill().await {
            debug!("failed to kill agent process: {e}");
        }
        let _ = self.child.wait().await;
    }

    /// Read JSONL messages from stdout.
    async fn read_stdout(
        mut reader: BufReader<tokio::process::ChildStdout>,
        tx: mpsc::Sender<AppServerMessage>,
    ) {
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line).await {
                Ok(0) => break, // EOF
                Ok(_) => {
                    let msg = AppServerMessage::parse_line(&line);
                    if tx.send(msg).await.is_err() {
                        break;
                    }
                }
                Err(e) => {
                    error!("stdout read error: {e}");
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
                        debug!(target: "symphony::agent::stderr", "{trimmed}");
                    }
                }
                Err(_) => break,
            }
        }
    }
}
