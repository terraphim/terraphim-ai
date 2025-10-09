//! Knowledge graph-based planning agent implementation
//!
//! This module provides a specialized GenAgent implementation for intelligent
//! task planning using knowledge graph analysis and decomposition.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};

use terraphim_automata::Automata;
use terraphim_gen_agent::{GenAgent, GenAgentResult};
use terraphim_rolegraph::RoleGraph;
use terraphim_task_decomposition::{
    DecompositionConfig, KnowledgeGraphTaskDecomposer, Task, TaskDecomposer,
};

use crate::{KgAgentError, KgAgentResult};

/// Message types for the planning agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlanningMessage {
    /// Request to create a plan for a task
    CreatePlan {
        task: Task,
        config: Option<DecompositionConfig>,
    },
    /// Request to validate a plan
    ValidatePlan { plan: ExecutionPlan },
    /// Request to optimize a plan
    OptimizePlan { plan: ExecutionPlan },
    /// Request to update a plan based on execution feedback
    UpdatePlan {
        plan: ExecutionPlan,
        feedback: PlanningFeedback,
    },
}

/// Planning agent state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningState {
    /// Active plans being managed
    pub active_plans: HashMap<String, ExecutionPlan>,
    /// Planning statistics
    pub stats: PlanningStats,
    /// Configuration
    pub config: PlanningConfig,
}

/// Execution plan created by the planning agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    /// Plan identifier
    pub plan_id: String,
    /// Original task
    pub original_task: Task,
    /// Decomposed subtasks
    pub subtasks: Vec<Task>,
    /// Task dependencies
    pub dependencies: HashMap<String, Vec<String>>,
    /// Estimated execution time
    pub estimated_duration: std::time::Duration,
    /// Plan confidence score
    pub confidence: f64,
    /// Knowledge graph concepts involved
    pub concepts: Vec<String>,
    /// Plan status
    pub status: PlanStatus,
}

/// Plan execution status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PlanStatus {
    Draft,
    Validated,
    Optimized,
    Executing,
    Completed,
    Failed,
}

/// Planning feedback for plan updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningFeedback {
    /// Plan identifier
    pub plan_id: String,
    /// Execution results
    pub execution_results: Vec<TaskExecutionResult>,
    /// Performance metrics
    pub performance_metrics: HashMap<String, f64>,
    /// Issues encountered
    pub issues: Vec<String>,
}

/// Task execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskExecutionResult {
    /// Task identifier
    pub task_id: String,
    /// Execution success
    pub success: bool,
    /// Execution time
    pub execution_time: std::time::Duration,
    /// Error message if failed
    pub error_message: Option<String>,
}

/// Planning statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningStats {
    /// Total plans created
    pub plans_created: u64,
    /// Plans successfully executed
    pub plans_executed: u64,
    /// Average plan confidence
    pub average_confidence: f64,
    /// Average execution time accuracy
    pub time_accuracy: f64,
}

impl Default for PlanningStats {
    fn default() -> Self {
        Self {
            plans_created: 0,
            plans_executed: 0,
            average_confidence: 0.0,
            time_accuracy: 0.0,
        }
    }
}

/// Planning agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningConfig {
    /// Default decomposition configuration
    pub default_decomposition_config: DecompositionConfig,
    /// Maximum number of active plans
    pub max_active_plans: usize,
    /// Minimum confidence threshold for plans
    pub min_confidence_threshold: f64,
    /// Enable plan optimization
    pub enable_optimization: bool,
    /// Plan validation timeout
    pub validation_timeout: std::time::Duration,
}

impl Default for PlanningConfig {
    fn default() -> Self {
        Self {
            default_decomposition_config: DecompositionConfig::default(),
            max_active_plans: 100,
            min_confidence_threshold: 0.6,
            enable_optimization: true,
            validation_timeout: std::time::Duration::from_secs(30),
        }
    }
}

