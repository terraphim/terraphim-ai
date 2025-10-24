//! Orchestrator-Workers workflow pattern
//!
//! This pattern implements hierarchical task execution where an orchestrator agent
//! plans and coordinates work, while worker agents execute specific subtasks.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use chrono::Utc;
use futures::future::join_all;
use serde::{Deserialize, Serialize};

use crate::{
    workflows::{
        ExecutionStep, ResourceUsage, StepType, TaskAnalysis, TaskComplexity, WorkflowInput,
        WorkflowMetadata, WorkflowOutput, WorkflowPattern,
    },
    CompletionOptions, EvolutionResult, LlmAdapter,
};

/// Orchestrator-Workers workflow with hierarchical task management
pub struct OrchestratorWorkers {
    orchestrator_adapter: Arc<dyn LlmAdapter>,
    worker_adapters: HashMap<WorkerRole, Arc<dyn LlmAdapter>>,
    orchestration_config: OrchestrationConfig,
}

/// Configuration for orchestrator-workers execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationConfig {
    pub max_planning_iterations: usize,
    pub max_workers: usize,
    pub worker_timeout: Duration,
    pub coordination_strategy: CoordinationStrategy,
    pub quality_gate_threshold: f64,
    pub enable_worker_feedback: bool,
}

impl Default for OrchestrationConfig {
    fn default() -> Self {
        Self {
            max_planning_iterations: 3,
            max_workers: 6,
            worker_timeout: Duration::from_secs(180),
            coordination_strategy: CoordinationStrategy::Sequential,
            quality_gate_threshold: 0.7,
            enable_worker_feedback: true,
        }
    }
}

/// Strategy for coordinating workers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoordinationStrategy {
    /// Execute workers one after another
    Sequential,
    /// Execute workers in parallel with coordination
    ParallelCoordinated,
    /// Execute in pipeline stages
    Pipeline,
    /// Dynamic scheduling based on dependencies
    Dynamic,
}

/// Specialized worker roles
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkerRole {
    Analyst,
    Researcher,
    Writer,
    Reviewer,
    Validator,
    Synthesizer,
}

/// Task assigned to a worker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerTask {
    pub task_id: String,
    pub worker_role: WorkerRole,
    pub instruction: String,
    pub context: String,
    pub dependencies: Vec<String>,
    pub priority: TaskPriority,
    pub expected_deliverable: String,
    pub quality_criteria: Vec<String>,
}

/// Priority levels for worker tasks
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    Low,
    Medium,
    High,
    Critical,
}

/// Execution plan created by the orchestrator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    pub plan_id: String,
    pub description: String,
    pub worker_tasks: Vec<WorkerTask>,
    pub execution_order: Vec<String>,
    pub success_criteria: Vec<String>,
    pub estimated_duration: Duration,
}

/// Result from a worker execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerResult {
    pub task_id: String,
    pub worker_role: WorkerRole,
    pub deliverable: String,
    pub success: bool,
    pub quality_score: f64,
    pub execution_time: Duration,
    pub feedback: Option<String>,
    pub dependencies_met: bool,
}

/// Coordination message between orchestrator and workers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinationMessage {
    pub message_type: MessageType,
    pub task_id: String,
    pub content: String,
    pub sender: String,
}

/// Types of coordination messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    TaskAssignment,
    ProgressUpdate,
    QualityFeedback,
    DependencyNotification,
    Coordination,
}

