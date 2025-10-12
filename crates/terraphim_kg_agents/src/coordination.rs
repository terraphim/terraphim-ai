//! Knowledge graph-based coordination agent implementation
//!
//! This module provides a specialized GenAgent implementation for supervising
//! and coordinating multiple agents in complex workflows.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};

use terraphim_agent_registry::{
    AgentMetadata, KnowledgeGraphAgentMatcher, TerraphimKnowledgeGraphMatcher,
};
use terraphim_automata::Automata;
use terraphim_gen_agent::{GenAgent, GenAgentResult};
use terraphim_rolegraph::RoleGraph;
use terraphim_task_decomposition::Task;

use crate::{KgAgentError, KgAgentResult};

/// Message types for the coordination agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoordinationMessage {
    /// Start coordinating a workflow
    StartWorkflow {
        workflow_id: String,
        tasks: Vec<Task>,
        available_agents: Vec<AgentMetadata>,
    },
    /// Monitor workflow progress
    MonitorWorkflow { workflow_id: String },
    /// Handle agent failure
    HandleAgentFailure {
        agent_id: String,
        workflow_id: String,
    },
    /// Reassign task to different agent
    ReassignTask {
        task_id: String,
        new_agent_id: String,
    },
    /// Get workflow status
    GetWorkflowStatus { workflow_id: String },
    /// Cancel workflow
    CancelWorkflow { workflow_id: String },
    /// Update agent availability
    UpdateAgentAvailability { agent_id: String, available: bool },
}

/// Coordination agent state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinationState {
    /// Active workflows
    pub active_workflows: HashMap<String, WorkflowExecution>,
    /// Agent availability tracking
    pub agent_availability: HashMap<String, AgentAvailability>,
    /// Coordination statistics
    pub stats: CoordinationStats,
    /// Configuration
    pub config: CoordinationConfig,
}

/// Workflow execution state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecution {
    /// Workflow identifier
    pub workflow_id: String,
    /// Original tasks
    pub tasks: Vec<Task>,
    /// Task assignments
    pub task_assignments: HashMap<String, String>, // task_id -> agent_id
    /// Task execution status
    pub task_status: HashMap<String, TaskExecutionStatus>,
    /// Workflow start time
    pub start_time: std::time::SystemTime,
    /// Workflow status
    pub status: WorkflowStatus,
    /// Progress percentage
    pub progress: f64,
    /// Issues encountered
    pub issues: Vec<WorkflowIssue>,
}

/// Task execution status within a workflow
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskExecutionStatus {
    Pending,
    Assigned,
    InProgress,
    Completed,
    Failed,
    Reassigned,
}

/// Workflow execution status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WorkflowStatus {
    Planning,
    Executing,
    Completed,
    Failed,
    Cancelled,
}

/// Workflow issue tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowIssue {
    /// Issue identifier
    pub issue_id: String,
    /// Issue type
    pub issue_type: IssueType,
    /// Description
    pub description: String,
    /// Affected task or agent
    pub affected_entity: String,
    /// Timestamp
    pub timestamp: std::time::SystemTime,
    /// Resolution status
    pub resolved: bool,
}

/// Types of workflow issues
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IssueType {
    AgentFailure,
    TaskTimeout,
    DependencyViolation,
    ResourceConstraint,
    QualityIssue,
}

/// Agent availability information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentAvailability {
    /// Agent identifier
    pub agent_id: String,
    /// Current availability status
    pub available: bool,
    /// Current workload (number of assigned tasks)
    pub current_workload: u32,
    /// Maximum capacity
    pub max_capacity: u32,
    /// Last seen timestamp
    pub last_seen: std::time::SystemTime,
    /// Performance metrics
    pub performance: AgentPerformance,
}

/// Agent performance tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPerformance {
    /// Success rate
    pub success_rate: f64,
    /// Average response time
    pub avg_response_time: std::time::Duration,
    /// Reliability score
    pub reliability_score: f64,
    /// Tasks completed
    pub tasks_completed: u64,
}

impl Default for AgentPerformance {
    fn default() -> Self {
        Self {
            success_rate: 1.0,
            avg_response_time: std::time::Duration::from_secs(60),
            reliability_score: 1.0,
            tasks_completed: 0,
        }
    }
}

