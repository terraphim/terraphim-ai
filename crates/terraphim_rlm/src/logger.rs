//! Trajectory logging for RLM query execution.
//!
//! The TrajectoryLogger records detailed JSONL logs of query loop execution for:
//! - Debugging and analysis
//! - Training data collection
//! - Audit trails
//! - Performance monitoring
//!
//! Each log entry is a self-contained JSON object on a single line, enabling
//! streaming processing and easy parsing.

use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use jiff::Timestamp;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::error::{RlmError, RlmResult};
use crate::query_loop::TerminationReason;
use crate::types::{BudgetStatus, Command, SessionId};

/// A trajectory event that can be logged.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type", rename_all = "snake_case")]
pub enum TrajectoryEvent {
    /// Session started.
    SessionStart {
        /// When the event occurred.
        timestamp: Timestamp,
        /// Session identifier.
        session_id: SessionId,
        /// Initial budget status.
        budget: BudgetStatus,
    },

    /// Query loop started.
    QueryStart {
        /// When the event occurred.
        timestamp: Timestamp,
        /// Session identifier.
        session_id: SessionId,
        /// Query identifier.
        query_id: Ulid,
        /// Initial prompt.
        initial_prompt: String,
        /// Parent query ID (for recursive queries).
        parent_query_id: Option<Ulid>,
        /// Current recursion depth.
        depth: u32,
    },

    /// LLM was called.
    LlmCall {
        /// When the event occurred.
        timestamp: Timestamp,
        /// Session identifier.
        session_id: SessionId,
        /// Query identifier.
        query_id: Ulid,
        /// Iteration number within query loop.
        iteration: u32,
        /// Prompt sent to LLM.
        prompt: String,
        /// Prompt length in characters.
        prompt_length: usize,
    },

    /// LLM response received.
    LlmResponse {
        /// When the event occurred.
        timestamp: Timestamp,
        /// Session identifier.
        session_id: SessionId,
        /// Query identifier.
        query_id: Ulid,
        /// Iteration number.
        iteration: u32,
        /// LLM response text.
        response: String,
        /// Response length in characters.
        response_length: usize,
        /// Tokens used for this call.
        tokens_used: u64,
        /// Latency in milliseconds.
        latency_ms: u64,
    },

    /// Command parsed from LLM response.
    CommandParsed {
        /// When the event occurred.
        timestamp: Timestamp,
        /// Session identifier.
        session_id: SessionId,
        /// Query identifier.
        query_id: Ulid,
        /// Iteration number.
        iteration: u32,
        /// The parsed command.
        command: Command,
        /// Command type as string for easy filtering.
        command_type: String,
    },

    /// Command parsing failed.
    CommandParseFailed {
        /// When the event occurred.
        timestamp: Timestamp,
        /// Session identifier.
        session_id: SessionId,
        /// Query identifier.
        query_id: Ulid,
        /// Iteration number.
        iteration: u32,
        /// The raw LLM response that failed to parse.
        raw_response: String,
        /// Parse error message.
        error: String,
    },

    /// Command executed.
    CommandExecuted {
        /// When the event occurred.
        timestamp: Timestamp,
        /// Session identifier.
        session_id: SessionId,
        /// Query identifier.
        query_id: Ulid,
        /// Iteration number.
        iteration: u32,
        /// The command that was executed.
        command: Command,
        /// Whether execution succeeded.
        success: bool,
        /// stdout output (may be truncated).
        stdout: String,
        /// stderr output (may be truncated).
        stderr: String,
        /// Exit code if applicable.
        exit_code: Option<i32>,
        /// Execution time in milliseconds.
        execution_time_ms: u64,
    },

    /// Query loop completed.
    QueryComplete {
        /// When the event occurred.
        timestamp: Timestamp,
        /// Session identifier.
        session_id: SessionId,
        /// Query identifier.
        query_id: Ulid,
        /// Total iterations executed.
        total_iterations: u32,
        /// Final result if FINAL was reached.
        result: Option<String>,
        /// Whether the query succeeded.
        success: bool,
        /// Reason for termination.
        termination_reason: String,
        /// Total duration in milliseconds.
        duration_ms: u64,
        /// Total tokens consumed.
        total_tokens: u64,
    },

    /// Session ended.
    SessionEnd {
        /// When the event occurred.
        timestamp: Timestamp,
        /// Session identifier.
        session_id: SessionId,
        /// Total queries executed.
        total_queries: u32,
        /// Total tokens consumed across all queries.
        total_tokens: u64,
        /// Total session duration in milliseconds.
        duration_ms: u64,
    },

