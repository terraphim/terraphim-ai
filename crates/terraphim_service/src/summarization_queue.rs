use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

use terraphim_config::{Config, Role};
use terraphim_types::Document;

/// Unique identifier for summarization tasks
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(pub Uuid);

impl Default for TaskId {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl std::fmt::Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Priority levels for summarization tasks
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
pub enum Priority {
    /// Low priority - batch processing
    Low = 0,
    /// Normal priority - standard requests
    #[default]
    Normal = 1,
    /// High priority - user-initiated requests
    High = 2,
    /// Critical priority - immediate processing required
    Critical = 3,
}

/// Status of a summarization task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    /// Task is queued and waiting to be processed
    Pending {
        queued_at: DateTime<Utc>,
        position_in_queue: Option<usize>,
    },
    /// Task is currently being processed
    Processing {
        started_at: DateTime<Utc>,
        progress: Option<f32>,
    },
    /// Task completed successfully
    Completed {
        summary: String,
        completed_at: DateTime<Utc>,
        processing_duration_seconds: u64,
    },
    /// Task failed with error
    Failed {
        error: String,
        failed_at: DateTime<Utc>,
        retry_count: u32,
        next_retry_at: Option<DateTime<Utc>>,
    },
    /// Task was cancelled
    Cancelled {
        cancelled_at: DateTime<Utc>,
        reason: String,
    },
}

impl TaskStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            TaskStatus::Completed { .. } | TaskStatus::Failed { .. } | TaskStatus::Cancelled { .. }
        )
    }

    pub fn is_processing(&self) -> bool {
        matches!(self, TaskStatus::Processing { .. })
    }

    pub fn is_pending(&self) -> bool {
        matches!(self, TaskStatus::Pending { .. })
    }
}

/// A summarization task to be processed
#[derive(Debug, Clone)]
pub struct SummarizationTask {
    /// Unique task identifier
    pub id: TaskId,
    /// Document to summarize
    pub document: Document,
    /// Role configuration for summarization
    pub role: Role,
    /// Global configuration for fallbacks
    pub config: Option<Config>,
    /// Task priority
    pub priority: Priority,
    /// Number of retry attempts
    pub retry_count: u32,
    /// Maximum number of retries allowed
    pub max_retries: u32,
    /// Task creation timestamp
    pub created_at: DateTime<Utc>,
    /// Optional maximum summary length override
    pub max_summary_length: Option<usize>,
    /// Whether to force regeneration even if summary exists
    pub force_regenerate: bool,
    /// Optional callback URL for completion notification
    pub callback_url: Option<String>,
}

impl SummarizationTask {
    pub fn new(document: Document, role: Role) -> Self {
        Self {
            id: TaskId::new(),
            document,
            role,
            config: None,
            priority: Priority::default(),
            retry_count: 0,
            max_retries: 3,
            created_at: Utc::now(),
            max_summary_length: None,
            force_regenerate: false,
            callback_url: None,
        }
    }

    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    pub fn with_max_summary_length(mut self, length: usize) -> Self {
        self.max_summary_length = Some(length);
        self
    }

    pub fn with_force_regenerate(mut self, force: bool) -> Self {
        self.force_regenerate = force;
        self
    }

    pub fn with_callback_url(mut self, url: String) -> Self {
        self.callback_url = Some(url);
        self
    }

    pub fn with_config(mut self, config: Config) -> Self {
        self.config = Some(config);
        self
    }

    pub fn can_retry(&self) -> bool {
        self.retry_count < self.max_retries
    }

    pub fn increment_retry(&mut self) {
        self.retry_count += 1;
    }

    pub fn get_summary_length(&self) -> usize {
        self.max_summary_length.unwrap_or(250)
    }
}

