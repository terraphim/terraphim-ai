//! Execution planning for decomposed tasks
//!
//! This module provides execution planning capabilities that create optimal
//! execution schedules for decomposed tasks, considering dependencies,
//! resource constraints, and agent capabilities.

use std::collections::{HashMap, HashSet, VecDeque};
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use log::{debug, info};
use serde::{Deserialize, Serialize};

use crate::{
    AgentPid, DecompositionResult, Task, TaskComplexity, TaskDecompositionError,
    TaskDecompositionResult, TaskId, TaskStatus,
};

/// Execution plan for a set of tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    /// Plan identifier
    pub plan_id: String,
    /// Tasks included in this plan
    pub tasks: Vec<TaskId>,
    /// Execution phases (tasks that can run in parallel)
    pub phases: Vec<ExecutionPhase>,
    /// Estimated total execution time
    pub estimated_duration: Duration,
    /// Resource requirements
    pub resource_requirements: ResourceRequirements,
    /// Plan metadata
    pub metadata: PlanMetadata,
}

/// A phase of execution containing tasks that can run in parallel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPhase {
    /// Phase number (0-based)
    pub phase_number: u32,
    /// Tasks in this phase
    pub tasks: Vec<TaskId>,
    /// Estimated phase duration
    pub estimated_duration: Duration,
    /// Required agents for this phase
    pub required_agents: Vec<AgentPid>,
    /// Phase dependencies (previous phases that must complete)
    pub dependencies: Vec<u32>,
}

/// Resource requirements for execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    /// Required agent capabilities
    pub agent_capabilities: HashMap<String, u32>,
    /// Memory requirements (in MB)
    pub memory_mb: u64,
    /// CPU requirements (cores)
    pub cpu_cores: u32,
    /// Network bandwidth requirements (Mbps)
    pub network_mbps: u32,
    /// Storage requirements (in MB)
    pub storage_mb: u64,
    /// Custom resource requirements
    pub custom_resources: HashMap<String, serde_json::Value>,
}

impl Default for ResourceRequirements {
    fn default() -> Self {
        Self {
            agent_capabilities: HashMap::new(),
            memory_mb: 512,
            cpu_cores: 1,
            network_mbps: 10,
            storage_mb: 100,
            custom_resources: HashMap::new(),
        }
    }
}

/// Metadata about the execution plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanMetadata {
    /// When the plan was created
    pub created_at: DateTime<Utc>,
    /// Plan creator
    pub created_by: String,
    /// Plan version
    pub version: u32,
    /// Optimization strategy used
    pub optimization_strategy: OptimizationStrategy,
    /// Parallelism factor achieved
    pub parallelism_factor: f64,
    /// Critical path length
    pub critical_path_length: u32,
    /// Plan confidence score
    pub confidence_score: f64,
}

/// Optimization strategies for execution planning
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OptimizationStrategy {
    /// Minimize total execution time
    MinimizeTime,
    /// Minimize resource usage
    MinimizeResources,
    /// Balance time and resources
    Balanced,
    /// Maximize parallelism
    MaximizeParallelism,
    /// Custom optimization strategy
    Custom(String),
}

/// Planning configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningConfig {
    /// Optimization strategy to use
    pub optimization_strategy: OptimizationStrategy,
    /// Maximum number of parallel tasks
    pub max_parallel_tasks: u32,
    /// Resource constraints
    pub resource_constraints: ResourceRequirements,
    /// Whether to consider agent capabilities
    pub consider_agent_capabilities: bool,
    /// Buffer time between phases (as fraction of phase duration)
    pub phase_buffer_factor: f64,
    /// Whether to optimize for fault tolerance
    pub optimize_for_fault_tolerance: bool,
}

impl Default for PlanningConfig {
    fn default() -> Self {
        Self {
            optimization_strategy: OptimizationStrategy::Balanced,
            max_parallel_tasks: 10,
            resource_constraints: ResourceRequirements::default(),
            consider_agent_capabilities: true,
            phase_buffer_factor: 0.1,
            optimize_for_fault_tolerance: true,
        }
    }
}

