//! Execution coordination and workflow management

use std::sync::Arc;

use chrono::Utc;
use futures_util::future::join_all;
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};

use crate::{
    OrchestrationError, OrchestrationResult, ScheduledWorkflow, TaskId, TaskResult, TaskScheduler,
};

/// Execution coordinator that manages workflow execution
pub struct ExecutionCoordinator {
    /// Task scheduler
    scheduler: Arc<TaskScheduler>,
}

impl ExecutionCoordinator {
    /// Create a new execution coordinator
    pub fn new(scheduler: Arc<TaskScheduler>) -> Self {
        Self { scheduler }
    }

    /// Execute a complete workflow
    pub async fn execute_workflow(
        &self,
        workflow: ScheduledWorkflow,
    ) -> OrchestrationResult<WorkflowResult> {
        info!(
            "Starting workflow execution with {} subtasks",
            workflow.subtask_count()
        );
        let start_time = Utc::now();

        // For now, execute all tasks in parallel (ignoring dependencies)
        // TODO: Implement proper dependency-aware execution
        let mut task_futures = Vec::new();

        for assignment in &workflow.agent_assignments {
            let task = workflow
                .workflow
                .decomposition
                .subtasks
                .iter()
                .find(|t| t.task_id == assignment.task_id)
                .ok_or_else(|| {
                    OrchestrationError::CoordinationError(format!(
                        "Task {} not found in workflow",
                        assignment.task_id
                    ))
                })?;

            let agent = assignment.agent.clone();
            let task_clone = task.clone();

            task_futures.push(async move { agent.execute_task(&task_clone).await });
        }

        // Execute all tasks concurrently
        debug!("Executing {} tasks concurrently", task_futures.len());
        let results = join_all(task_futures).await;

        // Collect results and check for failures
        let mut task_results = Vec::new();
        let mut failed_tasks = Vec::new();

        for result in results {
            match result {
                Ok(task_result) => {
                    if task_result.is_failure() {
                        failed_tasks.push(task_result.task_id.clone());
                    }
                    task_results.push(task_result);
                }
                Err(e) => {
                    warn!("Task execution error: {}", e);
                    return Err(e);
                }
            }
        }

        let end_time = Utc::now();
        let total_duration = (end_time - start_time)
            .to_std()
            .unwrap_or(std::time::Duration::from_secs(0));

        let workflow_status = if failed_tasks.is_empty() {
            WorkflowStatus::Completed
        } else {
            WorkflowStatus::PartiallyFailed(failed_tasks)
        };

        let workflow_result = WorkflowResult {
            workflow_id: workflow.workflow.original_task.task_id.clone(),
            status: workflow_status,
            task_results,
            started_at: start_time,
            completed_at: end_time,
            total_duration,
            metadata: workflow.workflow.clone(),
        };

        info!("Workflow execution completed in {:?}", total_duration);
        Ok(workflow_result)
    }

    /// Execute a single task (convenience method)
    pub async fn execute_single_task(
        &self,
        task: &crate::Task,
    ) -> OrchestrationResult<WorkflowResult> {
        let scheduled_workflow = self.scheduler.schedule_task(task).await?;
        self.execute_workflow(scheduled_workflow).await
    }
}

/// Result of workflow execution
#[derive(Debug, Clone)]
pub struct WorkflowResult {
    /// Workflow identifier
    pub workflow_id: String,
    /// Execution status
    pub status: WorkflowStatus,
    /// Results from individual tasks
    pub task_results: Vec<TaskResult>,
    /// Workflow start time
    pub started_at: chrono::DateTime<chrono::Utc>,
    /// Workflow completion time
    pub completed_at: chrono::DateTime<chrono::Utc>,
    /// Total execution duration
    pub total_duration: std::time::Duration,
    /// Workflow metadata
    pub metadata: crate::TaskDecompositionWorkflow,
}

impl WorkflowResult {
    /// Check if the workflow completed successfully
    pub fn is_success(&self) -> bool {
        matches!(self.status, WorkflowStatus::Completed)
    }

    /// Check if the workflow failed completely
    pub fn is_failure(&self) -> bool {
        matches!(self.status, WorkflowStatus::Failed(_))
    }

    /// Check if the workflow partially failed
    pub fn is_partial_failure(&self) -> bool {
        matches!(self.status, WorkflowStatus::PartiallyFailed(_))
    }

