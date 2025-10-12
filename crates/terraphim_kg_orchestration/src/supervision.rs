//! Supervision tree orchestration engine
//!
//! This module provides a supervision tree-based orchestration engine that combines
//! Erlang/OTP-style fault tolerance with knowledge graph-guided agent coordination.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use chrono::Utc;

use async_trait::async_trait;
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, RwLock};

use terraphim_agent_supervisor::{
    AgentFactory, AgentPid, AgentSpec, AgentStatus, AgentSupervisor, ExitReason, InitArgs,
    RestartIntensity, RestartPolicy, RestartStrategy, SupervisedAgent, SupervisionResult,
    SupervisorConfig, SupervisorId, SystemMessage, TerminateReason,
};
use terraphim_task_decomposition::{
    Task, TaskDecompositionWorkflow, TerraphimTaskDecompositionSystem,
};

use crate::{
    AgentAssignment, AgentPool, ExecutionCoordinator, OrchestrationError, OrchestrationResult,
    ScheduledWorkflow, SimpleAgent, TaskResult, TaskScheduler, WorkflowStatus,
};

/// Supervision tree orchestration engine
pub struct SupervisionTreeOrchestrator {
    /// Root supervisor for the orchestration tree
    root_supervisor: Arc<RwLock<AgentSupervisor>>,
    /// Task decomposition system
    task_decomposer: Arc<TerraphimTaskDecompositionSystem>,
    /// Task scheduler
    scheduler: Arc<TaskScheduler>,
    /// Execution coordinator
    coordinator: Arc<ExecutionCoordinator>,
    /// Active workflows
    active_workflows: Arc<RwLock<HashMap<String, SupervisedWorkflow>>>,
    /// Configuration
    config: SupervisionOrchestrationConfig,
    /// System message channel
    system_tx: mpsc::UnboundedSender<SupervisionMessage>,
    /// System message receiver
    system_rx: Arc<RwLock<Option<mpsc::UnboundedReceiver<SupervisionMessage>>>>,
}

/// Configuration for supervision tree orchestration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisionOrchestrationConfig {
    /// Maximum number of concurrent workflows
    pub max_concurrent_workflows: usize,
    /// Default restart strategy for agents
    pub default_restart_strategy: RestartStrategy,
    /// Maximum restart attempts before giving up
    pub max_restart_attempts: u32,
    /// Restart intensity (max restarts per time window)
    pub restart_intensity: u32,
    /// Restart time window in seconds
    pub restart_period_seconds: u64,
    /// Workflow timeout in seconds
    pub workflow_timeout_seconds: u64,
    /// Enable automatic fault recovery
    pub enable_auto_recovery: bool,
    /// Health check interval in seconds
    pub health_check_interval_seconds: u64,
}

impl Default for SupervisionOrchestrationConfig {
    fn default() -> Self {
        Self {
            max_concurrent_workflows: 10,
            default_restart_strategy: RestartStrategy::OneForOne,
            max_restart_attempts: 3,
            restart_intensity: 5,
            restart_period_seconds: 60,
            workflow_timeout_seconds: 3600,
            enable_auto_recovery: true,
            health_check_interval_seconds: 30,
        }
    }
}

/// Workflow execution state for supervision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecution {
    /// Workflow identifier
    pub workflow_id: String,
    /// Tasks in the workflow
    pub tasks: Vec<Task>,
    /// Execution status
    pub status: WorkflowStatus,
    /// Start time
    pub started_at: SystemTime,
    /// Completion time
    pub completed_at: Option<SystemTime>,
    /// Task results
    pub results: HashMap<String, TaskResult>,
    /// Execution errors
    pub errors: Vec<String>,
}

/// Supervised workflow execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisedWorkflow {
    /// Workflow identifier
    pub workflow_id: String,
    /// Workflow execution state
    pub execution: WorkflowExecution,
    /// Supervisor managing this workflow
    pub supervisor_id: SupervisorId,
    /// Agent assignments for tasks
    pub agent_assignments: HashMap<String, AgentPid>,
    /// Restart attempts per agent
    pub restart_attempts: HashMap<AgentPid, u32>,
    /// Workflow start time
    pub start_time: SystemTime,
    /// Last health check time
    pub last_health_check: SystemTime,
    /// Fault recovery actions taken
    pub recovery_actions: Vec<RecoveryAction>,
}

/// Recovery action taken during fault handling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryAction {
    /// Action type
    pub action_type: RecoveryActionType,
    /// Timestamp when action was taken
    pub timestamp: SystemTime,
    /// Target agent or task
    pub target: String,
    /// Action description
    pub description: String,
    /// Success status
    pub success: bool,
}

/// Types of recovery actions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RecoveryActionType {
    AgentRestart,
    TaskReassignment,
    WorkflowReschedule,
    SupervisorEscalation,
    GracefulShutdown,
}

