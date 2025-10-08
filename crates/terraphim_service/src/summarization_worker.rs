use std::collections::{BinaryHeap, HashMap};
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::Utc;

use tokio::sync::{mpsc, RwLock};
use tokio::task::JoinHandle;
use tokio::time::{sleep, timeout};

use terraphim_persistence::Persistable;

use crate::llm::SummarizeOptions;
// Rate limiter imports removed - not needed for sequential processing
use crate::summarization_queue::{
    QueueCommand, QueueConfig, QueueStats, SummarizationTask, TaskId, TaskStatus,
};
use crate::ServiceError;

/// A task wrapper for priority queue ordering
#[derive(Debug)]
struct PriorityTask {
    task: SummarizationTask,
    created_at: Instant,
}

impl PartialEq for PriorityTask {
    fn eq(&self, other: &Self) -> bool {
        self.task.priority == other.task.priority && self.created_at == other.created_at
    }
}

impl Eq for PriorityTask {}

impl PartialOrd for PriorityTask {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PriorityTask {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Higher priority first, then earlier creation time
        self.task
            .priority
            .cmp(&other.task.priority)
            .then(other.created_at.cmp(&self.created_at))
    }
}

/// Statistics tracking for the worker
#[derive(Debug, Default)]
struct WorkerStats {
    total_processed: u64,
    total_successful: u64,
    total_failed: u64,
    total_cancelled: u64,
    processing_times: Vec<Duration>,
}

impl WorkerStats {
    fn record_success(&mut self, duration: Duration) {
        self.total_processed += 1;
        self.total_successful += 1;
        self.processing_times.push(duration);

        // Keep only last 100 processing times for average calculation
        if self.processing_times.len() > 100 {
            self.processing_times.remove(0);
        }
    }

    fn record_failure(&mut self) {
        self.total_processed += 1;
        self.total_failed += 1;
    }

    fn record_cancelled(&mut self) {
        self.total_processed += 1;
        self.total_cancelled += 1;
    }

    fn avg_processing_time(&self) -> Option<Duration> {
        if self.processing_times.is_empty() {
            return None;
        }

        let total: Duration = self.processing_times.iter().sum();
        Some(total / self.processing_times.len() as u32)
    }
}

/// Background worker for processing summarization tasks
pub struct SummarizationWorker {
    config: QueueConfig,
    task_queue: BinaryHeap<PriorityTask>,
    task_status: Arc<RwLock<HashMap<TaskId, TaskStatus>>>,
    stats: Arc<RwLock<WorkerStats>>,
    is_paused: bool,
    active_workers: usize,
    worker_handles: Vec<JoinHandle<()>>,
}

impl SummarizationWorker {
    /// Create a new summarization worker
    pub fn new(config: QueueConfig, task_status: Arc<RwLock<HashMap<TaskId, TaskStatus>>>) -> Self {
        Self {
            config,
            task_queue: BinaryHeap::new(),
            task_status,
            stats: Arc::new(RwLock::new(WorkerStats::default())),
            is_paused: false,
            active_workers: 0,
            worker_handles: Vec::new(),
        }
    }

    /// Start the worker and begin processing tasks
    pub async fn run(
        mut self,
        mut command_receiver: mpsc::Receiver<QueueCommand>,
    ) -> Result<(), ServiceError> {
        log::info!(
            "Starting summarization worker with {} max concurrent workers",
            self.config.max_concurrent_workers
        );

        // Spawn worker tasks
        let (task_sender, task_receiver) = mpsc::channel(self.config.max_concurrent_workers * 2);
        let task_receiver = Arc::new(tokio::sync::Mutex::new(task_receiver));

        for worker_id in 0..self.config.max_concurrent_workers {
            let task_receiver = Arc::clone(&task_receiver);
            let task_status = Arc::clone(&self.task_status);
            let stats = Arc::clone(&self.stats);
            let retry_delay = self.config.retry_delay;
            let max_retry_delay = self.config.max_retry_delay;

            let handle = tokio::spawn(async move {
                Self::worker_loop(
                    worker_id,
                    task_receiver,
                    task_status,
                    stats,
                    retry_delay,
                    max_retry_delay,
                )
                .await;
            });

            self.worker_handles.push(handle);
        }

        // Main command processing loop
        loop {
            tokio::select! {
                // Handle commands
                command = command_receiver.recv() => {
                    match command {
                        Some(cmd) => {
                            match self.handle_command(cmd, &task_sender).await {
                                Ok(should_continue) => {
                                    if !should_continue {
                                        break;
                                    }
                                }
                                Err(e) => {
                                    log::error!("Error handling command: {:?}", e);
                                }
                            }
                        }
                        None => {
                            log::info!("Command channel closed, shutting down worker");
                            break;
                        }
                    }
                }

                // Periodic maintenance
                _ = sleep(Duration::from_secs(10)) => {
                    self.cleanup_old_tasks().await;
                    self.log_stats().await;
                }
            }
        }

        // Shutdown workers
        for handle in self.worker_handles {
            handle.abort();
        }

        log::info!("Summarization worker shut down");
        Ok(())
    }

