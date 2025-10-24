//! Task scheduling and decomposition integration

use std::sync::Arc;

use log::{debug, info};

use crate::{
    AgentPool, OrchestrationError, OrchestrationResult, SimpleAgent, Task, TaskDecompositionSystem,
    TaskDecompositionWorkflow, TerraphimTaskDecompositionSystem,
};

use terraphim_rolegraph::RoleGraph;
use terraphim_task_decomposition::{MockAutomata, TaskDecompositionSystemConfig};

/// Task scheduler that integrates with the task decomposition system
pub struct TaskScheduler {
    /// Task decomposition system
    decomposition_system: Arc<dyn TaskDecompositionSystem>,
    /// Agent pool for finding suitable agents
    agent_pool: Arc<AgentPool>,
}

impl TaskScheduler {
    /// Create a new task scheduler
    pub fn new(
        decomposition_system: Arc<dyn TaskDecompositionSystem>,
        agent_pool: Arc<AgentPool>,
    ) -> Self {
        Self {
            decomposition_system,
            agent_pool,
        }
    }

    /// Get the agent pool (for testing)
    pub fn agent_pool(&self) -> &Arc<AgentPool> {
        &self.agent_pool
    }

    /// Create a task scheduler with default decomposition system
    pub async fn with_default_decomposition(
        agent_pool: Arc<AgentPool>,
    ) -> OrchestrationResult<Self> {
        // Create mock automata and role graph for the decomposition system
        let automata = Arc::new(MockAutomata);
        let role_graph = Self::create_default_role_graph().await?;

        let decomposition_system = Arc::new(TerraphimTaskDecompositionSystem::with_default_config(
            automata, role_graph,
        ));

        Ok(Self::new(decomposition_system, agent_pool))
    }

    /// Schedule a task for execution
    pub async fn schedule_task(&self, task: &Task) -> OrchestrationResult<ScheduledWorkflow> {
        info!("Scheduling task: {}", task.task_id);

        // Step 1: Decompose the task using the task decomposition system
        let config = TaskDecompositionSystemConfig::default();
        let workflow = self
            .decomposition_system
            .decompose_task_workflow(task, &config)
            .await?;

        debug!(
            "Task decomposed into {} subtasks",
            workflow.decomposition.subtasks.len()
        );

        // Step 2: Find suitable agents for each subtask
        let mut agent_assignments = Vec::new();
        for subtask in &workflow.decomposition.subtasks {
            let suitable_agents = self.agent_pool().find_suitable_agents(subtask).await?;

            if suitable_agents.is_empty() {
                return Err(OrchestrationError::NoSuitableAgent(subtask.task_id.clone()));
            }

            // For now, just pick the first suitable agent
            // TODO: Implement more sophisticated agent selection
            let selected_agent = suitable_agents[0].clone();
            agent_assignments.push(AgentAssignment {
                task_id: subtask.task_id.clone(),
                agent: selected_agent,
            });
        }

        debug!("Assigned {} agents to subtasks", agent_assignments.len());

        Ok(ScheduledWorkflow {
            workflow,
            agent_assignments,
        })
    }

    /// Create a default role graph for testing
    async fn create_default_role_graph() -> OrchestrationResult<Arc<RoleGraph>> {
        use terraphim_automata::{load_thesaurus, AutomataPath};
        use terraphim_types::RoleName;

        let role_name = RoleName::new("orchestration_role");

        // Try to load a thesaurus, but fall back to empty if not available
        let thesaurus = match load_thesaurus(&AutomataPath::local_example()).await {
            Ok(thesaurus) => thesaurus,
            Err(_) => {
                debug!("Could not load thesaurus, using empty thesaurus for testing");
                // Create an empty thesaurus for testing
                use terraphim_types::Thesaurus;
                Thesaurus::new("empty_thesaurus".to_string())
            }
        };

        let role_graph = RoleGraph::new(role_name, thesaurus).await.map_err(|e| {
            OrchestrationError::System(format!("Failed to create role graph: {}", e))
        })?;

        Ok(Arc::new(role_graph))
    }
}

/// A scheduled workflow with agent assignments
#[derive(Debug)]
pub struct ScheduledWorkflow {
    /// The decomposed workflow
    pub workflow: TaskDecompositionWorkflow,
    /// Agent assignments for each subtask
    pub agent_assignments: Vec<AgentAssignment>,
}

/// Assignment of an agent to a specific task
pub struct AgentAssignment {
    /// Task ID
    pub task_id: String,
    /// Assigned agent
    pub agent: Arc<dyn SimpleAgent>,
}

impl std::fmt::Debug for AgentAssignment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentAssignment")
            .field("task_id", &self.task_id)
            .field("agent_id", &self.agent.agent_id())
            .finish()
    }
}

impl ScheduledWorkflow {
    /// Get the number of subtasks in the workflow
    pub fn subtask_count(&self) -> usize {
        self.workflow.decomposition.subtasks.len()
    }

    /// Get the estimated execution time
    pub fn estimated_duration(&self) -> std::time::Duration {
        self.workflow.execution_plan.estimated_duration
    }

    /// Get the confidence score of the workflow
    pub fn confidence_score(&self) -> f64 {
        self.workflow.metadata.confidence_score
    }

    /// Check if the workflow can be executed in parallel
    pub fn can_execute_in_parallel(&self) -> bool {
        self.workflow.metadata.parallelism_factor > 0.5
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ExampleAgent, TaskComplexity};

    async fn create_test_scheduler() -> TaskScheduler {
        let agent_pool = Arc::new(AgentPool::new());

        // Add a test agent
        let agent = Arc::new(ExampleAgent::new(
            "test_agent".to_string(),
            vec!["general".to_string()],
        ));
        agent_pool.register_agent(agent).await.unwrap();

        TaskScheduler::with_default_decomposition(agent_pool)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn test_scheduler_creation() {
        let scheduler = create_test_scheduler().await;
        assert_eq!(scheduler.agent_pool().agent_count().await, 1);
    }

    #[tokio::test]
    async fn test_task_scheduling() {
        let scheduler = create_test_scheduler().await;

        let task = Task::new(
            "test_task".to_string(),
            "Simple test task".to_string(),
            TaskComplexity::Simple,
            1,
        );

        let scheduled_workflow = scheduler.schedule_task(&task).await.unwrap();

        assert!(scheduled_workflow.subtask_count() > 0);
        assert!(!scheduled_workflow.agent_assignments.is_empty());
        assert!(scheduled_workflow.confidence_score() > 0.0);
    }

    #[tokio::test]
    async fn test_no_suitable_agent() {
        let agent_pool = Arc::new(AgentPool::new());

        // Add an agent with specific capabilities
        let agent = Arc::new(ExampleAgent::new(
            "specialized_agent".to_string(),
            vec!["specialized_capability".to_string()],
        ));
        agent_pool.register_agent(agent).await.unwrap();

        let scheduler = TaskScheduler::with_default_decomposition(agent_pool)
            .await
            .unwrap();

        let mut task = Task::new(
            "test_task".to_string(),
            "Task requiring different capability".to_string(),
            TaskComplexity::Simple,
            1,
        );
        task.required_capabilities
            .push("different_capability".to_string());

        // This should fail because no agent has the required capability
        let result = scheduler.schedule_task(&task).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            OrchestrationError::NoSuitableAgent(_)
        ));
    }
}
