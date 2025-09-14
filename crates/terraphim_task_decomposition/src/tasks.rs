//! Task representation and management
//!
//! Provides core task structures and management functionality for the task decomposition system.

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{AgentPid, GoalId, TaskDecompositionError, TaskDecompositionResult};

/// Task identifier type
pub type TaskId = String;

/// Task representation with knowledge graph context
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Task {
    /// Unique task identifier
    pub task_id: TaskId,
    /// Human-readable task description
    pub description: String,
    /// Task complexity level
    pub complexity: TaskComplexity,
    /// Required capabilities for task execution
    pub required_capabilities: Vec<String>,
    /// Knowledge graph context for the task
    pub knowledge_context: TaskKnowledgeContext,
    /// Task constraints and requirements
    pub constraints: Vec<TaskConstraint>,
    /// Dependencies on other tasks
    pub dependencies: Vec<TaskId>,
    /// Estimated effort required
    pub estimated_effort: Duration,
    /// Task priority (higher number = higher priority)
    pub priority: u32,
    /// Current task status
    pub status: TaskStatus,
    /// Task metadata and tracking
    pub metadata: TaskMetadata,
    /// Parent goal this task contributes to
    pub parent_goal: Option<GoalId>,
    /// Agents assigned to this task
    pub assigned_agents: Vec<AgentPid>,
    /// Subtasks (if this task has been decomposed)
    pub subtasks: Vec<TaskId>,
}

/// Task complexity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TaskComplexity {
    /// Simple, single-step tasks
    Simple,
    /// Multi-step tasks with clear sequence
    Moderate,
    /// Complex tasks requiring decomposition
    Complex,
    /// Highly complex tasks requiring sophisticated planning
    VeryComplex,
}

impl TaskComplexity {
    /// Get numeric complexity score
    pub fn score(&self) -> u32 {
        match self {
            TaskComplexity::Simple => 1,
            TaskComplexity::Moderate => 2,
            TaskComplexity::Complex => 3,
            TaskComplexity::VeryComplex => 4,
        }
    }

    /// Check if task requires decomposition
    pub fn requires_decomposition(&self) -> bool {
        matches!(self, TaskComplexity::Complex | TaskComplexity::VeryComplex)
    }

    /// Get recommended decomposition depth
    pub fn recommended_depth(&self) -> u32 {
        match self {
            TaskComplexity::Simple => 0,
            TaskComplexity::Moderate => 1,
            TaskComplexity::Complex => 2,
            TaskComplexity::VeryComplex => 3,
        }
    }
}

/// Knowledge graph context for tasks
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskKnowledgeContext {
    /// Knowledge domains this task operates in
    pub domains: Vec<String>,
    /// Ontology concepts related to this task
    pub concepts: Vec<String>,
    /// Relationships this task involves
    pub relationships: Vec<String>,
    /// Keywords for semantic matching
    pub keywords: Vec<String>,
    /// Input types this task expects
    pub input_types: Vec<String>,
    /// Output types this task produces
    pub output_types: Vec<String>,
    /// Semantic similarity thresholds
    pub similarity_thresholds: HashMap<String, f64>,
}

impl Default for TaskKnowledgeContext {
    fn default() -> Self {
        Self {
            domains: Vec::new(),
            concepts: Vec::new(),
            relationships: Vec::new(),
            keywords: Vec::new(),
            input_types: Vec::new(),
            output_types: Vec::new(),
            similarity_thresholds: HashMap::new(),
        }
    }
}

/// Task constraints and requirements
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskConstraint {
    /// Constraint type
    pub constraint_type: TaskConstraintType,
    /// Constraint description
    pub description: String,
    /// Constraint parameters
    pub parameters: HashMap<String, serde_json::Value>,
    /// Whether this constraint is hard (must be satisfied) or soft (preferred)
    pub is_hard: bool,
    /// Constraint priority for conflict resolution
    pub priority: u32,
}

/// Types of task constraints
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskConstraintType {
    /// Time-based constraints
    Temporal,
    /// Resource constraints
    Resource,
    /// Quality constraints
    Quality,
    /// Security constraints
    Security,
    /// Performance constraints
    Performance,
    /// Dependency constraints
    Dependency,
    /// Custom constraint type
    Custom(String),
}

