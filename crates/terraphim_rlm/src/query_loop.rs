//! Query loop orchestration for RLM.
//!
//! The QueryLoop is the core execution engine that:
//! 1. Sends prompts to the LLM
//! 2. Parses commands from LLM responses
//! 3. Executes commands in the execution environment
//! 4. Feeds results back to the LLM
//! 5. Repeats until FINAL or budget exhaustion

use std::sync::Arc;

use jiff::Timestamp;
use tokio::sync::mpsc;

use crate::budget::BudgetTracker;
use crate::error::{RlmError, RlmResult};
use crate::executor::{ExecutionContext, ExecutionEnvironment, ExecutionResult};
use crate::llm_bridge::{LlmBridge, QueryRequest, QueryResponse};
use crate::parser::CommandParser;
use crate::session::SessionManager;
use crate::types::{Command, CommandHistory, CommandHistoryEntry, QueryMetadata, SessionId};

// Re-export QueryResponse for external use
pub use crate::llm_bridge::QueryResponse as LlmResponse;

/// Default maximum iterations per query loop.
pub const DEFAULT_MAX_ITERATIONS: u32 = 100;

/// Result of a query loop execution.
#[derive(Debug, Clone)]
pub struct QueryLoopResult {
    /// The final result (if FINAL was reached).
    pub result: Option<String>,
    /// Whether the loop completed successfully.
    pub success: bool,
    /// Reason for termination.
    pub termination_reason: TerminationReason,
    /// Number of iterations executed.
    pub iterations: u32,
    /// Command history from this execution.
    pub history: CommandHistory,
    /// Query metadata.
    pub metadata: QueryMetadata,
}

/// Reason why the query loop terminated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminationReason {
    /// FINAL command was executed.
    FinalReached,
    /// FINAL_VAR command was executed.
    FinalVarReached { variable: String },
    /// Token budget exhausted.
    TokenBudgetExhausted,
    /// Time budget exhausted.
    TimeBudgetExhausted,
    /// Maximum iterations reached.
    MaxIterationsReached,
    /// Maximum recursion depth reached.
    RecursionDepthExhausted,
    /// Error occurred during execution.
    Error { message: String },
    /// Cancelled by user.
    Cancelled,
}

/// Configuration for the query loop.
#[derive(Debug, Clone)]
pub struct QueryLoopConfig {
    /// Maximum iterations before forced termination.
    pub max_iterations: u32,
    /// Whether to allow recursive LLM calls.
    pub allow_recursion: bool,
    /// Maximum recursion depth.
    pub max_recursion_depth: u32,
    /// Whether to use strict command parsing.
    pub strict_parsing: bool,
    /// Timeout for individual command execution (ms).
    pub command_timeout_ms: u64,
}

impl Default for QueryLoopConfig {
    fn default() -> Self {
        Self {
            max_iterations: DEFAULT_MAX_ITERATIONS,
            allow_recursion: true,
            max_recursion_depth: crate::DEFAULT_MAX_RECURSION_DEPTH,
            strict_parsing: false,
            command_timeout_ms: 30_000,
        }
    }
}

/// The query loop orchestrator.
pub struct QueryLoop<E: ExecutionEnvironment + ?Sized> {
    /// Session manager for session state.
    session_manager: Arc<SessionManager>,
    /// Budget tracker for resource limits (per-session).
    budget_tracker: Arc<BudgetTracker>,
    /// LLM bridge for recursive calls.
    llm_bridge: Arc<LlmBridge>,
    /// Execution environment.
    executor: Arc<E>,
    /// Command parser.
    parser: CommandParser,
    /// Configuration.
    config: QueryLoopConfig,
    /// Session ID this loop is executing for.
    session_id: SessionId,
    /// Cancellation receiver.
    cancel_rx: Option<mpsc::Receiver<()>>,
}

impl<E: ExecutionEnvironment + ?Sized> QueryLoop<E> {
    /// Create a new query loop for a specific session.
    pub fn new(
        session_id: SessionId,
        session_manager: Arc<SessionManager>,
        budget_tracker: Arc<BudgetTracker>,
        llm_bridge: Arc<LlmBridge>,
        executor: Arc<E>,
        config: QueryLoopConfig,
    ) -> Self {
        let parser = if config.strict_parsing {
            CommandParser::strict()
        } else {
            CommandParser::new()
        };

        Self {
            session_manager,
            budget_tracker,
            llm_bridge,
            executor,
            parser,
            config,
            session_id,
            cancel_rx: None,
        }
    }