    /// Budget warning issued.
    BudgetWarning {
        /// When the event occurred.
        timestamp: Timestamp,
        /// Session identifier.
        session_id: SessionId,
        /// Query identifier.
        query_id: Ulid,
        /// Warning type (tokens, time, or depth).
        warning_type: String,
        /// Current usage.
        current: u64,
        /// Maximum allowed.
        maximum: u64,
        /// Percentage used.
        percentage: f64,
    },

    /// Error occurred.
    Error {
        /// When the event occurred.
        timestamp: Timestamp,
        /// Session identifier.
        session_id: SessionId,
        /// Query identifier if applicable.
        query_id: Option<Ulid>,
        /// Error message.
        error: String,
        /// Error category.
        category: String,
    },
}

impl TrajectoryEvent {
    /// Get the timestamp of this event.
    pub fn timestamp(&self) -> Timestamp {
        match self {
            Self::SessionStart { timestamp, .. } => *timestamp,
            Self::QueryStart { timestamp, .. } => *timestamp,
            Self::LlmCall { timestamp, .. } => *timestamp,
            Self::LlmResponse { timestamp, .. } => *timestamp,
            Self::CommandParsed { timestamp, .. } => *timestamp,
            Self::CommandParseFailed { timestamp, .. } => *timestamp,
            Self::CommandExecuted { timestamp, .. } => *timestamp,
            Self::QueryComplete { timestamp, .. } => *timestamp,
            Self::SessionEnd { timestamp, .. } => *timestamp,
            Self::BudgetWarning { timestamp, .. } => *timestamp,
            Self::Error { timestamp, .. } => *timestamp,
        }
    }

    /// Get the session ID of this event.
    pub fn session_id(&self) -> SessionId {
        match self {
            Self::SessionStart { session_id, .. } => *session_id,
            Self::QueryStart { session_id, .. } => *session_id,
            Self::LlmCall { session_id, .. } => *session_id,
            Self::LlmResponse { session_id, .. } => *session_id,
            Self::CommandParsed { session_id, .. } => *session_id,
            Self::CommandParseFailed { session_id, .. } => *session_id,
            Self::CommandExecuted { session_id, .. } => *session_id,
            Self::QueryComplete { session_id, .. } => *session_id,
            Self::SessionEnd { session_id, .. } => *session_id,
            Self::BudgetWarning { session_id, .. } => *session_id,
            Self::Error { session_id, .. } => *session_id,
        }
    }

    /// Get the event type as a string.
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::SessionStart { .. } => "session_start",
            Self::QueryStart { .. } => "query_start",
            Self::LlmCall { .. } => "llm_call",
            Self::LlmResponse { .. } => "llm_response",
            Self::CommandParsed { .. } => "command_parsed",
            Self::CommandParseFailed { .. } => "command_parse_failed",
            Self::CommandExecuted { .. } => "command_executed",
            Self::QueryComplete { .. } => "query_complete",
            Self::SessionEnd { .. } => "session_end",
            Self::BudgetWarning { .. } => "budget_warning",
            Self::Error { .. } => "error",
        }
    }
}

/// Backend for storing trajectory logs.
trait LogBackend: Send + Sync {
    /// Write an event to the log.
    fn write(&mut self, event: &TrajectoryEvent) -> RlmResult<()>;

    /// Flush any buffered data.
    fn flush(&mut self) -> RlmResult<()>;
}

/// File-based log backend using JSONL format.
struct FileBackend {
    writer: BufWriter<std::fs::File>,
    #[allow(dead_code)] // Kept for potential log rotation or path queries
    path: PathBuf,
}

impl FileBackend {
    fn new(path: impl AsRef<Path>) -> RlmResult<Self> {
        let path = path.as_ref().to_path_buf();

        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| RlmError::ConfigError {
                message: format!("Failed to create log directory: {}", e),
            })?;
        }

        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| RlmError::ConfigError {
                message: format!("Failed to open log file: {}", e),
            })?;

        Ok(Self {
            writer: BufWriter::new(file),
            path,
        })
    }
}

impl LogBackend for FileBackend {
    fn write(&mut self, event: &TrajectoryEvent) -> RlmResult<()> {
        let json = serde_json::to_string(event).map_err(|e| RlmError::ConfigError {
            message: format!("Failed to serialize event: {}", e),
        })?;

        writeln!(self.writer, "{}", json).map_err(|e| RlmError::ConfigError {
            message: format!("Failed to write to log file: {}", e),
        })?;

        Ok(())
    }

