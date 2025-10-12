//! AI Agent workflow patterns implementation
//!
//! This module implements the 5 key workflow patterns for AI agent orchestration:
//! 1. Prompt Chaining - Serial execution of linked prompts
//! 2. Routing - Intelligent task distribution based on complexity
//! 3. Parallelization - Concurrent execution and result aggregation
//! 4. Orchestrator-Workers - Hierarchical planning and execution
//! 5. Evaluator-Optimizer - Feedback loop for quality improvement

pub mod evaluator_optimizer;
pub mod orchestrator_workers;
pub mod parallelization;
pub mod prompt_chaining;
pub mod routing;

pub use evaluator_optimizer::*;
pub use orchestrator_workers::{
    CoordinationMessage, CoordinationStrategy, ExecutionPlan, MessageType, OrchestrationConfig,
    OrchestratorWorkers, TaskPriority as OrchestratorTaskPriority, WorkerResult, WorkerRole,
    WorkerTask,
};
pub use parallelization::{
    AggregationStrategy, ParallelConfig, ParallelTask, ParallelTaskResult, Parallelization,
    TaskPriority as ParallelTaskPriority,
};
pub use prompt_chaining::*;
pub use routing::*;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{AgentId, EvolutionResult, LlmAdapter, TaskId};

/// Base trait for all workflow patterns
#[async_trait]
pub trait WorkflowPattern: Send + Sync {
    /// Get the workflow pattern name
    fn pattern_name(&self) -> &'static str;

    /// Execute the workflow with the given input
    async fn execute(&self, input: WorkflowInput) -> EvolutionResult<WorkflowOutput>;

    /// Determine if this pattern is suitable for the given task
    fn is_suitable_for(&self, task_analysis: &TaskAnalysis) -> bool;

    /// Get the expected execution time estimate
    fn estimate_execution_time(&self, input: &WorkflowInput) -> std::time::Duration;
}

/// Input for workflow execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowInput {
    pub task_id: TaskId,
    pub agent_id: AgentId,
    pub prompt: String,
    pub context: Option<String>,
    pub parameters: WorkflowParameters,
    pub timestamp: DateTime<Utc>,
}

/// Output from workflow execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowOutput {
    pub task_id: TaskId,
    pub agent_id: AgentId,
    pub result: String,
    pub metadata: WorkflowMetadata,
    pub execution_trace: Vec<ExecutionStep>,
    pub timestamp: DateTime<Utc>,
}

/// Parameters for workflow configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowParameters {
    pub max_steps: Option<usize>,
    pub timeout: Option<std::time::Duration>,
    pub quality_threshold: Option<f64>,
    pub parallel_degree: Option<usize>,
    pub retry_attempts: Option<usize>,
}

impl Default for WorkflowParameters {
    fn default() -> Self {
        Self {
            max_steps: Some(10),
            timeout: Some(std::time::Duration::from_secs(300)),
            quality_threshold: Some(0.8),
            parallel_degree: Some(4),
            retry_attempts: Some(3),
        }
    }
}

/// Metadata about workflow execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowMetadata {
    pub pattern_used: String,
    pub execution_time: std::time::Duration,
    pub steps_executed: usize,
    pub success: bool,
    pub quality_score: Option<f64>,
    pub resources_used: ResourceUsage,
}

/// Resource usage tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub llm_calls: usize,
    pub tokens_consumed: usize,
    pub parallel_tasks: usize,
    pub memory_peak_mb: f64,
}

impl Default for ResourceUsage {
    fn default() -> Self {
        Self {
            llm_calls: 0,
            tokens_consumed: 0,
            parallel_tasks: 0,
            memory_peak_mb: 0.0,
        }
    }
}

/// Individual execution step in a workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStep {
    pub step_id: String,
    pub step_type: StepType,
    pub input: String,
    pub output: String,
    pub duration: std::time::Duration,
    pub success: bool,
    pub metadata: serde_json::Value,
}

