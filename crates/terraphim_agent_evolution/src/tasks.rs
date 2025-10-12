//! Agent task list evolution with complete lifecycle tracking

use std::collections::{BTreeMap, HashMap};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use terraphim_persistence::Persistable;
use uuid::Uuid;

use crate::{AgentId, EvolutionError, EvolutionResult, TaskId};

/// Safe conversion from chrono::Duration to std::Duration
fn chrono_to_std_duration(chrono_duration: chrono::Duration) -> Option<std::time::Duration> {
    let nanos = chrono_duration.num_nanoseconds()?;
    if nanos < 0 {
        None
    } else {
        Some(std::time::Duration::from_nanos(nanos as u64))
    }
}

/// Versioned task list evolution system
#[derive(Debug, Clone)]
pub struct TasksEvolution {
    pub agent_id: AgentId,
    pub current_state: TasksState,
    pub history: BTreeMap<DateTime<Utc>, TasksState>,
}

impl TasksEvolution {
    /// Create a new task evolution tracker
    pub fn new(agent_id: AgentId) -> Self {
        Self {
            agent_id,
            current_state: TasksState::default(),
            history: BTreeMap::new(),
        }
    }

    /// Add a new task
    pub async fn add_task(&mut self, task: AgentTask) -> EvolutionResult<()> {
        log::debug!("Adding task: {} - {}", task.id, task.content);

        self.current_state.add_task(task);
        self.save_current_state().await?;

        Ok(())
    }

    /// Start working on a task (move to in_progress)
    pub async fn start_task(&mut self, task_id: &TaskId) -> EvolutionResult<()> {
        log::debug!("Starting task: {}", task_id);

        self.current_state.start_task(task_id)?;
        self.save_current_state().await?;

        Ok(())
    }

    /// Complete a task
    pub async fn complete_task(&mut self, task_id: &TaskId, result: &str) -> EvolutionResult<()> {
        log::info!("Completing task: {} with result: {}", task_id, result);

        self.current_state.complete_task(task_id, result)?;
        self.save_current_state().await?;

        Ok(())
    }

    /// Block a task (waiting on dependencies)
    pub async fn block_task(&mut self, task_id: &TaskId, reason: String) -> EvolutionResult<()> {
        log::debug!("Blocking task: {} - reason: {}", task_id, reason);

        self.current_state.block_task(task_id, reason)?;
        self.save_current_state().await?;

        Ok(())
    }

    /// Cancel a task
    pub async fn cancel_task(&mut self, task_id: &TaskId, reason: String) -> EvolutionResult<()> {
        log::debug!("Cancelling task: {} - reason: {}", task_id, reason);

        self.current_state.cancel_task(task_id, reason)?;
        self.save_current_state().await?;

        Ok(())
    }

    /// Update task progress
    pub async fn update_progress(
        &mut self,
        task_id: &TaskId,
        progress: &str,
    ) -> EvolutionResult<()> {
        self.current_state.update_progress(task_id, progress)?;
        self.save_current_state().await?;

        Ok(())
    }

    /// Add workflow tasks (multiple tasks from a workflow)
    pub async fn add_workflow_tasks(
        &mut self,
        workflow_steps: &[crate::WorkflowStep],
    ) -> EvolutionResult<()> {
        for (i, step) in workflow_steps.iter().enumerate() {
            let task = AgentTask {
                id: format!("workflow_task_{}", i),
                content: step.description.clone(),
                active_form: format!("Working on: {}", step.description),
                status: TaskStatus::Pending,
                priority: Priority::Medium,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                deadline: None,
                dependencies: vec![],
                subtasks: vec![],
                parent_task: None,
                goal_alignment_score: 0.8, // Default alignment
                estimated_duration: step.estimated_duration,
                actual_duration: None,
                metadata: HashMap::new(),
            };

            self.add_task(task).await?;
        }

        Ok(())
    }

    /// Save a versioned snapshot
    pub async fn save_version(&self, timestamp: DateTime<Utc>) -> EvolutionResult<()> {
        let versioned_tasks = VersionedTaskList {
            agent_id: self.agent_id.clone(),
            timestamp,
            state: self.current_state.clone(),
        };

        versioned_tasks.save().await?;
        log::debug!(
            "Saved task list version for agent {} at {}",
            self.agent_id,
            timestamp
        );

        Ok(())
    }

    /// Load task state at a specific time
    pub async fn load_version(&self, timestamp: DateTime<Utc>) -> EvolutionResult<TasksState> {
        let mut versioned_tasks = VersionedTaskList::new(self.get_version_key(timestamp));
        let loaded = versioned_tasks.load().await?;
        Ok(loaded.state)
    }