/// Task execution status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskStatus {
    /// Task is defined but not yet started
    Pending,
    /// Task is ready to be executed (dependencies met)
    Ready,
    /// Task is currently being executed
    InProgress,
    /// Task is paused or waiting
    Paused,
    /// Task has been completed successfully
    Completed,
    /// Task has failed
    Failed(String),
    /// Task has been cancelled
    Cancelled(String),
    /// Task is blocked by dependencies or constraints
    Blocked(String),
}

/// Task metadata and tracking information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskMetadata {
    /// When the task was created
    pub created_at: DateTime<Utc>,
    /// When the task was last updated
    pub updated_at: DateTime<Utc>,
    /// Task creator/owner
    pub created_by: String,
    /// Task version for change tracking
    pub version: u32,
    /// Actual start time
    pub started_at: Option<DateTime<Utc>>,
    /// Actual completion time
    pub completed_at: Option<DateTime<Utc>>,
    /// Task progress (0.0 to 1.0)
    pub progress: f64,
    /// Success criteria
    pub success_criteria: Vec<SuccessCriterion>,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Custom metadata fields
    pub custom_fields: HashMap<String, serde_json::Value>,
}

impl Default for TaskMetadata {
    fn default() -> Self {
        Self {
            created_at: Utc::now(),
            updated_at: Utc::now(),
            created_by: "system".to_string(),
            version: 1,
            started_at: None,
            completed_at: None,
            progress: 0.0,
            success_criteria: Vec::new(),
            tags: Vec::new(),
            custom_fields: HashMap::new(),
        }
    }
}

/// Success criteria for task completion
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SuccessCriterion {
    /// Criterion description
    pub description: String,
    /// Metric to measure
    pub metric: String,
    /// Target value
    pub target_value: f64,
    /// Current value
    pub current_value: f64,
    /// Whether this criterion has been met
    pub is_met: bool,
    /// Weight of this criterion (0.0 to 1.0)
    pub weight: f64,
}

impl Task {
    /// Create a new task
    pub fn new(
        task_id: TaskId,
        description: String,
        complexity: TaskComplexity,
        priority: u32,
    ) -> Self {
        Self {
            task_id,
            description,
            complexity,
            required_capabilities: Vec::new(),
            knowledge_context: TaskKnowledgeContext::default(),
            constraints: Vec::new(),
            dependencies: Vec::new(),
            estimated_effort: Duration::from_secs(3600), // 1 hour default
            priority,
            status: TaskStatus::Pending,
            metadata: TaskMetadata::default(),
            parent_goal: None,
            assigned_agents: Vec::new(),
            subtasks: Vec::new(),
        }
    }

    /// Add a constraint to the task
    pub fn add_constraint(&mut self, constraint: TaskConstraint) -> TaskDecompositionResult<()> {
        // Validate constraint
        self.validate_constraint(&constraint)?;
        self.constraints.push(constraint);
        self.metadata.updated_at = Utc::now();
        self.metadata.version += 1;
        Ok(())
    }

    /// Add a dependency to the task
    pub fn add_dependency(&mut self, dependency_task_id: TaskId) -> TaskDecompositionResult<()> {
        if dependency_task_id == self.task_id {
            return Err(TaskDecompositionError::DependencyCycle(format!(
                "Task {} cannot depend on itself",
                self.task_id
            )));
        }

        if !self.dependencies.contains(&dependency_task_id) {
            self.dependencies.push(dependency_task_id);
            self.metadata.updated_at = Utc::now();
            self.metadata.version += 1;
        }

        Ok(())
    }

    /// Assign an agent to the task
    pub fn assign_agent(&mut self, agent_id: AgentPid) -> TaskDecompositionResult<()> {
        if !self.assigned_agents.contains(&agent_id) {
            self.assigned_agents.push(agent_id);
            self.metadata.updated_at = Utc::now();
            self.metadata.version += 1;
        }
        Ok(())
    }

    /// Remove an agent from the task
    pub fn unassign_agent(&mut self, agent_id: &AgentPid) -> TaskDecompositionResult<()> {
        self.assigned_agents.retain(|id| id != agent_id);
        self.metadata.updated_at = Utc::now();
        self.metadata.version += 1;
        Ok(())
    }

