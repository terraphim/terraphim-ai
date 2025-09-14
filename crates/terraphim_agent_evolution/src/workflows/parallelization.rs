//! Parallelization workflow pattern
//!
//! This pattern executes multiple prompts concurrently and aggregates their results.
//! It's ideal for tasks that can be decomposed into independent subtasks.

use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use chrono::Utc;
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use tokio::time::timeout;

use crate::{
    workflows::{
        ExecutionStep, ResourceUsage, StepType, TaskAnalysis, TaskComplexity, WorkflowInput,
        WorkflowMetadata, WorkflowOutput, WorkflowPattern,
    },
    CompletionOptions, EvolutionResult, LlmAdapter,
};

/// Parallelization workflow that executes multiple prompts concurrently
pub struct Parallelization {
    llm_adapter: Arc<dyn LlmAdapter>,
    parallel_config: ParallelConfig,
}

/// Configuration for parallel execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelConfig {
    pub max_parallel_tasks: usize,
    pub task_timeout: Duration,
    pub aggregation_strategy: AggregationStrategy,
    pub failure_threshold: f64,
    pub retry_failed_tasks: bool,
}

impl Default for ParallelConfig {
    fn default() -> Self {
        Self {
            max_parallel_tasks: 4,
            task_timeout: Duration::from_secs(120),
            aggregation_strategy: AggregationStrategy::Concatenation,
            failure_threshold: 0.5, // 50% of tasks must succeed
            retry_failed_tasks: false,
        }
    }
}

/// Strategy for aggregating parallel results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AggregationStrategy {
    /// Simple concatenation of all results
    Concatenation,
    /// Best result based on quality scoring
    BestResult,
    /// Synthesis of all results using LLM
    Synthesis,
    /// Majority consensus for classification tasks
    MajorityVote,
    /// Structured combination with sections
    StructuredCombination,
}

/// Individual parallel task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelTask {
    pub task_id: String,
    pub prompt: String,
    pub description: String,
    pub priority: TaskPriority,
    pub expected_output_type: String,
}

/// Priority levels for parallel tasks
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Result from a parallel task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelTaskResult {
    pub task_id: String,
    pub result: Option<String>,
    pub success: bool,
    pub duration: Duration,
    pub error: Option<String>,
    pub quality_score: Option<f64>,
}