    /// Get the storage key for a specific version
    pub fn get_version_key(&self, timestamp: DateTime<Utc>) -> String {
        format!("agent_{}/tasks/v_{}", self.agent_id, timestamp.timestamp())
    }

    /// Save the current state
    async fn save_current_state(&self) -> EvolutionResult<()> {
        let current_tasks = CurrentTasksState {
            agent_id: self.agent_id.clone(),
            state: self.current_state.clone(),
        };

        current_tasks.save().await?;
        Ok(())
    }
}

/// Current task state of an agent
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TasksState {
    pub pending: Vec<AgentTask>,
    pub in_progress: Vec<AgentTask>,
    pub completed: Vec<CompletedTask>,
    pub blocked: Vec<BlockedTask>,
    pub cancelled: Vec<CancelledTask>,
    pub metadata: TasksMetadata,
}

impl TasksState {
    /// Add a new task
    pub fn add_task(&mut self, task: AgentTask) {
        self.pending.push(task);
        self.metadata.last_updated = Utc::now();
        self.metadata.total_tasks_created += 1;
    }

    /// Start a task (move from pending to in_progress)
    pub fn start_task(&mut self, task_id: &TaskId) -> EvolutionResult<()> {
        if let Some(pos) = self.pending.iter().position(|t| t.id == *task_id) {
            let mut task = self.pending.remove(pos);
            task.status = TaskStatus::InProgress;
            task.updated_at = Utc::now();
            self.in_progress.push(task);
            self.metadata.last_updated = Utc::now();
            Ok(())
        } else {
            Err(EvolutionError::TaskNotFound(task_id.clone()))
        }
    }

    /// Complete a task
    pub fn complete_task(&mut self, task_id: &TaskId, result: &str) -> EvolutionResult<()> {
        if let Some(pos) = self.in_progress.iter().position(|t| t.id == *task_id) {
            let task = self.in_progress.remove(pos);
            let completed_task = CompletedTask {
                original_task: task.clone(),
                completed_at: Utc::now(),
                result: result.to_string(),
                actual_duration: chrono_to_std_duration(Utc::now() - task.created_at),
                success: true,
            };

            self.completed.push(completed_task);
            self.metadata.last_updated = Utc::now();
            self.metadata.total_completed += 1;
            Ok(())
        } else if let Some(pos) = self.pending.iter().position(|t| t.id == *task_id) {
            // Allow completing pending tasks directly
            let task = self.pending.remove(pos);
            let completed_task = CompletedTask {
                original_task: task.clone(),
                completed_at: Utc::now(),
                result: result.to_string(),
                actual_duration: chrono_to_std_duration(Utc::now() - task.created_at),
                success: true,
            };

            self.completed.push(completed_task);
            self.metadata.last_updated = Utc::now();
            self.metadata.total_completed += 1;
            Ok(())
        } else {
            Err(EvolutionError::TaskNotFound(task_id.clone()))
        }
    }

    /// Block a task
    pub fn block_task(&mut self, task_id: &TaskId, reason: String) -> EvolutionResult<()> {
        if let Some(pos) = self.in_progress.iter().position(|t| t.id == *task_id) {
            let task = self.in_progress.remove(pos);
            let blocked_task = BlockedTask {
                original_task: task,
                blocked_at: Utc::now(),
                reason,
                dependencies: vec![],
            };

            self.blocked.push(blocked_task);
            self.metadata.last_updated = Utc::now();
            Ok(())
        } else {
            Err(EvolutionError::TaskNotFound(task_id.clone()))
        }
    }

    /// Cancel a task
    pub fn cancel_task(&mut self, task_id: &TaskId, reason: String) -> EvolutionResult<()> {
        let mut found = false;

        // Try pending first
        if let Some(pos) = self.pending.iter().position(|t| t.id == *task_id) {
            let task = self.pending.remove(pos);
            let cancelled_task = CancelledTask {
                original_task: task,
                cancelled_at: Utc::now(),
                reason: reason.clone(),
            };
            self.cancelled.push(cancelled_task);
            found = true;
        }

        // Try in_progress
        if !found {
            if let Some(pos) = self.in_progress.iter().position(|t| t.id == *task_id) {
                let task = self.in_progress.remove(pos);
                let cancelled_task = CancelledTask {
                    original_task: task,
                    cancelled_at: Utc::now(),
                    reason: reason.clone(),
                };
                self.cancelled.push(cancelled_task);
                found = true;
            }
        }

        if found {
            self.metadata.last_updated = Utc::now();
            self.metadata.total_cancelled += 1;
            Ok(())
        } else {
            Err(EvolutionError::TaskNotFound(task_id.clone()))
        }
    }