impl Default for PlanningState {
    fn default() -> Self {
        Self {
            active_plans: HashMap::new(),
            stats: PlanningStats::default(),
            config: PlanningConfig::default(),
        }
    }
}

/// Knowledge graph-based planning agent
pub struct KnowledgeGraphPlanningAgent {
    /// Agent identifier
    agent_id: String,
    /// Task decomposer
    decomposer: Arc<KnowledgeGraphTaskDecomposer>,
    /// Agent state
    state: PlanningState,
}

impl KnowledgeGraphPlanningAgent {
    /// Create a new planning agent
    pub fn new(
        agent_id: String,
        automata: Arc<Automata>,
        role_graph: Arc<RoleGraph>,
        config: PlanningConfig,
    ) -> Self {
        let decomposer = Arc::new(KnowledgeGraphTaskDecomposer::new(automata, role_graph));

        let state = PlanningState {
            active_plans: HashMap::new(),
            stats: PlanningStats::default(),
            config,
        };

        Self {
            agent_id,
            decomposer,
            state,
        }
    }

    /// Create a plan for a task
    async fn create_plan(
        &mut self,
        task: Task,
        config: Option<DecompositionConfig>,
    ) -> KgAgentResult<ExecutionPlan> {
        info!("Creating plan for task: {}", task.task_id);

        let decomposition_config =
            config.unwrap_or(self.state.config.default_decomposition_config.clone());

        // Decompose the task
        let decomposition_result = self
            .decomposer
            .decompose_task(&task, &decomposition_config)
            .await
            .map_err(|e| KgAgentError::DecompositionFailed(e.to_string()))?;

        // Create execution plan
        let plan_id = format!("plan_{}", uuid::Uuid::new_v4());
        let plan = ExecutionPlan {
            plan_id: plan_id.clone(),
            original_task: task,
            subtasks: decomposition_result.subtasks,
            dependencies: decomposition_result.dependencies,
            estimated_duration: std::time::Duration::from_secs(3600), // TODO: Calculate from subtasks
            confidence: decomposition_result.metadata.confidence_score,
            concepts: decomposition_result.metadata.concepts_analyzed,
            status: PlanStatus::Draft,
        };

        // Check confidence threshold
        if plan.confidence < self.state.config.min_confidence_threshold {
            return Err(KgAgentError::PlanningError(format!(
                "Plan confidence {} below threshold {}",
                plan.confidence, self.state.config.min_confidence_threshold
            )));
        }

        // Store the plan
        if self.state.active_plans.len() >= self.state.config.max_active_plans {
            return Err(KgAgentError::PlanningError(
                "Maximum number of active plans reached".to_string(),
            ));
        }

        self.state
            .active_plans
            .insert(plan_id.clone(), plan.clone());
        self.state.stats.plans_created += 1;

        info!(
            "Created plan {} with {} subtasks and {:.2} confidence",
            plan_id,
            plan.subtasks.len(),
            plan.confidence
        );

        Ok(plan)
    }

    /// Validate a plan
    async fn validate_plan(&mut self, mut plan: ExecutionPlan) -> KgAgentResult<ExecutionPlan> {
        debug!("Validating plan: {}", plan.plan_id);

        // Validate task decomposition
        let decomposition_result = terraphim_task_decomposition::DecompositionResult {
            original_task: plan.original_task.task_id.clone(),
            subtasks: plan.subtasks.clone(),
            dependencies: plan.dependencies.clone(),
            metadata: terraphim_task_decomposition::DecompositionMetadata {
                strategy_used:
                    terraphim_task_decomposition::DecompositionStrategy::KnowledgeGraphBased,
                depth: 1,
                subtask_count: plan.subtasks.len() as u32,
                concepts_analyzed: plan.concepts.clone(),
                roles_identified: Vec::new(),
                confidence_score: plan.confidence,
                parallelism_factor: 0.5,
            },
        };

        let is_valid = self
            .decomposer
            .validate_decomposition(&decomposition_result)
            .await
            .map_err(|e| KgAgentError::PlanningError(e.to_string()))?;

        if !is_valid {
            return Err(KgAgentError::PlanningError(format!(
                "Plan {} failed validation",
                plan.plan_id
            )));
        }

        plan.status = PlanStatus::Validated;
        self.state
            .active_plans
            .insert(plan.plan_id.clone(), plan.clone());

        debug!("Plan {} validated successfully", plan.plan_id);
        Ok(plan)
    }