/// System messages for supervision orchestration
#[derive(Debug, Clone)]
pub enum SupervisionMessage {
    /// Agent failure notification
    AgentFailed {
        agent_id: AgentPid,
        workflow_id: String,
        reason: ExitReason,
    },
    /// Agent recovery notification
    AgentRecovered {
        agent_id: AgentPid,
        workflow_id: String,
    },
    /// Workflow timeout
    WorkflowTimeout { workflow_id: String },
    /// Health check request
    HealthCheck { workflow_id: String },
    /// Supervisor escalation
    SupervisorEscalation {
        supervisor_id: SupervisorId,
        reason: String,
    },
    /// System shutdown
    Shutdown,
}

/// Supervision tree orchestration trait
#[async_trait]
pub trait SupervisionOrchestration: Send + Sync {
    /// Start a supervised workflow
    async fn start_supervised_workflow(
        &self,
        workflow_id: String,
        tasks: Vec<Task>,
        agents: Vec<Box<dyn SimpleAgent>>,
    ) -> OrchestrationResult<SupervisedWorkflow>;

    /// Monitor workflow health
    async fn monitor_workflow_health(&self, workflow_id: &str) -> OrchestrationResult<bool>;

    /// Handle agent failure with supervision tree recovery
    async fn handle_agent_failure(
        &self,
        agent_id: &AgentPid,
        workflow_id: &str,
        reason: ExitReason,
    ) -> OrchestrationResult<RecoveryAction>;

    /// Restart failed agent
    async fn restart_agent(
        &self,
        agent_id: &AgentPid,
        workflow_id: &str,
    ) -> OrchestrationResult<AgentPid>;

    /// Escalate to supervisor
    async fn escalate_to_supervisor(
        &self,
        supervisor_id: &SupervisorId,
        reason: &str,
    ) -> OrchestrationResult<()>;

    /// Get workflow supervision status
    async fn get_supervision_status(
        &self,
        workflow_id: &str,
    ) -> OrchestrationResult<SupervisionStatus>;
}

/// Supervision status for a workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisionStatus {
    /// Workflow identifier
    pub workflow_id: String,
    /// Overall health status
    pub health_status: HealthStatus,
    /// Agent statuses
    pub agent_statuses: HashMap<AgentPid, AgentStatus>,
    /// Active recovery actions
    pub active_recovery_actions: Vec<RecoveryAction>,
    /// Restart statistics
    pub restart_stats: RestartStatistics,
    /// Supervision tree depth
    pub supervision_depth: u32,
}

/// Health status of a supervised workflow
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Critical,
    Failed,
}

/// Restart statistics for supervision monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestartStatistics {
    /// Total restart attempts
    pub total_restarts: u32,
    /// Successful restarts
    pub successful_restarts: u32,
    /// Failed restarts
    pub failed_restarts: u32,
    /// Restart rate (restarts per hour)
    pub restart_rate: f64,
    /// Time since last restart
    pub time_since_last_restart: Duration,
}

impl Default for RestartStatistics {
    fn default() -> Self {
        Self {
            total_restarts: 0,
            successful_restarts: 0,
            failed_restarts: 0,
            restart_rate: 0.0,
            time_since_last_restart: Duration::ZERO,
        }
    }
}

impl SupervisionTreeOrchestrator {
    /// Create a new supervision tree orchestrator
    pub async fn new(config: SupervisionOrchestrationConfig) -> OrchestrationResult<Self> {
        let root_supervisor = Arc::new(RwLock::new(AgentSupervisor::new(
            SupervisorConfig {
                supervisor_id: SupervisorId::new(),
                restart_policy: RestartPolicy {
                    strategy: config.default_restart_strategy.clone(),
                    intensity: RestartIntensity {
                        max_restarts: config.max_restart_attempts,
                        time_window: Duration::from_secs(config.restart_period_seconds),
                    },
                },
                agent_timeout: Duration::from_secs(30),
                health_check_interval: Duration::from_secs(10),
                max_children: 100,
            },
            std::sync::Arc::new(TestAgentFactory),
        )));

        // Create mock dependencies for now - in a real implementation these would be properly configured
        let automata = Arc::new(terraphim_task_decomposition::MockAutomata);
        let role_name = terraphim_types::RoleName::new("supervisor");
        let thesaurus =
            terraphim_automata::load_thesaurus(&terraphim_automata::AutomataPath::local_example())
                .await
                .map_err(|e| OrchestrationError::SystemError(e.to_string()))?;
        let role_graph = Arc::new(
            terraphim_rolegraph::RoleGraph::new(role_name, thesaurus)
                .await
                .map_err(|e| OrchestrationError::SystemError(e.to_string()))?,
        );

        let task_decomposer = Arc::new(TerraphimTaskDecompositionSystem::new(
            automata,
            role_graph,
            terraphim_task_decomposition::TaskDecompositionSystemConfig::default(),
        ));

        let agent_pool = Arc::new(AgentPool::new());
        let scheduler = Arc::new(TaskScheduler::new(
            task_decomposer.clone()
                as Arc<dyn terraphim_task_decomposition::TaskDecompositionSystem>,
            agent_pool,
        ));
        let coordinator = Arc::new(ExecutionCoordinator::new(scheduler.clone()));

        let (system_tx, system_rx) = mpsc::unbounded_channel();

        Ok(Self {
            root_supervisor,
            task_decomposer,
            scheduler,
            coordinator,
            active_workflows: Arc::new(RwLock::new(HashMap::new())),
            config,
            system_tx,
            system_rx: Arc::new(RwLock::new(Some(system_rx))),
        })
    }