    /// Get successful task results
    pub fn successful_results(&self) -> Vec<&TaskResult> {
        self.task_results
            .iter()
            .filter(|r| r.is_success())
            .collect()
    }

    /// Get failed task results
    pub fn failed_results(&self) -> Vec<&TaskResult> {
        self.task_results
            .iter()
            .filter(|r| r.is_failure())
            .collect()
    }

    /// Get overall success rate
    pub fn success_rate(&self) -> f64 {
        if self.task_results.is_empty() {
            return 0.0;
        }

        let successful_count = self.successful_results().len();
        successful_count as f64 / self.task_results.len() as f64
    }

    /// Aggregate all successful results into a single JSON value
    pub fn aggregate_results(&self) -> serde_json::Value {
        let successful_results: Vec<serde_json::Value> = self
            .successful_results()
            .iter()
            .filter_map(|r| r.result_data.clone())
            .collect();

        serde_json::json!({
            "workflow_id": self.workflow_id,
            "status": format!("{:?}", self.status),
            "total_tasks": self.task_results.len(),
            "successful_tasks": successful_results.len(),
            "success_rate": self.success_rate(),
            "total_duration_ms": self.total_duration.as_millis(),
            "results": successful_results
        })
    }
}

/// Status of workflow execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowStatus {
    /// Workflow is currently running
    Running,
    /// All tasks completed successfully
    Completed,
    /// Some tasks failed, but others succeeded
    PartiallyFailed(Vec<TaskId>),
    /// All tasks failed
    Failed(String),
    /// Workflow was cancelled
    Cancelled,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AgentPool, ExampleAgent, TaskComplexity};

    async fn create_test_coordinator() -> ExecutionCoordinator {
        let agent_pool = Arc::new(AgentPool::new());

        // Add test agents
        let agent1 = Arc::new(ExampleAgent::new(
            "agent1".to_string(),
            vec!["general".to_string()],
        ));
        let agent2 = Arc::new(ExampleAgent::new(
            "agent2".to_string(),
            vec!["general".to_string()],
        ));

        agent_pool.register_agent(agent1).await.unwrap();
        agent_pool.register_agent(agent2).await.unwrap();

        let scheduler = Arc::new(
            TaskScheduler::with_default_decomposition(agent_pool)
                .await
                .unwrap(),
        );
        ExecutionCoordinator::new(scheduler)
    }

    #[tokio::test]
    async fn test_coordinator_creation() {
        let coordinator = create_test_coordinator().await;
        assert_eq!(coordinator.scheduler.agent_pool().agent_count().await, 2);
    }

    #[tokio::test]
    async fn test_single_task_execution() {
        let coordinator = create_test_coordinator().await;

        let task = crate::Task::new(
            "test_task".to_string(),
            "Simple test task".to_string(),
            TaskComplexity::Simple,
            1,
        );

        let result = coordinator.execute_single_task(&task).await.unwrap();

        assert!(result.is_success());
        assert!(!result.task_results.is_empty());
        assert!(result.success_rate() > 0.0);
        assert_eq!(result.workflow_id, "test_task");
    }

    #[tokio::test]
    async fn test_workflow_result_aggregation() {
        let coordinator = create_test_coordinator().await;

        let task = crate::Task::new(
            "aggregation_test".to_string(),
            "Test task for result aggregation".to_string(),
            TaskComplexity::Simple,
            1,
        );

        let result = coordinator.execute_single_task(&task).await.unwrap();
        let aggregated = result.aggregate_results();

        assert_eq!(aggregated["workflow_id"], "aggregation_test");
        assert!(aggregated["total_tasks"].as_u64().unwrap() > 0);
        assert!(aggregated["success_rate"].as_f64().unwrap() > 0.0);
        assert!(aggregated["results"].is_array());
    }

    #[tokio::test]
    async fn test_workflow_status_methods() {
        let coordinator = create_test_coordinator().await;

        let task = crate::Task::new(
            "status_test".to_string(),
            "Test task for status methods".to_string(),
            TaskComplexity::Simple,
            1,
        );

        let result = coordinator.execute_single_task(&task).await.unwrap();

        // Should be successful since ExampleAgent always succeeds
        assert!(result.is_success());
        assert!(!result.is_failure());
        assert!(!result.is_partial_failure());
        assert_eq!(result.success_rate(), 1.0);
    }
}