/// Types of execution steps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StepType {
    LlmCall,
    Routing,
    Aggregation,
    Evaluation,
    Decomposition,
    Parallel,
}

/// Analysis of a task to determine workflow suitability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskAnalysis {
    pub complexity: TaskComplexity,
    pub domain: String,
    pub requires_decomposition: bool,
    pub suitable_for_parallel: bool,
    pub quality_critical: bool,
    pub estimated_steps: usize,
}

/// Task complexity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskComplexity {
    Simple,
    Moderate,
    Complex,
    VeryComplex,
}

/// Factory for creating workflow patterns
pub struct WorkflowFactory;

impl WorkflowFactory {
    /// Create a workflow pattern based on task analysis
    pub fn create_for_task(
        task_analysis: &TaskAnalysis,
        llm_adapter: Arc<dyn LlmAdapter>,
    ) -> Arc<dyn WorkflowPattern> {
        match task_analysis.complexity {
            TaskComplexity::Simple => {
                if task_analysis.quality_critical {
                    Arc::new(EvaluatorOptimizer::new(llm_adapter))
                } else {
                    Arc::new(PromptChaining::new(llm_adapter))
                }
            }
            TaskComplexity::Moderate => {
                if task_analysis.suitable_for_parallel {
                    Arc::new(Parallelization::new(llm_adapter))
                } else {
                    Arc::new(Routing::new(llm_adapter))
                }
            }
            TaskComplexity::Complex | TaskComplexity::VeryComplex => {
                if task_analysis.requires_decomposition {
                    Arc::new(OrchestratorWorkers::new(llm_adapter))
                } else {
                    Arc::new(Routing::new(llm_adapter))
                }
            }
        }
    }

    /// Create a specific workflow pattern by name
    pub fn create_by_name(
        pattern_name: &str,
        llm_adapter: Arc<dyn LlmAdapter>,
    ) -> EvolutionResult<Arc<dyn WorkflowPattern>> {
        match pattern_name {
            "prompt_chaining" => Ok(Arc::new(PromptChaining::new(llm_adapter))),
            "routing" => Ok(Arc::new(Routing::new(llm_adapter))),
            "parallelization" => Ok(Arc::new(Parallelization::new(llm_adapter))),
            "orchestrator_workers" => Ok(Arc::new(OrchestratorWorkers::new(llm_adapter))),
            "evaluator_optimizer" => Ok(Arc::new(EvaluatorOptimizer::new(llm_adapter))),
            _ => Err(crate::EvolutionError::WorkflowError(format!(
                "Unknown workflow pattern: {}",
                pattern_name
            ))),
        }
    }

    /// Get all available workflow patterns
    pub fn available_patterns() -> Vec<&'static str> {
        vec![
            "prompt_chaining",
            "routing",
            "parallelization",
            "orchestrator_workers",
            "evaluator_optimizer",
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_parameters_default() {
        let params = WorkflowParameters::default();
        assert_eq!(params.max_steps, Some(10));
        assert_eq!(params.timeout, Some(std::time::Duration::from_secs(300)));
        assert_eq!(params.quality_threshold, Some(0.8));
    }

    #[test]
    fn test_factory_available_patterns() {
        let patterns = WorkflowFactory::available_patterns();
        assert_eq!(patterns.len(), 5);
        assert!(patterns.contains(&"prompt_chaining"));
        assert!(patterns.contains(&"routing"));
        assert!(patterns.contains(&"parallelization"));
        assert!(patterns.contains(&"orchestrator_workers"));
        assert!(patterns.contains(&"evaluator_optimizer"));
    }

    #[test]
    fn test_task_complexity_levels() {
        assert_eq!(TaskComplexity::Simple, TaskComplexity::Simple);
        assert_ne!(TaskComplexity::Simple, TaskComplexity::Complex);
    }
}
