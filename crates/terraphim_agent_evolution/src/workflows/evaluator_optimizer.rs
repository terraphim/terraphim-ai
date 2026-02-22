//! Evaluator-Optimizer workflow pattern
//!
//! This pattern implements a feedback loop where an evaluator agent assesses
//! the quality of outputs and an optimizer agent improves them iteratively.

use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::{
    CompletionOptions, EvolutionResult, LlmAdapter,
    workflows::{
        ExecutionStep, ResourceUsage, StepType, TaskAnalysis, TaskComplexity, WorkflowInput,
        WorkflowMetadata, WorkflowOutput, WorkflowPattern,
    },
};

/// Evaluator-Optimizer workflow with iterative improvement
pub struct EvaluatorOptimizer {
    generator_adapter: Arc<dyn LlmAdapter>,
    evaluator_adapter: Arc<dyn LlmAdapter>,
    optimizer_adapter: Arc<dyn LlmAdapter>,
    optimization_config: OptimizationConfig,
}

/// Configuration for evaluation and optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationConfig {
    pub max_iterations: usize,
    pub quality_threshold: f64,
    pub improvement_threshold: f64,
    pub evaluation_criteria: Vec<EvaluationCriterion>,
    pub optimization_strategy: OptimizationStrategy,
    pub early_stopping: bool,
}

impl Default for OptimizationConfig {
    fn default() -> Self {
        Self {
            max_iterations: 3,
            quality_threshold: 0.85,
            improvement_threshold: 0.05, // Minimum 5% improvement required
            evaluation_criteria: vec![
                EvaluationCriterion::Accuracy,
                EvaluationCriterion::Completeness,
                EvaluationCriterion::Clarity,
                EvaluationCriterion::Relevance,
            ],
            optimization_strategy: OptimizationStrategy::Incremental,
            early_stopping: true,
        }
    }
}

/// Strategy for optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationStrategy {
    /// Make incremental improvements to existing content
    Incremental,
    /// Regenerate sections that need improvement
    Selective,
    /// Complete regeneration with feedback incorporated
    Complete,
    /// Adaptive strategy based on evaluation results
    Adaptive,
}

/// Evaluation criteria for assessing output quality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EvaluationCriterion {
    Accuracy,
    Completeness,
    Clarity,
    Relevance,
    Coherence,
    Depth,
    Creativity,
    Conciseness,
}

/// Detailed evaluation of content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evaluation {
    pub iteration: usize,
    pub overall_score: f64,
    pub criterion_scores: std::collections::HashMap<EvaluationCriterion, f64>,
    pub strengths: Vec<String>,
    pub weaknesses: Vec<String>,
    pub improvement_suggestions: Vec<String>,
    pub meets_threshold: bool,
}

/// Optimization action to be taken
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationAction {
    pub action_type: ActionType,
    pub target_section: Option<String>,
    pub improvement_instruction: String,
    pub priority: ActionPriority,
}

/// Types of optimization actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionType {
    Enhance,
    Rewrite,
    Expand,
    Clarify,
    Restructure,
    AddContent,
    RemoveContent,
}

/// Priority levels for optimization actions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum ActionPriority {
    Low,
    Medium,
    High,
    Critical,
}

/// Result from an optimization iteration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationIteration {
    pub iteration: usize,
    pub content: String,
    pub evaluation: Evaluation,
    pub actions_taken: Vec<OptimizationAction>,
    pub improvement_delta: f64,
    pub duration: Duration,
}