    /// Set a cancellation channel.
    pub fn with_cancel_channel(mut self, rx: mpsc::Receiver<()>) -> Self {
        self.cancel_rx = Some(rx);
        self
    }

    /// Execute the query loop.
    pub async fn execute(&mut self, initial_prompt: &str) -> RlmResult<QueryLoopResult> {
        let mut metadata = QueryMetadata::new(self.session_id);
        let mut history = CommandHistory::new();
        let mut current_prompt = initial_prompt.to_string();
        let mut context_messages: Vec<String> = Vec::new();

        // Build execution context
        let exec_ctx = ExecutionContext {
            session_id: self.session_id,
            timeout_ms: self.config.command_timeout_ms,
            ..Default::default()
        };

        loop {
            // Check for cancellation
            if let Some(ref mut rx) = self.cancel_rx {
                if rx.try_recv().is_ok() {
                    return Ok(QueryLoopResult {
                        result: None,
                        success: false,
                        termination_reason: TerminationReason::Cancelled,
                        iterations: metadata.iteration,
                        history,
                        metadata,
                    });
                }
            }

            // Check iteration limit
            if metadata.iteration >= self.config.max_iterations {
                log::warn!(
                    "Query loop reached max iterations ({}) for session {}",
                    self.config.max_iterations,
                    self.session_id
                );
                return Ok(QueryLoopResult {
                    result: None,
                    success: false,
                    termination_reason: TerminationReason::MaxIterationsReached,
                    iterations: metadata.iteration,
                    history,
                    metadata,
                });
            }

            // Check budget
            if let Some(reason) = self.check_budget() {
                return Ok(QueryLoopResult {
                    result: None,
                    success: false,
                    termination_reason: reason,
                    iterations: metadata.iteration,
                    history,
                    metadata,
                });
            }

            metadata.iteration += 1;

            // Build full prompt with context
            let full_prompt = self.build_prompt(&current_prompt, &context_messages);

            // Call LLM
            let llm_response = match self.call_llm(&full_prompt).await {
                Ok(resp) => resp,
                Err(e) => {
                    return Ok(QueryLoopResult {
                        result: None,
                        success: false,
                        termination_reason: TerminationReason::Error {
                            message: e.to_string(),
                        },
                        iterations: metadata.iteration,
                        history,
                        metadata,
                    });
                }
            };

            // Parse command from response
            let command = match self.parser.parse_one(&llm_response.response) {
                Ok(cmd) => cmd,
                Err(e) => {
                    log::warn!("Failed to parse command from LLM response: {}", e);
                    // Add error to context and retry
                    context_messages.push(format!(
                        "Error: Could not parse your response. Please use a valid command format.\nYour response was: {}\nError: {}",
                        truncate(&llm_response.response, 500),
                        e
                    ));
                    continue;
                }
            };

            // Execute command
            match self
                .execute_command(&command, &exec_ctx, &mut history)
                .await
            {
                Ok(ExecuteResult::Final(result)) => {
                    metadata.complete();
                    return Ok(QueryLoopResult {
                        result: Some(result),
                        success: true,
                        termination_reason: TerminationReason::FinalReached,
                        iterations: metadata.iteration,
                        history,
                        metadata,
                    });
                }
                Ok(ExecuteResult::FinalVar { variable, value }) => {
                    metadata.complete();
                    return Ok(QueryLoopResult {
                        result: Some(value),
                        success: true,
                        termination_reason: TerminationReason::FinalVarReached { variable },
                        iterations: metadata.iteration,
                        history,
                        metadata,
                    });
                }
                Ok(ExecuteResult::Continue { output }) => {
                    // Add output to context for next iteration
                    context_messages.push(output);
                    current_prompt =
                        "Continue with the task based on the above output.".to_string();
                }
                Ok(ExecuteResult::RecursiveResult { output }) => {
                    // Add recursive LLM result to context
                    context_messages.push(format!("LLM sub-query result: {output}"));
                    current_prompt =
                        "Continue with the task using the sub-query result.".to_string();
                }
                Err(e) => {
                    // Add error to context
                    context_messages.push(format!("Error executing command: {e}"));
                    current_prompt =
                        "The previous command failed. Please try a different approach.".to_string();
                }
            }
        }
    }

