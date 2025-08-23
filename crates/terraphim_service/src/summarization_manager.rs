use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::task::JoinHandle;

use terraphim_config::Role;
use terraphim_types::Document;

use crate::rate_limiter::RateLimiterManager;
use crate::summarization_queue::{
    Priority, QueueCommand, QueueConfig, QueueStats, SubmitResult, SummarizationQueue,
    SummarizationTask, TaskId, TaskStatus,
};
use crate::summarization_worker::SummarizationWorker;
use crate::ServiceError;

/// High-level manager for the summarization system
pub struct SummarizationManager {
    queue: SummarizationQueue,
    worker_handle: Option<JoinHandle<Result<(), ServiceError>>>,
    command_sender: mpsc::Sender<QueueCommand>,
}

impl SummarizationManager {
    /// Create a new summarization manager
    pub fn new(config: QueueConfig) -> Self {
        let (command_sender, command_receiver) = mpsc::channel(100);
        let queue = SummarizationQueue::new(config.clone());

        // Get shared task status storage
        let task_status = queue.get_task_status_storage();

        // Create and start the worker
        let worker = SummarizationWorker::new(config, task_status);
        let worker_handle = Some(tokio::spawn(async move {
            worker.run(command_receiver).await
        }));

        Self {
            queue,
            worker_handle,
            command_sender,
        }
    }

    /// Submit a document for summarization
    pub async fn summarize_document(
        &self,
        document: Document,
        role: Role,
        priority: Option<Priority>,
        max_summary_length: Option<usize>,
        force_regenerate: Option<bool>,
        callback_url: Option<String>,
    ) -> Result<SubmitResult, ServiceError> {
        let mut task = SummarizationTask::new(document, role);

        if let Some(priority) = priority {
            task = task.with_priority(priority);
        }

        if let Some(length) = max_summary_length {
            task = task.with_max_summary_length(length);
        }

        if let Some(force) = force_regenerate {
            task = task.with_force_regenerate(force);
        }

        if let Some(url) = callback_url {
            task = task.with_callback_url(url);
        }

        self.queue.submit_task(task).await
    }

    /// Get the status of a specific task
    pub async fn get_task_status(&self, task_id: &TaskId) -> Option<TaskStatus> {
        self.queue.get_task_status(task_id).await
    }

    /// Cancel a task
    pub async fn cancel_task(&self, task_id: TaskId, reason: String) -> Result<bool, ServiceError> {
        self.queue.cancel_task(task_id, reason).await
    }

    /// Get queue statistics
    pub async fn get_stats(&self) -> Result<QueueStats, ServiceError> {
        self.queue.get_stats().await
    }

    /// Pause queue processing
    pub async fn pause(&self) -> Result<(), ServiceError> {
        self.queue.pause().await
    }

    /// Resume queue processing
    pub async fn resume(&self) -> Result<(), ServiceError> {
        self.queue.resume().await
    }

    /// Shutdown the manager and all workers
    pub async fn shutdown(&mut self) -> Result<(), ServiceError> {
        // Send shutdown command
        self.queue.shutdown().await?;

        // Wait for worker to finish
        if let Some(handle) = self.worker_handle.take() {
            match handle.await {
                Ok(result) => result?,
                Err(e) => {
                    log::error!("Worker task panicked: {:?}", e);
                    return Err(ServiceError::Config("Worker task panicked".to_string()));
                }
            }
        }

        log::info!("Summarization manager shut down successfully");
        Ok(())
    }

    /// Check if the manager is healthy (worker is running)
    pub fn is_healthy(&self) -> bool {
        self.worker_handle.as_ref().map_or(false, |handle| !handle.is_finished())
    }

    /// Get a reference to the internal queue for direct access if needed
    pub fn get_queue(&self) -> &SummarizationQueue {
        &self.queue
    }
}

/// Builder for creating summarization managers with custom configurations
pub struct SummarizationManagerBuilder {
    config: QueueConfig,
}

impl SummarizationManagerBuilder {
    /// Create a new builder with default configuration
    pub fn new() -> Self {
        Self {
            config: QueueConfig::default(),
        }
    }

    /// Set the maximum queue size
    pub fn max_queue_size(mut self, size: usize) -> Self {
        self.config.max_queue_size = size;
        self
    }

    /// Set the maximum number of concurrent workers
    pub fn max_concurrent_workers(mut self, workers: usize) -> Self {
        self.config.max_concurrent_workers = workers;
        self
    }