    /// Start the supervision system
    pub async fn start(&self) -> OrchestrationResult<()> {
        info!("Starting supervision tree orchestrator");

        // Start the system message handler
        self.start_message_handler().await?;

        // Start health monitoring
        self.start_health_monitor().await?;

        info!("Supervision tree orchestrator started successfully");
        Ok(())
    }

    /// Start the system message handler
    async fn start_message_handler(&self) -> OrchestrationResult<()> {
        let mut rx = self.system_rx.write().await.take().ok_or_else(|| {
            OrchestrationError::SystemError("Message handler already started".to_string())
        })?;

        let orchestrator = self.clone_for_handler();

        tokio::spawn(async move {
            while let Some(message) = rx.recv().await {
                if let Err(e) = orchestrator.handle_system_message(message).await {
                    error!("Error handling system message: {}", e);
                }
            }
        });

        Ok(())
    }

    /// Start health monitoring
    async fn start_health_monitor(&self) -> OrchestrationResult<()> {
        let orchestrator = self.clone_for_handler();
        let interval = Duration::from_secs(self.config.health_check_interval_seconds);

        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);
            loop {
                interval_timer.tick().await;
                if let Err(e) = orchestrator.perform_health_checks().await {
                    error!("Error during health checks: {}", e);
                }
            }
        });

        Ok(())
    }

    /// Clone for use in async handlers
    fn clone_for_handler(&self) -> Self {
        Self {
            root_supervisor: self.root_supervisor.clone(),
            task_decomposer: self.task_decomposer.clone(),
            scheduler: self.scheduler.clone(),
            coordinator: self.coordinator.clone(),
            active_workflows: self.active_workflows.clone(),
            config: self.config.clone(),
            system_tx: self.system_tx.clone(),
            system_rx: Arc::new(RwLock::new(None)), // Don't clone the receiver
        }
    }

    /// Handle system messages
    async fn handle_system_message(&self, message: SupervisionMessage) -> OrchestrationResult<()> {
        match message {
            SupervisionMessage::AgentFailed {
                agent_id,
                workflow_id,
                reason,
            } => {
                warn!(
                    "Agent {} failed in workflow {}: {:?}",
                    agent_id, workflow_id, reason
                );
                self.handle_agent_failure(&agent_id, &workflow_id, reason)
                    .await?;
            }
            SupervisionMessage::AgentRecovered {
                agent_id,
                workflow_id,
            } => {
                info!("Agent {} recovered in workflow {}", agent_id, workflow_id);
                self.handle_agent_recovery(&agent_id, &workflow_id).await?;
            }
            SupervisionMessage::WorkflowTimeout { workflow_id } => {
                warn!("Workflow {} timed out", workflow_id);
                self.handle_workflow_timeout(&workflow_id).await?;
            }
            SupervisionMessage::HealthCheck { workflow_id } => {
                debug!("Health check for workflow {}", workflow_id);
                self.monitor_workflow_health(&workflow_id).await?;
            }
            SupervisionMessage::SupervisorEscalation {
                supervisor_id,
                reason,
            } => {
                error!("Supervisor {} escalation: {}", supervisor_id, reason);
                self.escalate_to_supervisor(&supervisor_id, &reason).await?;
            }
            SupervisionMessage::Shutdown => {
                info!("Received shutdown signal");
                self.shutdown().await?;
            }
        }
        Ok(())
    }

    /// Perform health checks on all active workflows
    async fn perform_health_checks(&self) -> OrchestrationResult<()> {
        let workflows = self.active_workflows.read().await;
        let workflow_ids: Vec<String> = workflows.keys().cloned().collect();
        drop(workflows);

        for workflow_id in workflow_ids {
            if let Err(e) = self.monitor_workflow_health(&workflow_id).await {
                warn!("Health check failed for workflow {}: {}", workflow_id, e);
            }
        }

        Ok(())
    }

    /// Handle agent recovery
    async fn handle_agent_recovery(
        &self,
        agent_id: &AgentPid,
        workflow_id: &str,
    ) -> OrchestrationResult<()> {
        let mut workflows = self.active_workflows.write().await;
        if let Some(workflow) = workflows.get_mut(workflow_id) {
            workflow.recovery_actions.push(RecoveryAction {
                action_type: RecoveryActionType::AgentRestart,
                timestamp: SystemTime::now(),
                target: agent_id.to_string(),
                description: "Agent successfully recovered".to_string(),
                success: true,
            });

            // Reset restart attempts for this agent
            workflow.restart_attempts.insert(agent_id.clone(), 0);
        }
        Ok(())
    }

    /// Handle workflow timeout
    async fn handle_workflow_timeout(&self, workflow_id: &str) -> OrchestrationResult<()> {
        let mut workflows = self.active_workflows.write().await;
        if let Some(workflow) = workflows.get_mut(workflow_id) {
            workflow.execution.status =
                WorkflowStatus::Failed("Agent health check failed".to_string());
            workflow.recovery_actions.push(RecoveryAction {
                action_type: RecoveryActionType::GracefulShutdown,
                timestamp: SystemTime::now(),
                target: workflow_id.to_string(),
                description: "Workflow timed out and was terminated".to_string(),
                success: true,
            });

            // Terminate all agents in this workflow
            for agent_id in workflow.agent_assignments.values() {
                if let Err(e) = self.terminate_agent(agent_id, workflow_id).await {
                    error!(
                        "Failed to terminate agent {} in workflow {}: {}",
                        agent_id, workflow_id, e
                    );
                }
            }
        }
        Ok(())
    }

    /// Terminate an agent
    async fn terminate_agent(
        &self,
        agent_id: &AgentPid,
        workflow_id: &str,
    ) -> OrchestrationResult<()> {
        let mut supervisor = self.root_supervisor.write().await;
        supervisor
            .stop_agent(agent_id)
            .await
            .map_err(|e| OrchestrationError::SupervisionError(e.to_string()))?;

        debug!("Terminated agent {} in workflow {}", agent_id, workflow_id);
        Ok(())
    }

    /// Shutdown the orchestrator
    async fn shutdown(&self) -> OrchestrationResult<()> {
        info!("Shutting down supervision tree orchestrator");

        // Terminate all active workflows
        let workflows = self.active_workflows.read().await;
        let workflow_ids: Vec<String> = workflows.keys().cloned().collect();
        drop(workflows);

        for workflow_id in workflow_ids {
            if let Err(e) = self.handle_workflow_timeout(&workflow_id).await {
                error!("Error during workflow shutdown {}: {}", workflow_id, e);
            }
        }

        // Shutdown root supervisor
        let mut supervisor = self.root_supervisor.write().await;
        supervisor
            .stop()
            .await
            .map_err(|e| OrchestrationError::SupervisionError(e.to_string()))?;

        info!("Supervision tree orchestrator shutdown complete");
        Ok(())
    }

    /// Calculate restart statistics for a workflow
    fn calculate_restart_stats(&self, workflow: &SupervisedWorkflow) -> RestartStatistics {
        let total_restarts: u32 = workflow.restart_attempts.values().sum();
        let successful_restarts = workflow
            .recovery_actions
            .iter()
            .filter(|a| a.action_type == RecoveryActionType::AgentRestart && a.success)
            .count() as u32;
        let failed_restarts = total_restarts.saturating_sub(successful_restarts);

        let elapsed = workflow.start_time.elapsed().unwrap_or(Duration::ZERO);
        let restart_rate = if elapsed.as_secs() > 0 {
            (total_restarts as f64) / (elapsed.as_secs() as f64 / 3600.0)
        } else {
            0.0
        };

        let time_since_last_restart = workflow
            .recovery_actions
            .iter()
            .filter(|a| a.action_type == RecoveryActionType::AgentRestart)
            .next_back()
            .map(|a| a.timestamp.elapsed().unwrap_or(Duration::ZERO))
            .unwrap_or(elapsed);

        RestartStatistics {
            total_restarts,
            successful_restarts,
            failed_restarts,
            restart_rate,
            time_since_last_restart,
        }
    }
}

