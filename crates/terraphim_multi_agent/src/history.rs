//! Command and interaction history for agents

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::{AgentContext, AgentId, TokenUsageRecord};

/// A complete record of an agent command/interaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandRecord {
    /// Unique command ID
    pub command_id: Uuid,
    /// Agent that executed the command
    pub agent_id: AgentId,
    /// When the command was issued
    pub timestamp: DateTime<Utc>,
    /// Duration of execution in milliseconds
    pub duration_ms: u64,

    /// Input and context
    pub input: CommandInput,
    pub context_snapshot: HistoryContextSnapshot,

    /// Execution details
    pub execution: CommandExecution,

    /// Results and outputs
    pub output: CommandOutput,

    /// Resource usage
    pub token_usage: Option<TokenUsageRecord>,
    pub cost_usd: Option<f64>,

    /// Quality and learning
    pub quality_score: Option<f64>,
    pub lessons_learned: Vec<String>,
    pub memory_updates: Vec<String>,

    /// Error information if command failed
    pub error: Option<CommandError>,
}

impl CommandRecord {
    pub fn new(agent_id: AgentId, input: CommandInput) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            agent_id,
            timestamp: Utc::now(),
            duration_ms: 0,
            input,
            context_snapshot: HistoryContextSnapshot::empty(agent_id),
            execution: CommandExecution::default(),
            output: CommandOutput::default(),
            token_usage: None,
            cost_usd: None,
            quality_score: None,
            lessons_learned: Vec::new(),
            memory_updates: Vec::new(),
            error: None,
        }
    }

    /// Mark command as completed with results
    pub fn complete(mut self, output: CommandOutput, duration_ms: u64) -> Self {
        self.output = output;
        self.duration_ms = duration_ms;
        self
    }

    /// Add token usage information
    pub fn with_token_usage(mut self, token_usage: TokenUsageRecord, cost_usd: f64) -> Self {
        self.token_usage = Some(token_usage);
        self.cost_usd = Some(cost_usd);
        self
    }

    /// Add quality score
    pub fn with_quality_score(mut self, score: f64) -> Self {
        self.quality_score = Some(score.clamp(0.0, 1.0));
        self
    }

    /// Add context snapshot
    pub fn with_context_snapshot(mut self, snapshot: HistoryContextSnapshot) -> Self {
        self.context_snapshot = snapshot;
        self
    }

    /// Add execution details
    pub fn with_execution(mut self, execution: CommandExecution) -> Self {
        self.execution = execution;
        self
    }

    /// Add lessons learned
    pub fn with_lessons(mut self, lessons: Vec<String>) -> Self {
        self.lessons_learned = lessons;
        self
    }

    /// Add memory updates
    pub fn with_memory_updates(mut self, updates: Vec<String>) -> Self {
        self.memory_updates = updates;
        self
    }

    /// Mark command as failed
    pub fn with_error(mut self, error: CommandError) -> Self {
        self.error = Some(error);
        self
    }

    /// Check if command was successful
    pub fn is_successful(&self) -> bool {
        self.error.is_none()
    }

    /// Get effective quality score (0.0 if failed)
    pub fn effective_quality_score(&self) -> f64 {
        if self.is_successful() {
            self.quality_score.unwrap_or(0.5)
        } else {
            0.0
        }
    }
}

/// Input for a command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandInput {
    /// The raw input text/request
    pub text: String,
    /// Type of command
    pub command_type: CommandType,
    /// Parameters or arguments
    pub parameters: HashMap<String, serde_json::Value>,
    /// Source of the command (user, system, another agent)
    pub source: CommandSource,
    /// Priority level
    pub priority: CommandPriority,
    /// Timeout in milliseconds
    pub timeout_ms: Option<u64>,
}

impl CommandInput {
    pub fn new(text: String, command_type: CommandType) -> Self {
        Self {
            text,
            command_type,
            parameters: HashMap::new(),
            source: CommandSource::User,
            priority: CommandPriority::Normal,
            timeout_ms: None,
        }
    }

    pub fn with_parameters(mut self, parameters: HashMap<String, serde_json::Value>) -> Self {
        self.parameters = parameters;
        self
    }

    pub fn with_source(mut self, source: CommandSource) -> Self {
        self.source = source;
        self
    }