/// Task execution planner
#[async_trait]
pub trait ExecutionPlanner: Send + Sync {
    /// Create an execution plan from decomposed tasks
    async fn create_plan(
        &self,
        decomposition: &DecompositionResult,
        config: &PlanningConfig,
    ) -> TaskDecompositionResult<ExecutionPlan>;

    /// Optimize an existing execution plan
    async fn optimize_plan(
        &self,
        plan: &ExecutionPlan,
        config: &PlanningConfig,
    ) -> TaskDecompositionResult<ExecutionPlan>;

    /// Validate an execution plan
    async fn validate_plan(&self, plan: &ExecutionPlan) -> TaskDecompositionResult<bool>;

    /// Update plan based on task status changes
    async fn update_plan(
        &self,
        plan: &ExecutionPlan,
        task_updates: &HashMap<TaskId, TaskStatus>,
    ) -> TaskDecompositionResult<ExecutionPlan>;
}

/// Knowledge graph-aware execution planner
pub struct KnowledgeGraphExecutionPlanner {
    /// Planning cache for performance
    cache: tokio::sync::RwLock<HashMap<String, ExecutionPlan>>,
}

impl KnowledgeGraphExecutionPlanner {
    /// Create a new execution planner
    pub fn new() -> Self {
        Self {
            cache: tokio::sync::RwLock::new(HashMap::new()),
        }
    }

    /// Perform topological sort on tasks to determine execution order
    fn topological_sort(
        &self,
        tasks: &[Task],
        dependencies: &HashMap<TaskId, Vec<TaskId>>,
    ) -> TaskDecompositionResult<Vec<Vec<TaskId>>> {
        let mut in_degree: HashMap<TaskId, u32> = HashMap::new();
        let mut graph: HashMap<TaskId, Vec<TaskId>> = HashMap::new();

        // Initialize in-degree and graph
        for task in tasks {
            in_degree.insert(task.task_id.clone(), 0);
            graph.insert(task.task_id.clone(), Vec::new());
        }

        // Build graph and calculate in-degrees
        for (task_id, deps) in dependencies {
            for dep in deps {
                if let Some(dependents) = graph.get_mut(dep) {
                    dependents.push(task_id.clone());
                }
                *in_degree.get_mut(task_id).unwrap() += 1;
            }
        }

        let mut phases = Vec::new();
        let mut queue: VecDeque<TaskId> = VecDeque::new();

        // Find tasks with no dependencies (in-degree 0)
        for (task_id, &degree) in &in_degree {
            if degree == 0 {
                queue.push_back(task_id.clone());
            }
        }

        while !queue.is_empty() {
            let mut current_phase = Vec::new();
            let phase_size = queue.len();

            // Process all tasks in current phase
            for _ in 0..phase_size {
                if let Some(task_id) = queue.pop_front() {
                    current_phase.push(task_id.clone());

                    // Update in-degrees of dependent tasks
                    if let Some(dependents) = graph.get(&task_id) {
                        for dependent in dependents {
                            if let Some(degree) = in_degree.get_mut(dependent) {
                                *degree -= 1;
                                if *degree == 0 {
                                    queue.push_back(dependent.clone());
                                }
                            }
                        }
                    }
                }
            }

            if !current_phase.is_empty() {
                phases.push(current_phase);
            }
        }

        // Check for cycles
        if phases.iter().map(|p| p.len()).sum::<usize>() != tasks.len() {
            return Err(TaskDecompositionError::DependencyCycle(
                "Circular dependency detected in task graph".to_string(),
            ));
        }

        debug!("Topological sort produced {} phases", phases.len());
        Ok(phases)
    }

    /// Calculate resource requirements for a set of tasks
    fn calculate_resource_requirements(&self, tasks: &[&Task]) -> ResourceRequirements {
        let mut requirements = ResourceRequirements::default();

        for task in tasks {
            // Aggregate capability requirements
            for capability in &task.required_capabilities {
                *requirements
                    .agent_capabilities
                    .entry(capability.clone())
                    .or_insert(0) += 1;
            }

            // Estimate resource needs based on task complexity
            let complexity_multiplier = match task.complexity {
                TaskComplexity::Simple => 1.0,
                TaskComplexity::Moderate => 2.0,
                TaskComplexity::Complex => 4.0,
                TaskComplexity::VeryComplex => 8.0,
            };

            requirements.memory_mb = (requirements.memory_mb as f64 * complexity_multiplier) as u64;
            requirements.cpu_cores = (requirements.cpu_cores as f64 * complexity_multiplier) as u32;
        }

        requirements
    }