/// Coordination statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinationStats {
    /// Total workflows coordinated
    pub total_workflows: u64,
    /// Successful workflows
    pub successful_workflows: u64,
    /// Average workflow completion time
    pub avg_completion_time: std::time::Duration,
    /// Agent utilization rates
    pub agent_utilization: HashMap<String, f64>,
    /// Issue resolution rate
    pub issue_resolution_rate: f64,
}

impl Default for CoordinationStats {
    fn default() -> Self {
        Self {
            total_workflows: 0,
            successful_workflows: 0,
            avg_completion_time: std::time::Duration::ZERO,
            agent_utilization: HashMap::new(),
            issue_resolution_rate: 1.0,
        }
    }
}

/// Coordination agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinationConfig {
    /// Maximum concurrent workflows
    pub max_concurrent_workflows: usize,
    /// Workflow monitoring interval
    pub monitoring_interval: std::time::Duration,
    /// Task timeout threshold
    pub task_timeout: std::time::Duration,
    /// Agent failure detection timeout
    pub agent_failure_timeout: std::time::Duration,
    /// Enable automatic task reassignment
    pub enable_auto_reassignment: bool,
    /// Maximum reassignment attempts
    pub max_reassignment_attempts: u32,
}

impl Default for CoordinationConfig {
    fn default() -> Self {
        Self {
            max_concurrent_workflows: 10,
            monitoring_interval: std::time::Duration::from_secs(30),
            task_timeout: std::time::Duration::from_secs(300),
            agent_failure_timeout: std::time::Duration::from_secs(60),
            enable_auto_reassignment: true,
            max_reassignment_attempts: 3,
        }
    }
}

impl Default for CoordinationState {
    fn default() -> Self {
        Self {
            active_workflows: HashMap::new(),
            agent_availability: HashMap::new(),
            stats: CoordinationStats::default(),
            config: CoordinationConfig::default(),
        }
    }
}

/// Knowledge graph-based coordination agent
pub struct KnowledgeGraphCoordinationAgent {
    /// Agent identifier
    agent_id: String,
    /// Agent matcher for task assignment
    agent_matcher: Arc<TerraphimKnowledgeGraphMatcher>,
    /// Agent state
    state: CoordinationState,
}

impl KnowledgeGraphCoordinationAgent {
    /// Create a new coordination agent
    pub fn new(
        agent_id: String,
        automata: Arc<Automata>,
        role_graphs: HashMap<String, Arc<RoleGraph>>,
        config: CoordinationConfig,
    ) -> Self {
        let agent_matcher = Arc::new(TerraphimKnowledgeGraphMatcher::with_default_config(
            automata,
            role_graphs,
        ));

        let state = CoordinationState {
            active_workflows: HashMap::new(),
            agent_availability: HashMap::new(),
            stats: CoordinationStats::default(),
            config,
        };

        Self {
            agent_id,
            agent_matcher,
            state,
        }
    }

    /// Start coordinating a workflow
    async fn start_workflow(
        &mut self,
        workflow_id: String,
        tasks: Vec<Task>,
        available_agents: Vec<AgentMetadata>,
    ) -> KgAgentResult<WorkflowExecution> {
        info!("Starting workflow coordination: {}", workflow_id);

        if self.state.active_workflows.len() >= self.state.config.max_concurrent_workflows {
            return Err(KgAgentError::CoordinationFailed(
                "Maximum concurrent workflows reached".to_string(),
            ));
        }

        // Initialize agent availability tracking
        for agent in &available_agents {
            self.state.agent_availability.insert(
                agent.agent_id.to_string(),
                AgentAvailability {
                    agent_id: agent.agent_id.to_string(),
                    available: true,
                    current_workload: 0,
                    max_capacity: 5, // Default capacity
                    last_seen: std::time::SystemTime::now(),
                    performance: AgentPerformance::default(),
                },
            );
        }

        // Create initial workflow execution state
        let mut workflow = WorkflowExecution {
            workflow_id: workflow_id.clone(),
            tasks: tasks.clone(),
            task_assignments: HashMap::new(),
            task_status: HashMap::new(),
            start_time: std::time::SystemTime::now(),
            status: WorkflowStatus::Planning,
            progress: 0.0,
            issues: Vec::new(),
        };

        // Initialize task status
        for task in &tasks {
            workflow
                .task_status
                .insert(task.task_id.clone(), TaskExecutionStatus::Pending);
        }

        // Assign tasks to agents using knowledge graph matching
        let coordination_result = self
            .agent_matcher
            .coordinate_workflow(&tasks, &available_agents)
            .await
            .map_err(|e| KgAgentError::CoordinationFailed(e.to_string()))?;

        // Update task assignments based on coordination result
        for step in &coordination_result.steps {
            workflow
                .task_assignments
                .insert(step.step_id.clone(), step.assigned_agent.clone());
            workflow
                .task_status
                .insert(step.step_id.clone(), TaskExecutionStatus::Assigned);

            // Update agent workload
            if let Some(availability) = self.state.agent_availability.get_mut(&step.assigned_agent)
            {
                availability.current_workload += 1;
            }
        }

        workflow.status = WorkflowStatus::Executing;
        self.state
            .active_workflows
            .insert(workflow_id.clone(), workflow.clone());
        self.state.stats.total_workflows += 1;

        info!(
            "Workflow {} started with {} tasks assigned to {} agents",
            workflow_id,
            tasks.len(),
            coordination_result.agent_assignments.len()
        );

        Ok(workflow)
    }