    /// Check budget and return termination reason if exhausted.
    fn check_budget(&self) -> Option<TerminationReason> {
        let status = self.budget_tracker.status();

        if status.tokens_exhausted() {
            return Some(TerminationReason::TokenBudgetExhausted);
        }
        if status.time_exhausted() {
            return Some(TerminationReason::TimeBudgetExhausted);
        }
        if status.depth_exhausted() {
            return Some(TerminationReason::RecursionDepthExhausted);
        }

        None
    }

    /// Build a full prompt including context messages.
    fn build_prompt(&self, prompt: &str, context: &[String]) -> String {
        if context.is_empty() {
            return prompt.to_string();
        }

        let mut full = String::new();
        for (i, msg) in context.iter().enumerate() {
            full.push_str(&format!("[Step {}] {}\n\n", i + 1, msg));
        }
        full.push_str(prompt);
        full
    }

    /// Call the LLM.
    async fn call_llm(&self, prompt: &str) -> RlmResult<QueryResponse> {
        let request = QueryRequest {
            prompt: prompt.to_string(),
            model: None,
            temperature: None,
            max_tokens: None,
        };

        self.llm_bridge.query(&self.session_id, request).await
    }

    /// Execute a single command.
    async fn execute_command(
        &self,
        command: &Command,
        ctx: &ExecutionContext,
        history: &mut CommandHistory,
    ) -> RlmResult<ExecuteResult> {
        let start = Timestamp::now();

        match command {
            Command::Final(result) => {
                history.push(CommandHistoryEntry {
                    command: command.clone(),
                    success: true,
                    stdout: result.clone(),
                    stderr: String::new(),
                    exit_code: Some(0),
                    execution_time_ms: 0,
                    executed_at: start,
                });
                Ok(ExecuteResult::Final(result.clone()))
            }

            Command::FinalVar(variable) => {
                // Get variable value from session context
                let session = self.session_manager.get_session(&self.session_id)?;
                let value = session
                    .context_variables
                    .get(variable)
                    .cloned()
                    .unwrap_or_else(|| format!("<undefined: {variable}>"));

                history.push(CommandHistoryEntry {
                    command: command.clone(),
                    success: true,
                    stdout: value.clone(),
                    stderr: String::new(),
                    exit_code: Some(0),
                    execution_time_ms: 0,
                    executed_at: start,
                });

                Ok(ExecuteResult::FinalVar {
                    variable: variable.clone(),
                    value,
                })
            }

            Command::Run(bash_cmd) => {
                let result = self
                    .executor
                    .execute_command(&bash_cmd.command, ctx)
                    .await
                    .map_err(|e| RlmError::ExecutionFailed {
                        message: e.to_string(),
                        exit_code: None,
                        stdout: None,
                        stderr: None,
                    })?;
                let elapsed = elapsed_ms(start);

                let success = result.exit_code == 0;
                history.push(CommandHistoryEntry {
                    command: command.clone(),
                    success,
                    stdout: result.stdout.clone(),
                    stderr: result.stderr.clone(),
                    exit_code: Some(result.exit_code),
                    execution_time_ms: elapsed,
                    executed_at: start,
                });

                let output = format_execution_output(&result);
                Ok(ExecuteResult::Continue { output })
            }

            Command::Code(python_code) => {
                let result = self
                    .executor
                    .execute_code(&python_code.code, ctx)
                    .await
                    .map_err(|e| RlmError::ExecutionFailed {
                        message: e.to_string(),
                        exit_code: None,
                        stdout: None,
                        stderr: None,
                    })?;
                let elapsed = elapsed_ms(start);

                let success = result.exit_code == 0;
                history.push(CommandHistoryEntry {
                    command: command.clone(),
                    success,
                    stdout: result.stdout.clone(),
                    stderr: result.stderr.clone(),
                    exit_code: Some(result.exit_code),
                    execution_time_ms: elapsed,
                    executed_at: start,
                });

                let output = format_execution_output(&result);
                Ok(ExecuteResult::Continue { output })
            }

            Command::Snapshot(name) => {
                let snapshot_id = self
                    .executor
                    .create_snapshot(&self.session_id, name)
                    .await
                    .map_err(|e| RlmError::SnapshotCreationFailed {
                        message: e.to_string(),
                    })?;
                let elapsed = elapsed_ms(start);

                history.push(CommandHistoryEntry {
                    command: command.clone(),
                    success: true,
                    stdout: format!("Snapshot created: {}", snapshot_id.name),
                    stderr: String::new(),
                    exit_code: Some(0),
                    execution_time_ms: elapsed,
                    executed_at: start,
                });

                Ok(ExecuteResult::Continue {
                    output: format!("Snapshot '{}' created successfully.", name),
                })
            }

            Command::Rollback(name) => {
                // Find snapshot by name
                let snapshots = self
                    .executor
                    .list_snapshots(&self.session_id)
                    .await
                    .map_err(|e| RlmError::SnapshotRestoreFailed {
                        message: e.to_string(),
                    })?;
                let snapshot = snapshots.iter().find(|s| s.name == *name).ok_or_else(|| {
                    RlmError::SnapshotNotFound {
                        snapshot_id: name.clone(),
                    }
                })?;

                self.executor
                    .restore_snapshot(snapshot)
                    .await
                    .map_err(|e| RlmError::SnapshotRestoreFailed {
                        message: e.to_string(),
                    })?;
                let elapsed = elapsed_ms(start);

                history.push(CommandHistoryEntry {
                    command: command.clone(),
                    success: true,
                    stdout: format!("Restored to snapshot: {name}"),
                    stderr: String::new(),
                    exit_code: Some(0),
                    execution_time_ms: elapsed,
                    executed_at: start,
                });

                Ok(ExecuteResult::Continue {
                    output: format!("Rolled back to snapshot '{name}'"),
                })
            }

            Command::QueryLlm(query) => {
                if !self.config.allow_recursion {
                    return Err(RlmError::RecursionDepthExceeded {
                        depth: 1,
                        max_depth: 0,
                    });
                }

                // Increment recursion depth
                self.budget_tracker.push_recursion()?;

                let response = self.call_llm(&query.prompt).await?;
                let elapsed = elapsed_ms(start);

                // Consume tokens
                self.budget_tracker.add_tokens(response.tokens_used)?;

                history.push(CommandHistoryEntry {
                    command: command.clone(),
                    success: true,
                    stdout: response.response.clone(),
                    stderr: String::new(),
                    exit_code: Some(0),
                    execution_time_ms: elapsed,
                    executed_at: start,
                });

                // Decrement depth after completion
                self.budget_tracker.pop_recursion();

                Ok(ExecuteResult::RecursiveResult {
                    output: response.response,
                })
            }

            Command::QueryLlmBatched(queries) => {
                if !self.config.allow_recursion {
                    return Err(RlmError::RecursionDepthExceeded {
                        depth: 1,
                        max_depth: 0,
                    });
                }

                // Increment recursion depth
                self.budget_tracker.push_recursion()?;

                let mut results = Vec::new();
                for query in queries {
                    let response = self.call_llm(&query.prompt).await?;
                    self.budget_tracker.add_tokens(response.tokens_used)?;
                    results.push(response.response);
                }

                let elapsed = elapsed_ms(start);
                let combined = results.join("\n---\n");

                history.push(CommandHistoryEntry {
                    command: command.clone(),
                    success: true,
                    stdout: combined.clone(),
                    stderr: String::new(),
                    exit_code: Some(0),
                    execution_time_ms: elapsed,
                    executed_at: start,
                });

                // Decrement depth after completion
                self.budget_tracker.pop_recursion();

                Ok(ExecuteResult::RecursiveResult { output: combined })
            }
        }
    }
}