    /// Calculate estimated duration for a phase
    fn calculate_phase_duration(&self, tasks: &[&Task], config: &PlanningConfig) -> Duration {
        if tasks.is_empty() {
            return Duration::from_secs(0);
        }

        // Use the maximum estimated effort among tasks in the phase
        let max_effort = tasks
            .iter()
            .map(|task| task.estimated_effort)
            .max()
            .unwrap_or(Duration::from_secs(3600));

        // Add buffer time
        let buffer = max_effort.mul_f64(config.phase_buffer_factor);
        max_effort + buffer
    }

    /// Calculate parallelism factor for the plan
    fn calculate_parallelism_factor(&self, phases: &[ExecutionPhase]) -> f64 {
        if phases.is_empty() {
            return 1.0;
        }

        let total_tasks: usize = phases.iter().map(|p| p.tasks.len()).sum();
        let sequential_phases = phases.len();

        if sequential_phases == 0 {
            1.0
        } else {
            total_tasks as f64 / sequential_phases as f64
        }
    }

    /// Find critical path in the execution plan
    fn find_critical_path(&self, phases: &[ExecutionPhase]) -> u32 {
        // Simple heuristic: number of phases is the critical path length
        phases.len() as u32
    }

    /// Calculate plan confidence score
    fn calculate_confidence_score(
        &self,
        tasks: &[Task],
        phases: &[ExecutionPhase],
        parallelism_factor: f64,
    ) -> f64 {
        let mut score = 0.0;

        // Factor 1: Task distribution balance
        if !phases.is_empty() {
            let phase_sizes: Vec<usize> = phases.iter().map(|p| p.tasks.len()).collect();
            let mean_size = phase_sizes.iter().sum::<usize>() as f64 / phase_sizes.len() as f64;
            let variance = phase_sizes
                .iter()
                .map(|&size| (size as f64 - mean_size).powi(2))
                .sum::<f64>()
                / phase_sizes.len() as f64;

            let balance_score = 1.0 / (1.0 + variance);
            score += balance_score * 0.4;
        }

        // Factor 2: Parallelism utilization
        let parallelism_score = parallelism_factor.min(4.0) / 4.0; // Cap at 4x parallelism
        score += parallelism_score * 0.3;

        // Factor 3: Task complexity distribution
        let complexity_scores: Vec<u32> = tasks.iter().map(|t| t.complexity.score()).collect();
        let complexity_variance = if !complexity_scores.is_empty() {
            let mean =
                complexity_scores.iter().sum::<u32>() as f64 / complexity_scores.len() as f64;
            complexity_scores
                .iter()
                .map(|&score| (score as f64 - mean).powi(2))
                .sum::<f64>()
                / complexity_scores.len() as f64
        } else {
            0.0
        };

        let complexity_score = 1.0 / (1.0 + complexity_variance);
        score += complexity_score * 0.3;

        score.min(1.0).max(0.0)
    }
}