    /// Update task progress
    pub fn update_progress(&mut self, task_id: &TaskId, progress: &str) -> EvolutionResult<()> {
        if let Some(task) = self.in_progress.iter_mut().find(|t| t.id == *task_id) {
            task.metadata.insert(
                "progress".to_string(),
                serde_json::Value::String(progress.to_string()),
            );
            task.updated_at = Utc::now();
            self.metadata.last_updated = Utc::now();
            Ok(())
        } else {
            Err(EvolutionError::TaskNotFound(task_id.clone()))
        }
    }

    /// Calculate task completion rate
    pub fn calculate_completion_rate(&self) -> f64 {
        let total = self.total_tasks();
        if total > 0 {
            self.completed.len() as f64 / total as f64
        } else {
            0.0
        }
    }

    /// Calculate goal alignment score based on completed tasks
    pub fn calculate_alignment_score(&self) -> f64 {
        if self.completed.is_empty() {
            return 0.5; // Neutral if no completed tasks
        }

        let total_alignment: f64 = self
            .completed
            .iter()
            .map(|ct| ct.original_task.goal_alignment_score)
            .sum();

        total_alignment / self.completed.len() as f64
    }

    /// Get total number of tasks
    pub fn total_tasks(&self) -> usize {
        self.pending.len()
            + self.in_progress.len()
            + self.completed.len()
            + self.blocked.len()
            + self.cancelled.len()
    }

    /// Get number of completed tasks
    pub fn completed_tasks(&self) -> usize {
        self.completed.len()
    }

    /// Get number of pending tasks
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Get number of in-progress tasks
    pub fn in_progress_count(&self) -> usize {
        self.in_progress.len()
    }

    /// Get number of blocked tasks
    pub fn blocked_count(&self) -> usize {
        self.blocked.len()
    }

    /// Calculate average task complexity
    pub fn calculate_average_complexity(&self) -> f64 {
        let all_tasks: Vec<&AgentTask> = self
            .pending
            .iter()
            .chain(self.in_progress.iter())
            .chain(self.completed.iter().map(|ct| &ct.original_task))
            .chain(self.blocked.iter().map(|bt| &bt.original_task))
            .chain(self.cancelled.iter().map(|ct| &ct.original_task))
            .collect();

        if all_tasks.is_empty() {
            0.0
        } else {
            // Use complexity as a simple metric based on content length and priority
            let total_complexity: f64 = all_tasks
                .iter()
                .map(|task| {
                    let length_complexity = task.content.len() as f64 / 100.0; // Normalize by 100 chars
                    let priority_complexity = match task.priority {
                        Priority::Low => 1.0,
                        Priority::Medium => 2.0,
                        Priority::High => 3.0,
                        Priority::Critical => 4.0,
                    };
                    length_complexity + priority_complexity
                })
                .sum();
            total_complexity / all_tasks.len() as f64
        }
    }
}

/// Individual agent task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTask {
    pub id: TaskId,
    pub content: String,
    pub active_form: String, // "Working on X" vs "Work on X"
    pub status: TaskStatus,
    pub priority: Priority,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deadline: Option<DateTime<Utc>>,
    pub dependencies: Vec<TaskId>,
    pub subtasks: Vec<TaskId>,
    pub parent_task: Option<TaskId>,
    pub goal_alignment_score: f64,
    pub estimated_duration: Option<std::time::Duration>,
    pub actual_duration: Option<std::time::Duration>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl AgentTask {
    /// Create a new task
    pub fn new(content: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            active_form: format!("Working on: {}", content),
            content,
            status: TaskStatus::Pending,
            priority: Priority::Medium,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deadline: None,
            dependencies: vec![],
            subtasks: vec![],
            parent_task: None,
            goal_alignment_score: 0.5,
            estimated_duration: None,
            actual_duration: None,
            metadata: HashMap::new(),
        }
    }

    /// Check if task is overdue
    pub fn is_overdue(&self) -> bool {
        if let Some(deadline) = self.deadline {
            Utc::now() > deadline
        } else {
            false
        }
    }

    /// Get task age
    pub fn age(&self) -> chrono::Duration {
        Utc::now() - self.created_at
    }
}

/// Task status enumeration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Blocked,
    Cancelled,
}

/// Task priority levels
#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

/// Completed task record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletedTask {
    pub original_task: AgentTask,
    pub completed_at: DateTime<Utc>,
    pub result: String,
    pub actual_duration: Option<std::time::Duration>,
    pub success: bool,
}

/// Blocked task record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockedTask {
    pub original_task: AgentTask,
    pub blocked_at: DateTime<Utc>,
    pub reason: String,
    pub dependencies: Vec<TaskId>,
}

/// Cancelled task record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelledTask {
    pub original_task: AgentTask,
    pub cancelled_at: DateTime<Utc>,
    pub reason: String,
}