    /// Monitor workflow progress
    async fn monitor_workflow(&mut self, workflow_id: &str) -> KgAgentResult<WorkflowExecution> {
        debug!("Monitoring workflow: {}", workflow_id);

        let workflow = self
            .state
            .active_workflows
            .get_mut(workflow_id)
            .ok_or_else(|| {
                KgAgentError::CoordinationFailed(format!("Workflow {} not found", workflow_id))
            })?;

        // Check for task timeouts
        let now = std::time::SystemTime::now();
        let timeout_threshold = self.state.config.task_timeout;

        for (task_id, status) in &mut workflow.task_status {
            if *status == TaskExecutionStatus::InProgress {
                let elapsed = now.duration_since(workflow.start_time).unwrap_or_default();
                if elapsed > timeout_threshold {
                    *status = TaskExecutionStatus::Failed;
                    workflow.issues.push(WorkflowIssue {
                        issue_id: format!("timeout_{}", uuid::Uuid::new_v4()),
                        issue_type: IssueType::TaskTimeout,
                        description: format!(
                            "Task {} timed out after {:.2}s",
                            task_id,
                            elapsed.as_secs_f64()
                        ),
                        affected_entity: task_id.clone(),
                        timestamp: now,
                        resolved: false,
                    });
                }
            }
        }

        // Check agent availability
        for (agent_id, availability) in &mut self.state.agent_availability {
            let elapsed = now
                .duration_since(availability.last_seen)
                .unwrap_or_default();
            if elapsed > self.state.config.agent_failure_timeout {
                availability.available = false;
                workflow.issues.push(WorkflowIssue {
                    issue_id: format!("agent_failure_{}", uuid::Uuid::new_v4()),
                    issue_type: IssueType::AgentFailure,
                    description: format!("Agent {} appears to be unavailable", agent_id),
                    affected_entity: agent_id.clone(),
                    timestamp: now,
                    resolved: false,
                });
            }
        }

        // Update progress
        let total_tasks = workflow.task_status.len();
        let completed_tasks = workflow
            .task_status
            .values()
            .filter(|&status| *status == TaskExecutionStatus::Completed)
            .count();

        workflow.progress = if total_tasks > 0 {
            completed_tasks as f64 / total_tasks as f64
        } else {
            0.0
        };

        // Update workflow status
        if workflow.progress >= 1.0 {
            workflow.status = WorkflowStatus::Completed;
            self.state.stats.successful_workflows += 1;
        } else if workflow
            .task_status
            .values()
            .any(|status| *status == TaskExecutionStatus::Failed)
        {
            workflow.status = WorkflowStatus::Failed;
        }

        debug!(
            "Workflow {} progress: {:.1}%, status: {:?}",
            workflow_id,
            workflow.progress * 100.0,
            workflow.status
        );

        Ok(workflow.clone())
    }