impl Default for KnowledgeGraphExecutionPlanner {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ExecutionPlanner for KnowledgeGraphExecutionPlanner {
    async fn create_plan(
        &self,
        decomposition: &DecompositionResult,
        config: &PlanningConfig,
    ) -> TaskDecompositionResult<ExecutionPlan> {
        info!(
            "Creating execution plan for {} tasks",
            decomposition.subtasks.len()
        );

        // Check cache
        let cache_key = format!(
            "{}_{:?}",
            decomposition.original_task, config.optimization_strategy
        );
        {
            let cache = self.cache.read().await;
            if let Some(cached_plan) = cache.get(&cache_key) {
                debug!("Using cached execution plan");
                return Ok(cached_plan.clone());
            }
        }

        // Perform topological sort to determine execution phases
        let phase_tasks =
            self.topological_sort(&decomposition.subtasks, &decomposition.dependencies)?;

        let mut phases = Vec::new();
        let mut total_duration = Duration::from_secs(0);

        for (phase_num, task_ids) in phase_tasks.iter().enumerate() {
            // Get task references for this phase
            let phase_task_refs: Vec<&Task> = task_ids
                .iter()
                .filter_map(|id| decomposition.subtasks.iter().find(|t| &t.task_id == id))
                .collect();

            if phase_task_refs.is_empty() {
                continue;
            }

            // Calculate phase duration
            let phase_duration = self.calculate_phase_duration(&phase_task_refs, config);
            total_duration += phase_duration;

            // Collect required agents
            let required_agents: Vec<AgentPid> = phase_task_refs
                .iter()
                .flat_map(|task| task.assigned_agents.iter().cloned())
                .collect::<HashSet<_>>()
                .into_iter()
                .collect();

            // Determine phase dependencies
            let dependencies = if phase_num == 0 {
                Vec::new()
            } else {
                vec![(phase_num - 1) as u32]
            };

            let phase = ExecutionPhase {
                phase_number: phase_num as u32,
                tasks: task_ids.clone(),
                estimated_duration: phase_duration,
                required_agents,
                dependencies,
            };

            phases.push(phase);
        }

        // Calculate resource requirements
        let all_task_refs: Vec<&Task> = decomposition.subtasks.iter().collect();
        let resource_requirements = self.calculate_resource_requirements(&all_task_refs);

        // Calculate metadata
        let parallelism_factor = self.calculate_parallelism_factor(&phases);
        let critical_path_length = self.find_critical_path(&phases);
        let confidence_score =
            self.calculate_confidence_score(&decomposition.subtasks, &phases, parallelism_factor);

        let plan = ExecutionPlan {
            plan_id: format!("plan_{}", decomposition.original_task),
            tasks: decomposition
                .subtasks
                .iter()
                .map(|t| t.task_id.clone())
                .collect(),
            phases,
            estimated_duration: total_duration,
            resource_requirements,
            metadata: PlanMetadata {
                created_at: Utc::now(),
                created_by: "system".to_string(),
                version: 1,
                optimization_strategy: config.optimization_strategy.clone(),
                parallelism_factor,
                critical_path_length,
                confidence_score,
            },
        };

        // Cache the plan
        {
            let mut cache = self.cache.write().await;
            cache.insert(cache_key, plan.clone());
        }

        info!(
            "Created execution plan with {} phases, estimated duration: {:?}",
            plan.phases.len(),
            plan.estimated_duration
        );

        Ok(plan)
    }

    async fn optimize_plan(
        &self,
        plan: &ExecutionPlan,
        _config: &PlanningConfig,
    ) -> TaskDecompositionResult<ExecutionPlan> {
        // For now, return the original plan
        // TODO: Implement optimization algorithms based on strategy
        debug!("Plan optimization not yet implemented, returning original plan");
        Ok(plan.clone())
    }

    async fn validate_plan(&self, plan: &ExecutionPlan) -> TaskDecompositionResult<bool> {
        // Basic validation checks
        if plan.phases.is_empty() {
            return Ok(false);
        }

        // Check that all tasks are included in phases
        let phase_tasks: HashSet<TaskId> = plan
            .phases
            .iter()
            .flat_map(|p| p.tasks.iter().cloned())
            .collect();

        let plan_tasks: HashSet<TaskId> = plan.tasks.iter().cloned().collect();

        if phase_tasks != plan_tasks {
            return Ok(false);
        }

        // Check phase dependencies are valid
        for phase in &plan.phases {
            for &dep_phase in &phase.dependencies {
                if dep_phase >= phase.phase_number {
                    return Ok(false); // Invalid dependency
                }
            }
        }

        // Check confidence score threshold
        if plan.metadata.confidence_score < 0.3 {
            return Ok(false);
        }

        Ok(true)
    }

