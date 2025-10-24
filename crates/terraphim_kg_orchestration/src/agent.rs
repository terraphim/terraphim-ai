//! Simple agent abstractions for orchestration

use std::collections::HashMap;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{OrchestrationResult, Task, TaskId};

/// Simple agent trait for task execution
#[async_trait]
pub trait SimpleAgent: Send + Sync {
    /// Execute a task and return the result
    async fn execute_task(&self, task: &Task) -> OrchestrationResult<TaskResult>;

    /// Get the agent's capabilities
    fn capabilities(&self) -> &[String];

    /// Get the agent's unique identifier
    fn agent_id(&self) -> &str;

    /// Get the agent's current status
    fn status(&self) -> AgentStatus {
        AgentStatus::Available
    }

    /// Check if the agent can handle a specific task
    fn can_handle_task(&self, task: &Task) -> bool {
        // Default implementation: check if any required capability matches
        if task.required_capabilities.is_empty() {
            return true; // No specific requirements
        }

        let agent_caps: std::collections::HashSet<&String> = self.capabilities().iter().collect();
        task.required_capabilities
            .iter()
            .any(|req| agent_caps.contains(req))
    }

    /// Get agent metadata
    fn metadata(&self) -> AgentMetadata {
        AgentMetadata {
            agent_id: self.agent_id().to_string(),
            capabilities: self.capabilities().to_vec(),
            status: self.status(),
            created_at: Utc::now(),
            last_active: Utc::now(),
            total_tasks_completed: 0,
            average_execution_time: Duration::from_secs(0),
            success_rate: 1.0,
            custom_fields: HashMap::new(),
        }
    }
}

/// Result of task execution by an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    /// Task that was executed
    pub task_id: TaskId,
    /// Agent that executed the task
    pub agent_id: String,
    /// Execution status
    pub status: TaskExecutionStatus,
    /// Result data (if successful)
    pub result_data: Option<serde_json::Value>,
    /// Error message (if failed)
    pub error_message: Option<String>,
    /// Execution start time
    pub started_at: DateTime<Utc>,
    /// Execution completion time
    pub completed_at: DateTime<Utc>,
    /// Execution duration
    pub duration: Duration,
    /// Confidence score of the result (0.0 to 1.0)
    pub confidence_score: f64,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl TaskResult {
    /// Create a successful task result
    pub fn success(
        task_id: TaskId,
        agent_id: String,
        result_data: serde_json::Value,
        started_at: DateTime<Utc>,
    ) -> Self {
        let completed_at = Utc::now();
        let duration = (completed_at - started_at)
            .to_std()
            .unwrap_or(Duration::from_secs(0));

        Self {
            task_id,
            agent_id,
            status: TaskExecutionStatus::Completed,
            result_data: Some(result_data),
            error_message: None,
            started_at,
            completed_at,
            duration,
            confidence_score: 1.0,
            metadata: HashMap::new(),
        }
    }

    /// Create a failed task result
    pub fn failure(
        task_id: TaskId,
        agent_id: String,
        error_message: String,
        started_at: DateTime<Utc>,
    ) -> Self {
        let completed_at = Utc::now();
        let duration = (completed_at - started_at)
            .to_std()
            .unwrap_or(Duration::from_secs(0));

        Self {
            task_id,
            agent_id,
            status: TaskExecutionStatus::Failed,
            result_data: None,
            error_message: Some(error_message),
            started_at,
            completed_at,
            duration,
            confidence_score: 0.0,
            metadata: HashMap::new(),
        }
    }

    /// Check if the task execution was successful
    pub fn is_success(&self) -> bool {
        matches!(self.status, TaskExecutionStatus::Completed)
    }

    /// Check if the task execution failed
    pub fn is_failure(&self) -> bool {
        matches!(self.status, TaskExecutionStatus::Failed)
    }
}

/// Status of task execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskExecutionStatus {
    /// Task execution completed successfully
    Completed,
    /// Task execution failed
    Failed,
    /// Task execution was cancelled
    Cancelled,
    /// Task execution timed out
    TimedOut,
}

/// Agent status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AgentStatus {
    /// Agent is available for new tasks
    Available,
    /// Agent is currently executing a task
    Busy,
    /// Agent is temporarily unavailable
    Unavailable,
    /// Agent has failed and needs attention
    Failed,
}