    pub fn with_priority(mut self, priority: CommandPriority) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }
}

/// Types of commands
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CommandType {
    /// Text generation/completion
    Generate,
    /// Question answering
    Answer,
    /// Task execution
    Execute,
    /// Information search
    Search,
    /// Analysis or reasoning
    Analyze,
    /// Content creation
    Create,
    /// Content editing/modification
    Edit,
    /// Review or evaluation
    Review,
    /// Planning or strategy
    Plan,
    /// System or administrative
    System,
    /// Custom command type
    Custom(String),
}

/// Source of a command
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CommandSource {
    /// Direct user input
    User,
    /// System-generated
    System,
    /// From another agent
    Agent(AgentId),
    /// Automated/scheduled
    Automated,
    /// API call
    Api,
}

/// Command priority levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum CommandPriority {
    Low,
    Normal,
    High,
    Critical,
    Emergency,
}

/// Execution details of a command
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CommandExecution {
    /// Workflow pattern used
    pub workflow_pattern: Option<String>,
    /// Execution steps taken
    pub steps: Vec<ExecutionStep>,
    /// Models/tools used
    pub tools_used: Vec<String>,
    /// Intermediate results
    pub intermediate_results: Vec<String>,
    /// Retries attempted
    pub retry_count: u32,
    /// Whether execution used cache
    pub used_cache: bool,
}

/// A single step in command execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStep {
    /// Step ID
    pub step_id: Uuid,
    /// Step name/description
    pub name: String,
    /// When step started
    pub started_at: DateTime<Utc>,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Step status
    pub status: StepStatus,
    /// Step input
    pub input: Option<String>,
    /// Step output
    pub output: Option<String>,
    /// Error if step failed
    pub error: Option<String>,
}

impl ExecutionStep {
    pub fn new(name: String) -> Self {
        Self {
            step_id: Uuid::new_v4(),
            name,
            started_at: Utc::now(),
            duration_ms: 0,
            status: StepStatus::Running,
            input: None,
            output: None,
            error: None,
        }
    }

    pub fn complete(mut self, output: String, duration_ms: u64) -> Self {
        self.output = Some(output);
        self.duration_ms = duration_ms;
        self.status = StepStatus::Completed;
        self
    }

    pub fn fail(mut self, error: String, duration_ms: u64) -> Self {
        self.error = Some(error);
        self.duration_ms = duration_ms;
        self.status = StepStatus::Failed;
        self
    }
}

/// Status of an execution step
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StepStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
}

/// Output from a command
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CommandOutput {
    /// Main output text
    pub text: String,
    /// Structured data output
    pub data: Option<serde_json::Value>,
    /// Output type
    pub output_type: OutputType,
    /// Confidence score (0.0 - 1.0)
    pub confidence: Option<f64>,
    /// Sources used
    pub sources: Vec<String>,
    /// Metadata about the output
    pub metadata: HashMap<String, serde_json::Value>,
}

impl CommandOutput {
    pub fn new(text: String) -> Self {
        Self {
            text,
            data: None,
            output_type: OutputType::Text,
            confidence: None,
            sources: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self.output_type = OutputType::Structured;
        self
    }

    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = Some(confidence.clamp(0.0, 1.0));
        self
    }

    pub fn with_sources(mut self, sources: Vec<String>) -> Self {
        self.sources = sources;
        self
    }
}

/// Types of command output
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum OutputType {
    #[default]
    Text,
    Structured,
    Binary,
    Stream,
}

/// Error information for failed commands
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandError {
    /// Error type
    pub error_type: ErrorType,
    /// Error message
    pub message: String,
    /// Error code if applicable
    pub code: Option<String>,
    /// Stack trace or additional details
    pub details: Option<String>,
    /// Whether error is recoverable
    pub recoverable: bool,
    /// Suggested actions
    pub suggestions: Vec<String>,
}

impl CommandError {
    pub fn new(error_type: ErrorType, message: String) -> Self {
        Self {
            error_type,
            message,
            code: None,
            details: None,
            recoverable: false,
            suggestions: Vec::new(),
        }
    }

    pub fn with_code(mut self, code: String) -> Self {
        self.code = Some(code);
        self
    }

    pub fn with_details(mut self, details: String) -> Self {
        self.details = Some(details);
        self
    }

