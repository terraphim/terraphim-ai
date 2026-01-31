//! Prompt Chaining workflow pattern
//!
//! This pattern chains multiple LLM calls where the output of one call becomes
//! the input to the next. This breaks complex tasks into smaller, more manageable steps.

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

/// Prompt chaining workflow that executes prompts in sequence
pub struct PromptChaining {
    llm_adapter: Arc<dyn LlmAdapter>,
    chain_config: ChainConfig,
}

/// Configuration for prompt chaining
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainConfig {
    pub max_chain_length: usize,
    pub step_timeout: Duration,
    pub preserve_context: bool,
    pub quality_check: bool,
}

impl Default for ChainConfig {
    fn default() -> Self {
        Self {
            max_chain_length: 5,
            step_timeout: Duration::from_secs(60),
            preserve_context: true,
            quality_check: true,
        }
    }
}

/// Individual link in the prompt chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainLink {
    pub step_id: String,
    pub prompt_template: String,
    pub description: String,
    pub required: bool,
}

impl PromptChaining {
    /// Create a new prompt chaining workflow
    pub fn new(llm_adapter: Arc<dyn LlmAdapter>) -> Self {
        Self {
            llm_adapter,
            chain_config: ChainConfig::default(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(llm_adapter: Arc<dyn LlmAdapter>, config: ChainConfig) -> Self {
        Self {
            llm_adapter,
            chain_config: config,
        }
    }

    /// Execute a predefined chain based on task type
    async fn execute_predefined_chain(
        &self,
        input: &WorkflowInput,
    ) -> EvolutionResult<WorkflowOutput> {
        let chain = self.create_default_chain(&input.prompt);
        self.execute_chain(input, chain).await
    }

    /// Execute a custom chain of prompts
    async fn execute_chain(
        &self,
        input: &WorkflowInput,
        chain: Vec<ChainLink>,
    ) -> EvolutionResult<WorkflowOutput> {
        let start_time = Instant::now();
        let mut execution_trace = Vec::new();
        let mut resource_usage = ResourceUsage::default();
        let mut current_output = input.prompt.clone();
        let mut context = input.context.clone().unwrap_or_default();

        for (index, link) in chain.iter().enumerate() {
            let step_start = Instant::now();
            let step_input = if self.chain_config.preserve_context && !context.is_empty() {
                format!(
                    "{}\n\nContext: {}\nInput: {}",
                    link.prompt_template, context, current_output
                )
            } else {
                format!("{}\n\nInput: {}", link.prompt_template, current_output)
            };

            log::debug!(
                "Executing chain step {}/{}: {}",
                index + 1,
                chain.len(),
                link.description
            );

            // Execute the LLM call
            let completion_options = CompletionOptions::default();
            let step_output = match tokio::time::timeout(
                self.chain_config.step_timeout,
                self.llm_adapter.complete(&step_input, completion_options),
            )
            .await
            {
                Ok(Ok(output)) => output,
                Ok(Err(e)) => {
                    if link.required {
                        return Err(crate::EvolutionError::WorkflowError(format!(
                            "Required chain step '{}' failed: {}",
                            link.description, e
                        )));
                    } else {
                        log::warn!(
                            "Optional chain step '{}' failed: {}, continuing...",
                            link.description,
                            e
                        );
                        current_output.clone()
                    }
                }
                Err(_) => {
                    return Err(crate::EvolutionError::WorkflowError(format!(
                        "Chain step '{}' timed out after {:?}",
                        link.description, self.chain_config.step_timeout
                    )));
                }
            };

            let step_duration = step_start.elapsed();
            resource_usage.llm_calls += 1;
            resource_usage.tokens_consumed += step_input.len() + step_output.len(); // Rough estimate

            // Record the execution step
            execution_trace.push(ExecutionStep {
                step_id: link.step_id.clone(),
                step_type: StepType::LlmCall,
                input: step_input,
                output: step_output.clone(),
                duration: step_duration,
                success: true,
                metadata: serde_json::json!({
                    "chain_position": index,
                    "description": link.description,
                    "required": link.required,
                }),
            });

            // Update context and output for next step
            if self.chain_config.preserve_context {
                context = format!("{}\nStep {}: {}", context, index + 1, step_output);
            }
            current_output = step_output;

            // Break if we hit max chain length
            if index + 1 >= self.chain_config.max_chain_length {
                log::warn!(
                    "Reached maximum chain length of {}, stopping execution",
                    self.chain_config.max_chain_length
                );
                break;
            }
        }

        let total_duration = start_time.elapsed();

        // Perform quality check if enabled
        let quality_score = if self.chain_config.quality_check {
            Some(self.assess_output_quality(&current_output).await?)
        } else {
            None
        };

        let metadata = WorkflowMetadata {
            pattern_used: "prompt_chaining".to_string(),
            execution_time: total_duration,
            steps_executed: execution_trace.len(),
            success: true,
            quality_score,
            resources_used: resource_usage,
        };

        Ok(WorkflowOutput {
            task_id: input.task_id.clone(),
            agent_id: input.agent_id.clone(),
            result: current_output,
            metadata,
            execution_trace,
            timestamp: Utc::now(),
        })
    }

    /// Create a default prompt chain based on the task
    fn create_default_chain(&self, task: &str) -> Vec<ChainLink> {
        // Analyze the task to determine appropriate chain
        if task.contains("analyze") || task.contains("research") {
            self.create_analysis_chain()
        } else if task.contains("write") || task.contains("create") {
            self.create_generation_chain()
        } else if task.contains("solve") || task.contains("calculate") {
            self.create_problem_solving_chain()
        } else {
            self.create_generic_chain()
        }
    }

    /// Create a chain for analysis tasks
    fn create_analysis_chain(&self) -> Vec<ChainLink> {
        vec![
            ChainLink {
                step_id: "extract_info".to_string(),
                prompt_template: "Extract the key information and data from the following:"
                    .to_string(),
                description: "Information extraction".to_string(),
                required: true,
            },
            ChainLink {
                step_id: "identify_patterns".to_string(),
                prompt_template:
                    "Identify patterns, trends, and relationships in the extracted information:"
                        .to_string(),
                description: "Pattern identification".to_string(),
                required: true,
            },
            ChainLink {
                step_id: "synthesize_analysis".to_string(),
                prompt_template:
                    "Synthesize the findings into a comprehensive analysis with conclusions:"
                        .to_string(),
                description: "Analysis synthesis".to_string(),
                required: true,
            },
        ]
    }

    /// Create a chain for content generation tasks
    fn create_generation_chain(&self) -> Vec<ChainLink> {
        vec![
            ChainLink {
                step_id: "plan_structure".to_string(),
                prompt_template: "Create an outline and structure for the following request:"
                    .to_string(),
                description: "Content planning".to_string(),
                required: true,
            },
            ChainLink {
                step_id: "generate_content".to_string(),
                prompt_template: "Based on the outline, generate the requested content:"
                    .to_string(),
                description: "Content generation".to_string(),
                required: true,
            },
            ChainLink {
                step_id: "refine_output".to_string(),
                prompt_template:
                    "Review and refine the content for clarity, coherence, and quality:".to_string(),
                description: "Content refinement".to_string(),
                required: false,
            },
        ]
    }

    /// Create a chain for problem-solving tasks
    fn create_problem_solving_chain(&self) -> Vec<ChainLink> {
        vec![
            ChainLink {
                step_id: "understand_problem".to_string(),
                prompt_template: "Break down and clearly understand the problem:".to_string(),
                description: "Problem understanding".to_string(),
                required: true,
            },
            ChainLink {
                step_id: "identify_approach".to_string(),
                prompt_template: "Identify the best approach or method to solve this problem:"
                    .to_string(),
                description: "Solution approach".to_string(),
                required: true,
            },
            ChainLink {
                step_id: "solve_step_by_step".to_string(),
                prompt_template: "Solve the problem step by step using the identified approach:"
                    .to_string(),
                description: "Step-by-step solution".to_string(),
                required: true,
            },
            ChainLink {
                step_id: "verify_solution".to_string(),
                prompt_template: "Verify the solution and check for any errors or improvements:"
                    .to_string(),
                description: "Solution verification".to_string(),
                required: false,
            },
        ]
    }

    /// Create a generic chain for general tasks
    fn create_generic_chain(&self) -> Vec<ChainLink> {
        vec![
            ChainLink {
                step_id: "understand_task".to_string(),
                prompt_template: "Understand and clarify what is being requested:".to_string(),
                description: "Task understanding".to_string(),
                required: true,
            },
            ChainLink {
                step_id: "execute_task".to_string(),
                prompt_template: "Execute the task based on the understanding:".to_string(),
                description: "Task execution".to_string(),
                required: true,
            },
        ]
    }

    /// Assess the quality of the output
    async fn assess_output_quality(&self, output: &str) -> EvolutionResult<f64> {
        let quality_prompt = format!(
            "Rate the quality of the following output on a scale of 0.0 to 1.0, considering clarity, completeness, and accuracy. Respond with only the numerical score:\n\n{}",
            output
        );

        let quality_response = self
            .llm_adapter
            .complete(&quality_prompt, CompletionOptions::default())
            .await?;

        // Parse the quality score
        quality_response.trim().parse::<f64>().map_err(|e| {
            crate::EvolutionError::WorkflowError(format!("Failed to parse quality score: {}", e))
        })
    }
}

#[async_trait]
impl WorkflowPattern for PromptChaining {
    fn pattern_name(&self) -> &'static str {
        "prompt_chaining"
    }

    async fn execute(&self, input: WorkflowInput) -> EvolutionResult<WorkflowOutput> {
        log::info!(
            "Executing prompt chaining workflow for task: {}",
            input.task_id
        );
        self.execute_predefined_chain(&input).await
    }

    fn is_suitable_for(&self, task_analysis: &TaskAnalysis) -> bool {
        // Prompt chaining is suitable for:
        // - Simple to moderate complexity tasks
        // - Tasks that benefit from step-by-step processing
        // - Sequential analysis or generation tasks
        match task_analysis.complexity {
            TaskComplexity::Simple | TaskComplexity::Moderate => true,
            TaskComplexity::Complex => task_analysis.estimated_steps <= 5,
            TaskComplexity::VeryComplex => false,
        }
    }

    fn estimate_execution_time(&self, input: &WorkflowInput) -> Duration {
        // Estimate based on chain length and complexity
        let estimated_steps = if input.prompt.len() > 1000 { 4 } else { 3 };
        Duration::from_secs(estimated_steps * 30) // Rough estimate: 30s per step
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chain_config_default() {
        let config = ChainConfig::default();
        assert_eq!(config.max_chain_length, 5);
        assert_eq!(config.step_timeout, Duration::from_secs(60));
        assert!(config.preserve_context);
        assert!(config.quality_check);
    }

    #[test]
    fn test_analysis_chain_creation() {
        use crate::llm_adapter::LlmAdapterFactory;

        let mock_adapter = LlmAdapterFactory::create_mock("test");
        let chaining = PromptChaining::new(mock_adapter);

        let chain = chaining.create_analysis_chain();
        assert_eq!(chain.len(), 3);
        assert_eq!(chain[0].step_id, "extract_info");
        assert_eq!(chain[1].step_id, "identify_patterns");
        assert_eq!(chain[2].step_id, "synthesize_analysis");
    }
}