/// Agent metadata for monitoring and management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetadata {
    /// Agent unique identifier
    pub agent_id: String,
    /// Agent capabilities
    pub capabilities: Vec<String>,
    /// Current agent status
    pub status: AgentStatus,
    /// When the agent was created
    pub created_at: DateTime<Utc>,
    /// Last activity timestamp
    pub last_active: DateTime<Utc>,
    /// Total number of tasks completed
    pub total_tasks_completed: u64,
    /// Average task execution time
    pub average_execution_time: Duration,
    /// Success rate (0.0 to 1.0)
    pub success_rate: f64,
    /// Custom metadata fields
    pub custom_fields: HashMap<String, serde_json::Value>,
}

/// Example agent implementation for testing
pub struct ExampleAgent {
    agent_id: String,
    capabilities: Vec<String>,
    status: AgentStatus,
}

impl ExampleAgent {
    pub fn new(agent_id: String, capabilities: Vec<String>) -> Self {
        Self {
            agent_id,
            capabilities,
            status: AgentStatus::Available,
        }
    }
}

#[async_trait]
impl SimpleAgent for ExampleAgent {
    async fn execute_task(&self, task: &Task) -> OrchestrationResult<TaskResult> {
        let started_at = Utc::now();

        // Simulate some work
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Simple success result
        let result_data = serde_json::json!({
            "task_id": task.task_id,
            "description": task.description,
            "agent_id": self.agent_id,
            "message": "Task completed successfully"
        });

        Ok(TaskResult::success(
            task.task_id.clone(),
            self.agent_id.clone(),
            result_data,
            started_at,
        ))
    }

    fn capabilities(&self) -> &[String] {
        &self.capabilities
    }

    fn agent_id(&self) -> &str {
        &self.agent_id
    }

    fn status(&self) -> AgentStatus {
        self.status.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TaskComplexity;

    #[tokio::test]
    async fn test_example_agent_execution() {
        let agent = ExampleAgent::new(
            "test_agent".to_string(),
            vec!["general".to_string(), "testing".to_string()],
        );

        let task = Task::new(
            "test_task".to_string(),
            "Test task description".to_string(),
            TaskComplexity::Simple,
            1,
        );

        let result = agent.execute_task(&task).await.unwrap();
        assert!(result.is_success());
        assert_eq!(result.task_id, "test_task");
        assert_eq!(result.agent_id, "test_agent");
    }

    #[test]
    fn test_agent_capabilities() {
        let agent = ExampleAgent::new(
            "test_agent".to_string(),
            vec!["capability1".to_string(), "capability2".to_string()],
        );

        assert_eq!(agent.capabilities().len(), 2);
        assert!(agent.capabilities().contains(&"capability1".to_string()));
        assert!(agent.capabilities().contains(&"capability2".to_string()));
    }

    #[test]
    fn test_can_handle_task() {
        let agent = ExampleAgent::new(
            "test_agent".to_string(),
            vec!["analysis".to_string(), "processing".to_string()],
        );

        let mut task = Task::new(
            "test_task".to_string(),
            "Test task".to_string(),
            TaskComplexity::Simple,
            1,
        );

        // Task with no requirements should be handleable
        assert!(agent.can_handle_task(&task));

        // Task with matching requirement
        task.required_capabilities.push("analysis".to_string());
        assert!(agent.can_handle_task(&task));

        // Task with non-matching requirement
        task.required_capabilities.clear();
        task.required_capabilities
            .push("unknown_capability".to_string());
        assert!(!agent.can_handle_task(&task));
    }

    #[test]
    fn test_task_result_creation() {
        let started_at = Utc::now();

        // Test successful result
        let success_result = TaskResult::success(
            "task1".to_string(),
            "agent1".to_string(),
            serde_json::json!({"result": "success"}),
            started_at,
        );
        assert!(success_result.is_success());
        assert!(!success_result.is_failure());

        // Test failed result
        let failure_result = TaskResult::failure(
            "task2".to_string(),
            "agent1".to_string(),
            "Task failed".to_string(),
            started_at,
        );
        assert!(!failure_result.is_success());
        assert!(failure_result.is_failure());
    }
}