impl Parallelization {
    /// Create a new parallelization workflow
    pub fn new(llm_adapter: Arc<dyn LlmAdapter>) -> Self {
        Self {
            llm_adapter,
            parallel_config: ParallelConfig::default(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(llm_adapter: Arc<dyn LlmAdapter>, config: ParallelConfig) -> Self {
        Self {
            llm_adapter,
            parallel_config: config,
        }
    }

    /// Execute parallel tasks
    async fn execute_parallel_tasks(
        &self,
        input: &WorkflowInput,
    ) -> EvolutionResult<WorkflowOutput> {
        let start_time = Instant::now();
        let tasks = self.decompose_into_parallel_tasks(&input.prompt)?;

        log::info!(
            "Executing {} parallel tasks for workflow: {}",
            tasks.len(),
            input.task_id
        );

        // Execute tasks in batches to respect max_parallel_tasks limit
        let task_results = self.execute_task_batches(tasks).await?;

        // Check if we meet the failure threshold
        let success_count = task_results.iter().filter(|r| r.success).count();
        let success_rate = success_count as f64 / task_results.len() as f64;

        if success_rate < self.parallel_config.failure_threshold {
            return Err(crate::EvolutionError::WorkflowError(format!(
                "Parallel execution failed: only {:.1}% of tasks succeeded (threshold: {:.1}%)",
                success_rate * 100.0,
                self.parallel_config.failure_threshold * 100.0
            )));
        }

        // Aggregate successful results
        let successful_results: Vec<_> = task_results.iter().filter(|r| r.success).collect();

        let aggregated_result = self.aggregate_results(&successful_results).await?;

        // Create execution trace
        let execution_trace = self.create_execution_trace(&task_results, &aggregated_result);

        let resource_usage = ResourceUsage {
            llm_calls: task_results.len()
                + if matches!(
                    self.parallel_config.aggregation_strategy,
                    AggregationStrategy::Synthesis
                ) {
                    1
                } else {
                    0
                },
            tokens_consumed: self.estimate_tokens_consumed(&task_results, &aggregated_result),
            parallel_tasks: task_results.len(),
            memory_peak_mb: (task_results.len() as f64) * 5.0, // Rough estimate
        };

        let metadata = WorkflowMetadata {
            pattern_used: "parallelization".to_string(),
            execution_time: start_time.elapsed(),
            steps_executed: task_results.len(),
            success: true,
            quality_score: self.calculate_overall_quality_score(&task_results),
            resources_used: resource_usage,
        };

        Ok(WorkflowOutput {
            task_id: input.task_id.clone(),
            agent_id: input.agent_id.clone(),
            result: aggregated_result,
            metadata,
            execution_trace,
            timestamp: Utc::now(),
        })
    }

    /// Decompose input into parallel tasks
    fn decompose_into_parallel_tasks(&self, prompt: &str) -> EvolutionResult<Vec<ParallelTask>> {
        // Task decomposition based on prompt analysis
        if prompt.contains("compare") || prompt.contains("analyze different") {
            self.create_comparison_tasks(prompt)
        } else if prompt.contains("research") || prompt.contains("investigate") {
            self.create_research_tasks(prompt)
        } else if prompt.contains("generate") || prompt.contains("create multiple") {
            self.create_generation_tasks(prompt)
        } else if prompt.contains("evaluate") || prompt.contains("assess") {
            self.create_evaluation_tasks(prompt)
        } else {
            self.create_generic_parallel_tasks(prompt)
        }
    }

    /// Create tasks for comparison scenarios
    fn create_comparison_tasks(&self, prompt: &str) -> EvolutionResult<Vec<ParallelTask>> {
        Ok(vec![
            ParallelTask {
                task_id: "comparison_analysis".to_string(),
                prompt: format!("Analyze the key aspects and criteria for: {}", prompt),
                description: "Identify comparison criteria".to_string(),
                priority: TaskPriority::High,
                expected_output_type: "analysis".to_string(),
            },
            ParallelTask {
                task_id: "pros_cons".to_string(),
                prompt: format!("List the pros and cons for each option in: {}", prompt),
                description: "Evaluate advantages and disadvantages".to_string(),
                priority: TaskPriority::High,
                expected_output_type: "evaluation".to_string(),
            },
            ParallelTask {
                task_id: "recommendations".to_string(),
                prompt: format!("Provide recommendations based on: {}", prompt),
                description: "Generate actionable recommendations".to_string(),
                priority: TaskPriority::Normal,
                expected_output_type: "recommendations".to_string(),
            },
        ])
    }

    /// Create tasks for research scenarios
    fn create_research_tasks(&self, prompt: &str) -> EvolutionResult<Vec<ParallelTask>> {
        Ok(vec![
            ParallelTask {
                task_id: "background_research".to_string(),
                prompt: format!("Research the background and context for: {}", prompt),
                description: "Gather background information".to_string(),
                priority: TaskPriority::High,
                expected_output_type: "background".to_string(),
            },
            ParallelTask {
                task_id: "current_state".to_string(),
                prompt: format!(
                    "Analyze the current state and recent developments regarding: {}",
                    prompt
                ),
                description: "Current state analysis".to_string(),
                priority: TaskPriority::High,
                expected_output_type: "analysis".to_string(),
            },
            ParallelTask {
                task_id: "implications".to_string(),
                prompt: format!("Identify implications and potential impacts of: {}", prompt),
                description: "Impact and implications analysis".to_string(),
                priority: TaskPriority::Normal,
                expected_output_type: "implications".to_string(),
            },
            ParallelTask {
                task_id: "future_trends".to_string(),
                prompt: format!(
                    "Predict future trends and developments related to: {}",
                    prompt
                ),
                description: "Future trends analysis".to_string(),
                priority: TaskPriority::Low,
                expected_output_type: "predictions".to_string(),
            },
        ])
    }

    /// Create tasks for generation scenarios
    fn create_generation_tasks(&self, prompt: &str) -> EvolutionResult<Vec<ParallelTask>> {
        Ok(vec![
            ParallelTask {
                task_id: "concept_generation".to_string(),
                prompt: format!("Generate initial concepts and ideas for: {}", prompt),
                description: "Initial concept generation".to_string(),
                priority: TaskPriority::High,
                expected_output_type: "concepts".to_string(),
            },
            ParallelTask {
                task_id: "detailed_development".to_string(),
                prompt: format!("Develop detailed content based on: {}", prompt),
                description: "Detailed content development".to_string(),
                priority: TaskPriority::High,
                expected_output_type: "content".to_string(),
            },
            ParallelTask {
                task_id: "alternative_approaches".to_string(),
                prompt: format!("Explore alternative approaches for: {}", prompt),
                description: "Alternative approach exploration".to_string(),
                priority: TaskPriority::Normal,
                expected_output_type: "alternatives".to_string(),
            },
        ])
    }

    /// Create tasks for evaluation scenarios
    fn create_evaluation_tasks(&self, prompt: &str) -> EvolutionResult<Vec<ParallelTask>> {
        Ok(vec![
            ParallelTask {
                task_id: "criteria_evaluation".to_string(),
                prompt: format!("Define evaluation criteria for: {}", prompt),
                description: "Define evaluation criteria".to_string(),
                priority: TaskPriority::Critical,
                expected_output_type: "criteria".to_string(),
            },
            ParallelTask {
                task_id: "scoring_assessment".to_string(),
                prompt: format!("Assess and score based on the criteria: {}", prompt),
                description: "Scoring and assessment".to_string(),
                priority: TaskPriority::High,
                expected_output_type: "scores".to_string(),
            },
            ParallelTask {
                task_id: "validation_check".to_string(),
                prompt: format!("Validate the assessment results for: {}", prompt),
                description: "Result validation".to_string(),
                priority: TaskPriority::Normal,
                expected_output_type: "validation".to_string(),
            },
        ])
    }

    /// Create generic parallel tasks
    fn create_generic_parallel_tasks(&self, prompt: &str) -> EvolutionResult<Vec<ParallelTask>> {
        Ok(vec![
            ParallelTask {
                task_id: "analysis_perspective".to_string(),
                prompt: format!("Analyze from an analytical perspective: {}", prompt),
                description: "Analytical perspective".to_string(),
                priority: TaskPriority::High,
                expected_output_type: "analysis".to_string(),
            },
            ParallelTask {
                task_id: "practical_perspective".to_string(),
                prompt: format!("Consider the practical aspects of: {}", prompt),
                description: "Practical perspective".to_string(),
                priority: TaskPriority::High,
                expected_output_type: "practical".to_string(),
            },
            ParallelTask {
                task_id: "creative_perspective".to_string(),
                prompt: format!("Approach creatively and innovatively: {}", prompt),
                description: "Creative perspective".to_string(),
                priority: TaskPriority::Normal,
                expected_output_type: "creative".to_string(),
            },
        ])
    }

    /// Execute tasks in controlled batches
    async fn execute_task_batches(
        &self,
        mut tasks: Vec<ParallelTask>,
    ) -> EvolutionResult<Vec<ParallelTaskResult>> {
        // Sort tasks by priority (Critical first)
        tasks.sort_by(|a, b| b.priority.cmp(&a.priority));

        let mut all_results = Vec::new();

        // Process tasks in batches
        for batch in tasks.chunks(self.parallel_config.max_parallel_tasks) {
            let batch_futures: Vec<_> = batch
                .iter()
                .map(|task| self.execute_single_task(task.clone()))
                .collect();

            let batch_results = join_all(batch_futures).await;
            all_results.extend(batch_results);

            // Small delay between batches to prevent overwhelming the system
            if batch.len() == self.parallel_config.max_parallel_tasks {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }

        Ok(all_results)
    }

    /// Execute a single parallel task
    async fn execute_single_task(&self, task: ParallelTask) -> ParallelTaskResult {
        let start_time = Instant::now();

        log::debug!(
            "Executing parallel task: {} - {}",
            task.task_id,
            task.description
        );

        let result = timeout(
            self.parallel_config.task_timeout,
            self.llm_adapter
                .complete(&task.prompt, CompletionOptions::default()),
        )
        .await;

        let duration = start_time.elapsed();

        match result {
            Ok(Ok(output)) => {
                let quality_score =
                    self.estimate_quality_score(&output, &task.expected_output_type);

                ParallelTaskResult {
                    task_id: task.task_id,
                    result: Some(output),
                    success: true,
                    duration,
                    error: None,
                    quality_score: Some(quality_score),
                }
            }
            Ok(Err(e)) => {
                log::warn!("Task {} failed: {}", task.task_id, e);
                ParallelTaskResult {
                    task_id: task.task_id,
                    result: None,
                    success: false,
                    duration,
                    error: Some(e.to_string()),
                    quality_score: None,
                }
            }
            Err(_) => {
                log::warn!(
                    "Task {} timed out after {:?}",
                    task.task_id,
                    self.parallel_config.task_timeout
                );
                ParallelTaskResult {
                    task_id: task.task_id,
                    result: None,
                    success: false,
                    duration,
                    error: Some("Task timed out".to_string()),
                    quality_score: None,
                }
            }
        }
    }

    /// Aggregate results based on configured strategy
    async fn aggregate_results(&self, results: &[&ParallelTaskResult]) -> EvolutionResult<String> {
        if results.is_empty() {
            return Ok("No successful results to aggregate".to_string());
        }

        match self.parallel_config.aggregation_strategy {
            AggregationStrategy::Concatenation => {
                let combined = results
                    .iter()
                    .filter_map(|r| r.result.as_ref())
                    .enumerate()
                    .map(|(i, result)| format!("## Result {}\n{}\n", i + 1, result))
                    .collect::<Vec<_>>()
                    .join("\n");
                Ok(combined)
            }

            AggregationStrategy::BestResult => {
                let best_result = results
                    .iter()
                    .max_by(|a, b| {
                        let score_a = a.quality_score.unwrap_or(0.0);
                        let score_b = b.quality_score.unwrap_or(0.0);
                        score_a
                            .partial_cmp(&score_b)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .and_then(|r| r.result.as_ref())
                    .map(|s| s.clone())
                    .unwrap_or_else(|| "No valid result found".to_string());
                Ok(best_result)
            }

            AggregationStrategy::Synthesis => {
                let combined_input = results
                    .iter()
                    .filter_map(|r| r.result.as_ref())
                    .enumerate()
                    .map(|(i, result)| format!("Perspective {}: {}", i + 1, result))
                    .collect::<Vec<_>>()
                    .join("\n\n");

                let synthesis_prompt = format!(
                    "Synthesize the following perspectives into a comprehensive, coherent response:\n\n{}",
                    combined_input
                );

                self.llm_adapter
                    .complete(&synthesis_prompt, CompletionOptions::default())
                    .await
                    .map_err(|e| {
                        crate::EvolutionError::WorkflowError(format!("Synthesis failed: {}", e))
                    })
            }

            AggregationStrategy::MajorityVote => {
                // Simple majority vote implementation (could be enhanced)
                let most_common = results
                    .iter()
                    .filter_map(|r| r.result.as_ref())
                    .max_by_key(|result| {
                        results
                            .iter()
                            .filter_map(|r| r.result.as_ref())
                            .filter(|r| r == result)
                            .count()
                    })
                    .map(|s| s.clone())
                    .unwrap_or_else(|| "No consensus reached".to_string());
                Ok(most_common)
            }

            AggregationStrategy::StructuredCombination => {
                let mut structured_result = String::new();
                structured_result.push_str("# Comprehensive Analysis\n\n");

                for (i, result) in results.iter().enumerate() {
                    if let Some(content) = &result.result {
                        structured_result.push_str(&format!(
                            "## Section {}: {}\n{}\n\n",
                            i + 1,
                            result.task_id.replace('_', " ").to_uppercase(),
                            content
                        ));
                    }
                }

                Ok(structured_result)
            }
        }
    }

    /// Create execution trace from task results
    fn create_execution_trace(
        &self,
        task_results: &[ParallelTaskResult],
        final_result: &str,
    ) -> Vec<ExecutionStep> {
        let mut trace = Vec::new();

        // Add steps for each parallel task
        for result in task_results {
            trace.push(ExecutionStep {
                step_id: result.task_id.clone(),
                step_type: StepType::Parallel,
                input: format!("Parallel task: {}", result.task_id),
                output: result.result.clone().unwrap_or_else(|| {
                    result
                        .error
                        .clone()
                        .unwrap_or_else(|| "No output".to_string())
                }),
                duration: result.duration,
                success: result.success,
                metadata: serde_json::json!({
                    "quality_score": result.quality_score,
                    "error": result.error,
                }),
            });
        }

        // Add aggregation step
        trace.push(ExecutionStep {
            step_id: "result_aggregation".to_string(),
            step_type: StepType::Aggregation,
            input: format!("Aggregating {} results", task_results.len()),
            output: final_result.to_string(),
            duration: Duration::from_millis(50), // Rough estimate for aggregation time
            success: true,
            metadata: serde_json::json!({
                "strategy": format!("{:?}", self.parallel_config.aggregation_strategy),
                "successful_tasks": task_results.iter().filter(|r| r.success).count(),
                "total_tasks": task_results.len(),
            }),
        });

        trace
    }

    /// Estimate quality score for a result
    fn estimate_quality_score(&self, output: &str, expected_type: &str) -> f64 {
        let mut score: f64 = 0.5; // Base score

        // Length-based scoring
        match output.len() {
            0..=50 => score -= 0.2,
            51..=200 => score += 0.1,
            201..=1000 => score += 0.2,
            _ => score += 0.3,
        }

        // Content type matching
        match expected_type {
            "analysis" => {
                if output.contains("analyze")
                    || output.contains("because")
                    || output.contains("therefore")
                {
                    score += 0.2;
                }
            }
            "recommendations" => {
                if output.contains("recommend")
                    || output.contains("suggest")
                    || output.contains("should")
                {
                    score += 0.2;
                }
            }
            "evaluation" => {
                if output.contains("pros")
                    || output.contains("cons")
                    || output.contains("advantage")
                {
                    score += 0.2;
                }
            }
            _ => {} // No specific bonus for other types
        }

        score.clamp(0.0, 1.0)
    }

    /// Calculate overall quality score from all task results
    fn calculate_overall_quality_score(&self, results: &[ParallelTaskResult]) -> Option<f64> {
        let quality_scores: Vec<f64> = results.iter().filter_map(|r| r.quality_score).collect();

        if quality_scores.is_empty() {
            None
        } else {
            let average = quality_scores.iter().sum::<f64>() / quality_scores.len() as f64;
            Some(average)
        }
    }

    /// Estimate total tokens consumed
    fn estimate_tokens_consumed(
        &self,
        results: &[ParallelTaskResult],
        final_result: &str,
    ) -> usize {
        let task_tokens: usize = results
            .iter()
            .filter_map(|r| r.result.as_ref())
            .map(|r| r.len())
            .sum();

        task_tokens + final_result.len()
    }
}

#[async_trait]
impl WorkflowPattern for Parallelization {
    fn pattern_name(&self) -> &'static str {
        "parallelization"
    }

    async fn execute(&self, input: WorkflowInput) -> EvolutionResult<WorkflowOutput> {
        log::info!(
            "Executing parallelization workflow for task: {}",
            input.task_id
        );
        self.execute_parallel_tasks(&input).await
    }

    fn is_suitable_for(&self, task_analysis: &TaskAnalysis) -> bool {
        // Parallelization is suitable for:
        // - Tasks that can be decomposed into independent subtasks
        // - Moderate to complex tasks that benefit from multiple perspectives
        // - Tasks explicitly marked as suitable for parallel processing

        task_analysis.suitable_for_parallel
            || matches!(
                task_analysis.complexity,
                TaskComplexity::Moderate | TaskComplexity::Complex
            )
            || task_analysis.domain.contains("comparison")
            || task_analysis.domain.contains("research")
            || task_analysis.domain.contains("analysis")
    }

    fn estimate_execution_time(&self, input: &WorkflowInput) -> Duration {
        // Estimate based on task complexity and parallel configuration
        let base_time_per_task = if input.prompt.len() > 1000 {
            Duration::from_secs(60)
        } else {
            Duration::from_secs(30)
        };

        // Parallel execution reduces total time but adds overhead
        let estimated_tasks = if input.prompt.len() > 2000 { 4 } else { 3 };
        let batches = (estimated_tasks + self.parallel_config.max_parallel_tasks - 1)
            / self.parallel_config.max_parallel_tasks;

        base_time_per_task * batches as u32 + Duration::from_secs(10)
        // aggregation overhead
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parallel_config_default() {
        let config = ParallelConfig::default();
        assert_eq!(config.max_parallel_tasks, 4);
        assert_eq!(config.task_timeout, Duration::from_secs(120));
        assert_eq!(config.failure_threshold, 0.5);
        assert!(!config.retry_failed_tasks);
    }

    #[test]
    fn test_task_priority_ordering() {
        let mut priorities = vec![
            TaskPriority::Low,
            TaskPriority::Critical,
            TaskPriority::Normal,
            TaskPriority::High,
        ];
        priorities.sort();

        assert_eq!(
            priorities,
            vec![
                TaskPriority::Low,
                TaskPriority::Normal,
                TaskPriority::High,
                TaskPriority::Critical,
            ]
        );
    }

    #[test]
    fn test_quality_score_estimation() {
        use crate::llm_adapter::LlmAdapterFactory;

        let mock_adapter = LlmAdapterFactory::create_mock("test");
        let parallelization = Parallelization::new(mock_adapter);

        let score = parallelization.estimate_quality_score(
            "This is a comprehensive analysis because it covers multiple aspects and therefore provides valuable insights",
            "analysis"
        );

        assert!(score > 0.5);
        assert!(score <= 1.0);
    }
}