    /// Update task status
    pub fn update_status(&mut self, status: TaskStatus) -> TaskDecompositionResult<()> {
        let old_status = self.status.clone();
        self.status = status;
        self.metadata.updated_at = Utc::now();
        self.metadata.version += 1;

        // Update timestamps based on status changes
        match (&old_status, &self.status) {
            (TaskStatus::Pending | TaskStatus::Ready, TaskStatus::InProgress) => {
                self.metadata.started_at = Some(Utc::now());
            }
            (_, TaskStatus::Completed) => {
                self.metadata.completed_at = Some(Utc::now());
                self.metadata.progress = 1.0;
            }
            (_, TaskStatus::Failed(_)) | (_, TaskStatus::Cancelled(_)) => {
                self.metadata.completed_at = Some(Utc::now());
            }
            _ => {}
        }

        Ok(())
    }

    /// Update task progress
    pub fn update_progress(&mut self, progress: f64) -> TaskDecompositionResult<()> {
        if !(0.0..=1.0).contains(&progress) {
            return Err(TaskDecompositionError::InvalidTaskSpec(
                self.task_id.clone(),
                "Progress must be between 0.0 and 1.0".to_string(),
            ));
        }

        self.metadata.progress = progress;
        self.metadata.updated_at = Utc::now();
        self.metadata.version += 1;

        // Auto-complete if progress reaches 100%
        if progress >= 1.0 && !matches!(self.status, TaskStatus::Completed) {
            self.update_status(TaskStatus::Completed)?;
        }

        Ok(())
    }

    /// Add a subtask
    pub fn add_subtask(&mut self, subtask_id: TaskId) -> TaskDecompositionResult<()> {
        if !self.subtasks.contains(&subtask_id) {
            self.subtasks.push(subtask_id);
            self.metadata.updated_at = Utc::now();
            self.metadata.version += 1;
        }
        Ok(())
    }

    /// Check if task can be started (all dependencies met)
    pub fn can_start(&self, completed_tasks: &HashSet<TaskId>) -> bool {
        self.dependencies
            .iter()
            .all(|dep| completed_tasks.contains(dep))
    }

    /// Check if task is ready for execution
    pub fn is_ready(&self) -> bool {
        matches!(self.status, TaskStatus::Ready)
    }

    /// Check if task is in progress
    pub fn is_in_progress(&self) -> bool {
        matches!(self.status, TaskStatus::InProgress)
    }

    /// Check if task is completed
    pub fn is_completed(&self) -> bool {
        matches!(self.status, TaskStatus::Completed)
    }

    /// Check if task has failed
    pub fn has_failed(&self) -> bool {
        matches!(self.status, TaskStatus::Failed(_))
    }

    /// Check if task is blocked
    pub fn is_blocked(&self) -> bool {
        matches!(self.status, TaskStatus::Blocked(_))
    }

    /// Get task duration if completed
    pub fn get_duration(&self) -> Option<chrono::Duration> {
        if let (Some(started), Some(completed)) =
            (self.metadata.started_at, self.metadata.completed_at)
        {
            Some(completed - started)
        } else {
            None
        }
    }

    /// Calculate overall success score based on criteria
    pub fn calculate_success_score(&self) -> f64 {
        if self.metadata.success_criteria.is_empty() {
            return if self.is_completed() { 1.0 } else { 0.0 };
        }

        let total_weight: f64 = self
            .metadata
            .success_criteria
            .iter()
            .map(|c| c.weight)
            .sum();
        if total_weight == 0.0 {
            return 0.0;
        }

        let weighted_score: f64 = self
            .metadata
            .success_criteria
            .iter()
            .map(|criterion| {
                let score = if criterion.is_met {
                    1.0
                } else {
                    (criterion.current_value / criterion.target_value)
                        .min(1.0)
                        .max(0.0)
                };
                score * criterion.weight
            })
            .sum();

        weighted_score / total_weight
    }