/// Configuration for the summarization queue
#[derive(Debug, Clone)]
pub struct QueueConfig {
    /// Maximum number of tasks in queue
    pub max_queue_size: usize,
    /// Maximum number of concurrent workers
    pub max_concurrent_workers: usize,
    /// Maximum time a task can stay in queue
    pub max_queue_time: Duration,
    /// How long to keep completed tasks in memory
    pub task_retention_time: Duration,
    /// Rate limiting settings per provider
    pub rate_limits: HashMap<String, RateLimitConfig>,
    /// Default retry delay
    pub retry_delay: Duration,
    /// Maximum retry delay (for exponential backoff)
    pub max_retry_delay: Duration,
}

impl Default for QueueConfig {
    fn default() -> Self {
        let mut rate_limits = HashMap::new();
        rate_limits.insert(
            "openrouter".to_string(),
            RateLimitConfig {
                max_requests_per_minute: 60,
                max_tokens_per_minute: 10000,
                burst_size: 10,
            },
        );
        rate_limits.insert(
            "ollama".to_string(),
            RateLimitConfig {
                max_requests_per_minute: 300,
                max_tokens_per_minute: 50000,
                burst_size: 50,
            },
        );

        Self {
            max_queue_size: 1000,
            max_concurrent_workers: 3,
            max_queue_time: Duration::from_secs(300), // 5 minutes
            task_retention_time: Duration::from_secs(3600), // 1 hour
            rate_limits,
            retry_delay: Duration::from_secs(1),
            max_retry_delay: Duration::from_secs(60),
        }
    }
}

/// Rate limiting configuration for LLM providers
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum requests per minute
    pub max_requests_per_minute: u32,
    /// Maximum tokens per minute (if applicable)
    pub max_tokens_per_minute: u32,
    /// Burst size for token bucket
    pub burst_size: u32,
}

/// Commands that can be sent to the queue worker
#[derive(Debug)]
pub enum QueueCommand {
    /// Submit a new task
    SubmitTask(Box<SummarizationTask>),
    /// Cancel a task
    CancelTask(TaskId, String),
    /// Pause processing
    Pause,
    /// Resume processing
    Resume,
    /// Get queue statistics
    GetStats(tokio::sync::oneshot::Sender<QueueStats>),
    /// Shutdown the worker
    Shutdown,
}

/// Statistics about the queue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStats {
    /// Total tasks in queue
    pub queue_size: usize,
    /// Number of pending tasks
    pub pending_tasks: usize,
    /// Number of processing tasks
    pub processing_tasks: usize,
    /// Number of completed tasks
    pub completed_tasks: usize,
    /// Number of failed tasks
    pub failed_tasks: usize,
    /// Number of cancelled tasks
    pub cancelled_tasks: usize,
    /// Average processing time in seconds
    pub avg_processing_time_seconds: Option<u64>,
    /// Whether queue is paused
    pub is_paused: bool,
    /// Active worker count
    pub active_workers: usize,
    /// Rate limiter status per provider
    pub rate_limiter_status: HashMap<String, RateLimiterStatus>,
}

/// Status of a rate limiter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimiterStatus {
    /// Current token count
    pub current_tokens: f64,
    /// Maximum tokens
    pub max_tokens: f64,
    /// Requests in current window
    pub requests_in_window: u32,
    /// Time until reset
    pub reset_in_seconds: u32,
}

/// Result of submitting a task to the queue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SubmitResult {
    /// Task was successfully queued
    Queued {
        task_id: TaskId,
        position_in_queue: usize,
        estimated_wait_time_seconds: Option<u64>,
    },
    /// Task was rejected due to queue being full
    QueueFull,
    /// Task was rejected due to duplicate
    Duplicate(TaskId),
    /// Task was rejected due to validation error
    ValidationError(String),
}

/// The main summarization queue
pub struct SummarizationQueue {
    /// Channel for sending commands to the worker
    command_sender: mpsc::Sender<QueueCommand>,
    /// Task status storage
    pub(crate) task_status: Arc<RwLock<HashMap<TaskId, TaskStatus>>>,
    /// Queue configuration
    config: QueueConfig,
}