    pub fn recoverable(mut self) -> Self {
        self.recoverable = true;
        self
    }

    pub fn with_suggestions(mut self, suggestions: Vec<String>) -> Self {
        self.suggestions = suggestions;
        self
    }
}

/// Types of errors
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ErrorType {
    /// Input validation error
    Validation,
    /// Authentication/authorization error
    Auth,
    /// Rate limiting error
    RateLimit,
    /// Resource not found
    NotFound,
    /// Service unavailable
    ServiceUnavailable,
    /// Timeout error
    Timeout,
    /// Network error
    Network,
    /// LLM provider error
    LlmProvider,
    /// Configuration error
    Configuration,
    /// Internal system error
    Internal,
    /// Unknown error
    Unknown,
}

/// Simplified context snapshot for history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryContextSnapshot {
    /// Agent ID
    pub agent_id: AgentId,
    /// Snapshot timestamp
    pub timestamp: DateTime<Utc>,
    /// Number of context items
    pub item_count: usize,
    /// Total tokens in context
    pub token_count: u64,
    /// Key context items (summarized)
    pub key_items: Vec<ContextSummary>,
}

impl HistoryContextSnapshot {
    pub fn empty(agent_id: AgentId) -> Self {
        Self {
            agent_id,
            timestamp: Utc::now(),
            item_count: 0,
            token_count: 0,
            key_items: Vec::new(),
        }
    }

    pub fn from_context(context: &AgentContext) -> Self {
        let key_items = context
            .get_relevant_items(1000) // Top 1000 tokens worth
            .into_iter()
            .map(|item| ContextSummary {
                item_type: format!("{:?}", item.item_type),
                content_preview: item.content.chars().take(100).collect(),
                token_count: item.token_count,
                relevance_score: item.relevance_score,
            })
            .collect();

        Self {
            agent_id: context.agent_id,
            timestamp: Utc::now(),
            item_count: context.items.len(),
            token_count: context.current_tokens,
            key_items,
        }
    }
}

/// Summary of a context item for history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSummary {
    /// Type of context item
    pub item_type: String,
    /// Preview of content (first 100 chars)
    pub content_preview: String,
    /// Token count
    pub token_count: u64,
    /// Relevance score
    pub relevance_score: f64,
}

/// History manager for tracking agent commands
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandHistory {
    /// Agent this history belongs to
    pub agent_id: AgentId,
    /// All command records
    pub records: Vec<CommandRecord>,
    /// Maximum records to keep
    pub max_records: usize,
    /// When history was last updated
    pub last_updated: DateTime<Utc>,
}

impl CommandHistory {
    pub fn new(agent_id: AgentId, max_records: usize) -> Self {
        Self {
            agent_id,
            records: Vec::new(),
            max_records,
            last_updated: Utc::now(),
        }
    }

    /// Add a command record
    pub fn add_record(&mut self, record: CommandRecord) {
        self.records.push(record);
        self.last_updated = Utc::now();

        // Enforce max records limit
        if self.records.len() > self.max_records {
            self.records.remove(0);
        }
    }

    /// Get records in time range
    pub fn get_records_in_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Vec<&CommandRecord> {
        self.records
            .iter()
            .filter(|record| record.timestamp >= start && record.timestamp <= end)
            .collect()
    }

    /// Get records by command type
    pub fn get_records_by_type(&self, command_type: CommandType) -> Vec<&CommandRecord> {
        self.records
            .iter()
            .filter(|record| record.input.command_type == command_type)
            .collect()
    }

    /// Get successful records only
    pub fn get_successful_records(&self) -> Vec<&CommandRecord> {
        self.records
            .iter()
            .filter(|record| record.is_successful())
            .collect()
    }

    /// Get failed records only
    pub fn get_failed_records(&self) -> Vec<&CommandRecord> {
        self.records
            .iter()
            .filter(|record| !record.is_successful())
            .collect()
    }

    /// Calculate success rate
    pub fn success_rate(&self) -> f64 {
        if self.records.is_empty() {
            return 0.0;
        }

        let successful_count = self.get_successful_records().len();
        successful_count as f64 / self.records.len() as f64
    }