impl OrchestratorWorkers {
    /// Create a new orchestrator-workers workflow
    pub fn new(orchestrator_adapter: Arc<dyn LlmAdapter>) -> Self {
        let mut worker_adapters = HashMap::new();

        // Use the same adapter for all roles initially (can be specialized later)
        for role in [
            WorkerRole::Analyst,
            WorkerRole::Researcher,
            WorkerRole::Writer,
            WorkerRole::Reviewer,
            WorkerRole::Validator,
            WorkerRole::Synthesizer,
        ] {
            worker_adapters.insert(role, orchestrator_adapter.clone());
        }

        Self {
            orchestrator_adapter,
            worker_adapters,
            orchestration_config: OrchestrationConfig::default(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(
        orchestrator_adapter: Arc<dyn LlmAdapter>,
        config: OrchestrationConfig,
    ) -> Self {
        let mut instance = Self::new(orchestrator_adapter);
        instance.orchestration_config = config;
        instance
    }

    /// Add a specialized worker adapter
    pub fn add_worker(mut self, role: WorkerRole, adapter: Arc<dyn LlmAdapter>) -> Self {
        self.worker_adapters.insert(role, adapter);
        self
    }

    /// Execute orchestrated workflow
    async fn execute_orchestrated_workflow(
        &self,
        input: &WorkflowInput,
    ) -> EvolutionResult<WorkflowOutput> {
        let start_time = Instant::now();
        let mut execution_trace = Vec::new();
        let mut resource_usage = ResourceUsage::default();

        // Phase 1: Planning
        log::info!("Orchestrator planning phase for task: {}", input.task_id);
        let execution_plan = self
            .create_execution_plan(&input.prompt, &input.context)
            .await?;

        execution_trace.push(ExecutionStep {
            step_id: "orchestrator_planning".to_string(),
            step_type: StepType::Decomposition,
            input: input.prompt.clone(),
            output: format!(
                "Created execution plan with {} tasks",
                execution_plan.worker_tasks.len()
            ),
            duration: start_time.elapsed(),
            success: true,
            metadata: serde_json::json!({
                "plan_id": execution_plan.plan_id,
                "worker_count": execution_plan.worker_tasks.len(),
                "estimated_duration": execution_plan.estimated_duration.as_secs(),
            }),
        });

        resource_usage.llm_calls += 1;

        // Phase 2: Worker Execution
        log::info!(
            "Executing {} worker tasks",
            execution_plan.worker_tasks.len()
        );
        let worker_results = self.execute_workers(&execution_plan).await?;

        // Add worker execution steps to trace
        for result in &worker_results {
            execution_trace.push(ExecutionStep {
                step_id: result.task_id.clone(),
                step_type: StepType::LlmCall,
                input: format!("Worker task for {:?}", result.worker_role),
                output: result.deliverable.clone(),
                duration: result.execution_time,
                success: result.success,
                metadata: serde_json::json!({
                    "worker_role": result.worker_role,
                    "quality_score": result.quality_score,
                    "dependencies_met": result.dependencies_met,
                }),
            });
            resource_usage.llm_calls += 1;
        }

        // Phase 3: Quality Gate
        let quality_gate_passed = self.evaluate_quality_gate(&worker_results).await?;
        if !quality_gate_passed {
            return Err(crate::EvolutionError::WorkflowError(
                "Quality gate failed - results do not meet threshold".to_string(),
            ));
        }

        // Phase 4: Final Synthesis
        let final_result = self
            .synthesize_worker_results(&worker_results, &input.prompt)
            .await?;

        execution_trace.push(ExecutionStep {
            step_id: "final_synthesis".to_string(),
            step_type: StepType::Aggregation,
            input: format!("Synthesizing {} worker results", worker_results.len()),
            output: final_result.clone(),
            duration: Duration::from_millis(100), // Rough estimate
            success: true,
            metadata: serde_json::json!({
                "coordination_strategy": format!("{:?}", self.orchestration_config.coordination_strategy),
                "quality_gate_passed": quality_gate_passed,
            }),
        });

        resource_usage.llm_calls += 1;
        resource_usage.tokens_consumed = self.estimate_token_consumption(&execution_trace);
        resource_usage.parallel_tasks = worker_results.len();

        let overall_quality = self.calculate_overall_quality(&worker_results);

        let metadata = WorkflowMetadata {
            pattern_used: "orchestrator_workers".to_string(),
            execution_time: start_time.elapsed(),
            steps_executed: execution_trace.len(),
            success: true,
            quality_score: Some(overall_quality),
            resources_used: resource_usage,
        };

        Ok(WorkflowOutput {
            task_id: input.task_id.clone(),
            agent_id: input.agent_id.clone(),
            result: final_result,
            metadata,
            execution_trace,
            timestamp: Utc::now(),
        })
    }

    /// Create execution plan using the orchestrator
    async fn create_execution_plan(
        &self,
        prompt: &str,
        context: &Option<String>,
    ) -> EvolutionResult<ExecutionPlan> {
        let context_str = context.as_deref().unwrap_or("");
        let planning_prompt = format!(
            r#"You are an expert orchestrator responsible for creating detailed execution plans.

Task: {}

Context: {}

Create a comprehensive execution plan that breaks down this task into specific worker assignments. Consider:

1. What specialized workers (Analyst, Researcher, Writer, Reviewer, Validator, Synthesizer) are needed?
2. What are the specific deliverables for each worker?
3. What dependencies exist between tasks?
4. What quality criteria should be applied?

Provide a structured plan with:
- Clear task assignments for each worker role
- Specific instructions and expected deliverables
- Dependencies between tasks
- Success criteria for the overall execution

Format your response as a detailed execution plan."#,
            prompt, context_str
        );

        let planning_result = self
            .orchestrator_adapter
            .complete(&planning_prompt, CompletionOptions::default())
            .await
            .map_err(|e| crate::EvolutionError::WorkflowError(format!("Planning failed: {}", e)))?;

        // Parse the planning result into structured tasks
        let worker_tasks = self.parse_worker_tasks(&planning_result, prompt)?;
        let execution_order = self.determine_execution_order(&worker_tasks)?;

        Ok(ExecutionPlan {
            plan_id: format!("plan_{}", uuid::Uuid::new_v4()),
            description: planning_result,
            worker_tasks,
            execution_order,
            success_criteria: vec![
                "All worker tasks completed successfully".to_string(),
                "Quality criteria met for each deliverable".to_string(),
                "Final synthesis provides comprehensive response".to_string(),
            ],
            estimated_duration: Duration::from_secs(300), // 5 minutes estimate
        })
    }

    /// Parse worker tasks from orchestrator planning output
    fn parse_worker_tasks(
        &self,
        planning_output: &str,
        original_prompt: &str,
    ) -> EvolutionResult<Vec<WorkerTask>> {
        // Simple task generation based on the planning output and task type
        let mut tasks = Vec::new();

        // Determine required workers based on task characteristics
        if planning_output.contains("research") || original_prompt.contains("research") {
            tasks.push(WorkerTask {
                task_id: "research_task".to_string(),
                worker_role: WorkerRole::Researcher,
                instruction: format!("Research background information for: {}", original_prompt),
                context: planning_output.to_string(),
                dependencies: vec![],
                priority: TaskPriority::High,
                expected_deliverable: "Comprehensive research findings".to_string(),
                quality_criteria: vec!["Accuracy".to_string(), "Completeness".to_string()],
            });
        }

        if planning_output.contains("analy") || original_prompt.contains("analy") {
            tasks.push(WorkerTask {
                task_id: "analysis_task".to_string(),
                worker_role: WorkerRole::Analyst,
                instruction: format!("Analyze the key aspects of: {}", original_prompt),
                context: planning_output.to_string(),
                dependencies: if tasks.is_empty() {
                    vec![]
                } else {
                    vec!["research_task".to_string()]
                },
                priority: TaskPriority::High,
                expected_deliverable: "Detailed analysis with insights".to_string(),
                quality_criteria: vec!["Depth".to_string(), "Clarity".to_string()],
            });
        }

        // Always include a writer for content generation
        tasks.push(WorkerTask {
            task_id: "writing_task".to_string(),
            worker_role: WorkerRole::Writer,
            instruction: format!("Create well-structured content for: {}", original_prompt),
            context: planning_output.to_string(),
            dependencies: tasks.iter().map(|t| t.task_id.clone()).collect(),
            priority: TaskPriority::Medium,
            expected_deliverable: "Well-written response".to_string(),
            quality_criteria: vec!["Clarity".to_string(), "Structure".to_string()],
        });

        // Add reviewer for quality assurance
        tasks.push(WorkerTask {
            task_id: "review_task".to_string(),
            worker_role: WorkerRole::Reviewer,
            instruction: "Review and provide feedback on the generated content".to_string(),
            context: planning_output.to_string(),
            dependencies: vec!["writing_task".to_string()],
            priority: TaskPriority::Medium,
            expected_deliverable: "Quality review with recommendations".to_string(),
            quality_criteria: vec!["Thoroughness".to_string(), "Constructiveness".to_string()],
        });

        // Add synthesizer for final integration
        tasks.push(WorkerTask {
            task_id: "synthesis_task".to_string(),
            worker_role: WorkerRole::Synthesizer,
            instruction: "Synthesize all worker contributions into final response".to_string(),
            context: planning_output.to_string(),
            dependencies: tasks.iter().map(|t| t.task_id.clone()).collect(),
            priority: TaskPriority::Critical,
            expected_deliverable: "Final synthesized response".to_string(),
            quality_criteria: vec!["Coherence".to_string(), "Completeness".to_string()],
        });

        Ok(tasks)
    }

    /// Determine execution order based on dependencies
    fn determine_execution_order(&self, tasks: &[WorkerTask]) -> EvolutionResult<Vec<String>> {
        let mut order = Vec::new();
        let mut remaining_tasks: HashMap<String, &WorkerTask> =
            tasks.iter().map(|t| (t.task_id.clone(), t)).collect();

        // Simple topological sort
        while !remaining_tasks.is_empty() {
            let ready_tasks: Vec<_> = remaining_tasks
                .iter()
                .filter(|(_, task)| task.dependencies.iter().all(|dep| order.contains(dep)))
                .map(|(id, _)| id.clone())
                .collect();

            if ready_tasks.is_empty() {
                return Err(crate::EvolutionError::WorkflowError(
                    "Circular dependency detected in worker tasks".to_string(),
                ));
            }

            for task_id in ready_tasks {
                order.push(task_id.clone());
                remaining_tasks.remove(&task_id);
            }
        }

        Ok(order)
    }

    /// Execute all workers according to the coordination strategy
    async fn execute_workers(&self, plan: &ExecutionPlan) -> EvolutionResult<Vec<WorkerResult>> {
        match self.orchestration_config.coordination_strategy {
            CoordinationStrategy::Sequential => self.execute_workers_sequential(plan).await,
            CoordinationStrategy::ParallelCoordinated => {
                self.execute_workers_parallel_coordinated(plan).await
            }
            CoordinationStrategy::Pipeline => self.execute_workers_pipeline(plan).await,
            CoordinationStrategy::Dynamic => self.execute_workers_dynamic(plan).await,
        }
    }

    /// Execute workers sequentially
    async fn execute_workers_sequential(
        &self,
        plan: &ExecutionPlan,
    ) -> EvolutionResult<Vec<WorkerResult>> {
        let mut results = Vec::new();
        let mut context_accumulator = String::new();

        for task_id in &plan.execution_order {
            let task = plan
                .worker_tasks
                .iter()
                .find(|t| t.task_id == *task_id)
                .ok_or_else(|| {
                    crate::EvolutionError::WorkflowError(format!(
                        "Task {} not found in plan",
                        task_id
                    ))
                })?;

            let result = self
                .execute_single_worker(task, &context_accumulator)
                .await?;

            // Accumulate context for subsequent workers
            if result.success {
                context_accumulator.push_str(&format!(
                    "\n\n{:?}:\n{}",
                    task.worker_role, result.deliverable
                ));
            }

            results.push(result);
        }

        Ok(results)
    }

    /// Execute workers in parallel with coordination
    async fn execute_workers_parallel_coordinated(
        &self,
        plan: &ExecutionPlan,
    ) -> EvolutionResult<Vec<WorkerResult>> {
        // Group tasks by dependency level
        let mut dependency_levels: Vec<Vec<&WorkerTask>> = Vec::new();
        let mut processed_tasks = std::collections::HashSet::new();

        while processed_tasks.len() < plan.worker_tasks.len() {
            let mut current_level = Vec::new();

            for task in &plan.worker_tasks {
                if processed_tasks.contains(&task.task_id) {
                    continue;
                }

                let dependencies_met = task
                    .dependencies
                    .iter()
                    .all(|dep| processed_tasks.contains(dep));

                if dependencies_met {
                    current_level.push(task);
                }
            }

            if current_level.is_empty() {
                return Err(crate::EvolutionError::WorkflowError(
                    "Unable to resolve task dependencies".to_string(),
                ));
            }

            for task in &current_level {
                processed_tasks.insert(task.task_id.clone());
            }

            dependency_levels.push(current_level);
        }

        // Execute each level in parallel
        let mut all_results = Vec::new();
        let mut accumulated_context = String::new();

        for level in dependency_levels {
            let level_futures: Vec<_> = level
                .iter()
                .map(|task| self.execute_single_worker(task, &accumulated_context))
                .collect();

            let level_results = join_all(level_futures).await;

            for result in level_results {
                let worker_result = result?;
                if worker_result.success {
                    accumulated_context.push_str(&format!(
                        "\n\n{:?}:\n{}",
                        worker_result.worker_role, worker_result.deliverable
                    ));
                }
                all_results.push(worker_result);
            }
        }

        Ok(all_results)
    }

    /// Execute workers in pipeline fashion (simplified - same as sequential for now)
    async fn execute_workers_pipeline(
        &self,
        plan: &ExecutionPlan,
    ) -> EvolutionResult<Vec<WorkerResult>> {
        // For now, pipeline is implemented as sequential execution
        // In a more advanced implementation, this could use streaming between workers
        self.execute_workers_sequential(plan).await
    }

    /// Execute workers with dynamic scheduling
    async fn execute_workers_dynamic(
        &self,
        plan: &ExecutionPlan,
    ) -> EvolutionResult<Vec<WorkerResult>> {
        // For now, dynamic scheduling is implemented as parallel coordinated
        // In a more advanced implementation, this could dynamically adjust based on performance
        self.execute_workers_parallel_coordinated(plan).await
    }

    /// Execute a single worker task
    async fn execute_single_worker(
        &self,
        task: &WorkerTask,
        context: &str,
    ) -> EvolutionResult<WorkerResult> {
        let start_time = Instant::now();

        let worker_adapter = self.worker_adapters.get(&task.worker_role).ok_or_else(|| {
            crate::EvolutionError::WorkflowError(format!(
                "No adapter available for worker role: {:?}",
                task.worker_role
            ))
        })?;

        let worker_prompt = self.create_worker_prompt(task, context);

        log::debug!(
            "Executing worker task: {} ({:?})",
            task.task_id,
            task.worker_role
        );

        let result = tokio::time::timeout(
            self.orchestration_config.worker_timeout,
            worker_adapter.complete(&worker_prompt, CompletionOptions::default()),
        )
        .await;

        let execution_time = start_time.elapsed();

        match result {
            Ok(Ok(deliverable)) => {
                let quality_score = self.assess_worker_quality(&deliverable, task);

                Ok(WorkerResult {
                    task_id: task.task_id.clone(),
                    worker_role: task.worker_role.clone(),
                    deliverable,
                    success: true,
                    quality_score,
                    execution_time,
                    feedback: None,
                    dependencies_met: true, // Simplified - would check actual dependencies
                })
            }
            Ok(Err(e)) => {
                log::warn!("Worker task {} failed: {}", task.task_id, e);
                Ok(WorkerResult {
                    task_id: task.task_id.clone(),
                    worker_role: task.worker_role.clone(),
                    deliverable: format!("Task failed: {}", e),
                    success: false,
                    quality_score: 0.0,
                    execution_time,
                    feedback: Some(format!("Execution error: {}", e)),
                    dependencies_met: true,
                })
            }
            Err(_) => {
                log::warn!("Worker task {} timed out", task.task_id);
                Ok(WorkerResult {
                    task_id: task.task_id.clone(),
                    worker_role: task.worker_role.clone(),
                    deliverable: "Task timed out".to_string(),
                    success: false,
                    quality_score: 0.0,
                    execution_time,
                    feedback: Some("Task execution timed out".to_string()),
                    dependencies_met: true,
                })
            }
        }
    }

    /// Create specialized prompt for each worker role
    fn create_worker_prompt(&self, task: &WorkerTask, context: &str) -> String {
        let role_instructions = match task.worker_role {
            WorkerRole::Analyst => "You are a skilled analyst. Focus on breaking down complex information, identifying patterns, and providing insights.",
            WorkerRole::Researcher => "You are a thorough researcher. Gather comprehensive information, verify facts, and provide well-sourced findings.",
            WorkerRole::Writer => "You are an expert writer. Create clear, engaging, and well-structured content that effectively communicates ideas.",
            WorkerRole::Reviewer => "You are a meticulous reviewer. Evaluate content for quality, accuracy, completeness, and provide constructive feedback.",
            WorkerRole::Validator => "You are a validation specialist. Verify claims, check consistency, and ensure accuracy of information.",
            WorkerRole::Synthesizer => "You are a synthesis expert. Combine multiple inputs into a coherent, comprehensive, and well-integrated response.",
        };

        let context_section = if context.is_empty() {
            String::new()
        } else {
            format!("\n\nPrevious work context:\n{}\n", context)
        };

        format!(
            "{}\n\nTask: {}\n\nInstructions: {}\n\nExpected deliverable: {}\n\nQuality criteria: {}{}\n\nProvide your response:",
            role_instructions,
            task.instruction,
            task.instruction,
            task.expected_deliverable,
            task.quality_criteria.join(", "),
            context_section
        )
    }

    /// Assess quality of worker output
    fn assess_worker_quality(&self, deliverable: &str, task: &WorkerTask) -> f64 {
        let mut score: f64 = 0.5; // Base score

        // Length assessment
        match deliverable.len() {
            0..=50 => score -= 0.3,
            51..=200 => score += 0.1,
            201..=1000 => score += 0.2,
            _ => score += 0.3,
        }

        // Role-specific quality checks
        match task.worker_role {
            WorkerRole::Analyst => {
                if deliverable.contains("analysis") || deliverable.contains("insight") {
                    score += 0.2;
                }
            }
            WorkerRole::Researcher => {
                if deliverable.contains("research") || deliverable.contains("finding") {
                    score += 0.2;
                }
            }
            WorkerRole::Writer => {
                if deliverable.split_whitespace().count() > 100 {
                    score += 0.2;
                }
            }
            _ => {}
        }

        // Quality criteria matching
        for criterion in &task.quality_criteria {
            match criterion.to_lowercase().as_str() {
                "accuracy" if deliverable.contains("accurate") => score += 0.1,
                "completeness" if deliverable.len() > 300 => score += 0.1,
                "clarity" if !deliverable.contains("unclear") => score += 0.1,
                _ => {}
            }
        }

        score.clamp(0.0, 1.0)
    }

    /// Evaluate quality gate for all worker results
    async fn evaluate_quality_gate(&self, results: &[WorkerResult]) -> EvolutionResult<bool> {
        let successful_results: Vec<_> = results.iter().filter(|r| r.success).collect();

        if successful_results.is_empty() {
            return Ok(false);
        }

        let average_quality: f64 = successful_results
            .iter()
            .map(|r| r.quality_score)
            .sum::<f64>()
            / successful_results.len() as f64;

        let success_rate = successful_results.len() as f64 / results.len() as f64;

        log::info!(
            "Quality gate evaluation: avg_quality={:.2}, success_rate={:.2}, threshold={:.2}",
            average_quality,
            success_rate,
            self.orchestration_config.quality_gate_threshold
        );

        Ok(
            average_quality >= self.orchestration_config.quality_gate_threshold
                && success_rate >= 0.5,
        )
    }

    /// Synthesize worker results into final output
    async fn synthesize_worker_results(
        &self,
        results: &[WorkerResult],
        original_prompt: &str,
    ) -> EvolutionResult<String> {
        let successful_results: Vec<_> = results.iter().filter(|r| r.success).collect();

        if successful_results.is_empty() {
            return Ok("No successful results to synthesize".to_string());
        }

        let synthesis_input = successful_results
            .iter()
            .map(|r| format!("{:?} contribution:\n{}\n", r.worker_role, r.deliverable))
            .collect::<Vec<_>>()
            .join("\n");

        let synthesis_prompt = format!(
            "Original request: {}\n\nWorker contributions:\n{}\n\nSynthesize these contributions into a comprehensive, coherent response that addresses the original request:",
            original_prompt,
            synthesis_input
        );

        self.orchestrator_adapter
            .complete(&synthesis_prompt, CompletionOptions::default())
            .await
            .map_err(|e| crate::EvolutionError::WorkflowError(format!("Synthesis failed: {}", e)))
    }

    /// Calculate overall quality from worker results
    fn calculate_overall_quality(&self, results: &[WorkerResult]) -> f64 {
        let successful_results: Vec<_> = results.iter().filter(|r| r.success).collect();

        if successful_results.is_empty() {
            return 0.0;
        }

        successful_results
            .iter()
            .map(|r| r.quality_score)
            .sum::<f64>()
            / successful_results.len() as f64
    }

    /// Estimate token consumption from execution trace
    fn estimate_token_consumption(&self, trace: &[ExecutionStep]) -> usize {
        trace
            .iter()
            .map(|step| step.input.len() + step.output.len())
            .sum()
    }
}

#[async_trait]
impl WorkflowPattern for OrchestratorWorkers {
    fn pattern_name(&self) -> &'static str {
        "orchestrator_workers"
    }

    async fn execute(&self, input: WorkflowInput) -> EvolutionResult<WorkflowOutput> {
        log::info!(
            "Executing orchestrator-workers workflow for task: {}",
            input.task_id
        );
        self.execute_orchestrated_workflow(&input).await
    }

    fn is_suitable_for(&self, task_analysis: &TaskAnalysis) -> bool {
        // Orchestrator-workers is suitable for:
        // - Complex and very complex tasks that require decomposition
        // - Tasks that benefit from specialized roles
        // - Multi-step processes that need coordination

        matches!(
            task_analysis.complexity,
            TaskComplexity::Complex | TaskComplexity::VeryComplex
        ) || task_analysis.requires_decomposition
            || task_analysis.estimated_steps > 3
    }

    fn estimate_execution_time(&self, input: &WorkflowInput) -> Duration {
        // Estimate based on complexity and number of expected workers
        let base_time = Duration::from_secs(if input.prompt.len() > 2000 { 120 } else { 60 });
        let estimated_workers = if input.prompt.len() > 1000 { 5 } else { 3 };

        // Add coordination overhead
        match self.orchestration_config.coordination_strategy {
            CoordinationStrategy::Sequential => base_time * estimated_workers,
            CoordinationStrategy::ParallelCoordinated => base_time + Duration::from_secs(30),
            CoordinationStrategy::Pipeline => base_time + Duration::from_secs(60),
            CoordinationStrategy::Dynamic => base_time + Duration::from_secs(45),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orchestration_config_default() {
        let config = OrchestrationConfig::default();
        assert_eq!(config.max_planning_iterations, 3);
        assert_eq!(config.max_workers, 6);
        assert_eq!(config.worker_timeout, Duration::from_secs(180));
        assert_eq!(config.quality_gate_threshold, 0.7);
    }

    #[test]
    fn test_worker_role_variants() {
        let roles = vec![
            WorkerRole::Analyst,
            WorkerRole::Researcher,
            WorkerRole::Writer,
            WorkerRole::Reviewer,
            WorkerRole::Validator,
            WorkerRole::Synthesizer,
        ];

        assert_eq!(roles.len(), 6);

        // Test that roles can be used as HashMap keys
        let mut role_map = HashMap::new();
        for role in roles {
            role_map.insert(role, "test");
        }
        assert_eq!(role_map.len(), 6);
    }

    #[test]
    fn test_task_priority_ordering() {
        let mut priorities = vec![
            TaskPriority::Low,
            TaskPriority::Critical,
            TaskPriority::Medium,
            TaskPriority::High,
        ];
        priorities.sort();

        assert_eq!(
            priorities,
            vec![
                TaskPriority::Low,
                TaskPriority::Medium,
                TaskPriority::High,
                TaskPriority::Critical,
            ]
        );
    }
}