    /// Validate the task
    pub fn validate(&self) -> TaskDecompositionResult<()> {
        if self.task_id.is_empty() {
            return Err(TaskDecompositionError::InvalidTaskSpec(
                self.task_id.clone(),
                "Task ID cannot be empty".to_string(),
            ));
        }

        if self.description.is_empty() {
            return Err(TaskDecompositionError::InvalidTaskSpec(
                self.task_id.clone(),
                "Task description cannot be empty".to_string(),
            ));
        }

        if !(0.0..=1.0).contains(&self.metadata.progress) {
            return Err(TaskDecompositionError::InvalidTaskSpec(
                self.task_id.clone(),
                "Progress must be between 0.0 and 1.0".to_string(),
            ));
        }

        // Validate constraints
        for constraint in &self.constraints {
            self.validate_constraint(constraint)?;
        }

        // Validate success criteria weights
        let total_weight: f64 = self
            .metadata
            .success_criteria
            .iter()
            .map(|c| c.weight)
            .sum();
        if !self.metadata.success_criteria.is_empty() && (total_weight - 1.0).abs() > 0.01 {
            return Err(TaskDecompositionError::InvalidTaskSpec(
                self.task_id.clone(),
                "Success criteria weights must sum to 1.0".to_string(),
            ));
        }

        Ok(())
    }

