//! GitHub Runner agent implementation

use crate::{
    RunnerConfig, RunnerId, RunnerResult, RunnerError, RunnerState,
    WorkflowJobEvent, Step,
};
use crate::registration::RunnerRegistration;
use crate::event_handler::WorkflowEventHandler;
use crate::executor::ActionExecutor;
use crate::knowledge_graph::{ThesaurusBuilder, ActionGraph};
use crate::history::ExecutionHistory;
use ahash::AHashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

/// GitHub Runner agent implementing supervised agent patterns
pub struct GitHubRunnerAgent {
    /// Agent ID
    id: RunnerId,
    /// Agent name
    name: String,
    /// Registration with GitHub
    registration: RunnerRegistration,
    /// Event handler
    event_handler: WorkflowEventHandler,
    /// Action executor
    executor: ActionExecutor,
    /// Knowledge graph
    graph: Option<ActionGraph>,
    /// Current state
    state: Arc<RwLock<RunnerState>>,
    /// Shutdown signal
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl GitHubRunnerAgent {
    /// Create a new GitHub runner agent
    pub fn new(config: RunnerConfig) -> Self {
        let name = config.name.clone();
        let registration = RunnerRegistration::new(config.clone());
        let executor = ActionExecutor::new(config);

        Self {
            id: RunnerId::new(),
            name,
            registration,
            event_handler: WorkflowEventHandler::new(100),
            executor,
            graph: None,
            state: Arc::new(RwLock::new(RunnerState::Initializing)),
            shutdown_tx: None,
        }
    }

    /// Set knowledge graph from thesaurus
    pub fn with_knowledge_graph(mut self, thesaurus: ThesaurusBuilder) -> Self {
        self.graph = Some(ActionGraph::from_thesaurus(thesaurus));
        self
    }

    /// Get agent ID
    pub fn id(&self) -> &RunnerId {
        &self.id
    }

    /// Get agent name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get current state
    pub async fn state(&self) -> RunnerState {
        *self.state.read().await
    }

    /// Start the agent
    pub async fn start(&mut self) -> RunnerResult<()> {
        log::info!("Starting GitHub runner agent: {}", self.name);

        // Register with GitHub
        self.registration.register().await?;
        *self.state.write().await = RunnerState::Idle;

        // Set up shutdown channel
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel(1);
        self.shutdown_tx = Some(shutdown_tx);

        // Get event receiver
        let mut event_rx = self.event_handler.take_receiver()
            .ok_or_else(|| RunnerError::InvalidState("Event receiver already taken".to_string()))?;

        // Start event handler
        if let Some(runner_id) = self.registration.github_runner_id() {
            self.event_handler.start(runner_id, "").await?;
        }

        // Main agent loop
        let state = Arc::clone(&self.state);
        let executor = &self.executor;

        log::info!("Agent {} ready for jobs", self.name);

        loop {
            tokio::select! {
                // Check for shutdown signal
                _ = shutdown_rx.recv() => {
                    log::info!("Agent {} received shutdown signal", self.name);
                    break;
                }

                // Process events
                Some(event) = event_rx.recv() => {
                    log::info!("Received event: {:?}", event.action);

                    match event.action.as_str() {
                        "queued" => {
                            *state.write().await = RunnerState::Busy;

                            // Execute the job
                            match self.execute_job(&event).await {
                                Ok(_) => {
                                    log::info!("Job completed successfully");
                                }
                                Err(e) => {
                                    log::error!("Job failed: {}", e);
                                }
                            }

                            *state.write().await = RunnerState::Idle;
                        }
                        "completed" => {
                            log::debug!("Job completed notification");
                        }
                        _ => {
                            log::debug!("Ignoring event action: {}", event.action);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Execute a workflow job
    async fn execute_job(&self, event: &WorkflowJobEvent) -> RunnerResult<()> {
        log::info!("Executing job {} for {}", event.workflow_job.name, event.repository.full_name);

        // In a real implementation, we would:
        // 1. Fetch the workflow file
        // 2. Parse the job steps
        // 3. Set up environment
        // 4. Execute each step

        // For now, this is a placeholder
        let env = self.build_environment(event);

        // The steps would come from parsing the workflow
        let steps: Vec<Step> = Vec::new();

        if !steps.is_empty() {
            self.executor.execute_job(&steps, &event.workflow_job.id.to_string(), &env).await?;
        }

        Ok(())
    }

    /// Build environment variables for job
    fn build_environment(&self, event: &WorkflowJobEvent) -> AHashMap<String, String> {
        let mut env = AHashMap::new();

        // Standard GitHub environment variables
        env.insert("GITHUB_ACTIONS".to_string(), "true".to_string());
        env.insert("GITHUB_REPOSITORY".to_string(), event.repository.full_name.clone());
        env.insert("GITHUB_SHA".to_string(), event.workflow_job.head_sha.clone());
        env.insert("GITHUB_REF".to_string(), event.workflow_job.head_branch.clone().unwrap_or_default());
        env.insert("GITHUB_WORKFLOW".to_string(), event.workflow_job.workflow_name.clone().unwrap_or_default());
        env.insert("GITHUB_JOB".to_string(), event.workflow_job.name.clone());
        env.insert("GITHUB_RUN_ID".to_string(), event.workflow_job.run_id.to_string());
        env.insert("GITHUB_RUN_NUMBER".to_string(), event.workflow_job.id.to_string());
        env.insert("GITHUB_SERVER_URL".to_string(), "https://github.com".to_string());
        env.insert("RUNNER_NAME".to_string(), self.name.clone());

        env
    }

    /// Stop the agent
    pub async fn stop(&mut self) -> RunnerResult<()> {
        log::info!("Stopping agent: {}", self.name);

        // Send shutdown signal
        if let Some(tx) = &self.shutdown_tx {
            let _ = tx.send(()).await;
        }

        // Stop event handler
        self.event_handler.stop();

        // Unregister from GitHub
        self.registration.unregister().await?;

        // Shutdown executor
        self.executor.shutdown().await?;

        *self.state.write().await = RunnerState::Offline;

        log::info!("Agent {} stopped", self.name);
        Ok(())
    }

    /// Get execution history
    pub async fn history(&self) -> ExecutionHistory {
        self.executor.history().await
    }

    /// Inject an event for testing
    pub async fn inject_event(&self, event: WorkflowJobEvent) -> RunnerResult<()> {
        self.event_handler.send_event(event).await
    }

    /// Check if agent is healthy
    pub async fn is_healthy(&self) -> bool {
        match *self.state.read().await {
            RunnerState::Idle | RunnerState::Busy => true,
            _ => false,
        }
    }

    /// Get statistics
    pub async fn statistics(&self) -> AgentStatistics {
        let history = self.executor.history().await;
        let stats = history.statistics();

        AgentStatistics {
            agent_id: self.id.0.clone(),
            agent_name: self.name.clone(),
            state: *self.state.read().await,
            total_executions: stats.total_executions,
            total_successes: stats.total_successes,
            total_failures: stats.total_failures,
            is_registered: self.registration.is_registered(),
        }
    }
}

/// Statistics about agent operation
#[derive(Debug, Clone)]
pub struct AgentStatistics {
    /// Agent ID
    pub agent_id: String,
    /// Agent name
    pub agent_name: String,
    /// Current state
    pub state: RunnerState,
    /// Total executions
    pub total_executions: usize,
    /// Successful executions
    pub total_successes: usize,
    /// Failed executions
    pub total_failures: usize,
    /// Whether registered with GitHub
    pub is_registered: bool,
}

/// Builder for creating runner agents
pub struct GitHubRunnerAgentBuilder {
    config: Option<RunnerConfig>,
    thesaurus: Option<ThesaurusBuilder>,
}

impl GitHubRunnerAgentBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            config: None,
            thesaurus: None,
        }
    }

    /// Set configuration
    pub fn with_config(mut self, config: RunnerConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// Set knowledge graph from thesaurus
    pub fn with_thesaurus(mut self, thesaurus: ThesaurusBuilder) -> Self {
        self.thesaurus = Some(thesaurus);
        self
    }

    /// Build the agent
    pub fn build(self) -> RunnerResult<GitHubRunnerAgent> {
        let config = self.config
            .ok_or_else(|| RunnerError::Configuration("Configuration required".to_string()))?;

        let mut agent = GitHubRunnerAgent::new(config);

        if let Some(thesaurus) = self.thesaurus {
            agent = agent.with_knowledge_graph(thesaurus);
        }

        Ok(agent)
    }
}

impl Default for GitHubRunnerAgentBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RunnerLabels, RunnerScope};

    fn create_test_config() -> RunnerConfig {
        RunnerConfig {
            name: "test-runner".to_string(),
            scope: RunnerScope::Repository {
                owner: "test".to_string(),
                repo: "repo".to_string(),
            },
            registration_token: "test-token".to_string(),
            labels: RunnerLabels::default(),
            work_directory: "/tmp/runner".to_string(),
            max_concurrent_jobs: 1,
            vm_pool_size: 2,
            enable_llm: false,
            llm_config: None,
        }
    }

    #[tokio::test]
    async fn test_agent_creation() {
        let config = create_test_config();
        let agent = GitHubRunnerAgent::new(config);

        assert_eq!(agent.name(), "test-runner");
        assert_eq!(agent.state().await, RunnerState::Initializing);
    }

    #[tokio::test]
    async fn test_agent_builder() {
        let config = create_test_config();
        let mut builder = ThesaurusBuilder::new("https://github.com/test/repo");
        builder.add_builtin_terms();

        let agent = GitHubRunnerAgentBuilder::new()
            .with_config(config)
            .with_thesaurus(builder)
            .build()
            .unwrap();

        assert!(agent.graph.is_some());
    }

    #[tokio::test]
    async fn test_agent_statistics() {
        let config = create_test_config();
        let agent = GitHubRunnerAgent::new(config);

        let stats = agent.statistics().await;
        assert_eq!(stats.agent_name, "test-runner");
        assert_eq!(stats.total_executions, 0);
        assert!(!stats.is_registered);
    }
}