    /// Optimize a plan
    async fn optimize_plan(&mut self, mut plan: ExecutionPlan) -> KgAgentResult<ExecutionPlan> {
        debug!("Optimizing plan: {}", plan.plan_id);

        if !self.state.config.enable_optimization {
            debug!("Plan optimization disabled, returning original plan");
            return Ok(plan);
        }

        // Simple optimization: reorder tasks to minimize dependencies
        let optimized_subtasks = self.optimize_task_order(&plan.subtasks, &plan.dependencies);
        plan.subtasks = optimized_subtasks;

        // Recalculate estimated duration based on parallelism
        let parallelism_factor = self.calculate_parallelism_factor(&plan.dependencies);
        let base_duration: std::time::Duration = plan
            .subtasks
            .iter()
            .map(|t| t.estimated_effort)
            .sum::<std::time::Duration>();

        plan.estimated_duration = base_duration.mul_f64(1.0 / parallelism_factor.max(0.1));

        plan.status = PlanStatus::Optimized;
        self.state
            .active_plans
            .insert(plan.plan_id.clone(), plan.clone());

        debug!(
            "Plan {} optimized: {} subtasks, {:.2}s estimated duration",
            plan.plan_id,
            plan.subtasks.len(),
            plan.estimated_duration.as_secs_f64()
        );

        Ok(plan)
    }

    /// Update a plan based on execution feedback
    async fn update_plan(
        &mut self,
        mut plan: ExecutionPlan,
        feedback: PlanningFeedback,
    ) -> KgAgentResult<ExecutionPlan> {
        debug!("Updating plan {} with feedback", plan.plan_id);

        // Update statistics based on feedback
        let successful_tasks = feedback
            .execution_results
            .iter()
            .filter(|r| r.success)
            .count();
        let total_tasks = feedback.execution_results.len();

        if total_tasks > 0 {
            let success_rate = successful_tasks as f64 / total_tasks as f64;
            debug!(
                "Plan {} execution success rate: {:.2}%",
                plan.plan_id,
                success_rate * 100.0
            );

            // Update plan status based on success rate
            if success_rate >= 0.8 {
                plan.status = PlanStatus::Completed;
                self.state.stats.plans_executed += 1;
            } else if success_rate < 0.5 {
                plan.status = PlanStatus::Failed;
            }
        }

        // Update time accuracy statistics
        let actual_times: Vec<f64> = feedback
            .execution_results
            .iter()
            .map(|r| r.execution_time.as_secs_f64())
            .collect();

        if !actual_times.is_empty() {
            let actual_total: f64 = actual_times.iter().sum();
            let estimated_total = plan.estimated_duration.as_secs_f64();
            let accuracy = 1.0 - (actual_total - estimated_total).abs() / estimated_total.max(1.0);

            // Update running average
            let current_accuracy = self.state.stats.time_accuracy;
            let plans_count = self.state.stats.plans_executed.max(1) as f64;
            self.state.stats.time_accuracy =
                (current_accuracy * (plans_count - 1.0) + accuracy) / plans_count;
        }

        self.state
            .active_plans
            .insert(plan.plan_id.clone(), plan.clone());

        debug!("Plan {} updated successfully", plan.plan_id);
        Ok(plan)
    }

