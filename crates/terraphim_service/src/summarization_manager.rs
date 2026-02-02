use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tokio::task::JoinHandle;

use terraphim_config::Role;
use terraphim_types::Document;

use crate::ServiceError;
use crate::summarization_queue::{
    Priority, QueueCommand, QueueConfig, QueueStats, SubmitResult, SummarizationQueue,
    SummarizationTask, TaskId, TaskStatus,
};
use crate::summarization_worker::SummarizationWorker;

/// High-level manager for the summarization system
pub struct SummarizationManager {
    queue: SummarizationQueue,
    worker_handle: Option<JoinHandle<Result<(), ServiceError>>>,
    #[allow(dead_code)] // Kept alive to prevent channel closure, queue has its own clone
    command_sender: mpsc::Sender<QueueCommand>,
}

impl SummarizationManager {
    /// Create a new summarization manager
    pub fn new(config: QueueConfig) -> Self {
        let (command_sender, command_receiver) = mpsc::channel(100);
        let queue = SummarizationQueue::new(config.clone(), command_sender.clone());

        // Get shared task status storage
        let task_status = queue.get_task_status_storage();

        // Create and start the worker
        let worker = SummarizationWorker::new(config, task_status);
        let worker_handle = Some(tokio::spawn(
            async move { worker.run(command_receiver).await },
        ));

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

    /// Submit a document for summarization with config
    #[allow(clippy::too_many_arguments)]
    pub async fn summarize_document_with_config(
        &self,
        document: Document,
        role: Role,
        config: terraphim_config::Config,
        priority: Option<Priority>,
        max_summary_length: Option<usize>,
        force_regenerate: Option<bool>,
        callback_url: Option<String>,
    ) -> Result<SubmitResult, ServiceError> {
        let mut task = SummarizationTask::new(document, role).with_config(config);

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

    /// Process document to generate description and optionally summarization
    ///
    /// This method provides immediate processing for documents that need:
    /// 1. Description extraction from content if missing
    /// 2. Optionally queue AI summarization if available
    ///
    /// # Arguments
    /// * `doc` - Mutable reference to document to process
    /// * `role` - Role configuration for summarization
    /// * `extract_description` - Whether to extract description from content if missing
    /// * `queue_summarization` - Whether to queue AI summarization task
    ///
    /// # Returns
    /// * `Result<Option<TaskId>, ServiceError>` - Task ID if summarization was queued, None if only description was processed
    pub async fn process_document_fields(
        &self,
        doc: &mut Document,
        role: &Role,
        extract_description: bool,
        queue_summarization: bool,
    ) -> Result<Option<TaskId>, ServiceError> {
        let mut task_id = None;

        // Extract description from content if requested and missing
        if extract_description && doc.description.is_none() && !doc.body.is_empty() {
            match Self::extract_description_from_body(&doc.body, 200) {
                Ok(description) => {
                    log::debug!(
                        "Generated description for document '{}': {} chars",
                        doc.id,
                        description.len()
                    );
                    doc.description = Some(description);
                }
                Err(e) => {
                    log::warn!(
                        "Failed to extract description for document '{}': {}",
                        doc.id,
                        e
                    );
                }
            }
        }

        // Queue AI summarization if requested and content is substantial
        if queue_summarization && doc.body.len() >= 500 {
            let submit_result = self
                .summarize_document(
                    doc.clone(),
                    role.clone(),
                    Some(Priority::Normal),
                    Some(300),   // max summary length
                    Some(false), // don't force regenerate
                    None,        // no callback URL
                )
                .await?;

            match submit_result {
                SubmitResult::Queued {
                    task_id: queued_task_id,
                    ..
                } => {
                    task_id = Some(queued_task_id.clone());
                    log::debug!(
                        "Queued AI summarization for document '{}' with task ID: {:?}",
                        doc.id,
                        queued_task_id
                    );
                }
                SubmitResult::Duplicate(existing_task_id) => {
                    task_id = Some(existing_task_id.clone());
                    log::debug!(
                        "Document '{}' already has summarization task: {:?}",
                        doc.id,
                        existing_task_id
                    );
                }
                SubmitResult::ValidationError(error) => {
                    log::warn!("Validation error for document '{}': {}", doc.id, error);
                }
                SubmitResult::QueueFull => {
                    log::warn!(
                        "Summarization queue is full, cannot queue document '{}'",
                        doc.id
                    );
                }
            }
        }

        Ok(task_id)
    }

    /// Extract description from document body content
    ///
    /// Takes the first substantial paragraph or line, up to max_length characters.
    /// Attempts to break at sentence boundaries when possible.
    ///
    /// # Arguments
    /// * `body` - Document body content
    /// * `max_length` - Maximum character count for description
    ///
    /// # Returns
    /// * `Result<String, ServiceError>` - Extracted description or error
    pub fn extract_description_from_body(
        body: &str,
        max_length: usize,
    ) -> Result<String, ServiceError> {
        if body.is_empty() {
            return Err(ServiceError::Config("Document body is empty".to_string()));
        }

        // Find the first substantial paragraph or line
        let first_paragraph = body
            .split('\n')
            .map(|line| line.trim())
            .find(|line| !line.is_empty() && line.len() > 10)
            .unwrap_or_else(|| body.trim());

        // If the paragraph is short enough, use it as-is
        if first_paragraph.len() <= max_length {
            return Ok(first_paragraph.to_string());
        }

        // Try to break at sentence boundary
        let truncated = &first_paragraph[..max_length];
        if let Some(last_period) = truncated.rfind(". ") {
            if last_period > max_length / 2 {
                return Ok(truncated[..=last_period].to_string());
            }
        }

        // Try to break at word boundary
        if let Some(last_space) = truncated.rfind(' ') {
            if last_space > max_length / 2 {
                return Ok(format!("{}...", &truncated[..last_space]));
            }
        }

        // Fallback to character truncation
        Ok(format!("{}...", &first_paragraph[..max_length - 3]))
    }

    /// Process multiple documents for description and summarization
    ///
    /// Efficiently processes a batch of documents, extracting descriptions
    /// and optionally queuing summarization tasks.
    ///
    /// # Arguments
    /// * `documents` - Mutable slice of documents to process
    /// * `role` - Role configuration for summarization
    /// * `extract_description` - Whether to extract descriptions
    /// * `queue_summarization` - Whether to queue AI summarization
    ///
    /// # Returns
    /// * `Result<Vec<Option<TaskId>>, ServiceError>` - Task IDs for queued summarizations
    pub async fn process_documents_batch(
        &self,
        documents: &mut [Document],
        role: &Role,
        extract_description: bool,
        queue_summarization: bool,
    ) -> Result<Vec<Option<TaskId>>, ServiceError> {
        log::info!(
            "Processing {} documents for description and summarization",
            documents.len()
        );

        let mut task_ids = Vec::with_capacity(documents.len());
        let mut successful_count = 0;
        let mut error_count = 0;

        for doc in documents.iter_mut() {
            match self
                .process_document_fields(doc, role, extract_description, queue_summarization)
                .await
            {
                Ok(task_id) => {
                    task_ids.push(task_id);
                    successful_count += 1;
                }
                Err(e) => {
                    log::error!("Failed to process document '{}': {}", doc.id, e);
                    task_ids.push(None);
                    error_count += 1;
                }
            }
        }

        log::info!(
            "Completed batch processing: {} successful, {} errors",
            successful_count,
            error_count
        );
        Ok(task_ids)
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
        self.worker_handle
            .as_ref()
            .is_some_and(|handle| !handle.is_finished())
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
    pub(crate) fn get_task_status_storage(
        &self,
    ) -> Arc<RwLock<std::collections::HashMap<TaskId, TaskStatus>>> {
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
            summarization: None,
            stub: None,
            tags: Some(vec![]),
            rank: None,
            source_haystack: None,
            doc_type: terraphim_types::DocumentType::KgEntry,
            synonyms: None,
            route: None,
            priority: None,
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
            extra: {
                let mut extra = ahash::AHashMap::new();
                extra.insert(
                    "llm_provider".to_string(),
                    serde_json::Value::String("test".to_string()),
                );
                extra
            },
            llm_router_enabled: false,
            llm_router_config: None,
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
        // Give worker time to start
        sleep(Duration::from_millis(100)).await;

        let document = create_test_document();
        let role = create_test_role();

        let result = manager
            .summarize_document(
                document,
                role,
                Some(Priority::High),
                Some(200),
                Some(false),
                Some("http://callback.com".to_string()),
            )
            .await;

        // Should succeed even without real LLM (task will fail in worker, but submission succeeds)
        assert!(result.is_ok());

        match result.unwrap() {
            SubmitResult::Queued { task_id, .. } => {
                // Wait a bit for the task to be processed by the worker
                sleep(Duration::from_millis(100)).await;

                // Should be able to get task status
                let status = manager.get_task_status(&task_id).await;
                assert!(status.is_some());
            }
            other => panic!("Unexpected result: {:?}", other),
        }
    }

    #[tokio::test]
    #[ignore = "Flaky test - timing dependent"]
    async fn test_task_cancellation() {
        let manager = SummarizationManager::new(QueueConfig::default());
        // Give worker time to start
        sleep(Duration::from_millis(100)).await;

        // Pause the queue so tasks don't get processed immediately
        manager.pause().await.expect("Failed to pause manager");

        let document = create_test_document();
        let role = create_test_role();

        let result = manager
            .summarize_document(
                document,
                role,
                Some(Priority::Low), // Low priority so it stays queued
                None,
                None,
                None,
            )
            .await
            .unwrap();

        if let SubmitResult::Queued { task_id, .. } = result {
            // Cancel the task while the queue is paused (so it's still queued)
            let cancelled = manager
                .cancel_task(task_id.clone(), "Test cancellation".to_string())
                .await
                .unwrap();
            assert!(cancelled, "Task cancellation should succeed");

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
        // Give worker time to start
        sleep(Duration::from_millis(100)).await;

        let stats = manager.get_stats().await.unwrap();
        assert_eq!(stats.queue_size, 0);
        assert_eq!(stats.pending_tasks, 0);
        assert_eq!(stats.processing_tasks, 0);
        assert!(!stats.is_paused);
    }

    #[tokio::test]
    async fn test_pause_resume() {
        let manager = SummarizationManager::new(QueueConfig::default());
        // Give worker time to start
        sleep(Duration::from_millis(100)).await;

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
        // Give worker time to start
        sleep(Duration::from_millis(100)).await;

        assert!(manager.is_healthy());

        manager.shutdown().await.unwrap();
        assert!(!manager.is_healthy());
    }
}