#[async_trait]
impl SupervisionOrchestration for SupervisionTreeOrchestrator {
    async fn start_supervised_workflow(
        &self,
        workflow_id: String,
        tasks: Vec<Task>,
        agents: Vec<Box<dyn SimpleAgent>>,
    ) -> OrchestrationResult<SupervisedWorkflow> {
        info!("Starting supervised workflow: {}", workflow_id);

        // Check workflow limit
        let workflows_count = self.active_workflows.read().await.len();
        if workflows_count >= self.config.max_concurrent_workflows {
            return Err(OrchestrationError::ResourceExhausted(
                "Maximum concurrent workflows reached".to_string(),
            ));
        }

        // Create child supervisor for this workflow
        let workflow_supervisor_id = SupervisorId::new();
        let mut root_supervisor = self.root_supervisor.write().await;

        // Create agent specs for supervision
        let mut agent_specs = Vec::new();
        let mut agent_assignments = HashMap::new();

        for (i, agent) in agents.iter().enumerate() {
            let agent_id = AgentPid::new();
            let task_id = if i < tasks.len() {
                tasks[i].task_id.clone()
            } else {
                format!("task_{}", i)
            };

            agent_assignments.insert(task_id, agent_id.clone());

            let spec = AgentSpec {
                agent_id: agent_id.clone(),
                agent_type: "workflow_agent".to_string(),
                config: serde_json::json!({
                    "workflow_id": workflow_id,
                    "capabilities": agent.capabilities(),
                    "supervisor_id": workflow_supervisor_id.clone(),
                    "restart_strategy": self.config.default_restart_strategy,
                    "max_restart_attempts": self.config.max_restart_attempts
                }),
                name: Some(format!("WorkflowAgent-{}", agent_id)),
            };
            agent_specs.push(spec);
        }

        // Start agents under supervision
        for spec in agent_specs {
            root_supervisor
                .spawn_agent(spec)
                .await
                .map_err(|e| OrchestrationError::SupervisionError(e.to_string()))?;
        }

        drop(root_supervisor);

        // Create a scheduled workflow for execution
        let scheduled_workflow = ScheduledWorkflow {
            workflow: TaskDecompositionWorkflow {
                original_task: Task {
                    task_id: "test_workflow".to_string(),
                    description: "Test workflow execution".to_string(),
                    complexity: terraphim_task_decomposition::TaskComplexity::Moderate,
                    required_capabilities: vec![],
                    knowledge_context: terraphim_task_decomposition::TaskKnowledgeContext::default(
                    ),
                    constraints: vec![],
                    dependencies: vec![],
                    estimated_effort: Duration::from_secs(300),
                    priority: 1,
                    status: terraphim_task_decomposition::TaskStatus::Pending,
                    metadata: terraphim_task_decomposition::TaskMetadata::default(),
                    parent_goal: None,
                    assigned_agents: vec![],
                    subtasks: vec![],
                },
                analysis: terraphim_task_decomposition::TaskAnalysis {
                    task_id: "test_workflow".to_string(),
                    complexity: terraphim_task_decomposition::TaskComplexity::Moderate,
                    required_capabilities: vec![],
                    knowledge_domains: vec![],
                    complexity_factors: vec![],
                    recommended_strategy: None,
                    confidence_score: 0.8,
                    estimated_effort_hours: 5.0,
                    risk_factors: vec![],
                },
                decomposition: terraphim_task_decomposition::DecompositionResult {
                    original_task: "test_workflow".to_string(),
                    subtasks: vec![],
                    dependencies: HashMap::new(),
                    metadata: terraphim_task_decomposition::DecompositionMetadata {
                        strategy_used:
                            terraphim_task_decomposition::DecompositionStrategy::ComplexityBased,
                        depth: 1,
                        subtask_count: 0,
                        concepts_analyzed: vec![],
                        roles_identified: vec![],
                        confidence_score: 0.8,
                        parallelism_factor: 1.0,
                    },
                },
                execution_plan: terraphim_task_decomposition::ExecutionPlan {
                    plan_id: "test_plan".to_string(),
                    tasks: tasks.iter().map(|t| t.task_id.clone()).collect(),
                    phases: vec![],
                    estimated_duration: Duration::from_secs(300),
                    resource_requirements: Default::default(),
                    metadata: terraphim_task_decomposition::PlanMetadata {
                        created_at: Utc::now(),
                        created_by: "supervision_engine".to_string(),
                        version: 1,
                        optimization_strategy:
                            terraphim_task_decomposition::OptimizationStrategy::Balanced,
                        parallelism_factor: 1.0,
                        critical_path_length: 1,
                        confidence_score: 0.8,
                    },
                },
                metadata: terraphim_task_decomposition::WorkflowMetadata {
                    executed_at: Utc::now(),
                    total_execution_time_ms: 0,
                    confidence_score: 0.8,
                    subtask_count: tasks.len() as u32,
                    parallelism_factor: 1.0,
                    version: 1,
                },
            },
            agent_assignments: agents
                .into_iter()
                .enumerate()
                .map(|(i, agent)| AgentAssignment {
                    task_id: format!("task_{}", i),
                    agent: Arc::from(agent),
                })
                .collect(),
        };

        // Start workflow execution
        let _execution_result = self
            .coordinator
            .execute_workflow(scheduled_workflow)
            .await?;

        // Convert WorkflowResult to WorkflowExecution for supervision tracking
        let execution = WorkflowExecution {
            workflow_id: workflow_id.clone(),
            tasks: tasks.clone(),
            status: WorkflowStatus::Running,
            started_at: SystemTime::now(),
            completed_at: None,
            results: HashMap::new(),
            errors: Vec::new(),
        };

        // Create supervised workflow
        let supervised_workflow = SupervisedWorkflow {
            workflow_id: workflow_id.clone(),
            execution,
            supervisor_id: workflow_supervisor_id,
            agent_assignments,
            restart_attempts: HashMap::new(),
            start_time: SystemTime::now(),
            last_health_check: SystemTime::now(),
            recovery_actions: Vec::new(),
        };

        // Store the workflow
        self.active_workflows
            .write()
            .await
            .insert(workflow_id.clone(), supervised_workflow.clone());

        info!("Supervised workflow {} started successfully", workflow_id);
        Ok(supervised_workflow)
    }