    /// Optimize task order to minimize dependencies
    fn optimize_task_order(
        &self,
        tasks: &[Task],
        dependencies: &HashMap<String, Vec<String>>,
    ) -> Vec<Task> {
        // Simple topological sort to optimize execution order
        let mut result = Vec::new();
        let mut remaining: Vec<Task> = tasks.to_vec();
        let mut processed = std::collections::HashSet::new();

        while !remaining.is_empty() {
            let mut made_progress = false;

            remaining.retain(|task| {
                let deps = dependencies.get(&task.task_id).unwrap_or(&Vec::new());
                let all_deps_satisfied = deps.iter().all(|dep| processed.contains(dep));

                if all_deps_satisfied {
                    result.push(task.clone());
                    processed.insert(task.task_id.clone());
                    made_progress = true;
                    false // Remove from remaining
                } else {
                    true // Keep in remaining
                }
            });

            if !made_progress && !remaining.is_empty() {
                // Circular dependency or other issue, add remaining tasks
                warn!("Possible circular dependency detected, adding remaining tasks");
                result.extend(remaining);
                break;
            }
        }

        result
    }

    /// Calculate parallelism factor from dependencies
    fn calculate_parallelism_factor(&self, dependencies: &HashMap<String, Vec<String>>) -> f64 {
        if dependencies.is_empty() {
            return 1.0;
        }

        let total_tasks = dependencies.len();
        let independent_tasks = dependencies.values().filter(|deps| deps.is_empty()).count();

        if total_tasks == 0 {
            1.0
        } else {
            (independent_tasks as f64 / total_tasks as f64).max(0.1)
        }
    }
}

#[async_trait]
impl GenAgent<PlanningState> for KnowledgeGraphPlanningAgent {
    type Message = PlanningMessage;

    async fn init(&mut self, _init_args: serde_json::Value) -> GenAgentResult<()> {
        info!("Initializing planning agent: {}", self.agent_id);
        Ok(())
    }

    async fn handle_call(&mut self, message: Self::Message) -> GenAgentResult<serde_json::Value> {
        match message {
            PlanningMessage::CreatePlan { task, config } => {
                let plan = self.create_plan(task, config).await.map_err(|e| {
                    terraphim_gen_agent::GenAgentError::ExecutionError(
                        self.agent_id.clone(),
                        e.to_string(),
                    )
                })?;
                Ok(serde_json::to_value(plan).unwrap())
            }
            PlanningMessage::ValidatePlan { plan } => {
                let validated_plan = self.validate_plan(plan).await.map_err(|e| {
                    terraphim_gen_agent::GenAgentError::ExecutionError(
                        self.agent_id.clone(),
                        e.to_string(),
                    )
                })?;
                Ok(serde_json::to_value(validated_plan).unwrap())
            }
            PlanningMessage::OptimizePlan { plan } => {
                let optimized_plan = self.optimize_plan(plan).await.map_err(|e| {
                    terraphim_gen_agent::GenAgentError::ExecutionError(
                        self.agent_id.clone(),
                        e.to_string(),
                    )
                })?;
                Ok(serde_json::to_value(optimized_plan).unwrap())
            }
            PlanningMessage::UpdatePlan { plan, feedback } => {
                let updated_plan = self.update_plan(plan, feedback).await.map_err(|e| {
                    terraphim_gen_agent::GenAgentError::ExecutionError(
                        self.agent_id.clone(),
                        e.to_string(),
                    )
                })?;
                Ok(serde_json::to_value(updated_plan).unwrap())
            }
        }
    }

    async fn handle_cast(&mut self, message: Self::Message) -> GenAgentResult<()> {
        // For cast messages, we don't return results but still process them
        match message {
            PlanningMessage::CreatePlan { task, config } => {
                let _ = self.create_plan(task, config).await;
            }
            PlanningMessage::ValidatePlan { plan } => {
                let _ = self.validate_plan(plan).await;
            }
            PlanningMessage::OptimizePlan { plan } => {
                let _ = self.optimize_plan(plan).await;
            }
            PlanningMessage::UpdatePlan { plan, feedback } => {
                let _ = self.update_plan(plan, feedback).await;
            }
        }
        Ok(())
    }

