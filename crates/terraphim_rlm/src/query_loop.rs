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
use crate::validator::KnowledgeGraphValidator;

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
    /// Knowledge graph validator applied before every Run/Code command.
    validator: Arc<KnowledgeGraphValidator>,
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
        validator: Arc<KnowledgeGraphValidator>,
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
            validator,
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
                let vr = self.validator.validate(&bash_cmd.command)?;
                if !vr.passed {
                    if vr.escalation_required {
                        return Err(RlmError::KgEscalationRequired {
                            unknown_terms: vr.unmatched_words,
                            suggested_action: vr.suggestions.join("; "),
                            context: vr.message,
                        });
                    }
                    return Err(RlmError::KgValidationFailed {
                        unknown_terms: vr.unmatched_words,
                    });
                }
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
                let vr = self.validator.validate(&python_code.code)?;
                if !vr.passed {
                    if vr.escalation_required {
                        return Err(RlmError::KgEscalationRequired {
                            unknown_terms: vr.unmatched_words,
                            suggested_action: vr.suggestions.join("; "),
                            context: vr.message,
                        });
                    }
                    return Err(RlmError::KgValidationFailed {
                        unknown_terms: vr.unmatched_words,
                    });
                }
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
#[derive(Debug)]
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
        let boundary = s.floor_char_boundary(max_len);
        format!("{}...", &s[..boundary])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_multibyte() {
        // Each CJK char is 3 bytes; 200 of them = 600 bytes, > 500 limit.
        // Without floor_char_boundary, slicing at byte 500 panics mid-char.
        let cjk = "中".repeat(200);
        let result = truncate(&cjk, 500);
        assert!(result.ends_with("..."));
        assert!(std::str::from_utf8(result.as_bytes()).is_ok());
    }

    #[test]
    fn test_truncate_ascii() {
        let s = "hello world";
        assert_eq!(truncate(s, 5), "hello...");
        assert_eq!(truncate(s, 100), "hello world");
    }

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

    // -----------------------------------------------------------------------
    // KG validation wiring tests (require kg-validation feature)
    // -----------------------------------------------------------------------

    #[cfg(feature = "kg-validation")]
    mod validation {
        use super::*;
        use crate::budget::BudgetTracker;
        use crate::config::{BackendType, RlmConfig};
        use crate::executor::{
            Capability, ExecutionContext, ExecutionEnvironment, ExecutionResult, SnapshotId,
            ValidationResult,
        };
        use crate::llm_bridge::{LlmBridge, LlmBridgeConfig};
        use crate::session::SessionManager;
        use crate::types::{BashCommand, Command, CommandHistory, PythonCode, SessionId};
        use crate::validator::{KnowledgeGraphValidator, ValidatorConfig};
        use async_trait::async_trait;
        use terraphim_types::Thesaurus;

        /// Minimal stub executor that returns fixed success results.
        struct StubExecutor;

        #[async_trait]
        impl ExecutionEnvironment for StubExecutor {
            type Error = crate::error::RlmError;

            async fn execute_code(
                &self,
                code: &str,
                _ctx: &ExecutionContext,
            ) -> Result<ExecutionResult, Self::Error> {
                Ok(ExecutionResult::success(format!("stub_code:{code}")))
            }

            async fn execute_command(
                &self,
                cmd: &str,
                _ctx: &ExecutionContext,
            ) -> Result<ExecutionResult, Self::Error> {
                Ok(ExecutionResult::success(format!("stub_run:{cmd}")))
            }

            async fn validate(&self, _input: &str) -> Result<ValidationResult, Self::Error> {
                Ok(ValidationResult::valid(vec![]))
            }

            async fn create_snapshot(
                &self,
                session_id: &SessionId,
                name: &str,
            ) -> Result<SnapshotId, Self::Error> {
                Ok(SnapshotId::new(name, *session_id))
            }

            async fn restore_snapshot(&self, _id: &SnapshotId) -> Result<(), Self::Error> {
                Ok(())
            }

            async fn list_snapshots(
                &self,
                _session_id: &SessionId,
            ) -> Result<Vec<SnapshotId>, Self::Error> {
                Ok(vec![])
            }

            async fn delete_snapshot(&self, _id: &SnapshotId) -> Result<(), Self::Error> {
                Ok(())
            }

            async fn delete_session_snapshots(
                &self,
                _session_id: &SessionId,
            ) -> Result<(), Self::Error> {
                Ok(())
            }

            fn capabilities(&self) -> &[Capability] {
                &[Capability::PythonExecution, Capability::BashExecution]
            }

            fn backend_type(&self) -> BackendType {
                BackendType::Local
            }

            async fn health_check(&self) -> Result<bool, Self::Error> {
                Ok(true)
            }

            async fn cleanup(&self) -> Result<(), Self::Error> {
                Ok(())
            }
        }

        fn make_loop(validator: Arc<KnowledgeGraphValidator>) -> QueryLoop<StubExecutor> {
            let config = RlmConfig::minimal();
            let session_id = SessionId::new();
            let session_manager = Arc::new(SessionManager::new(config.clone()));
            let budget_tracker = Arc::new(BudgetTracker::new(&config));
            let llm_bridge = Arc::new(LlmBridge::new(
                LlmBridgeConfig::default(),
                session_manager.clone(),
            ));
            QueryLoop::new(
                session_id,
                session_manager,
                budget_tracker,
                llm_bridge,
                Arc::new(StubExecutor),
                QueryLoopConfig::default(),
                validator,
            )
        }

        fn strict_validator_empty_thesaurus() -> Arc<KnowledgeGraphValidator> {
            let thesaurus = Thesaurus::new("test".to_string());
            Arc::new(
                KnowledgeGraphValidator::new(ValidatorConfig::strict()).with_thesaurus(thesaurus),
            )
        }

        fn permissive_validator() -> Arc<KnowledgeGraphValidator> {
            Arc::new(KnowledgeGraphValidator::disabled())
        }

        #[tokio::test]
        async fn run_command_blocked_by_strict_validator() {
            let ql = make_loop(strict_validator_empty_thesaurus());
            let ctx = ExecutionContext::default();
            let mut history = CommandHistory::new();
            let cmd = Command::Run(BashCommand::new("rm -rf /"));
            let result = ql.execute_command(&cmd, &ctx, &mut history).await;
            assert!(
                matches!(result, Err(RlmError::KgValidationFailed { .. })),
                "Expected KgValidationFailed, got: {result:?}"
            );
            assert!(
                history.entries.is_empty(),
                "History must be empty on validation failure"
            );
        }

        #[tokio::test]
        async fn code_blocked_by_strict_validator() {
            let ql = make_loop(strict_validator_empty_thesaurus());
            let ctx = ExecutionContext::default();
            let mut history = CommandHistory::new();
            let cmd = Command::Code(PythonCode::new("import os; os.system('rm -rf /')"));
            let result = ql.execute_command(&cmd, &ctx, &mut history).await;
            assert!(
                matches!(result, Err(RlmError::KgValidationFailed { .. })),
                "Expected KgValidationFailed, got: {result:?}"
            );
            assert!(
                history.entries.is_empty(),
                "History must be empty on validation failure"
            );
        }

        #[tokio::test]
        async fn run_command_passes_with_permissive_validator() {
            let ql = make_loop(permissive_validator());
            let ctx = ExecutionContext::default();
            let mut history = CommandHistory::new();
            let cmd = Command::Run(BashCommand::new("ls -la"));
            let result = ql.execute_command(&cmd, &ctx, &mut history).await;
            assert!(result.is_ok(), "Expected Ok, got: {result:?}");
            assert_eq!(history.entries.len(), 1, "History must record the command");
        }

        #[tokio::test]
        async fn code_passes_with_permissive_validator() {
            let ql = make_loop(permissive_validator());
            let ctx = ExecutionContext::default();
            let mut history = CommandHistory::new();
            let cmd = Command::Code(PythonCode::new("print('hello')"));
            let result = ql.execute_command(&cmd, &ctx, &mut history).await;
            assert!(result.is_ok(), "Expected Ok, got: {result:?}");
            assert_eq!(history.entries.len(), 1, "History must record the command");
        }
    }
}