/// Task list metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TasksMetadata {
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
    pub total_tasks_created: u32,
    pub total_completed: u32,
    pub total_cancelled: u32,
    pub average_completion_time: Option<std::time::Duration>,
}

impl Default for TasksMetadata {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            created_at: now,
            last_updated: now,
            total_tasks_created: 0,
            total_completed: 0,
            total_cancelled: 0,
            average_completion_time: None,
        }
    }
}

/// Workflow step for task creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub description: String,
    pub estimated_duration: Option<std::time::Duration>,
}

/// Versioned task list for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionedTaskList {
    pub agent_id: AgentId,
    pub timestamp: DateTime<Utc>,
    pub state: TasksState,
}

#[async_trait]
impl Persistable for VersionedTaskList {
    fn new(_key: String) -> Self {
        Self {
            agent_id: String::new(),
            timestamp: Utc::now(),
            state: TasksState::default(),
        }
    }

    async fn save(&self) -> terraphim_persistence::Result<()> {
        self.save_to_all().await
    }

    async fn save_to_one(&self, profile_name: &str) -> terraphim_persistence::Result<()> {
        self.save_to_profile(profile_name).await
    }

    async fn load(&mut self) -> terraphim_persistence::Result<Self> {
        let key = self.get_key();
        self.load_from_operator(
            &key,
            &terraphim_persistence::DeviceStorage::instance()
                .await?
                .fastest_op,
        )
        .await
    }

    fn get_key(&self) -> String {
        format!(
            "agent_{}/tasks/v_{}",
            self.agent_id,
            self.timestamp.timestamp()
        )
    }
}

/// Current task state for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentTasksState {
    pub agent_id: AgentId,
    pub state: TasksState,
}

#[async_trait]
impl Persistable for CurrentTasksState {
    fn new(key: String) -> Self {
        Self {
            agent_id: key,
            state: TasksState::default(),
        }
    }

    async fn save(&self) -> terraphim_persistence::Result<()> {
        self.save_to_all().await
    }

    async fn save_to_one(&self, profile_name: &str) -> terraphim_persistence::Result<()> {
        self.save_to_profile(profile_name).await
    }

    async fn load(&mut self) -> terraphim_persistence::Result<Self> {
        let key = self.get_key();
        self.load_from_operator(
            &key,
            &terraphim_persistence::DeviceStorage::instance()
                .await?
                .fastest_op,
        )
        .await
    }

    fn get_key(&self) -> String {
        format!("agent_{}/tasks/current", self.agent_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_task_evolution_creation() {
        let agent_id = "test_agent".to_string();
        let tasks = TasksEvolution::new(agent_id.clone());

        assert_eq!(tasks.agent_id, agent_id);
        assert_eq!(tasks.current_state.total_tasks(), 0);
    }

    #[tokio::test]
    async fn test_task_lifecycle() {
        let mut tasks = TasksEvolution::new("test_agent".to_string());

        // Add a task
        let task = AgentTask::new("Test task".to_string());
        let task_id = task.id.clone();

        tasks.add_task(task).await.unwrap();
        assert_eq!(tasks.current_state.pending.len(), 1);

        // Start the task
        tasks.start_task(&task_id).await.unwrap();
        assert_eq!(tasks.current_state.pending.len(), 0);
        assert_eq!(tasks.current_state.in_progress.len(), 1);

        // Complete the task
        tasks
            .complete_task(&task_id, "Task completed successfully")
            .await
            .unwrap();
        assert_eq!(tasks.current_state.in_progress.len(), 0);
        assert_eq!(tasks.current_state.completed.len(), 1);
    }

    #[tokio::test]
    async fn test_task_completion_rate() {
        let mut state = TasksState::default();

        // Add some tasks
        state.add_task(AgentTask::new("Task 1".to_string()));
        state.add_task(AgentTask::new("Task 2".to_string()));

        let task_id = state.pending[0].id.clone();
        state.complete_task(&task_id, "Done").unwrap();

        assert_eq!(state.calculate_completion_rate(), 0.5);
    }

    #[tokio::test]
    async fn test_task_blocking() {
        let mut tasks = TasksEvolution::new("test_agent".to_string());

        let task = AgentTask::new("Blocking test".to_string());
        let task_id = task.id.clone();

        tasks.add_task(task).await.unwrap();
        tasks.start_task(&task_id).await.unwrap();
        tasks
            .block_task(&task_id, "Waiting for dependency".to_string())
            .await
            .unwrap();

        assert_eq!(tasks.current_state.blocked.len(), 1);
        assert_eq!(tasks.current_state.in_progress.len(), 0);
    }
}