    async fn handle_info(&mut self, _message: serde_json::Value) -> GenAgentResult<()> {
        // Handle system messages, monitoring, etc.
        Ok(())
    }

    async fn terminate(&mut self, _reason: String) -> GenAgentResult<()> {
        info!("Terminating planning agent: {}", self.agent_id);
        // Clean up resources, save state, etc.
        Ok(())
    }

    fn get_state(&self) -> &PlanningState {
        &self.state
    }

    fn get_state_mut(&mut self) -> &mut PlanningState {
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
            "Test task for planning".to_string(),
            TaskComplexity::Moderate,
            1,
        )
    }

    async fn create_test_agent() -> KnowledgeGraphPlanningAgent {
        use terraphim_automata::{load_thesaurus, AutomataPath};
        use terraphim_types::RoleName;

        let automata = Arc::new(terraphim_automata::Automata::default());

        let role_name = RoleName::new("planner");
        let thesaurus = load_thesaurus(&AutomataPath::local_example())
            .await
            .unwrap();
        let role_graph = Arc::new(RoleGraph::new(role_name, thesaurus).await.unwrap());

        KnowledgeGraphPlanningAgent::new(
            "test_planner".to_string(),
            automata,
            role_graph,
            PlanningConfig::default(),
        )
    }

    #[tokio::test]
    async fn test_planning_agent_creation() {
        let agent = create_test_agent().await;
        assert_eq!(agent.agent_id, "test_planner");
        assert_eq!(agent.state.active_plans.len(), 0);
    }

    #[tokio::test]
    async fn test_create_plan() {
        let mut agent = create_test_agent().await;
        let task = create_test_task();

        let result = agent.create_plan(task, None).await;
        assert!(result.is_ok());

        let plan = result.unwrap();
        assert!(!plan.plan_id.is_empty());
        assert_eq!(plan.status, PlanStatus::Draft);
        assert!(plan.confidence >= 0.0);
    }

    #[tokio::test]
    async fn test_validate_plan() {
        let mut agent = create_test_agent().await;
        let task = create_test_task();

        let plan = agent.create_plan(task, None).await.unwrap();
        let validated_plan = agent.validate_plan(plan).await.unwrap();

        assert_eq!(validated_plan.status, PlanStatus::Validated);
    }

    #[tokio::test]
    async fn test_optimize_plan() {
        let mut agent = create_test_agent().await;
        let task = create_test_task();

        let plan = agent.create_plan(task, None).await.unwrap();
        let optimized_plan = agent.optimize_plan(plan).await.unwrap();

        assert_eq!(optimized_plan.status, PlanStatus::Optimized);
    }

    #[tokio::test]
    async fn test_parallelism_calculation() {
        let agent = create_test_agent().await;

        // No dependencies - full parallelism
        let empty_deps = HashMap::new();
        assert_eq!(agent.calculate_parallelism_factor(&empty_deps), 1.0);

        // Some dependencies
        let mut deps = HashMap::new();
        deps.insert("task1".to_string(), vec![]);
        deps.insert("task2".to_string(), vec!["task1".to_string()]);
        let factor = agent.calculate_parallelism_factor(&deps);
        assert!(factor > 0.0 && factor <= 1.0);
    }

    #[tokio::test]
    async fn test_gen_agent_interface() {
        let mut agent = create_test_agent().await;

        // Test initialization
        let init_result = agent.init(serde_json::json!({})).await;
        assert!(init_result.is_ok());

        // Test call message
        let task = create_test_task();
        let message = PlanningMessage::CreatePlan { task, config: None };
        let call_result = agent.handle_call(message).await;
        assert!(call_result.is_ok());

        // Test cast message
        let task = create_test_task();
        let message = PlanningMessage::CreatePlan { task, config: None };
        let cast_result = agent.handle_cast(message).await;
        assert!(cast_result.is_ok());

        // Test termination
        let terminate_result = agent.terminate("test".to_string()).await;
        assert!(terminate_result.is_ok());
    }
}