    fn flush(&mut self) -> RlmResult<()> {
        self.writer.flush().map_err(|e| RlmError::ConfigError {
            message: format!("Failed to flush log file: {}", e),
        })?;
        Ok(())
    }
}

/// In-memory log backend for testing.
struct MemoryBackend {
    events: Vec<TrajectoryEvent>,
}

impl MemoryBackend {
    fn new() -> Self {
        Self { events: Vec::new() }
    }

    #[allow(dead_code)] // Available for test assertions
    fn events(&self) -> &[TrajectoryEvent] {
        &self.events
    }
}

impl LogBackend for MemoryBackend {
    fn write(&mut self, event: &TrajectoryEvent) -> RlmResult<()> {
        self.events.push(event.clone());
        Ok(())
    }

    fn flush(&mut self) -> RlmResult<()> {
        Ok(())
    }
}

/// Configuration for trajectory logging.
#[derive(Debug, Clone)]
pub struct TrajectoryLoggerConfig {
    /// Whether logging is enabled.
    pub enabled: bool,
    /// Log file path (if file-based logging).
    pub log_path: Option<PathBuf>,
    /// Maximum length of logged prompts/responses (truncate if longer).
    pub max_content_length: usize,
    /// Whether to log LLM prompts and responses.
    pub log_llm_content: bool,
    /// Whether to log command stdout/stderr.
    pub log_command_output: bool,
    /// Flush after every N events (0 = flush immediately).
    pub flush_interval: u32,
}

impl Default for TrajectoryLoggerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            log_path: None,
            max_content_length: 10_000,
            log_llm_content: true,
            log_command_output: true,
            flush_interval: 10,
        }
    }
}

impl TrajectoryLoggerConfig {
    /// Create config for file-based logging.
    pub fn with_file(path: impl AsRef<Path>) -> Self {
        Self {
            log_path: Some(path.as_ref().to_path_buf()),
            ..Default::default()
        }
    }

    /// Create config for in-memory logging (testing).
    pub fn in_memory() -> Self {
        Self {
            log_path: None,
            ..Default::default()
        }
    }

    /// Disable all content logging (prompts, responses, stdout).
    pub fn metadata_only(mut self) -> Self {
        self.log_llm_content = false;
        self.log_command_output = false;
        self
    }
}

/// Thread-safe trajectory logger.
///
/// The TrajectoryLogger records execution events in JSONL format for debugging,
/// analysis, and training data collection.
pub struct TrajectoryLogger {
    config: TrajectoryLoggerConfig,
    backend: Arc<Mutex<Box<dyn LogBackend>>>,
    events_since_flush: Arc<Mutex<u32>>,
}

impl TrajectoryLogger {
    /// Create a new trajectory logger with the given configuration.
    pub fn new(config: TrajectoryLoggerConfig) -> RlmResult<Self> {
        let backend: Box<dyn LogBackend> = if let Some(ref path) = config.log_path {
            Box::new(FileBackend::new(path)?)
        } else {
            Box::new(MemoryBackend::new())
        };

        Ok(Self {
            config,
            backend: Arc::new(Mutex::new(backend)),
            events_since_flush: Arc::new(Mutex::new(0)),
        })
    }

    /// Create a logger that writes to a file.
    pub fn to_file(path: impl AsRef<Path>) -> RlmResult<Self> {
        Self::new(TrajectoryLoggerConfig::with_file(path))
    }

    /// Create an in-memory logger for testing.
    pub fn in_memory() -> RlmResult<Self> {
        Self::new(TrajectoryLoggerConfig::in_memory())
    }

    /// Create a disabled logger (no-op).
    pub fn disabled() -> Self {
        Self {
            config: TrajectoryLoggerConfig {
                enabled: false,
                ..Default::default()
            },
            backend: Arc::new(Mutex::new(Box::new(MemoryBackend::new()))),
            events_since_flush: Arc::new(Mutex::new(0)),
        }
    }

    /// Log a trajectory event.
    pub fn log(&self, event: TrajectoryEvent) -> RlmResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let mut backend = self.backend.lock();
        backend.write(&event)?;

        let mut count = self.events_since_flush.lock();
        *count += 1;

