//! Workflow event handling from GitHub

use crate::{RunnerResult, RunnerError, WorkflowJobEvent, Job};
use reqwest::Client;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Handler for workflow events from GitHub
pub struct WorkflowEventHandler {
    /// HTTP client
    client: Client,
    /// Event channel sender
    event_tx: mpsc::Sender<WorkflowJobEvent>,
    /// Event channel receiver (for consumption by executor)
    event_rx: Option<mpsc::Receiver<WorkflowJobEvent>>,
    /// Running state
    running: Arc<std::sync::atomic::AtomicBool>,
}

impl WorkflowEventHandler {
    /// Create a new event handler
    pub fn new(buffer_size: usize) -> Self {
        let (tx, rx) = mpsc::channel(buffer_size);
        Self {
            client: Client::new(),
            event_tx: tx,
            event_rx: Some(rx),
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Take the event receiver (can only be called once)
    pub fn take_receiver(&mut self) -> Option<mpsc::Receiver<WorkflowJobEvent>> {
        self.event_rx.take()
    }

    /// Start listening for events
    pub async fn start(&self, runner_id: u64, access_token: &str) -> RunnerResult<()> {
        self.running
            .store(true, std::sync::atomic::Ordering::SeqCst);

        log::info!("Starting event handler for runner {}", runner_id);

        // In a real implementation, this would:
        // 1. Open a long-poll connection to GitHub Actions service
        // 2. Parse incoming job events
        // 3. Send them through the channel

        // For now, this is a placeholder that would be replaced with
        // actual GitHub Actions runner protocol implementation

        Ok(())
    }

    /// Stop listening for events
    pub fn stop(&self) {
        self.running
            .store(false, std::sync::atomic::Ordering::SeqCst);
        log::info!("Event handler stopped");
    }

    /// Send an event (for testing or manual job injection)
    pub async fn send_event(&self, event: WorkflowJobEvent) -> RunnerResult<()> {
        self.event_tx
            .send(event)
            .await
            .map_err(|e| RunnerError::InvalidState(format!("Failed to send event: {}", e)))
    }

    /// Parse a workflow file into jobs
    pub fn parse_workflow(content: &str) -> RunnerResult<Vec<Job>> {
        let workflow: crate::Workflow = serde_yaml::from_str(content)?;
        Ok(workflow.jobs.into_values().collect())
    }

    /// Check if handler is running
    pub fn is_running(&self) -> bool {
        self.running.load(std::sync::atomic::Ordering::SeqCst)
    }
}