    async fn monitor_workflow_health(&self, workflow_id: &str) -> OrchestrationResult<bool> {
        let mut workflows = self.active_workflows.write().await;
        let workflow = workflows
            .get_mut(workflow_id)
            .ok_or_else(|| OrchestrationError::WorkflowNotFound(workflow_id.to_string()))?;

        workflow.last_health_check = SystemTime::now();

        // Check workflow timeout
        let elapsed = workflow.start_time.elapsed().unwrap_or(Duration::ZERO);
        if elapsed > Duration::from_secs(self.config.workflow_timeout_seconds) {
            let _ = self.system_tx.send(SupervisionMessage::WorkflowTimeout {
                workflow_id: workflow_id.to_string(),
            });
            return Ok(false);
        }

        // Check agent health
        let supervisor = self.root_supervisor.read().await;
        for agent_id in workflow.agent_assignments.values() {
            if let Some(agent_info) = supervisor.get_child(agent_id).await {
                if matches!(
                    agent_info.status,
                    AgentStatus::Failed(_) | AgentStatus::Stopped
                ) {
                    let _ = self.system_tx.send(SupervisionMessage::AgentFailed {
                        agent_id: agent_id.clone(),
                        workflow_id: workflow_id.to_string(),
                        reason: ExitReason::Error("Health check failed".to_string()),
                    });
                    return Ok(false);
                }
            }
        }

        Ok(true)
    }