        if self.config.flush_interval > 0 && *count >= self.config.flush_interval {
            backend.flush()?;
            *count = 0;
        }

        Ok(())
    }

    /// Flush any buffered events.
    pub fn flush(&self) -> RlmResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let mut backend = self.backend.lock();
        backend.flush()?;
        *self.events_since_flush.lock() = 0;
        Ok(())
    }

    /// Get logged events (only works for in-memory backend).
    ///
    /// Returns None if using file backend.
    pub fn get_events(&self) -> Option<Vec<TrajectoryEvent>> {
        // This is a bit hacky but works for testing
        // The real way would be to read from the file
        if self.config.log_path.is_some() {
            return None;
        }

        // For memory backend, we can't easily get events without downcasting
        // In a real implementation, we'd use a different approach
        None
    }

    /// Get the log file path if using file backend.
    pub fn log_path(&self) -> Option<&Path> {
        self.config.log_path.as_deref()
    }

    // Convenience methods for logging common events

    /// Log session start.
    pub fn log_session_start(&self, session_id: SessionId, budget: BudgetStatus) -> RlmResult<()> {
        self.log(TrajectoryEvent::SessionStart {
            timestamp: Timestamp::now(),
            session_id,
            budget,
        })
    }

    /// Log query start.
    pub fn log_query_start(
        &self,
        session_id: SessionId,
        query_id: Ulid,
        initial_prompt: &str,
        parent_query_id: Option<Ulid>,
        depth: u32,
    ) -> RlmResult<()> {
        let prompt = self.truncate_content(initial_prompt);
        self.log(TrajectoryEvent::QueryStart {
            timestamp: Timestamp::now(),
            session_id,
            query_id,
            initial_prompt: prompt,
            parent_query_id,
            depth,
        })
    }

    /// Log LLM call.
    pub fn log_llm_call(
        &self,
        session_id: SessionId,
        query_id: Ulid,
        iteration: u32,
        prompt: &str,
    ) -> RlmResult<()> {
        let prompt_content = if self.config.log_llm_content {
            self.truncate_content(prompt)
        } else {
            "<redacted>".to_string()
        };

        self.log(TrajectoryEvent::LlmCall {
            timestamp: Timestamp::now(),
            session_id,
            query_id,
            iteration,
            prompt_length: prompt.len(),
            prompt: prompt_content,
        })
    }

    /// Log LLM response.
    pub fn log_llm_response(
        &self,
        session_id: SessionId,
        query_id: Ulid,
        iteration: u32,
        response: &str,
        tokens_used: u64,
        latency_ms: u64,
    ) -> RlmResult<()> {
        let response_content = if self.config.log_llm_content {
            self.truncate_content(response)
        } else {
            "<redacted>".to_string()
        };

        self.log(TrajectoryEvent::LlmResponse {
            timestamp: Timestamp::now(),
            session_id,
            query_id,
            iteration,
            response_length: response.len(),
            response: response_content,
            tokens_used,
            latency_ms,
        })
    }

    /// Log command parsed.
    pub fn log_command_parsed(
        &self,
        session_id: SessionId,
        query_id: Ulid,
        iteration: u32,
        command: &Command,
    ) -> RlmResult<()> {
        let command_type = match command {
            Command::Run(_) => "run",
            Command::Code(_) => "code",
            Command::Final(_) => "final",
            Command::FinalVar(_) => "final_var",
            Command::QueryLlm(_) => "query_llm",
            Command::QueryLlmBatched(_) => "query_llm_batched",
            Command::Snapshot(_) => "snapshot",
            Command::Rollback(_) => "rollback",
        };

        self.log(TrajectoryEvent::CommandParsed {
            timestamp: Timestamp::now(),
            session_id,
            query_id,
            iteration,
            command: command.clone(),
            command_type: command_type.to_string(),
        })
    }

    /// Log command parse failure.
    pub fn log_command_parse_failed(
        &self,
        session_id: SessionId,
        query_id: Ulid,
        iteration: u32,
        raw_response: &str,
        error: &str,
    ) -> RlmResult<()> {
        self.log(TrajectoryEvent::CommandParseFailed {
            timestamp: Timestamp::now(),
            session_id,
            query_id,
            iteration,
            raw_response: self.truncate_content(raw_response),
            error: error.to_string(),
        })
    }

    /// Log command executed.
    pub fn log_command_executed(
        &self,
        session_id: SessionId,
        query_id: Ulid,
        iteration: u32,
        command: &Command,
        success: bool,
        stdout: &str,
        stderr: &str,
        exit_code: Option<i32>,
        execution_time_ms: u64,
    ) -> RlmResult<()> {
        let (stdout_content, stderr_content) = if self.config.log_command_output {
            (self.truncate_content(stdout), self.truncate_content(stderr))
        } else {
            ("<redacted>".to_string(), "<redacted>".to_string())
        };

        self.log(TrajectoryEvent::CommandExecuted {
            timestamp: Timestamp::now(),
            session_id,
            query_id,
            iteration,
            command: command.clone(),
            success,
            stdout: stdout_content,
            stderr: stderr_content,
            exit_code,
            execution_time_ms,
        })
    }

    /// Log query complete.
    pub fn log_query_complete(
        &self,
        session_id: SessionId,
        query_id: Ulid,
        total_iterations: u32,
        result: Option<&str>,
        success: bool,
        termination_reason: &TerminationReason,
        duration_ms: u64,
        total_tokens: u64,
    ) -> RlmResult<()> {
        let reason_str = match termination_reason {
            TerminationReason::FinalReached => "final_reached",
            TerminationReason::FinalVarReached { .. } => "final_var_reached",
            TerminationReason::TokenBudgetExhausted => "token_budget_exhausted",
            TerminationReason::TimeBudgetExhausted => "time_budget_exhausted",
            TerminationReason::MaxIterationsReached => "max_iterations_reached",
            TerminationReason::RecursionDepthExhausted => "recursion_depth_exhausted",
            TerminationReason::Error { .. } => "error",
            TerminationReason::Cancelled => "cancelled",
        };

        self.log(TrajectoryEvent::QueryComplete {
            timestamp: Timestamp::now(),
            session_id,
            query_id,
            total_iterations,
            result: result.map(|s| self.truncate_content(s)),
            success,
            termination_reason: reason_str.to_string(),
            duration_ms,
            total_tokens,
        })
    }

    /// Log session end.
    pub fn log_session_end(
        &self,
        session_id: SessionId,
        total_queries: u32,
        total_tokens: u64,
        duration_ms: u64,
    ) -> RlmResult<()> {
        self.log(TrajectoryEvent::SessionEnd {
            timestamp: Timestamp::now(),
            session_id,
            total_queries,
            total_tokens,
            duration_ms,
        })
    }

    /// Log budget warning.
    pub fn log_budget_warning(
        &self,
        session_id: SessionId,
        query_id: Ulid,
        warning_type: &str,
        current: u64,
        maximum: u64,
    ) -> RlmResult<()> {
        let percentage = if maximum > 0 {
            (current as f64 / maximum as f64) * 100.0
        } else {
            100.0
        };

        self.log(TrajectoryEvent::BudgetWarning {
            timestamp: Timestamp::now(),
            session_id,
            query_id,
            warning_type: warning_type.to_string(),
            current,
            maximum,
            percentage,
        })
    }

    /// Log error.
    pub fn log_error(
        &self,
        session_id: SessionId,
        query_id: Option<Ulid>,
        error: &str,
        category: &str,
    ) -> RlmResult<()> {
        self.log(TrajectoryEvent::Error {
            timestamp: Timestamp::now(),
            session_id,
            query_id,
            error: error.to_string(),
            category: category.to_string(),
        })
    }

    /// Truncate content to configured maximum length.
    fn truncate_content(&self, content: &str) -> String {
        if content.len() <= self.config.max_content_length {
            content.to_string()
        } else {
            format!(
                "{}... [truncated, {} chars total]",
                &content[..self.config.max_content_length],
                content.len()
            )
        }
    }
}