    async fn update_plan(
        &self,
        plan: &ExecutionPlan,
        _task_updates: &HashMap<TaskId, TaskStatus>,
    ) -> TaskDecompositionResult<ExecutionPlan> {
        // For now, return the original plan
        // TODO: Implement plan updates based on task status changes
        debug!("Plan updates not yet implemented, returning original plan");
        Ok(plan.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        DecompositionMetadata, DecompositionResult, DecompositionStrategy, Task, TaskComplexity,
    };

    fn create_test_decomposition() -> DecompositionResult {
        let task1 = Task::new(
            "task1".to_string(),
            "Task 1".to_string(),
            TaskComplexity::Simple,
            1,
        );
        let task2 = Task::new(
            "task2".to_string(),
            "Task 2".to_string(),
            TaskComplexity::Simple,
            1,
        );
        let task3 = Task::new(
            "task3".to_string(),
            "Task 3".to_string(),
            TaskComplexity::Simple,
            1,
        );

        let mut dependencies = HashMap::new();
        dependencies.insert("task2".to_string(), vec!["task1".to_string()]);
        dependencies.insert("task3".to_string(), vec!["task2".to_string()]);

        DecompositionResult {
            original_task: "original".to_string(),
            subtasks: vec![task1, task2, task3],
            dependencies,
            metadata: DecompositionMetadata {
                strategy_used: DecompositionStrategy::KnowledgeGraphBased,
                depth: 1,
                subtask_count: 3,
                concepts_analyzed: vec!["concept1".to_string(), "concept2".to_string()],
                roles_identified: Vec::new(),
                confidence_score: 0.8,
                parallelism_factor: 0.5,
            },
        }
    }

    #[tokio::test]
    async fn test_execution_planner_creation() {
        let planner = KnowledgeGraphExecutionPlanner::new();
        assert!(planner.cache.read().await.is_empty());
    }

    #[tokio::test]
    async fn test_create_execution_plan() {
        let planner = KnowledgeGraphExecutionPlanner::new();
        let decomposition = create_test_decomposition();
        let config = PlanningConfig::default();

        let result = planner.create_plan(&decomposition, &config).await;
        assert!(result.is_ok());

        let plan = result.unwrap();
        assert_eq!(plan.tasks.len(), 3);
        assert!(!plan.phases.is_empty());
        assert!(plan.estimated_duration > Duration::from_secs(0));
    }

    #[tokio::test]
    async fn test_topological_sort() {
        let planner = KnowledgeGraphExecutionPlanner::new();
        let decomposition = create_test_decomposition();

        let result = planner.topological_sort(&decomposition.subtasks, &decomposition.dependencies);
        assert!(result.is_ok());

        let phases = result.unwrap();
        assert_eq!(phases.len(), 3); // Sequential execution due to dependencies
        assert_eq!(phases[0], vec!["task1".to_string()]);
        assert_eq!(phases[1], vec!["task2".to_string()]);
        assert_eq!(phases[2], vec!["task3".to_string()]);
    }

    #[tokio::test]
    async fn test_plan_validation() {
        let planner = KnowledgeGraphExecutionPlanner::new();
        let decomposition = create_test_decomposition();
        let config = PlanningConfig::default();

        let plan = planner.create_plan(&decomposition, &config).await.unwrap();
        let is_valid = planner.validate_plan(&plan).await.unwrap();
        assert!(is_valid);
    }

    #[test]
    fn test_resource_requirements_defaults() {
        let requirements = ResourceRequirements::default();
        assert_eq!(requirements.memory_mb, 512);
        assert_eq!(requirements.cpu_cores, 1);
        assert_eq!(requirements.network_mbps, 10);
        assert_eq!(requirements.storage_mb, 100);
    }

    #[test]
    fn test_planning_config_defaults() {
        let config = PlanningConfig::default();
        assert_eq!(config.optimization_strategy, OptimizationStrategy::Balanced);
        assert_eq!(config.max_parallel_tasks, 10);
        assert!(config.consider_agent_capabilities);
        assert_eq!(config.phase_buffer_factor, 0.1);
        assert!(config.optimize_for_fault_tolerance);
    }
}