    /// Handle a command from the main thread
    async fn handle_command(
        &mut self,
        command: QueueCommand,
        task_sender: &mpsc::Sender<SummarizationTask>,
    ) -> Result<bool, ServiceError> {
        match command {
            QueueCommand::SubmitTask(task) => {
                self.submit_task(*task, task_sender).await?;
            }
            QueueCommand::CancelTask(task_id, reason) => {
                self.cancel_task(task_id, reason).await;
            }
            QueueCommand::Pause => {
                self.is_paused = true;
                log::info!("Queue processing paused");
            }
            QueueCommand::Resume => {
                self.is_paused = false;
                log::info!("Queue processing resumed");
            }
            QueueCommand::GetStats(sender) => {
                let stats = self.get_current_stats().await;
                let _ = sender.send(stats);
            }
            QueueCommand::Shutdown => {
                log::info!("Received shutdown command");
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Submit a task to the processing queue
    async fn submit_task(
        &mut self,
        task: SummarizationTask,
        task_sender: &mpsc::Sender<SummarizationTask>,
    ) -> Result<(), ServiceError> {
        let task_id = task.id.clone();

        // Update task status
        {
            let mut status_map = self.task_status.write().await;
            status_map.insert(
                task_id.clone(),
                TaskStatus::Pending {
                    queued_at: Utc::now(),
                    position_in_queue: Some(self.task_queue.len() + 1),
                },
            );
        }

        // If not paused, try to send directly to workers
        if !self.is_paused
            && self.active_workers < self.config.max_concurrent_workers
            && task_sender.try_send(task.clone()).is_ok()
        {
            log::debug!("Task {} sent directly to worker", task_id);
            return Ok(());
        }

        // Add to priority queue
        self.task_queue.push(PriorityTask {
            task,
            created_at: Instant::now(),
        });

        log::debug!(
            "Task {} queued (queue size: {})",
            task_id,
            self.task_queue.len()
        );
        Ok(())
    }

    /// Cancel a task
    async fn cancel_task(&mut self, task_id: TaskId, reason: String) {
        // Update task status
        {
            let mut status_map = self.task_status.write().await;
            if let Some(status) = status_map.get(&task_id) {
                if !status.is_terminal() {
                    status_map.insert(
                        task_id.clone(),
                        TaskStatus::Cancelled {
                            cancelled_at: Utc::now(),
                            reason: reason.clone(),
                        },
                    );
                    drop(status_map);

                    // Record cancellation in stats
                    {
                        let mut worker_stats = self.stats.write().await;
                        worker_stats.record_cancelled();
                    }

                    log::info!("Task {} cancelled: {}", task_id, reason);
                }
            }
        }

        // Remove from queue if present
        let mut new_queue = BinaryHeap::new();
        while let Some(priority_task) = self.task_queue.pop() {
            if priority_task.task.id != task_id {
                new_queue.push(priority_task);
            }
        }
        self.task_queue = new_queue;
    }

    /// Get current queue statistics
    async fn get_current_stats(&self) -> QueueStats {
        let status_map = self.task_status.read().await;
        let mut pending = 0;
        let mut processing = 0;
        let mut completed = 0;
        let mut failed = 0;
        let mut cancelled = 0;

        for status in status_map.values() {
            match status {
                TaskStatus::Pending { .. } => pending += 1,
                TaskStatus::Processing { .. } => processing += 1,
                TaskStatus::Completed { .. } => completed += 1,
                TaskStatus::Failed { .. } => failed += 1,
                TaskStatus::Cancelled { .. } => cancelled += 1,
            }
        }

        QueueStats {
            queue_size: self.task_queue.len() + processing,
            pending_tasks: pending,
            processing_tasks: processing,
            completed_tasks: completed,
            failed_tasks: failed,
            cancelled_tasks: cancelled,
            avg_processing_time_seconds: {
                let stats = self.stats.read().await;
                stats.avg_processing_time().map(|d| d.as_secs())
            },
            is_paused: self.is_paused,
            active_workers: self.active_workers,
            rate_limiter_status: std::collections::HashMap::new(), // Rate limiting removed
        }
    }

    /// Clean up old completed tasks
    async fn cleanup_old_tasks(&mut self) {
        let mut status_map = self.task_status.write().await;
        let cutoff = Utc::now()
            - chrono::Duration::from_std(self.config.task_retention_time).unwrap_or_default();

        let mut to_remove = Vec::new();
        for (task_id, status) in status_map.iter() {
            let should_remove = match status {
                TaskStatus::Completed { completed_at, .. } => *completed_at < cutoff,
                TaskStatus::Failed { failed_at, .. } => *failed_at < cutoff,
                TaskStatus::Cancelled { cancelled_at, .. } => *cancelled_at < cutoff,
                _ => false,
            };

            if should_remove {
                to_remove.push(task_id.clone());
            }
        }

        for task_id in to_remove {
            status_map.remove(&task_id);
        }

        if !status_map.is_empty() {
            log::debug!("Cleaned up {} old tasks", status_map.len());
        }
    }

    /// Log periodic statistics
    async fn log_stats(&self) {
        let stats = self.stats.read().await;
        if stats.total_processed > 0 && stats.total_processed % 50 == 0 {
            log::info!(
                "Worker stats: {} processed, {} successful, {} failed, {} cancelled, avg time: {:?}",
                stats.total_processed,
                stats.total_successful,
                stats.total_failed,
                stats.total_cancelled,
                stats.avg_processing_time()
            );
        }
    }

    /// Individual worker loop
    async fn worker_loop(
        worker_id: usize,
        task_receiver: Arc<tokio::sync::Mutex<mpsc::Receiver<SummarizationTask>>>,
        task_status: Arc<RwLock<HashMap<TaskId, TaskStatus>>>,
        stats: Arc<RwLock<WorkerStats>>,
        retry_delay: Duration,
        max_retry_delay: Duration,
    ) {
        log::info!("Worker {} started", worker_id);

        loop {
            let task = {
                let mut receiver = task_receiver.lock().await;
                match receiver.recv().await {
                    Some(task) => task,
                    None => {
                        log::info!("Worker {} shutting down", worker_id);
                        break;
                    }
                }
            };

            let task_id = task.id.clone();
            log::debug!("Worker {} processing task {}", worker_id, task_id);

            // Update status to processing
            {
                let mut status_map = task_status.write().await;
                status_map.insert(
                    task_id.clone(),
                    TaskStatus::Processing {
                        started_at: Utc::now(),
                        progress: Some(0.0),
                    },
                );
            }

            let start_time = Instant::now();
            match Self::process_task(task.clone()).await {
                Ok(summary) => {
                    let duration = start_time.elapsed();

                    // Update the task status with completion
                    let mut status_map = task_status.write().await;
                    status_map.insert(
                        task_id.clone(),
                        TaskStatus::Completed {
                            summary: summary.clone(),
                            completed_at: Utc::now(),
                            processing_duration_seconds: duration.as_secs(),
                        },
                    );
                    drop(status_map);

                    // Persist the document with the summarization for future retrieval
                    let mut updated_document = task.document.clone();
                    updated_document.summarization = Some(summary.clone());

                    match updated_document.save().await {
                        Ok(_) => {
                            log::debug!(
                                "Worker {} persisted document {} with summarization",
                                worker_id,
                                task.document.id
                            );
                        }
                        Err(e) => {
                            log::warn!(
                                "Worker {} failed to persist document {} with summarization: {:?}",
                                worker_id,
                                task.document.id,
                                e
                            );
                            // Don't fail the task for persistence errors
                        }
                    }

                    // Record successful processing in stats
                    {
                        let mut worker_stats = stats.write().await;
                        worker_stats.record_success(duration);
                    }

                    log::info!(
                        "Worker {} completed task {} in {:?}",
                        worker_id,
                        task_id,
                        duration
                    );
                }
                Err(error) => {
                    // Handle retry logic
                    let mut retry_task = task.clone();
                    retry_task.increment_retry();

                    if retry_task.can_retry() {
                        let delay = Self::calculate_retry_delay(
                            retry_task.retry_count,
                            retry_delay,
                            max_retry_delay,
                        );
                        log::warn!(
                            "Worker {} task {} failed, retrying in {:?} (attempt {}/{}): {}",
                            worker_id,
                            task_id,
                            delay,
                            retry_task.retry_count,
                            retry_task.max_retries,
                            error
                        );

                        let next_retry =
                            Utc::now() + chrono::Duration::from_std(delay).unwrap_or_default();
                        let mut status_map = task_status.write().await;
                        status_map.insert(
                            task_id.clone(),
                            TaskStatus::Failed {
                                error: error.to_string(),
                                failed_at: Utc::now(),
                                retry_count: retry_task.retry_count,
                                next_retry_at: Some(next_retry),
                            },
                        );

                        // TODO: Re-queue the task after delay
                        // For now, we'll just mark it as failed
                    } else {
                        let mut status_map = task_status.write().await;
                        status_map.insert(
                            task_id.clone(),
                            TaskStatus::Failed {
                                error: error.to_string(),
                                failed_at: Utc::now(),
                                retry_count: retry_task.retry_count,
                                next_retry_at: None,
                            },
                        );
                        drop(status_map);

                        // Record failure in stats (final failure after retries exhausted)
                        {
                            let mut worker_stats = stats.write().await;
                            worker_stats.record_failure();
                        }
                        log::error!(
                            "Worker {} task {} failed permanently after {} retries: {}",
                            worker_id,
                            task_id,
                            retry_task.retry_count,
                            error
                        );
                    }
                }
            }
        }
    }

    /// Process a single task
    async fn process_task(task: SummarizationTask) -> Result<String, ServiceError> {
        // Check if summary already exists and force_regenerate is false
        if !task.force_regenerate {
            if let Some(existing_summary) = &task.document.description {
                if !existing_summary.trim().is_empty() && existing_summary.len() >= 50 {
                    log::info!(
                        "Worker bypassing LLM: Using existing description as summary for document '{}' (length: {})",
                        task.document.id, existing_summary.len()
                    );
                    return Ok(existing_summary.clone());
                }
            }

            // Check if document already has summarization (caching)
            if let Some(existing_summary) = &task.document.summarization {
                log::info!(
                    "Worker bypassing LLM: Using cached summarization for document '{}' (length: {})",
                    task.document.id, existing_summary.len()
                );
                return Ok(existing_summary.clone());
            }
        } else {
            log::info!(
                "Worker forcing regeneration: Skipping cached summaries for document '{}' (force_regenerate=true)",
                task.document.id
            );
        }

        // Build LLM client from role with config fallback
        let llm = crate::llm::build_llm_for_summarization(&task.role, task.config.as_ref())
            .ok_or_else(|| {
                ServiceError::Config("No LLM provider configured for role".to_string())
            })?;

        // Note: Rate limiting removed - not needed for sequential task processing

        // Create summarization options
        let options = SummarizeOptions {
            max_length: task.get_summary_length(),
        };

        // Call LLM with timeout
        log::info!(
            "Worker calling REAL LLM for document '{}' with {} chars of content",
            task.document.id,
            task.document.body.len()
        );
        let summary_future = llm.summarize(&task.document.body, options);
        let summary = timeout(Duration::from_secs(120), summary_future)
            .await
            .map_err(|_| ServiceError::Config("Summarization timeout".to_string()))?
            .map_err(|e| ServiceError::Config(format!("LLM error: {}", e)))?;

        if summary.trim().is_empty() {
            return Err(ServiceError::Config(
                "Generated summary is empty".to_string(),
            ));
        }

        Ok(summary)
    }

    /// Calculate retry delay with exponential backoff
    fn calculate_retry_delay(
        retry_count: u32,
        base_delay: Duration,
        max_delay: Duration,
    ) -> Duration {
        let delay = base_delay * 2_u32.pow(retry_count.saturating_sub(1));
        delay.min(max_delay)
    }
}

// Note: QueueBasedRateLimiterManager implements Clone automatically

#[cfg(test)]
mod tests {
    use super::*;
    use crate::summarization_queue::Priority;
    use terraphim_config::Role;
    use terraphim_types::Document;

    fn create_test_document() -> Document {
        Document {
            id: "test-doc".to_string(),
            title: "Test Document".to_string(),
            body: "This is a test document for summarization with enough content to make it interesting.".to_string(),
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
            #[cfg(feature = "openrouter")]
            openrouter_enabled: false,
            #[cfg(feature = "openrouter")]
            openrouter_api_key: None,
            #[cfg(feature = "openrouter")]
            openrouter_model: None,
            #[cfg(feature = "openrouter")]
            openrouter_auto_summarize: false,
            #[cfg(feature = "openrouter")]
            openrouter_chat_enabled: false,
            #[cfg(feature = "openrouter")]
            openrouter_chat_system_prompt: None,
            #[cfg(feature = "openrouter")]
            openrouter_chat_model: None,
            llm_system_prompt: None,
            extra: ahash::AHashMap::new(),
        }
    }

    #[test]
    fn test_priority_task_ordering() {
        let doc = create_test_document();
        let role = create_test_role();

        let task_low =
            SummarizationTask::new(doc.clone(), role.clone()).with_priority(Priority::Low);
        let task_high =
            SummarizationTask::new(doc.clone(), role.clone()).with_priority(Priority::High);

        let priority_low = PriorityTask {
            task: task_low,
            created_at: Instant::now(),
        };
        let priority_high = PriorityTask {
            task: task_high,
            created_at: Instant::now(),
        };

        assert!(priority_high > priority_low);
    }

    #[test]
    fn test_retry_delay_calculation() {
        let base_delay = Duration::from_secs(1);
        let max_delay = Duration::from_secs(30);

        assert_eq!(
            SummarizationWorker::calculate_retry_delay(1, base_delay, max_delay),
            Duration::from_secs(1)
        );
        assert_eq!(
            SummarizationWorker::calculate_retry_delay(2, base_delay, max_delay),
            Duration::from_secs(2)
        );
        assert_eq!(
            SummarizationWorker::calculate_retry_delay(5, base_delay, max_delay),
            Duration::from_secs(16)
        );

        // Should be capped at max_delay
        assert_eq!(
            SummarizationWorker::calculate_retry_delay(10, base_delay, max_delay),
            max_delay
        );
    }

    #[tokio::test]
    async fn test_worker_stats() {
        let mut stats = WorkerStats::default();

        stats.record_success(Duration::from_secs(5));
        stats.record_success(Duration::from_secs(3));
        stats.record_failure();

        assert_eq!(stats.total_processed, 3);
        assert_eq!(stats.total_successful, 2);
        assert_eq!(stats.total_failed, 1);
        assert_eq!(stats.avg_processing_time(), Some(Duration::from_secs(4)));
    }

    #[tokio::test]
    async fn test_task_status_updates() {
        let task_status = Arc::new(RwLock::new(HashMap::new()));
        let task_id = TaskId::new();

        // Test pending status
        {
            let mut status_map = task_status.write().await;
            status_map.insert(
                task_id.clone(),
                TaskStatus::Pending {
                    queued_at: Utc::now(),
                    position_in_queue: Some(1),
                },
            );
        }

        let status = task_status.read().await.get(&task_id).cloned();
        assert!(matches!(status, Some(TaskStatus::Pending { .. })));
    }
}