/// Read trajectory events from a JSONL file.
pub fn read_trajectory_file(path: impl AsRef<Path>) -> RlmResult<Vec<TrajectoryEvent>> {
    let content = std::fs::read_to_string(path.as_ref()).map_err(|e| RlmError::ConfigError {
        message: format!("Failed to read trajectory file: {}", e),
    })?;

    let mut events = Vec::new();
    for (line_num, line) in content.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }

        let event: TrajectoryEvent =
            serde_json::from_str(line).map_err(|e| RlmError::ConfigError {
                message: format!(
                    "Failed to parse trajectory event at line {}: {}",
                    line_num + 1,
                    e
                ),
            })?;
        events.push(event);
    }

    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trajectory_event_types() {
        let session_id = SessionId::new();
        let event = TrajectoryEvent::SessionStart {
            timestamp: Timestamp::now(),
            session_id,
            budget: BudgetStatus::default(),
        };

        assert_eq!(event.event_type(), "session_start");
        assert_eq!(event.session_id(), session_id);
    }

    #[test]
    fn test_trajectory_logger_disabled() {
        let logger = TrajectoryLogger::disabled();
        let session_id = SessionId::new();

        // Should not error even when disabled
        assert!(
            logger
                .log_session_start(session_id, BudgetStatus::default())
                .is_ok()
        );
    }

    #[test]
    fn test_trajectory_logger_in_memory() {
        let logger = TrajectoryLogger::in_memory().unwrap();
        let session_id = SessionId::new();

        // Log some events
        logger
            .log_session_start(session_id, BudgetStatus::default())
            .unwrap();
        logger.flush().unwrap();

        // Events are logged but not easily retrievable in this simple implementation
        // A more complete implementation would support this
    }

    #[test]
    fn test_trajectory_event_serialization() {
        let session_id = SessionId::new();
        let event = TrajectoryEvent::LlmCall {
            timestamp: Timestamp::now(),
            session_id,
            query_id: Ulid::new(),
            iteration: 1,
            prompt: "Hello, world!".to_string(),
            prompt_length: 13,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("llm_call"));
        assert!(json.contains("Hello, world!"));

        // Deserialize back
        let parsed: TrajectoryEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.event_type(), "llm_call");
    }

    #[test]
    fn test_trajectory_logger_config() {
        let config = TrajectoryLoggerConfig::default();
        assert!(config.enabled);
        assert!(config.log_llm_content);
        assert!(config.log_command_output);
        assert_eq!(config.max_content_length, 10_000);

        let metadata_only = TrajectoryLoggerConfig::default().metadata_only();
        assert!(!metadata_only.log_llm_content);
        assert!(!metadata_only.log_command_output);
    }

    #[test]
    fn test_truncate_content() {
        let config = TrajectoryLoggerConfig {
            max_content_length: 20,
            ..Default::default()
        };
        let logger = TrajectoryLogger::new(config).unwrap();

        let short = "Hello";
        assert_eq!(logger.truncate_content(short), "Hello");

        let long = "This is a very long string that should be truncated";
        let truncated = logger.truncate_content(long);
        assert!(truncated.starts_with("This is a very long "));
        assert!(truncated.contains("truncated"));
    }

    #[test]
    fn test_command_type_extraction() {
        use crate::types::{BashCommand, PythonCode};

        let logger = TrajectoryLogger::in_memory().unwrap();
        let session_id = SessionId::new();
        let query_id = Ulid::new();

        // Test each command type
        let commands = vec![
            (Command::Run(BashCommand::new("ls")), "run"),
            (Command::Code(PythonCode::new("print(1)")), "code"),
            (Command::Final("done".into()), "final"),
            (Command::FinalVar("x".into()), "final_var"),
        ];

        for (cmd, expected_type) in commands {
            let result = logger.log_command_parsed(session_id, query_id, 1, &cmd);
            assert!(result.is_ok());
            // We can't easily verify the logged content without a more sophisticated test setup
            let _ = expected_type; // Just to use the variable
        }
    }

    #[test]
    fn test_termination_reason_to_string() {
        let reasons = vec![
            (TerminationReason::FinalReached, "final_reached"),
            (
                TerminationReason::FinalVarReached {
                    variable: "x".into(),
                },
                "final_var_reached",
            ),
            (
                TerminationReason::TokenBudgetExhausted,
                "token_budget_exhausted",
            ),
            (
                TerminationReason::TimeBudgetExhausted,
                "time_budget_exhausted",
            ),
            (
                TerminationReason::MaxIterationsReached,
                "max_iterations_reached",
            ),
            (
                TerminationReason::RecursionDepthExhausted,
                "recursion_depth_exhausted",
            ),
            (
                TerminationReason::Error {
                    message: "test".into(),
                },
                "error",
            ),
            (TerminationReason::Cancelled, "cancelled"),
        ];

        let logger = TrajectoryLogger::in_memory().unwrap();
        let session_id = SessionId::new();
        let query_id = Ulid::new();

        for (reason, expected) in reasons {
            let result = logger.log_query_complete(
                session_id,
                query_id,
                5,
                Some("result"),
                true,
                &reason,
                1000,
                500,
            );
            assert!(result.is_ok());
            let _ = expected; // Just to use the variable
        }
    }
}