/// Result of executing a single command.
enum ExecuteResult {
    /// FINAL was reached with a result.
    Final(String),
    /// FINAL_VAR was reached.
    FinalVar { variable: String, value: String },
    /// Continue loop with output.
    Continue { output: String },
    /// Recursive LLM call completed.
    RecursiveResult { output: String },
}

/// Calculate elapsed milliseconds since a timestamp.
fn elapsed_ms(start: Timestamp) -> u64 {
    let now = Timestamp::now();
    let duration = now.since(start).ok();
    duration.map(|d| d.get_milliseconds() as u64).unwrap_or(0)
}

/// Format execution result for context.
fn format_execution_output(result: &ExecutionResult) -> String {
    let mut output = String::new();

    if !result.stdout.is_empty() {
        output.push_str(&format!("stdout:\n{}\n", result.stdout));
    }
    if !result.stderr.is_empty() {
        output.push_str(&format!("stderr:\n{}\n", result.stderr));
    }
    output.push_str(&format!("exit_code: {}", result.exit_code));

    if output.is_empty() {
        output = "(no output)".to_string();
    }

    output
}

/// Truncate a string for display.
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_loop_config_default() {
        let config = QueryLoopConfig::default();
        assert_eq!(config.max_iterations, DEFAULT_MAX_ITERATIONS);
        assert!(config.allow_recursion);
    }

    #[test]
    fn test_termination_reason_equality() {
        assert_eq!(
            TerminationReason::FinalReached,
            TerminationReason::FinalReached
        );
        assert_ne!(
            TerminationReason::FinalReached,
            TerminationReason::Cancelled
        );
    }

    #[test]
    fn test_format_execution_output() {
        let result = ExecutionResult {
            stdout: "hello".to_string(),
            stderr: "warning".to_string(),
            exit_code: 0,
            execution_time_ms: 0,
            output_truncated: false,
            output_file_path: None,
            timed_out: false,
            metadata: std::collections::HashMap::new(),
        };

        let output = format_execution_output(&result);
        assert!(output.contains("hello"));
        assert!(output.contains("warning"));
        assert!(output.contains("exit_code: 0"));
    }

    #[test]
    fn test_format_execution_output_empty() {
        let result = ExecutionResult {
            stdout: String::new(),
            stderr: String::new(),
            exit_code: 0,
            execution_time_ms: 0,
            output_truncated: false,
            output_file_path: None,
            timed_out: false,
            metadata: std::collections::HashMap::new(),
        };

        let output = format_execution_output(&result);
        // Even with empty stdout/stderr, we still show exit_code
        assert!(output.contains("exit_code: 0"));
    }

    #[test]
    fn test_truncate_short_string() {
        let s = "hello world";
        assert_eq!(truncate(s, 100), "hello world");
    }

    #[test]
    fn test_truncate_long_string() {
        let s = "a".repeat(200);
        let result = truncate(&s, 50);
        assert_eq!(result.len(), 53); // 50 chars + "..."
        assert!(result.ends_with("..."));
    }

    #[test]
    fn test_elapsed_ms() {
        let start = Timestamp::now();
        // Small delay to ensure some time passes
        std::thread::sleep(std::time::Duration::from_millis(10));
        let elapsed = elapsed_ms(start);
        assert!(elapsed >= 10);
    }

    #[test]
    fn test_query_loop_config_custom() {
        let config = QueryLoopConfig {
            max_iterations: 50,
            allow_recursion: false,
            max_recursion_depth: 5,
            strict_parsing: true,
            command_timeout_ms: 10_000,
        };
        assert_eq!(config.max_iterations, 50);
        assert!(!config.allow_recursion);
        assert!(config.strict_parsing);
    }

    #[test]
    fn test_termination_reason_variants() {
        // Test all termination reason variants
        let final_reached = TerminationReason::FinalReached;
        assert_eq!(format!("{:?}", final_reached), "FinalReached");

        let final_var = TerminationReason::FinalVarReached {
            variable: "result".to_string(),
        };
        assert!(matches!(
            final_var,
            TerminationReason::FinalVarReached { .. }
        ));

        let token_exhausted = TerminationReason::TokenBudgetExhausted;
        let time_exhausted = TerminationReason::TimeBudgetExhausted;
        let max_iters = TerminationReason::MaxIterationsReached;
        let depth_exhausted = TerminationReason::RecursionDepthExhausted;
        let cancelled = TerminationReason::Cancelled;

        assert_ne!(token_exhausted, time_exhausted);
        assert_ne!(max_iters, cancelled);
        assert_ne!(depth_exhausted, TerminationReason::FinalReached);

        let error = TerminationReason::Error {
            message: "test error".to_string(),
        };
        assert!(matches!(error, TerminationReason::Error { .. }));
    }

    #[test]
    fn test_execute_result_variants() {
        let final_result = ExecuteResult::Final("done".to_string());
        assert!(matches!(final_result, ExecuteResult::Final(s) if s == "done"));

        let final_var = ExecuteResult::FinalVar {
            variable: "x".to_string(),
            value: "42".to_string(),
        };
        assert!(matches!(final_var, ExecuteResult::FinalVar { .. }));

        let cont = ExecuteResult::Continue {
            output: "step done".to_string(),
        };
        assert!(matches!(cont, ExecuteResult::Continue { .. }));

        let recursive = ExecuteResult::RecursiveResult {
            output: "sub-result".to_string(),
        };
        assert!(matches!(recursive, ExecuteResult::RecursiveResult { .. }));
    }

    #[test]
    fn test_query_loop_result_creation() {
        let result = QueryLoopResult {
            result: Some("test result".to_string()),
            success: true,
            termination_reason: TerminationReason::FinalReached,
            iterations: 5,
            history: CommandHistory::new(),
            metadata: QueryMetadata::new(SessionId::new()),
        };

        assert!(result.success);
        assert_eq!(result.iterations, 5);
        assert_eq!(result.result, Some("test result".to_string()));
    }

    #[test]
    fn test_query_loop_result_with_error() {
        let result = QueryLoopResult {
            result: Some("error occurred".to_string()),
            success: false,
            termination_reason: TerminationReason::Error {
                message: "test error".to_string(),
            },
            iterations: 3,
            history: CommandHistory::new(),
            metadata: QueryMetadata::new(SessionId::new()),
        };

        assert!(!result.success);
        assert!(matches!(
            result.termination_reason,
            TerminationReason::Error { .. }
        ));
    }

    #[test]
    fn test_query_loop_result_budget_exhaustion() {
        let token_exhausted = QueryLoopResult {
            result: Some(String::new()),
            success: false,
            termination_reason: TerminationReason::TokenBudgetExhausted,
            iterations: 10,
            history: CommandHistory::new(),
            metadata: QueryMetadata::new(SessionId::new()),
        };
        assert!(matches!(
            token_exhausted.termination_reason,
            TerminationReason::TokenBudgetExhausted
        ));

        let time_exhausted = QueryLoopResult {
            result: Some(String::new()),
            success: false,
            termination_reason: TerminationReason::TimeBudgetExhausted,
            iterations: 10,
            history: CommandHistory::new(),
            metadata: QueryMetadata::new(SessionId::new()),
        };
        assert!(matches!(
            time_exhausted.termination_reason,
            TerminationReason::TimeBudgetExhausted
        ));

        let depth_exhausted = QueryLoopResult {
            result: Some(String::new()),
            success: false,
            termination_reason: TerminationReason::RecursionDepthExhausted,
            iterations: 10,
            history: CommandHistory::new(),
            metadata: QueryMetadata::new(SessionId::new()),
        };
        assert!(matches!(
            depth_exhausted.termination_reason,
            TerminationReason::RecursionDepthExhausted
        ));
    }

    #[test]
    fn test_query_loop_result_max_iterations() {
        let result = QueryLoopResult {
            result: Some("max iterations reached".to_string()),
            success: true,
            termination_reason: TerminationReason::MaxIterationsReached,
            iterations: 100,
            history: CommandHistory::new(),
            metadata: QueryMetadata::new(SessionId::new()),
        };

        assert!(result.success);
        assert!(matches!(
            result.termination_reason,
            TerminationReason::MaxIterationsReached
        ));
        assert_eq!(result.iterations, 100);
    }

    #[test]
    fn test_query_loop_result_cancelled() {
        let result = QueryLoopResult {
            result: None,
            success: false,
            termination_reason: TerminationReason::Cancelled,
            iterations: 2,
            history: CommandHistory::new(),
            metadata: QueryMetadata::new(SessionId::new()),
        };

        assert!(!result.success);
        assert!(matches!(
            result.termination_reason,
            TerminationReason::Cancelled
        ));
    }

    #[test]
    fn test_format_execution_output_with_truncation() {
        let result = ExecutionResult {
            stdout: "long output".to_string(),
            stderr: String::new(),
            exit_code: 1,
            execution_time_ms: 500,
            output_truncated: true,
            output_file_path: Some("/tmp/output.txt".to_string()),
            timed_out: false,
            metadata: std::collections::HashMap::new(),
        };

        let output = format_execution_output(&result);
        assert!(output.contains("long output"));
        assert!(output.contains("exit_code: 1"));
    }

    #[test]
    fn test_format_execution_output_timed_out() {
        let result = ExecutionResult {
            stdout: String::new(),
            stderr: "timeout".to_string(),
            exit_code: -1,
            execution_time_ms: 30_000,
            output_truncated: false,
            output_file_path: None,
            timed_out: true,
            metadata: std::collections::HashMap::new(),
        };

        let output = format_execution_output(&result);
        assert!(output.contains("timeout"));
        assert!(output.contains("exit_code: -1"));
    }
}