    /// Handle agent failure
    async fn handle_agent_failure(
        &mut self,
        agent_id: &str,
        workflow_id: &str,
    ) -> KgAgentResult<()> {
        warn!(
            "Handling agent failure: {} in workflow {}",
            agent_id, workflow_id
        );

        // Mark agent as unavailable
        if let Some(availability) = self.state.agent_availability.get_mut(agent_id) {
            availability.available = false;
            availability.current_workload = 0;
        }

        // Find tasks assigned to the failed agent
        let workflow = self
            .state
            .active_workflows
            .get_mut(workflow_id)
            .ok_or_else(|| {
                KgAgentError::CoordinationFailed(format!("Workflow {} not found", workflow_id))
            })?;

        let failed_tasks: Vec<String> = workflow
            .task_assignments
            .iter()
            .filter(|(_, assigned_agent)| *assigned_agent == agent_id)
            .map(|(task_id, _)| task_id.clone())
            .collect();

        // Attempt to reassign tasks if auto-reassignment is enabled
        if self.state.config.enable_auto_reassignment {
            for task_id in failed_tasks {
                if let Err(e) = self.reassign_task(&task_id, workflow_id).await {
                    warn!("Failed to reassign task {}: {}", task_id, e);
                    workflow
                        .task_status
                        .insert(task_id, TaskExecutionStatus::Failed);
                }
            }
        }

        Ok(())
    }

    /// Reassign a task to a different agent
    async fn reassign_task(&mut self, task_id: &str, workflow_id: &str) -> KgAgentResult<String> {
        debug!("Reassigning task {} in workflow {}", task_id, workflow_id);

        let workflow = self
            .state
            .active_workflows
            .get_mut(workflow_id)
            .ok_or_else(|| {
                KgAgentError::CoordinationFailed(format!("Workflow {} not found", workflow_id))
            })?;

        // Find the task
        let task = workflow
            .tasks
            .iter()
            .find(|t| t.task_id == task_id)
            .ok_or_else(|| {
                KgAgentError::CoordinationFailed(format!(
                    "Task {} not found in workflow {}",
                    task_id, workflow_id
                ))
            })?;

        // Find available agents
        let available_agents: Vec<AgentMetadata> = self
            .state
            .agent_availability
            .values()
            .filter(|a| a.available && a.current_workload < a.max_capacity)
            .map(|a| {
                // Create a minimal AgentMetadata for matching
                // In a real implementation, this would come from the agent registry
                let agent_id = crate::AgentPid::from_string(a.agent_id.clone());
                let supervisor_id = crate::SupervisorId::new();
                let role = terraphim_agent_registry::AgentRole::new(
                    "worker".to_string(),
                    "Worker Agent".to_string(),
                    "General purpose worker".to_string(),
                );
                AgentMetadata::new(agent_id, supervisor_id, role)
            })
            .collect();

        if available_agents.is_empty() {
            return Err(KgAgentError::CoordinationFailed(
                "No available agents for task reassignment".to_string(),
            ));
        }

        // Use agent matcher to find the best agent
        let matches = self
            .agent_matcher
            .match_task_to_agents(task, &available_agents, 1)
            .await
            .map_err(|e| KgAgentError::CoordinationFailed(e.to_string()))?;

        let best_match = matches.first().ok_or_else(|| {
            KgAgentError::CoordinationFailed("No suitable agent found for reassignment".to_string())
        })?;

        let new_agent_id = best_match.agent.agent_id.to_string();

        // Update task assignment
        workflow
            .task_assignments
            .insert(task_id.to_string(), new_agent_id.clone());
        workflow
            .task_status
            .insert(task_id.to_string(), TaskExecutionStatus::Reassigned);

        // Update agent workload
        if let Some(availability) = self.state.agent_availability.get_mut(&new_agent_id) {
            availability.current_workload += 1;
        }

        info!("Task {} reassigned to agent {}", task_id, new_agent_id);
        Ok(new_agent_id)
    }

    /// Update agent availability
    fn update_agent_availability(&mut self, agent_id: &str, available: bool) {
        if let Some(availability) = self.state.agent_availability.get_mut(agent_id) {
            availability.available = available;
            availability.last_seen = std::time::SystemTime::now();
            if !available {
                availability.current_workload = 0;
            }
        }
    }