    /// Set the maximum time a task can stay in queue
    pub fn max_queue_time(mut self, duration: std::time::Duration) -> Self {
        self.config.max_queue_time = duration;
        self
    }

    /// Set how long to keep completed tasks in memory
    pub fn task_retention_time(mut self, duration: std::time::Duration) -> Self {
        self.config.task_retention_time = duration;
        self
    }

    /// Build the summarization manager
    pub fn build(self) -> SummarizationManager {
        SummarizationManager::new(self.config)
    }
}

impl Default for SummarizationManagerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// Add a method to SummarizationQueue to expose task status storage
impl SummarizationQueue {
    pub(crate) fn get_task_status_storage(&self) -> Arc<RwLock<std::collections::HashMap<TaskId, TaskStatus>>> {
        Arc::clone(&self.task_status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::sleep;

    fn create_test_document() -> Document {
        Document {
            id: "test-doc".to_string(),
            title: "Test Document".to_string(),
            body: "This is a test document for summarization with enough content to make it interesting.".to_string(),
            url: "http://example.com".to_string(),
            description: None,
            stub: None,
            tags: Some(vec![]),
            rank: None,
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
            extra: {
                let mut extra = ahash::AHashMap::new();
                extra.insert("llm_provider".to_string(), serde_json::Value::String("test".to_string()));
                extra
            },
        }
    }

    #[tokio::test]
    async fn test_manager_creation() {
        let manager = SummarizationManager::new(QueueConfig::default());
        assert!(manager.is_healthy());

        // Let it run for a moment
        sleep(Duration::from_millis(100)).await;
        assert!(manager.is_healthy());
    }

    #[tokio::test]
    async fn test_manager_builder() {
        let manager = SummarizationManagerBuilder::new()
            .max_queue_size(500)
            .max_concurrent_workers(2)
            .max_queue_time(Duration::from_secs(180))
            .build();

        assert!(manager.is_healthy());
    }

    #[tokio::test]
    async fn test_task_submission() {
        let manager = SummarizationManager::new(QueueConfig::default());
        let document = create_test_document();
        let role = create_test_role();

        let result = manager.summarize_document(
            document,
            role,
            Some(Priority::High),
            Some(200),
            Some(false),
            Some("http://callback.com".to_string()),
        ).await;

        // Should succeed even without real LLM (task will fail in worker, but submission succeeds)
        assert!(result.is_ok());
        
        match result.unwrap() {
            SubmitResult::Queued { task_id, .. } => {
                // Should be able to get task status
                let status = manager.get_task_status(&task_id).await;
                assert!(status.is_some());
            }
            other => panic!("Unexpected result: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_task_cancellation() {
        let manager = SummarizationManager::new(QueueConfig::default());
        let document = create_test_document();
        let role = create_test_role();

        let result = manager.summarize_document(
            document,
            role,
            Some(Priority::Low), // Low priority so it stays queued
            None,
            None,
            None,
        ).await.unwrap();

        if let SubmitResult::Queued { task_id, .. } = result {
            // Cancel the task
            let cancelled = manager.cancel_task(task_id.clone(), "Test cancellation".to_string()).await.unwrap();
            assert!(cancelled);

            // Check status
            sleep(Duration::from_millis(100)).await;
            let status = manager.get_task_status(&task_id).await;
            if let Some(TaskStatus::Cancelled { reason, .. }) = status {
                assert_eq!(reason, "Test cancellation");
            } else {
                panic!("Task should be cancelled, got: {:?}", status);
            }
        }
    }

    #[tokio::test]
    async fn test_queue_stats() {
        let manager = SummarizationManager::new(QueueConfig::default());
        
        let stats = manager.get_stats().await.unwrap();
        assert_eq!(stats.queue_size, 0);
        assert_eq!(stats.pending_tasks, 0);
        assert_eq!(stats.processing_tasks, 0);
        assert!(!stats.is_paused);
    }

    #[tokio::test]
    async fn test_pause_resume() {
        let manager = SummarizationManager::new(QueueConfig::default());
        
        // Pause
        manager.pause().await.unwrap();
        let stats = manager.get_stats().await.unwrap();
        assert!(stats.is_paused);

        // Resume
        manager.resume().await.unwrap();
        let stats = manager.get_stats().await.unwrap();
        assert!(!stats.is_paused);
    }

    #[tokio::test]
    async fn test_manager_shutdown() {
        let mut manager = SummarizationManager::new(QueueConfig::default());
        assert!(manager.is_healthy());

        manager.shutdown().await.unwrap();
        assert!(!manager.is_healthy());
    }
}