    /// Calculate average quality score
    pub fn average_quality_score(&self) -> f64 {
        let quality_scores: Vec<f64> = self
            .records
            .iter()
            .filter_map(|record| record.quality_score)
            .collect();

        if quality_scores.is_empty() {
            return 0.0;
        }

        quality_scores.iter().sum::<f64>() / quality_scores.len() as f64
    }

    /// Get most recent records
    pub fn get_recent_records(&self, limit: usize) -> Vec<&CommandRecord> {
        self.records.iter().rev().take(limit).collect()
    }

    /// Get command statistics
    pub fn get_statistics(&self) -> CommandStatistics {
        let total_commands = self.records.len();
        let successful_commands = self.get_successful_records().len();
        let failed_commands = self.get_failed_records().len();

        let total_duration_ms: u64 = self.records.iter().map(|r| r.duration_ms).sum();
        let total_cost: f64 = self.records.iter().filter_map(|r| r.cost_usd).sum();
        let total_tokens: u64 = self
            .records
            .iter()
            .filter_map(|r| r.token_usage.as_ref().map(|t| t.total_tokens))
            .sum();

        CommandStatistics {
            total_commands: total_commands as u64,
            successful_commands: successful_commands as u64,
            failed_commands: failed_commands as u64,
            success_rate: self.success_rate(),
            average_quality_score: self.average_quality_score(),
            total_duration_ms,
            average_duration_ms: if total_commands > 0 {
                total_duration_ms / total_commands as u64
            } else {
                0
            },
            total_cost_usd: total_cost,
            average_cost_per_command: if total_commands > 0 {
                total_cost / total_commands as f64
            } else {
                0.0
            },
            total_tokens,
            average_tokens_per_command: if total_commands > 0 {
                total_tokens as f64 / total_commands as f64
            } else {
                0.0
            },
        }
    }
}

/// Statistics about command history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandStatistics {
    pub total_commands: u64,
    pub successful_commands: u64,
    pub failed_commands: u64,
    pub success_rate: f64,
    pub average_quality_score: f64,
    pub total_duration_ms: u64,
    pub average_duration_ms: u64,
    pub total_cost_usd: f64,
    pub average_cost_per_command: f64,
    pub total_tokens: u64,
    pub average_tokens_per_command: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_record_creation() {
        let agent_id = AgentId::new_v4();
        let input = CommandInput::new("Test command".to_string(), CommandType::Generate);
        let record = CommandRecord::new(agent_id, input);

        assert_eq!(record.agent_id, agent_id);
        assert_eq!(record.input.text, "Test command");
        assert_eq!(record.input.command_type, CommandType::Generate);
        assert!(record.is_successful());
    }

    #[test]
    fn test_command_history() {
        let agent_id = AgentId::new_v4();
        let mut history = CommandHistory::new(agent_id, 100);

        let input = CommandInput::new("Test".to_string(), CommandType::Answer);
        let record = CommandRecord::new(agent_id, input);

        history.add_record(record);

        assert_eq!(history.records.len(), 1);
        assert_eq!(history.success_rate(), 1.0);
    }

    #[test]
    fn test_execution_step() {
        let step =
            ExecutionStep::new("Test step".to_string()).complete("Success".to_string(), 1000);

        assert_eq!(step.name, "Test step");
        assert_eq!(step.status, StepStatus::Completed);
        assert_eq!(step.output, Some("Success".to_string()));
        assert_eq!(step.duration_ms, 1000);
    }

    #[test]
    fn test_command_statistics() {
        let agent_id = AgentId::new_v4();
        let mut history = CommandHistory::new(agent_id, 100);

        // Add successful command
        let input1 = CommandInput::new("Success".to_string(), CommandType::Generate);
        let record1 = CommandRecord::new(agent_id, input1)
            .complete(CommandOutput::new("Done".to_string()), 1000)
            .with_quality_score(0.8);
        history.add_record(record1);

        // Add failed command
        let input2 = CommandInput::new("Fail".to_string(), CommandType::Generate);
        let record2 = CommandRecord::new(agent_id, input2)
            .with_error(CommandError::new(ErrorType::Internal, "Failed".to_string()));
        history.add_record(record2);

        let stats = history.get_statistics();
        assert_eq!(stats.total_commands, 2);
        assert_eq!(stats.successful_commands, 1);
        assert_eq!(stats.failed_commands, 1);
        assert_eq!(stats.success_rate, 0.5);
    }
}