    /// Validate a constraint
    fn validate_constraint(&self, constraint: &TaskConstraint) -> TaskDecompositionResult<()> {
        if constraint.description.is_empty() {
            return Err(TaskDecompositionError::InvalidTaskSpec(
                self.task_id.clone(),
                "Constraint description cannot be empty".to_string(),
            ));
        }

        // Add constraint-specific validation based on type
        match &constraint.constraint_type {
            TaskConstraintType::Temporal => {
                // Validate temporal constraint parameters
                if !constraint.parameters.contains_key("deadline")
                    && !constraint.parameters.contains_key("duration")
                {
                    return Err(TaskDecompositionError::InvalidTaskSpec(
                        self.task_id.clone(),
                        "Temporal constraints must have deadline or duration parameter".to_string(),
                    ));
                }
            }
            TaskConstraintType::Resource => {
                // Validate resource constraint parameters
                if !constraint.parameters.contains_key("resource_type") {
                    return Err(TaskDecompositionError::InvalidTaskSpec(
                        self.task_id.clone(),
                        "Resource constraints must specify resource_type".to_string(),
                    ));
                }
            }
            _ => {
                // Basic validation for other constraint types
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation() {
        let task = Task::new(
            "test_task".to_string(),
            "Test task description".to_string(),
            TaskComplexity::Simple,
            1,
        );

        assert_eq!(task.task_id, "test_task");
        assert_eq!(task.description, "Test task description");
        assert_eq!(task.complexity, TaskComplexity::Simple);
        assert_eq!(task.priority, 1);
        assert_eq!(task.status, TaskStatus::Pending);
        assert!(task.dependencies.is_empty());
        assert!(task.assigned_agents.is_empty());
        assert!(task.subtasks.is_empty());
    }

    #[test]
    fn test_task_complexity_scoring() {
        assert_eq!(TaskComplexity::Simple.score(), 1);
        assert_eq!(TaskComplexity::Moderate.score(), 2);
        assert_eq!(TaskComplexity::Complex.score(), 3);
        assert_eq!(TaskComplexity::VeryComplex.score(), 4);
    }

    #[test]
    fn test_task_complexity_decomposition_requirements() {
        assert!(!TaskComplexity::Simple.requires_decomposition());
        assert!(!TaskComplexity::Moderate.requires_decomposition());
        assert!(TaskComplexity::Complex.requires_decomposition());
        assert!(TaskComplexity::VeryComplex.requires_decomposition());
    }

    #[test]
    fn test_task_dependency_management() {
        let mut task = Task::new(
            "test_task".to_string(),
            "Test task".to_string(),
            TaskComplexity::Simple,
            1,
        );

        // Add dependency
        assert!(task.add_dependency("dep_task".to_string()).is_ok());
        assert_eq!(task.dependencies.len(), 1);
        assert!(task.dependencies.contains(&"dep_task".to_string()));

        // Try to add self-dependency (should fail)
        assert!(task.add_dependency("test_task".to_string()).is_err());

        // Add duplicate dependency (should not duplicate)
        assert!(task.add_dependency("dep_task".to_string()).is_ok());
        assert_eq!(task.dependencies.len(), 1);
    }

    #[test]
    fn test_task_agent_assignment() {
        let mut task = Task::new(
            "test_task".to_string(),
            "Test task".to_string(),
            TaskComplexity::Simple,
            1,
        );

        let agent_id: AgentPid = "test_agent".to_string();

        // Assign agent
        assert!(task.assign_agent(agent_id.clone()).is_ok());
        assert_eq!(task.assigned_agents.len(), 1);
        assert!(task.assigned_agents.contains(&agent_id));

        // Assign same agent again (should not duplicate)
        assert!(task.assign_agent(agent_id.clone()).is_ok());
        assert_eq!(task.assigned_agents.len(), 1);

        // Unassign agent
        assert!(task.unassign_agent(&agent_id).is_ok());
        assert!(task.assigned_agents.is_empty());
    }

    #[test]
    fn test_task_status_updates() {
        let mut task = Task::new(
            "test_task".to_string(),
            "Test task".to_string(),
            TaskComplexity::Simple,
            1,
        );

        // Update to in progress
        assert!(task.update_status(TaskStatus::InProgress).is_ok());
        assert!(task.is_in_progress());
        assert!(task.metadata.started_at.is_some());

        // Update to completed
        assert!(task.update_status(TaskStatus::Completed).is_ok());
        assert!(task.is_completed());
        assert!(task.metadata.completed_at.is_some());
        assert_eq!(task.metadata.progress, 1.0);
    }

    #[test]
    fn test_task_progress_updates() {
        let mut task = Task::new(
            "test_task".to_string(),
            "Test task".to_string(),
            TaskComplexity::Simple,
            1,
        );

        // Update progress
        assert!(task.update_progress(0.5).is_ok());
        assert_eq!(task.metadata.progress, 0.5);

        // Invalid progress (should fail)
        assert!(task.update_progress(1.5).is_err());
        assert!(task.update_progress(-0.1).is_err());

        // Complete via progress
        assert!(task.update_progress(1.0).is_ok());
        assert!(task.is_completed());
    }

    #[test]
    fn test_task_readiness_check() {
        let mut task = Task::new(
            "test_task".to_string(),
            "Test task".to_string(),
            TaskComplexity::Simple,
            1,
        );

        task.add_dependency("dep1".to_string()).unwrap();
        task.add_dependency("dep2".to_string()).unwrap();

        let mut completed_tasks = HashSet::new();

        // Not ready - dependencies not met
        assert!(!task.can_start(&completed_tasks));

        // Partially ready
        completed_tasks.insert("dep1".to_string());
        assert!(!task.can_start(&completed_tasks));

        // Ready - all dependencies met
        completed_tasks.insert("dep2".to_string());
        assert!(task.can_start(&completed_tasks));
    }

    #[test]
    fn test_task_validation() {
        let mut task = Task::new(
            "test_task".to_string(),
            "Test task".to_string(),
            TaskComplexity::Simple,
            1,
        );

        // Valid task
        assert!(task.validate().is_ok());

        // Invalid task ID
        task.task_id = "".to_string();
        assert!(task.validate().is_err());

        // Fix task ID, invalid description
        task.task_id = "test_task".to_string();
        task.description = "".to_string();
        assert!(task.validate().is_err());

        // Fix description, invalid progress
        task.description = "Test task".to_string();
        task.metadata.progress = 1.5;
        assert!(task.validate().is_err());
    }

    #[test]
    fn test_success_score_calculation() {
        let mut task = Task::new(
            "test_task".to_string(),
            "Test task".to_string(),
            TaskComplexity::Simple,
            1,
        );

        // No criteria - completed task should score 1.0
        task.update_status(TaskStatus::Completed).unwrap();
        assert_eq!(task.calculate_success_score(), 1.0);

        // Add success criteria
        task.metadata.success_criteria = vec![
            SuccessCriterion {
                description: "Quality metric".to_string(),
                metric: "quality".to_string(),
                target_value: 100.0,
                current_value: 80.0,
                is_met: false,
                weight: 0.6,
            },
            SuccessCriterion {
                description: "Performance metric".to_string(),
                metric: "performance".to_string(),
                target_value: 50.0,
                current_value: 50.0,
                is_met: true,
                weight: 0.4,
            },
        ];

        // Calculate weighted score: (0.8 * 0.6) + (1.0 * 0.4) = 0.88
        let score = task.calculate_success_score();
        assert!((score - 0.88).abs() < 0.01);
    }
}