    /// Cancel a workflow
    async fn cancel_workflow(&mut self, workflow_id: &str) -> KgAgentResult<()> {
        info!("Cancelling workflow: {}", workflow_id);

        let workflow = self
            .state
            .active_workflows
            .get_mut(workflow_id)
            .ok_or_else(|| {
                KgAgentError::CoordinationFailed(format!("Workflow {} not found", workflow_id))
            })?;

        workflow.status = WorkflowStatus::Cancelled;

        // Free up agent resources
        for (_, agent_id) in &workflow.task_assignments {
            if let Some(availability) = self.state.agent_availability.get_mut(agent_id) {
                availability.current_workload = availability.current_workload.saturating_sub(1);
            }
        }

        Ok(())
    }
}

#[async_trait]
impl GenAgent<CoordinationState> for KnowledgeGraphCoordinationAgent {
    type Message = CoordinationMessage;

    async fn init(&mut self, _init_args: serde_json::Value) -> GenAgentResult<()> {
        info!("Initializing coordination agent: {}", self.agent_id);
        Ok(())
    }

    async fn handle_call(&mut self, message: Self::Message) -> GenAgentResult<serde_json::Value> {
        match message {
            CoordinationMessage::StartWorkflow {
                workflow_id,
                tasks,
                available_agents,
            } => {
                let workflow = self
                    .start_workflow(workflow_id, tasks, available_agents)
                    .await
                    .map_err(|e| {
                        terraphim_gen_agent::GenAgentError::ExecutionError(
                            self.agent_id.clone(),
                            e.to_string(),
                        )
                    })?;
                Ok(serde_json::to_value(workflow).unwrap())
            }
            CoordinationMessage::MonitorWorkflow { workflow_id } => {
                let workflow = self.monitor_workflow(&workflow_id).await.map_err(|e| {
                    terraphim_gen_agent::GenAgentError::ExecutionError(
                        self.agent_id.clone(),
                        e.to_string(),
                    )
                })?;
                Ok(serde_json::to_value(workflow).unwrap())
            }
            CoordinationMessage::GetWorkflowStatus { workflow_id } => {
                let workflow = self
                    .state
                    .active_workflows
                    .get(&workflow_id)
                    .ok_or_else(|| {
                        terraphim_gen_agent::GenAgentError::ExecutionError(
                            self.agent_id.clone(),
                            format!("Workflow {} not found", workflow_id),
                        )
                    })?;
                Ok(serde_json::to_value(&workflow.status).unwrap())
            }
            CoordinationMessage::ReassignTask {
                task_id,
                new_agent_id,
            } => {
                // Find workflow containing the task
                let workflow_id = self
                    .state
                    .active_workflows
                    .iter()
                    .find(|(_, workflow)| workflow.task_assignments.contains_key(&task_id))
                    .map(|(id, _)| id.clone())
                    .ok_or_else(|| {
                        terraphim_gen_agent::GenAgentError::ExecutionError(
                            self.agent_id.clone(),
                            format!("Task {} not found in any workflow", task_id),
                        )
                    })?;

                let assigned_agent =
                    self.reassign_task(&task_id, &workflow_id)
                        .await
                        .map_err(|e| {
                            terraphim_gen_agent::GenAgentError::ExecutionError(
                                self.agent_id.clone(),
                                e.to_string(),
                            )
                        })?;
                Ok(serde_json::to_value(assigned_agent).unwrap())
            }
            _ => {
                // Other messages don't return values in call context
                Ok(serde_json::Value::Null)
            }
        }
    }

    async fn handle_cast(&mut self, message: Self::Message) -> GenAgentResult<()> {
        match message {
            CoordinationMessage::HandleAgentFailure {
                agent_id,
                workflow_id,
            } => {
                let _ = self.handle_agent_failure(&agent_id, &workflow_id).await;
            }
            CoordinationMessage::UpdateAgentAvailability {
                agent_id,
                available,
            } => {
                self.update_agent_availability(&agent_id, available);
            }
            CoordinationMessage::CancelWorkflow { workflow_id } => {
                let _ = self.cancel_workflow(&workflow_id).await;
            }
            _ => {
                // Other messages handled in call context
            }
        }
        Ok(())
    }

    async fn handle_info(&mut self, _message: serde_json::Value) -> GenAgentResult<()> {
        // Handle periodic monitoring, health checks, etc.
        Ok(())
    }

    async fn terminate(&mut self, _reason: String) -> GenAgentResult<()> {
        info!("Terminating coordination agent: {}", self.agent_id);
        // Cancel all active workflows
        let workflow_ids: Vec<String> = self.state.active_workflows.keys().cloned().collect();
        for workflow_id in workflow_ids {
            let _ = self.cancel_workflow(&workflow_id).await;
        }
        Ok(())
    }