impl SummarizationQueue {
    /// Create a new summarization queue with the provided command sender
    pub fn new(config: QueueConfig, command_sender: mpsc::Sender<QueueCommand>) -> Self {
        let task_status = Arc::new(RwLock::new(HashMap::new()));

        Self {
            command_sender,
            task_status,
            config,
        }
    }

    /// Submit a task to the queue
    pub async fn submit_task(
        &self,
        task: SummarizationTask,
    ) -> Result<SubmitResult, crate::ServiceError> {
        // Check if task already exists
        let task_status = self.task_status.read().await;
        if task_status.contains_key(&task.id) {
            return Ok(SubmitResult::Duplicate(task.id.clone()));
        }
        drop(task_status);

        // Validate task
        if task.document.body.trim().is_empty() {
            return Ok(SubmitResult::ValidationError(
                "Document body is empty".to_string(),
            ));
        }

        // Check queue capacity
        let stats = self.get_stats().await?;
        if stats.queue_size >= self.config.max_queue_size {
            return Ok(SubmitResult::QueueFull);
        }

        // Submit to worker
        let task_id = task.id.clone();
        if (self
            .command_sender
            .send(QueueCommand::SubmitTask(Box::new(task)))
            .await)
            .is_err()
        {
            return Err(crate::ServiceError::Config(
                "Queue worker not running".to_string(),
            ));
        }

        // Return success with estimated position
        let estimated_wait = self.estimate_wait_time(stats.pending_tasks + 1).await;
        Ok(SubmitResult::Queued {
            task_id,
            position_in_queue: stats.pending_tasks + 1,
            estimated_wait_time_seconds: estimated_wait.map(|d| d.as_secs()),
        })
    }

    /// Cancel a task
    pub async fn cancel_task(
        &self,
        task_id: TaskId,
        reason: String,
    ) -> Result<bool, crate::ServiceError> {
        let task_status = self.task_status.read().await;
        if !task_status.contains_key(&task_id) {
            return Ok(false);
        }
        drop(task_status);

        if (self
            .command_sender
            .send(QueueCommand::CancelTask(task_id, reason))
            .await)
            .is_err()
        {
            return Err(crate::ServiceError::Config(
                "Queue worker not running".to_string(),
            ));
        }

        Ok(true)
    }

    /// Get task status
    pub async fn get_task_status(&self, task_id: &TaskId) -> Option<TaskStatus> {
        let task_status = self.task_status.read().await;
        task_status.get(task_id).cloned()
    }

    /// Get queue statistics
    pub async fn get_stats(&self) -> Result<QueueStats, crate::ServiceError> {
        let (sender, receiver) = tokio::sync::oneshot::channel();

        if (self
            .command_sender
            .send(QueueCommand::GetStats(sender))
            .await)
            .is_err()
        {
            return Err(crate::ServiceError::Config(
                "Queue worker not running".to_string(),
            ));
        }

        receiver
            .await
            .map_err(|_| crate::ServiceError::Config("Failed to get queue stats".to_string()))
    }

    /// Pause queue processing
    pub async fn pause(&self) -> Result<(), crate::ServiceError> {
        if (self.command_sender.send(QueueCommand::Pause).await).is_err() {
            return Err(crate::ServiceError::Config(
                "Queue worker not running".to_string(),
            ));
        }
        Ok(())
    }

    /// Resume queue processing
    pub async fn resume(&self) -> Result<(), crate::ServiceError> {
        if (self.command_sender.send(QueueCommand::Resume).await).is_err() {
            return Err(crate::ServiceError::Config(
                "Queue worker not running".to_string(),
            ));
        }
        Ok(())
    }