    async fn handle_agent_failure(
        &self,
        agent_id: &AgentPid,
        workflow_id: &str,
        _reason: ExitReason,
    ) -> OrchestrationResult<RecoveryAction> {
        warn!(
            "Handling agent failure: {} in workflow {}",
            agent_id, workflow_id
        );

        let mut workflows = self.active_workflows.write().await;
        let workflow = workflows
            .get_mut(workflow_id)
            .ok_or_else(|| OrchestrationError::WorkflowNotFound(workflow_id.to_string()))?;

        // Increment restart attempts
        let attempts = workflow
            .restart_attempts
            .entry(agent_id.clone())
            .or_insert(0);
        *attempts += 1;

        let recovery_action =
            if *attempts <= self.config.max_restart_attempts && self.config.enable_auto_recovery {
                // Attempt restart
                match self.restart_agent(agent_id, workflow_id).await {
                    Ok(_) => RecoveryAction {
                        action_type: RecoveryActionType::AgentRestart,
                        timestamp: SystemTime::now(),
                        target: agent_id.to_string(),
                        description: format!("Agent restarted (attempt {})", attempts),
                        success: true,
                    },
                    Err(e) => {
                        error!("Failed to restart agent {}: {}", agent_id, e);
                        RecoveryAction {
                            action_type: RecoveryActionType::AgentRestart,
                            timestamp: SystemTime::now(),
                            target: agent_id.to_string(),
                            description: format!("Agent restart failed: {}", e),
                            success: false,
                        }
                    }
                }
            } else {
                // Escalate to supervisor
                let _ = self
                    .system_tx
                    .send(SupervisionMessage::SupervisorEscalation {
                        supervisor_id: workflow.supervisor_id.clone(),
                        reason: format!("Agent {} exceeded restart attempts", agent_id),
                    });

                RecoveryAction {
                    action_type: RecoveryActionType::SupervisorEscalation,
                    timestamp: SystemTime::now(),
                    target: agent_id.to_string(),
                    description: format!("Escalated after {} restart attempts", attempts),
                    success: true,
                }
            };

        workflow.recovery_actions.push(recovery_action.clone());
        Ok(recovery_action)
    }