impl EvaluatorOptimizer {
    /// Create a new evaluator-optimizer workflow
    pub fn new(llm_adapter: Arc<dyn LlmAdapter>) -> Self {
        Self {
            generator_adapter: llm_adapter.clone(),
            evaluator_adapter: llm_adapter.clone(),
            optimizer_adapter: llm_adapter,
            optimization_config: OptimizationConfig::default(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(llm_adapter: Arc<dyn LlmAdapter>, config: OptimizationConfig) -> Self {
        Self {
            generator_adapter: llm_adapter.clone(),
            evaluator_adapter: llm_adapter.clone(),
            optimizer_adapter: llm_adapter,
            optimization_config: config,
        }
    }

    /// Set specialized adapters for different roles
    pub fn with_specialized_adapters(
        generator: Arc<dyn LlmAdapter>,
        evaluator: Arc<dyn LlmAdapter>,
        optimizer: Arc<dyn LlmAdapter>,
    ) -> Self {
        Self {
            generator_adapter: generator,
            evaluator_adapter: evaluator,
            optimizer_adapter: optimizer,
            optimization_config: OptimizationConfig::default(),
        }
    }

    /// Execute the evaluation-optimization workflow
    async fn execute_optimization_loop(
        &self,
        input: &WorkflowInput,
    ) -> EvolutionResult<WorkflowOutput> {
        let start_time = Instant::now();
        let mut execution_trace = Vec::new();
        let mut resource_usage = ResourceUsage::default();

        // Step 1: Generate initial content
        log::info!("Generating initial content for task: {}", input.task_id);
        let initial_content = self
            .generate_initial_content(&input.prompt, &input.context)
            .await?;

        execution_trace.push(ExecutionStep {
            step_id: "initial_generation".to_string(),
            step_type: StepType::LlmCall,
            input: input.prompt.clone(),
            output: initial_content.clone(),
            duration: Duration::from_secs(1), // Rough estimate
            success: true,
            metadata: serde_json::json!({
                "content_length": initial_content.len(),
            }),
        });
        resource_usage.llm_calls += 1;

        // Step 2: Iterative optimization loop
        let mut current_content = initial_content;
        let mut iterations = Vec::new();
        let mut best_score = 0.0;

        for iteration in 1..=self.optimization_config.max_iterations {
            let iteration_start = Instant::now();

            log::info!("Starting optimization iteration {}", iteration);

            // Evaluate current content
            let evaluation = self
                .evaluate_content(&current_content, &input.prompt, iteration)
                .await?;
            resource_usage.llm_calls += 1;

            // Check if we've met the quality threshold
            if evaluation.meets_threshold && self.optimization_config.early_stopping {
                log::info!(
                    "Quality threshold met at iteration {}, stopping early",
                    iteration
                );
                iterations.push(OptimizationIteration {
                    iteration,
                    content: current_content.clone(),
                    evaluation: evaluation.clone(),
                    actions_taken: vec![],
                    improvement_delta: evaluation.overall_score - best_score,
                    duration: iteration_start.elapsed(),
                });
                break;
            }

            // Check for sufficient improvement
            let improvement_delta = evaluation.overall_score - best_score;
            if iteration > 1 && improvement_delta < self.optimization_config.improvement_threshold {
                log::info!(
                    "Insufficient improvement at iteration {}, stopping",
                    iteration
                );
                iterations.push(OptimizationIteration {
                    iteration,
                    content: current_content.clone(),
                    evaluation,
                    actions_taken: vec![],
                    improvement_delta,
                    duration: iteration_start.elapsed(),
                });
                break;
            }

            best_score = evaluation.overall_score.max(best_score);

            // Generate optimization actions
            let actions = self.generate_optimization_actions(&evaluation).await?;

            // Apply optimizations
            let optimized_content = self
                .apply_optimizations(&current_content, &actions, &input.prompt)
                .await?;
            resource_usage.llm_calls += 1;

            let iteration_result = OptimizationIteration {
                iteration,
                content: optimized_content.clone(),
                evaluation,
                actions_taken: actions.clone(),
                improvement_delta,
                duration: iteration_start.elapsed(),
            };

            iterations.push(iteration_result.clone());
            current_content = optimized_content;

            // Add iteration to execution trace
            execution_trace.push(ExecutionStep {
                step_id: format!("optimization_iteration_{}", iteration),
                step_type: StepType::Evaluation,
                input: format!("Iteration {} content", iteration),
                output: current_content.clone(),
                duration: iteration_start.elapsed(),
                success: true,
                metadata: serde_json::json!({
                    "iteration": iteration,
                    "quality_score": iteration_result.evaluation.overall_score,
                    "improvement_delta": iteration_result.improvement_delta,
                    "actions_count": actions.len(),
                }),
            });
        }

        // Final evaluation
        let final_evaluation = if let Some(last_iteration) = iterations.last() {
            last_iteration.evaluation.clone()
        } else {
            self.evaluate_content(&current_content, &input.prompt, 0)
                .await?
        };

        resource_usage.tokens_consumed =
            self.estimate_token_consumption(&iterations, &current_content);
        resource_usage.parallel_tasks = 0; // Sequential execution

        let metadata = WorkflowMetadata {
            pattern_used: "evaluator_optimizer".to_string(),
            execution_time: start_time.elapsed(),
            steps_executed: iterations.len() + 1, // +1 for initial generation
            success: true,
            quality_score: Some(final_evaluation.overall_score),
            resources_used: resource_usage,
        };

        Ok(WorkflowOutput {
            task_id: input.task_id.clone(),
            agent_id: input.agent_id.clone(),
            result: current_content,
            metadata,
            execution_trace,
            timestamp: Utc::now(),
        })
    }

    /// Generate initial content
    async fn generate_initial_content(
        &self,
        prompt: &str,
        context: &Option<String>,
    ) -> EvolutionResult<String> {
        let context_str = context.as_deref().unwrap_or("");
        let generation_prompt = if context_str.is_empty() {
            format!("Please provide a comprehensive response to: {}", prompt)
        } else {
            format!(
                "Context: {}\n\nPlease provide a comprehensive response to: {}",
                context_str, prompt
            )
        };

        self.generator_adapter
            .complete(&generation_prompt, CompletionOptions::default())
            .await
            .map_err(|e| {
                crate::EvolutionError::WorkflowError(format!("Initial generation failed: {}", e))
            })
    }

    /// Evaluate content quality across multiple criteria
    async fn evaluate_content(
        &self,
        content: &str,
        original_prompt: &str,
        iteration: usize,
    ) -> EvolutionResult<Evaluation> {
        let criteria_descriptions = self.get_criteria_descriptions();

        let evaluation_prompt = format!(
            r#"Evaluate the following content against the original request and quality criteria:

Original Request: {}

Content to Evaluate:
{}

Evaluation Criteria:
{}

Please provide:
1. An overall quality score from 0.0 to 1.0
2. Individual scores for each criterion (0.0 to 1.0)
3. Key strengths of the content
4. Areas that need improvement
5. Specific suggestions for improvement

Format your response as a structured evaluation."#,
            original_prompt,
            content,
            criteria_descriptions.join("\n")
        );

        let evaluation_response = self
            .evaluator_adapter
            .complete(&evaluation_prompt, CompletionOptions::default())
            .await
            .map_err(|e| {
                crate::EvolutionError::WorkflowError(format!("Evaluation failed: {}", e))
            })?;

        // Parse evaluation response (simplified parsing)
        let overall_score = self.extract_overall_score(&evaluation_response);
        let criterion_scores = self.extract_criterion_scores(&evaluation_response);
        let (strengths, weaknesses, suggestions) = self.extract_feedback(&evaluation_response);

        let meets_threshold = overall_score >= self.optimization_config.quality_threshold;

        Ok(Evaluation {
            iteration,
            overall_score,
            criterion_scores,
            strengths,
            weaknesses,
            improvement_suggestions: suggestions,
            meets_threshold,
        })
    }

    /// Generate optimization actions based on evaluation
    async fn generate_optimization_actions(
        &self,
        evaluation: &Evaluation,
    ) -> EvolutionResult<Vec<OptimizationAction>> {
        if evaluation.improvement_suggestions.is_empty() {
            return Ok(vec![]);
        }

        let mut actions = Vec::new();

        // Convert improvement suggestions into concrete actions
        for (i, suggestion) in evaluation.improvement_suggestions.iter().enumerate() {
            let action_type = self.determine_action_type(suggestion);
            let priority = self.determine_action_priority(suggestion, &evaluation.criterion_scores);

            actions.push(OptimizationAction {
                action_type,
                target_section: None, // Could be more specific with better parsing
                improvement_instruction: suggestion.clone(),
                priority,
            });

            // Limit number of actions per iteration
            if i >= 3 {
                break;
            }
        }

        // Sort by priority (Critical first)
        actions.sort_by(|a, b| b.priority.cmp(&a.priority));

        Ok(actions)
    }

    /// Apply optimization actions to content
    async fn apply_optimizations(
        &self,
        content: &str,
        actions: &[OptimizationAction],
        original_prompt: &str,
    ) -> EvolutionResult<String> {
        if actions.is_empty() {
            return Ok(content.to_string());
        }

        let strategy = self.determine_optimization_strategy(actions);

        match strategy {
            OptimizationStrategy::Incremental => {
                self.apply_incremental_optimization(content, actions, original_prompt)
                    .await
            }
            OptimizationStrategy::Selective => {
                self.apply_selective_optimization(content, actions, original_prompt)
                    .await
            }
            OptimizationStrategy::Complete => {
                self.apply_complete_regeneration(content, actions, original_prompt)
                    .await
            }
            OptimizationStrategy::Adaptive => {
                // Choose strategy based on actions
                if actions.len() > 2
                    || actions
                        .iter()
                        .any(|a| a.priority == ActionPriority::Critical)
                {
                    self.apply_selective_optimization(content, actions, original_prompt)
                        .await
                } else {
                    self.apply_incremental_optimization(content, actions, original_prompt)
                        .await
                }
            }
        }
    }

    /// Apply incremental improvements
    async fn apply_incremental_optimization(
        &self,
        content: &str,
        actions: &[OptimizationAction],
        original_prompt: &str,
    ) -> EvolutionResult<String> {
        let improvements = actions
            .iter()
            .map(|a| a.improvement_instruction.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        let optimization_prompt = format!(
            r#"Original request: {}

Current content:
{}

Improvements needed:
{}

Please provide the improved version of the content, incorporating these improvements while maintaining the overall structure and flow."#,
            original_prompt, content, improvements
        );

        self.optimizer_adapter
            .complete(&optimization_prompt, CompletionOptions::default())
            .await
            .map_err(|e| {
                crate::EvolutionError::WorkflowError(format!(
                    "Incremental optimization failed: {}",
                    e
                ))
            })
    }

    /// Apply selective optimization (regenerate specific sections)
    async fn apply_selective_optimization(
        &self,
        content: &str,
        actions: &[OptimizationAction],
        original_prompt: &str,
    ) -> EvolutionResult<String> {
        // For simplicity, treat as incremental for now
        // In a more advanced implementation, this would identify and regenerate specific sections
        self.apply_incremental_optimization(content, actions, original_prompt)
            .await
    }

    /// Apply complete regeneration with feedback
    async fn apply_complete_regeneration(
        &self,
        _content: &str,
        actions: &[OptimizationAction],
        original_prompt: &str,
    ) -> EvolutionResult<String> {
        let feedback = actions
            .iter()
            .map(|a| a.improvement_instruction.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        let regeneration_prompt = format!(
            r#"Original request: {}

Important feedback to incorporate:
{}

Please provide a completely new response that addresses the original request while incorporating all the feedback provided."#,
            original_prompt, feedback
        );

        self.generator_adapter
            .complete(&regeneration_prompt, CompletionOptions::default())
            .await
            .map_err(|e| {
                crate::EvolutionError::WorkflowError(format!("Complete regeneration failed: {}", e))
            })
    }

    /// Get descriptions for evaluation criteria
    fn get_criteria_descriptions(&self) -> Vec<String> {
        self.optimization_config
            .evaluation_criteria
            .iter()
            .map(|criterion| match criterion {
                EvaluationCriterion::Accuracy => {
                    "Accuracy: Factual correctness and precision of information"
                }
                EvaluationCriterion::Completeness => {
                    "Completeness: Thorough coverage of all relevant aspects"
                }
                EvaluationCriterion::Clarity => "Clarity: Clear and understandable presentation",
                EvaluationCriterion::Relevance => {
                    "Relevance: Direct connection to the original request"
                }
                EvaluationCriterion::Coherence => "Coherence: Logical flow and consistency",
                EvaluationCriterion::Depth => "Depth: Thorough analysis and insight",
                EvaluationCriterion::Creativity => {
                    "Creativity: Original thinking and novel approaches"
                }
                EvaluationCriterion::Conciseness => {
                    "Conciseness: Efficient use of language without redundancy"
                }
            })
            .map(|s| s.to_string())
            .collect()
    }

    /// Extract overall score from evaluation response (simplified parsing)
    fn extract_overall_score(&self, response: &str) -> f64 {
        // Look for patterns like "overall score: 0.7" or "score: 7/10"
        let patterns = [
            r"overall.*score[:\s]+(\d+(?:\.\d+)?)",
            r"score[:\s]+(\d+(?:\.\d+)?)",
            r"(\d+(?:\.\d+)?)\s*/\s*10",
            r"(\d+(?:\.\d+)?)\s*%",
        ];

        let response_lower = response.to_lowercase();

        for pattern in &patterns {
            if let Ok(regex) = regex::Regex::new(pattern) {
                if let Some(captures) = regex.captures(&response_lower) {
                    if let Some(score_str) = captures.get(1) {
                        if let Ok(score) = score_str.as_str().parse::<f64>() {
                            return if score > 1.0 { score / 10.0 } else { score }.clamp(0.0, 1.0);
                        }
                    }
                }
            }
        }

        0.7 // Default reasonable score if parsing fails
    }

    /// Extract criterion scores (simplified - return default scores)
    fn extract_criterion_scores(
        &self,
        _response: &str,
    ) -> std::collections::HashMap<EvaluationCriterion, f64> {
        let mut scores = std::collections::HashMap::new();

        // Default scores (would be parsed from response in real implementation)
        for criterion in &self.optimization_config.evaluation_criteria {
            scores.insert(criterion.clone(), 0.7);
        }

        scores
    }

    /// Extract feedback from evaluation response
    fn extract_feedback(&self, _response: &str) -> (Vec<String>, Vec<String>, Vec<String>) {
        // Simplified parsing - would be more sophisticated in real implementation
        let strengths = vec!["Content addresses the main points".to_string()];
        let weaknesses = vec!["Could be more detailed in some areas".to_string()];
        let suggestions = vec![
            "Add more specific examples".to_string(),
            "Improve structure and flow".to_string(),
        ];

        (strengths, weaknesses, suggestions)
    }

    /// Determine action type from suggestion text
    fn determine_action_type(&self, suggestion: &str) -> ActionType {
        let suggestion_lower = suggestion.to_lowercase();

        if suggestion_lower.contains("rewrite") || suggestion_lower.contains("redo") {
            ActionType::Rewrite
        } else if suggestion_lower.contains("clarify") || suggestion_lower.contains("clearer") {
            ActionType::Clarify
        } else if suggestion_lower.contains("structure") || suggestion_lower.contains("organize") {
            ActionType::Restructure
        } else if suggestion_lower.contains("remove") || suggestion_lower.contains("delete") {
            ActionType::RemoveContent
        } else if suggestion_lower.contains("add") || suggestion_lower.contains("include") {
            ActionType::AddContent
        } else if suggestion_lower.contains("expand") || suggestion_lower.contains("add more") {
            ActionType::Expand
        } else {
            ActionType::Enhance
        }
    }

    /// Determine action priority based on suggestion and criterion scores
    fn determine_action_priority(
        &self,
        suggestion: &str,
        _scores: &std::collections::HashMap<EvaluationCriterion, f64>,
    ) -> ActionPriority {
        let suggestion_lower = suggestion.to_lowercase();

        if suggestion_lower.contains("critical") || suggestion_lower.contains("must") {
            ActionPriority::Critical
        } else if suggestion_lower.contains("important") || suggestion_lower.contains("should") {
            ActionPriority::High
        } else if suggestion_lower.contains("could") || suggestion_lower.contains("might") {
            ActionPriority::Medium
        } else {
            ActionPriority::Low
        }
    }

    /// Determine optimization strategy based on actions
    fn determine_optimization_strategy(
        &self,
        actions: &[OptimizationAction],
    ) -> OptimizationStrategy {
        match &self.optimization_config.optimization_strategy {
            OptimizationStrategy::Adaptive => {
                if actions
                    .iter()
                    .any(|a| a.priority == ActionPriority::Critical)
                {
                    OptimizationStrategy::Complete
                } else if actions.len() > 2 {
                    OptimizationStrategy::Selective
                } else {
                    OptimizationStrategy::Incremental
                }
            }
            strategy => strategy.clone(),
        }
    }

    /// Estimate token consumption from iterations
    fn estimate_token_consumption(
        &self,
        iterations: &[OptimizationIteration],
        final_content: &str,
    ) -> usize {
        let iteration_tokens: usize = iterations.iter().map(|i| i.content.len()).sum();

        iteration_tokens + final_content.len()
    }
}

// Required for HashMap keys
impl PartialEq for EvaluationCriterion {
    fn eq(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

impl Eq for EvaluationCriterion {}

impl std::hash::Hash for EvaluationCriterion {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
    }
}

#[async_trait]
impl WorkflowPattern for EvaluatorOptimizer {
    fn pattern_name(&self) -> &'static str {
        "evaluator_optimizer"
    }

    async fn execute(&self, input: WorkflowInput) -> EvolutionResult<WorkflowOutput> {
        log::info!(
            "Executing evaluator-optimizer workflow for task: {}",
            input.task_id
        );
        self.execute_optimization_loop(&input).await
    }

    fn is_suitable_for(&self, task_analysis: &TaskAnalysis) -> bool {
        // Evaluator-optimizer is suitable for:
        // - Quality-critical tasks that benefit from iterative improvement
        // - Complex tasks that require refinement
        // - Tasks where the first attempt might not meet high standards

        task_analysis.quality_critical
            || matches!(
                task_analysis.complexity,
                TaskComplexity::Complex | TaskComplexity::VeryComplex
            )
            || task_analysis.domain.contains("writing")
            || task_analysis.domain.contains("analysis")
    }

    fn estimate_execution_time(&self, input: &WorkflowInput) -> Duration {
        // Estimate based on complexity and maximum iterations
        let base_time_per_iteration = if input.prompt.len() > 1000 {
            Duration::from_secs(90)
        } else {
            Duration::from_secs(60)
        };

        // Account for evaluation and optimization overhead
        let estimated_iterations = self.optimization_config.max_iterations.min(3);
        base_time_per_iteration * (estimated_iterations as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimization_config_default() {
        let config = OptimizationConfig::default();
        assert_eq!(config.max_iterations, 3);
        assert_eq!(config.quality_threshold, 0.85);
        assert_eq!(config.improvement_threshold, 0.05);
        assert!(config.early_stopping);
    }

    #[test]
    fn test_evaluation_criterion_hash() {
        let mut criterion_scores = std::collections::HashMap::new();
        criterion_scores.insert(EvaluationCriterion::Accuracy, 0.8);
        criterion_scores.insert(EvaluationCriterion::Clarity, 0.9);

        assert_eq!(criterion_scores.len(), 2);
        assert_eq!(
            criterion_scores.get(&EvaluationCriterion::Accuracy),
            Some(&0.8)
        );
    }

    #[test]
    fn test_action_priority_ordering() {
        let mut priorities = vec![
            ActionPriority::Low,
            ActionPriority::Critical,
            ActionPriority::Medium,
            ActionPriority::High,
        ];
        priorities.sort();

        assert_eq!(
            priorities,
            vec![
                ActionPriority::Low,
                ActionPriority::Medium,
                ActionPriority::High,
                ActionPriority::Critical,
            ]
        );
    }

    #[test]
    fn test_action_type_determination() {
        use crate::llm_adapter::LlmAdapterFactory;

        let mock_adapter = LlmAdapterFactory::create_mock("test");
        let evaluator = EvaluatorOptimizer::new(mock_adapter);

        assert!(matches!(
            evaluator.determine_action_type("Please rewrite this section"),
            ActionType::Rewrite
        ));

        assert!(matches!(
            evaluator.determine_action_type("Add more examples"),
            ActionType::AddContent
        ));

        assert!(matches!(
            evaluator.determine_action_type("Clarify this point"),
            ActionType::Clarify
        ));
    }
}