    /// Estimate wait time for a task at given position
    async fn estimate_wait_time(&self, position: usize) -> Option<Duration> {
        if position == 0 {
            return Some(Duration::from_secs(0));
        }

        // Simple estimation: assume average processing time of 10 seconds per task
        // divided by number of concurrent workers
        let avg_processing_time = Duration::from_secs(10);
        let concurrent_workers = self.config.max_concurrent_workers;

        let estimated_seconds =
            (position as u64 * avg_processing_time.as_secs()) / concurrent_workers as u64;
        Some(Duration::from_secs(estimated_seconds))
    }

    /// Shutdown the queue
    pub async fn shutdown(&self) -> Result<(), crate::ServiceError> {
        if (self.command_sender.send(QueueCommand::Shutdown).await).is_err() {
            return Err(crate::ServiceError::Config(
                "Queue worker not running".to_string(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use terraphim_config::Role;
    use tokio::sync::mpsc;

    fn create_test_document() -> Document {
        Document {
            id: "test-doc".to_string(),
            title: "Test Document".to_string(),
            body: "This is a test document for summarization.".to_string(),
            url: "http://example.com".to_string(),
            description: None,
            summarization: None,
            stub: None,
            tags: Some(vec![]),
            rank: None,
            source_haystack: None,
        }
    }

    fn create_test_role() -> Role {
        Role {
            shortname: Some("test-role".to_string()),
            name: "Test Role".to_string().into(),
            relevance_function: terraphim_types::RelevanceFunction::TitleScorer,
            haystacks: vec![],
            terraphim_it: false,
            theme: "default".to_string(),
            kg: None,
            llm_enabled: false,
            llm_api_key: None,
            llm_model: None,
            llm_auto_summarize: false,
            llm_chat_enabled: false,
            llm_chat_system_prompt: None,
            llm_chat_model: None,
            llm_context_window: Some(32768),
            extra: ahash::AHashMap::new(),
            llm_router_enabled: false,
            llm_router_config: None,
        }
    }

    #[test]
    fn test_task_id_generation() {
        let id1 = TaskId::new();
        let id2 = TaskId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_task_creation() {
        let document = create_test_document();
        let role = create_test_role();

        let task = SummarizationTask::new(document.clone(), role.clone());
        assert_eq!(task.document.id, document.id);
        assert_eq!(task.role.name, role.name);
        assert_eq!(task.priority, Priority::Normal);
        assert_eq!(task.retry_count, 0);
        assert!(task.can_retry());
    }

    #[test]
    fn test_task_builder_methods() {
        let document = create_test_document();
        let role = create_test_role();

        let task = SummarizationTask::new(document, role)
            .with_priority(Priority::High)
            .with_max_retries(5)
            .with_max_summary_length(500)
            .with_force_regenerate(true)
            .with_callback_url("http://callback.com".to_string());

        assert_eq!(task.priority, Priority::High);
        assert_eq!(task.max_retries, 5);
        assert_eq!(task.get_summary_length(), 500);
        assert!(task.force_regenerate);
        assert_eq!(task.callback_url, Some("http://callback.com".to_string()));
    }

    #[test]
    fn test_priority_ordering() {
        assert!(Priority::Critical > Priority::High);
        assert!(Priority::High > Priority::Normal);
        assert!(Priority::Normal > Priority::Low);
    }

    #[test]
    fn test_task_status_checks() {
        let pending = TaskStatus::Pending {
            queued_at: Utc::now(),
            position_in_queue: Some(1),
        };
        assert!(pending.is_pending());
        assert!(!pending.is_terminal());

        let completed = TaskStatus::Completed {
            summary: "test".to_string(),
            completed_at: Utc::now(),
            processing_duration_seconds: 10,
        };
        assert!(completed.is_terminal());
        assert!(!completed.is_processing());
    }

    #[tokio::test]
    async fn test_queue_creation() {
        let config = QueueConfig::default();
        let (command_sender, _receiver) = mpsc::channel(10);
        let queue = SummarizationQueue::new(config, command_sender);

        // Queue should be created successfully
        assert!(queue.task_status.read().await.is_empty());
    }
}