    async fn restart_agent(
        &self,
        agent_id: &AgentPid,
        workflow_id: &str,
    ) -> OrchestrationResult<AgentPid> {
        info!("Restarting agent {} in workflow {}", agent_id, workflow_id);

        let mut supervisor = self.root_supervisor.write().await;

        // Stop the failed agent
        supervisor
            .stop_agent(agent_id)
            .await
            .map_err(|e| OrchestrationError::SupervisionError(e.to_string()))?;

        // Create a new agent spec for restart (simplified)
        let spec = AgentSpec {
            agent_id: agent_id.clone(),
            agent_type: "workflow_agent".to_string(),
            config: serde_json::json!({
                "workflow_id": workflow_id,
                "restart": true
            }),
            name: Some(format!("RestartedAgent-{}", agent_id)),
        };

        // Spawn a new agent
        supervisor
            .spawn_agent(spec)
            .await
            .map_err(|e| OrchestrationError::SupervisionError(e.to_string()))?;

        // Notify recovery
        let _ = self.system_tx.send(SupervisionMessage::AgentRecovered {
            agent_id: agent_id.clone(),
            workflow_id: workflow_id.to_string(),
        });

        Ok(agent_id.clone())
    }

    async fn escalate_to_supervisor(
        &self,
        supervisor_id: &SupervisorId,
        reason: &str,
    ) -> OrchestrationResult<()> {
        error!("Escalating to supervisor {}: {}", supervisor_id, reason);

        // In a real implementation, this would escalate to a parent supervisor
        // For now, we'll log the escalation and potentially shutdown the workflow

        // Find workflows managed by this supervisor
        let workflows = self.active_workflows.read().await;
        let affected_workflows: Vec<String> = workflows
            .iter()
            .filter(|(_, w)| w.supervisor_id == *supervisor_id)
            .map(|(id, _)| id.clone())
            .collect();
        drop(workflows);

        // Handle escalation by gracefully shutting down affected workflows
        for workflow_id in affected_workflows {
            let _ = self
                .system_tx
                .send(SupervisionMessage::WorkflowTimeout { workflow_id });
        }

        Ok(())
    }

    async fn get_supervision_status(
        &self,
        workflow_id: &str,
    ) -> OrchestrationResult<SupervisionStatus> {
        let workflows = self.active_workflows.read().await;
        let workflow = workflows
            .get(workflow_id)
            .ok_or_else(|| OrchestrationError::WorkflowNotFound(workflow_id.to_string()))?;

        // Get agent statuses
        let supervisor = self.root_supervisor.read().await;
        let mut agent_statuses = HashMap::new();
        for agent_id in workflow.agent_assignments.values() {
            if let Some(agent_info) = supervisor.get_child(agent_id).await {
                agent_statuses.insert(agent_id.clone(), agent_info.status);
            }
        }

        // Determine overall health status
        let health_status = if agent_statuses
            .values()
            .all(|s| matches!(s, AgentStatus::Running))
        {
            HealthStatus::Healthy
        } else if agent_statuses
            .values()
            .any(|s| matches!(s, AgentStatus::Failed(_)))
        {
            HealthStatus::Critical
        } else if agent_statuses
            .values()
            .any(|s| matches!(s, AgentStatus::Restarting))
        {
            HealthStatus::Degraded
        } else {
            HealthStatus::Failed
        };

        let restart_stats = self.calculate_restart_stats(workflow);

        Ok(SupervisionStatus {
            workflow_id: workflow_id.to_string(),
            health_status,
            agent_statuses,
            active_recovery_actions: workflow.recovery_actions.clone(),
            restart_stats,
            supervision_depth: 1, // Simple implementation - could be enhanced
        })
    }
}

// Test AgentFactory implementation for compilation
#[derive(Debug)]
struct TestAgentFactory;

#[async_trait]
impl AgentFactory for TestAgentFactory {
    async fn create_agent(&self, _spec: &AgentSpec) -> SupervisionResult<Box<dyn SupervisedAgent>> {
        // Return a minimal test agent
        Ok(Box::new(TestSupervisedAgent::new()))
    }

    fn validate_spec(&self, _spec: &AgentSpec) -> SupervisionResult<()> {
        Ok(())
    }

    fn supported_types(&self) -> Vec<String> {
        vec!["test".to_string()]
    }
}

// Test SupervisedAgent implementation
#[derive(Debug)]
struct TestSupervisedAgent {
    pid: AgentPid,
    supervisor_id: SupervisorId,
    status: AgentStatus,
}

impl TestSupervisedAgent {
    fn new() -> Self {
        Self {
            pid: AgentPid::new(),
            supervisor_id: SupervisorId::new(),
            status: AgentStatus::Stopped,
        }
    }
}

#[async_trait]
impl SupervisedAgent for TestSupervisedAgent {
    async fn init(&mut self, args: InitArgs) -> SupervisionResult<()> {
        self.pid = args.agent_id;
        self.supervisor_id = args.supervisor_id;
        self.status = AgentStatus::Starting;
        Ok(())
    }

    async fn start(&mut self) -> SupervisionResult<()> {
        self.status = AgentStatus::Running;
        Ok(())
    }

    async fn stop(&mut self) -> SupervisionResult<()> {
        self.status = AgentStatus::Stopped;
        Ok(())
    }