    fn get_state(&self) -> &CoordinationState {
        &self.state
    }

    fn get_state_mut(&mut self) -> &mut CoordinationState {
        &mut self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use terraphim_task_decomposition::TaskComplexity;

    fn create_test_task() -> Task {
        Task::new(
            "test_task".to_string(),
            "Test task for coordination".to_string(),
            TaskComplexity::Simple,
            1,
        )
    }

    fn create_test_agent_metadata() -> AgentMetadata {
        let agent_id = crate::AgentPid::new();
        let supervisor_id = crate::SupervisorId::new();
        let role = terraphim_agent_registry::AgentRole::new(
            "worker".to_string(),
            "Test Worker".to_string(),
            "Test worker agent".to_string(),
        );
        AgentMetadata::new(agent_id, supervisor_id, role)
    }

    async fn create_test_agent() -> KnowledgeGraphCoordinationAgent {
        use terraphim_automata::{load_thesaurus, AutomataPath};
        use terraphim_types::RoleName;

        let automata = Arc::new(terraphim_automata::Automata::default());

        let role_name = RoleName::new("coordinator");
        let thesaurus = load_thesaurus(&AutomataPath::local_example())
            .await
            .unwrap();
        let role_graph = Arc::new(RoleGraph::new(role_name, thesaurus).await.unwrap());

        let mut role_graphs = HashMap::new();
        role_graphs.insert("coordinator".to_string(), role_graph);

        KnowledgeGraphCoordinationAgent::new(
            "test_coordinator".to_string(),
            automata,
            role_graphs,
            CoordinationConfig::default(),
        )
    }

    #[tokio::test]
    async fn test_coordination_agent_creation() {
        let agent = create_test_agent().await;
        assert_eq!(agent.agent_id, "test_coordinator");
        assert_eq!(agent.state.active_workflows.len(), 0);
    }

    #[tokio::test]
    async fn test_start_workflow() {
        let mut agent = create_test_agent().await;
        let tasks = vec![create_test_task()];
        let agents = vec![create_test_agent_metadata()];

        let result = agent
            .start_workflow("test_workflow".to_string(), tasks, agents)
            .await;

        assert!(result.is_ok());
        let workflow = result.unwrap();
        assert_eq!(workflow.workflow_id, "test_workflow");
        assert_eq!(workflow.status, WorkflowStatus::Executing);
    }

    #[tokio::test]
    async fn test_monitor_workflow() {
        let mut agent = create_test_agent().await;
        let tasks = vec![create_test_task()];
        let agents = vec![create_test_agent_metadata()];

        let workflow = agent
            .start_workflow("test_workflow".to_string(), tasks, agents)
            .await
            .unwrap();

        let monitored = agent.monitor_workflow(&workflow.workflow_id).await.unwrap();
        assert_eq!(monitored.workflow_id, workflow.workflow_id);
    }

    #[tokio::test]
    async fn test_agent_availability_update() {
        let mut agent = create_test_agent().await;
        let agent_metadata = create_test_agent_metadata();
        let agent_id = agent_metadata.agent_id.to_string();

        // Initialize agent availability
        agent.state.agent_availability.insert(
            agent_id.clone(),
            AgentAvailability {
                agent_id: agent_id.clone(),
                available: true,
                current_workload: 0,
                max_capacity: 5,
                last_seen: std::time::SystemTime::now(),
                performance: AgentPerformance::default(),
            },
        );

        agent.update_agent_availability(&agent_id, false);
        assert!(!agent.state.agent_availability[&agent_id].available);
    }

    #[tokio::test]
    async fn test_gen_agent_interface() {
        let mut agent = create_test_agent().await;

        // Test initialization
        let init_result = agent.init(serde_json::json!({})).await;
        assert!(init_result.is_ok());

        // Test cast message
        let message = CoordinationMessage::UpdateAgentAvailability {
            agent_id: "test_agent".to_string(),
            available: true,
        };
        let cast_result = agent.handle_cast(message).await;
        assert!(cast_result.is_ok());

        // Test termination
        let terminate_result = agent.terminate("test".to_string()).await;
        assert!(terminate_result.is_ok());
    }
}