    async fn handle_system_message(&mut self, _message: SystemMessage) -> SupervisionResult<()> {
        Ok(())
    }

    fn status(&self) -> AgentStatus {
        self.status.clone()
    }

    fn pid(&self) -> &AgentPid {
        &self.pid
    }

    fn supervisor_id(&self) -> &SupervisorId {
        &self.supervisor_id
    }

    async fn health_check(&self) -> SupervisionResult<bool> {
        Ok(matches!(self.status, AgentStatus::Running))
    }

    async fn terminate(&mut self, _reason: TerminateReason) -> SupervisionResult<()> {
        self.status = AgentStatus::Stopped;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SimpleAgent, TaskResult};

    #[derive(Debug, Clone)]
    struct TestAgent {
        id: String,
        capabilities: Vec<String>,
    }

    #[async_trait]
    impl SimpleAgent for TestAgent {
        fn agent_id(&self) -> &str {
            &self.id
        }

        fn capabilities(&self) -> &[String] {
            &self.capabilities
        }

        fn can_handle_task(&self, task: &Task) -> bool {
            task.required_capabilities
                .iter()
                .all(|cap| self.capabilities.contains(cap))
        }

        async fn execute_task(&self, task: &Task) -> OrchestrationResult<TaskResult> {
            // Simulate task execution
            tokio::time::sleep(Duration::from_millis(100)).await;
            Ok(TaskResult {
                agent_id: self.id.clone(),
                task_id: task.task_id.clone(),
                status: crate::TaskExecutionStatus::Completed,
                result_data: Some(serde_json::json!({"status": "completed"})),
                error_message: None,
                started_at: Utc::now(),
                completed_at: Utc::now(),
                duration: Duration::from_millis(100),
                confidence_score: 0.9,
                metadata: HashMap::new(),
            })
        }
    }

    #[tokio::test]
    async fn test_supervision_orchestrator_creation() {
        let config = SupervisionOrchestrationConfig::default();
        let orchestrator = SupervisionTreeOrchestrator::new(config).await;
        assert!(orchestrator.is_ok());
    }

    #[tokio::test]
    async fn test_supervised_workflow_start() {
        let config = SupervisionOrchestrationConfig::default();
        let orchestrator = SupervisionTreeOrchestrator::new(config).await.unwrap();

        let tasks = vec![Task::new(
            "test_task".to_string(),
            "Test task".to_string(),
            terraphim_task_decomposition::TaskComplexity::Simple,
            1,
        )];

        let agents: Vec<Box<dyn SimpleAgent>> = vec![Box::new(TestAgent {
            id: "test_agent".to_string(),
            capabilities: vec!["test".to_string()],
        })];

        let result = orchestrator
            .start_supervised_workflow("test_workflow".to_string(), tasks, agents)
            .await;

        assert!(result.is_ok());
        let workflow = result.unwrap();
        assert_eq!(workflow.workflow_id, "test_workflow");
        assert!(!workflow.agent_assignments.is_empty());
    }

    #[tokio::test]
    async fn test_health_monitoring() {
        let config = SupervisionOrchestrationConfig::default();
        let orchestrator = SupervisionTreeOrchestrator::new(config).await.unwrap();

        let tasks = vec![Task::new(
            "test_task".to_string(),
            "Test task".to_string(),
            terraphim_task_decomposition::TaskComplexity::Simple,
            1,
        )];

        let agents: Vec<Box<dyn SimpleAgent>> = vec![Box::new(TestAgent {
            id: "test_agent".to_string(),
            capabilities: vec!["test".to_string()],
        })];

        let workflow = orchestrator
            .start_supervised_workflow("test_workflow".to_string(), tasks, agents)
            .await
            .unwrap();

        let health_result = orchestrator
            .monitor_workflow_health(&workflow.workflow_id)
            .await;

        assert!(health_result.is_ok());
    }

    #[tokio::test]
    async fn test_supervision_status() {
        let config = SupervisionOrchestrationConfig::default();
        let orchestrator = SupervisionTreeOrchestrator::new(config).await.unwrap();

        let tasks = vec![Task::new(
            "test_task".to_string(),
            "Test task".to_string(),
            terraphim_task_decomposition::TaskComplexity::Simple,
            1,
        )];

        let agents: Vec<Box<dyn SimpleAgent>> = vec![Box::new(TestAgent {
            id: "test_agent".to_string(),
            capabilities: vec!["test".to_string()],
        })];

        let workflow = orchestrator
            .start_supervised_workflow("test_workflow".to_string(), tasks, agents)
            .await
            .unwrap();

        let status_result = orchestrator
            .get_supervision_status(&workflow.workflow_id)
            .await;

        assert!(status_result.is_ok());
        let status = status_result.unwrap();
        assert_eq!(status.workflow_id, "test_workflow");
        assert!(!status.agent_statuses.is_empty());
    }
}
