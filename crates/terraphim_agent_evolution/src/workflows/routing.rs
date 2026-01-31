//! Routing workflow pattern
//!
//! This pattern intelligently routes tasks to the most appropriate model or workflow
//! based on task complexity, domain, and resource constraints.

use std::collections::HashMap;
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

/// Routing workflow that selects the best execution path
pub struct Routing {
    primary_adapter: Arc<dyn LlmAdapter>,
    route_config: RouteConfig,
    alternative_adapters: HashMap<String, Arc<dyn LlmAdapter>>,
}

/// Configuration for routing decisions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteConfig {
    pub enable_cost_optimization: bool,
    pub enable_performance_routing: bool,
    pub enable_domain_routing: bool,
    pub fallback_enabled: bool,
    pub routing_timeout: Duration,
}

impl Default for RouteConfig {
    fn default() -> Self {
        Self {
            enable_cost_optimization: true,
            enable_performance_routing: true,
            enable_domain_routing: true,
            fallback_enabled: true,
            routing_timeout: Duration::from_secs(10),
        }
    }
}

/// Route information for execution path
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Route {
    pub route_id: String,
    pub provider: String,
    pub model: String,
    pub reasoning: String,
    pub confidence: f64,
    pub estimated_cost: f64,
    pub estimated_time: Duration,
}

/// Router that makes routing decisions
pub struct TaskRouter {
    config: RouteConfig,
}

impl TaskRouter {
    pub fn new(config: RouteConfig) -> Self {
        Self { config }
    }

    /// Analyze task and select the best route
    pub async fn select_route(
        &self,
        input: &WorkflowInput,
        available_routes: &HashMap<String, Arc<dyn LlmAdapter>>,
    ) -> EvolutionResult<Route> {
        let task_analysis = self.analyze_task(input).await?;
        let routes = self
            .evaluate_routes(&task_analysis, available_routes)
            .await?;

        // Select the best route based on multiple criteria
        let best_route = self.select_best_route(routes)?;

        log::info!(
            "Selected route '{}' for task '{}': {}",
            best_route.route_id,
            input.task_id,
            best_route.reasoning
        );

        Ok(best_route)
    }

    /// Analyze the task to determine routing criteria
    async fn analyze_task(&self, input: &WorkflowInput) -> EvolutionResult<TaskAnalysis> {
        let prompt = &input.prompt;
        let mut complexity = TaskComplexity::Simple;
        let mut domain = "general".to_string();
        let mut estimated_steps = 1;

        // Simple heuristic-based analysis
        // In a real implementation, this might use ML models or more sophisticated analysis

        // Complexity analysis
        if prompt.len() > 2000 {
            complexity = TaskComplexity::VeryComplex;
            estimated_steps = 5;
        } else if prompt.len() > 1000 {
            complexity = TaskComplexity::Complex;
            estimated_steps = 3;
        } else if prompt.len() > 500 {
            complexity = TaskComplexity::Moderate;
            estimated_steps = 2;
        }

        // Domain detection
        if prompt.to_lowercase().contains("code") || prompt.to_lowercase().contains("programming") {
            domain = "coding".to_string();
        } else if prompt.to_lowercase().contains("math")
            || prompt.to_lowercase().contains("calculate")
        {
            domain = "mathematics".to_string();
        } else if prompt.to_lowercase().contains("write") || prompt.to_lowercase().contains("story")
        {
            domain = "creative".to_string();
        } else if prompt.to_lowercase().contains("analyze")
            || prompt.to_lowercase().contains("research")
        {
            domain = "analysis".to_string();
        }

        // Decomposition check
        let requires_decomposition = prompt.contains("step by step")
            || prompt.contains("break down")
            || matches!(
                complexity,
                TaskComplexity::Complex | TaskComplexity::VeryComplex
            );

        // Parallelization check
        let suitable_for_parallel = prompt.contains("compare")
            || prompt.contains("multiple")
            || prompt.contains("different approaches");

        // Quality critical check
        let quality_critical = prompt.contains("important")
            || prompt.contains("critical")
            || prompt.contains("precise")
            || prompt.contains("accurate");

        Ok(TaskAnalysis {
            complexity,
            domain,
            requires_decomposition,
            suitable_for_parallel,
            quality_critical,
            estimated_steps,
        })
    }

    /// Evaluate all available routes
    async fn evaluate_routes(
        &self,
        task_analysis: &TaskAnalysis,
        available_routes: &HashMap<String, Arc<dyn LlmAdapter>>,
    ) -> EvolutionResult<Vec<Route>> {
        let mut routes = Vec::new();

        for (route_id, adapter) in available_routes {
            let route = self
                .evaluate_single_route(route_id, adapter, task_analysis)
                .await?;
            routes.push(route);
        }

        Ok(routes)
    }

    /// Evaluate a single route
    async fn evaluate_single_route(
        &self,
        route_id: &str,
        _adapter: &Arc<dyn LlmAdapter>,
        task_analysis: &TaskAnalysis,
    ) -> EvolutionResult<Route> {
        // Route evaluation logic based on provider capabilities
        let (provider, model, confidence, cost, time, reasoning) = match route_id {
            "openai_gpt4" => {
                let confidence = match task_analysis.complexity {
                    TaskComplexity::Simple => 0.9,
                    TaskComplexity::Moderate => 0.95,
                    TaskComplexity::Complex => 0.98,
                    TaskComplexity::VeryComplex => 0.99,
                };
                let cost = match task_analysis.complexity {
                    TaskComplexity::Simple => 0.01,
                    TaskComplexity::Moderate => 0.03,
                    TaskComplexity::Complex => 0.08,
                    TaskComplexity::VeryComplex => 0.15,
                };
                let time = Duration::from_secs(match task_analysis.complexity {
                    TaskComplexity::Simple => 10,
                    TaskComplexity::Moderate => 20,
                    TaskComplexity::Complex => 45,
                    TaskComplexity::VeryComplex => 90,
                });
                (
                    "openai",
                    "gpt-4",
                    confidence,
                    cost,
                    time,
                    "High-quality model for complex tasks",
                )
            }
            "openai_gpt35" => {
                let confidence = match task_analysis.complexity {
                    TaskComplexity::Simple => 0.85,
                    TaskComplexity::Moderate => 0.80,
                    TaskComplexity::Complex => 0.70,
                    TaskComplexity::VeryComplex => 0.60,
                };
                let cost = match task_analysis.complexity {
                    TaskComplexity::Simple => 0.002,
                    TaskComplexity::Moderate => 0.005,
                    TaskComplexity::Complex => 0.012,
                    TaskComplexity::VeryComplex => 0.025,
                };
                let time = Duration::from_secs(match task_analysis.complexity {
                    TaskComplexity::Simple => 5,
                    TaskComplexity::Moderate => 8,
                    TaskComplexity::Complex => 15,
                    TaskComplexity::VeryComplex => 30,
                });
                (
                    "openai",
                    "gpt-3.5-turbo",
                    confidence,
                    cost,
                    time,
                    "Fast and cost-effective for simple tasks",
                )
            }
            "anthropic_claude" => {
                let confidence = match task_analysis.complexity {
                    TaskComplexity::Simple => 0.88,
                    TaskComplexity::Moderate => 0.92,
                    TaskComplexity::Complex => 0.95,
                    TaskComplexity::VeryComplex => 0.97,
                };
                let cost = match task_analysis.complexity {
                    TaskComplexity::Simple => 0.015,
                    TaskComplexity::Moderate => 0.035,
                    TaskComplexity::Complex => 0.085,
                    TaskComplexity::VeryComplex => 0.18,
                };
                let time = Duration::from_secs(match task_analysis.complexity {
                    TaskComplexity::Simple => 8,
                    TaskComplexity::Moderate => 15,
                    TaskComplexity::Complex => 35,
                    TaskComplexity::VeryComplex => 70,
                });
                (
                    "anthropic",
                    "claude-3",
                    confidence,
                    cost,
                    time,
                    "Excellent for analysis and reasoning tasks",
                )
            }
            _ => (
                "unknown",
                "unknown",
                0.5,
                0.1,
                Duration::from_secs(30),
                "Unknown provider",
            ),
        };

        Ok(Route {
            route_id: route_id.to_string(),
            provider: provider.to_string(),
            model: model.to_string(),
            reasoning: reasoning.to_string(),
            confidence,
            estimated_cost: cost,
            estimated_time: time,
        })
    }

    /// Select the best route from available options
    fn select_best_route(&self, routes: Vec<Route>) -> EvolutionResult<Route> {
        if routes.is_empty() {
            return Err(crate::EvolutionError::WorkflowError(
                "No routes available for selection".to_string(),
            ));
        }

        // Multi-criteria route selection
        let best_route = routes
            .into_iter()
            .max_by(|a, b| {
                let score_a = self.calculate_route_score(a);
                let score_b = self.calculate_route_score(b);
                score_a
                    .partial_cmp(&score_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .ok_or_else(|| {
                crate::error::EvolutionError::InvalidInput(
                    "No available routes for task routing".to_string(),
                )
            })?;

        Ok(best_route)
    }

    /// Calculate a composite score for route selection
    fn calculate_route_score(&self, route: &Route) -> f64 {
        let mut score = 0.0;

        // Confidence weight (40%)
        score += route.confidence * 0.4;

        // Cost optimization weight (30%) - lower cost is better
        let cost_score = if self.config.enable_cost_optimization {
            1.0 - (route.estimated_cost.min(1.0))
        } else {
            0.5 // Neutral if cost optimization disabled
        };
        score += cost_score * 0.3;

        // Performance weight (30%) - lower time is better
        let performance_score = if self.config.enable_performance_routing {
            let time_seconds = route.estimated_time.as_secs() as f64;
            1.0 - (time_seconds / 120.0).min(1.0) // Normalize to 2 minutes max
        } else {
            0.5 // Neutral if performance routing disabled
        };
        score += performance_score * 0.3;

        score
    }
}

impl Routing {
    /// Create a new routing workflow
    pub fn new(primary_adapter: Arc<dyn LlmAdapter>) -> Self {
        Self {
            primary_adapter,
            route_config: RouteConfig::default(),
            alternative_adapters: HashMap::new(),
        }
    }

    /// Add an alternative adapter for routing
    pub fn add_route(mut self, route_id: String, adapter: Arc<dyn LlmAdapter>) -> Self {
        self.alternative_adapters.insert(route_id, adapter);
        self
    }

    /// Execute with routing
    async fn execute_with_routing(&self, input: WorkflowInput) -> EvolutionResult<WorkflowOutput> {
        let start_time = Instant::now();
        let router = TaskRouter::new(self.route_config.clone());

        // Create available routes map
        let mut available_routes = self.alternative_adapters.clone();
        available_routes.insert("primary".to_string(), self.primary_adapter.clone());

        // Select the best route
        let route = router.select_route(&input, &available_routes).await?;
        let selected_adapter = available_routes.get(&route.route_id).ok_or_else(|| {
            crate::EvolutionError::WorkflowError(format!(
                "Selected route '{}' not found",
                route.route_id
            ))
        })?;

        // Execute the task with the selected adapter
        let execution_start = Instant::now();
        let result = selected_adapter
            .complete(&input.prompt, CompletionOptions::default())
            .await?;
        let execution_duration = execution_start.elapsed();

        // Create execution trace
        let execution_trace = vec![
            ExecutionStep {
                step_id: "route_selection".to_string(),
                step_type: StepType::Routing,
                input: format!("Task analysis and route evaluation for: {}", input.task_id),
                output: format!("Selected route: {} ({})", route.route_id, route.reasoning),
                duration: start_time.elapsed() - execution_duration,
                success: true,
                metadata: serde_json::json!({
                    "route": route,
                    "available_routes": available_routes.keys().collect::<Vec<_>>(),
                }),
            },
            ExecutionStep {
                step_id: "task_execution".to_string(),
                step_type: StepType::LlmCall,
                input: input.prompt.clone(),
                output: result.clone(),
                duration: execution_duration,
                success: true,
                metadata: serde_json::json!({
                    "provider": route.provider,
                    "model": route.model,
                }),
            },
        ];

        let resource_usage = ResourceUsage {
            llm_calls: 1,
            tokens_consumed: input.prompt.len() + result.len(),
            parallel_tasks: 0,
            memory_peak_mb: 10.0, // Rough estimate
        };

        let metadata = WorkflowMetadata {
            pattern_used: "routing".to_string(),
            execution_time: start_time.elapsed(),
            steps_executed: execution_trace.len(),
            success: true,
            quality_score: Some(route.confidence),
            resources_used: resource_usage,
        };

        Ok(WorkflowOutput {
            task_id: input.task_id,
            agent_id: input.agent_id,
            result,
            metadata,
            execution_trace,
            timestamp: Utc::now(),
        })
    }
}

#[async_trait]
impl WorkflowPattern for Routing {
    fn pattern_name(&self) -> &'static str {
        "routing"
    }

    async fn execute(&self, input: WorkflowInput) -> EvolutionResult<WorkflowOutput> {
        log::info!("Executing routing workflow for task: {}", input.task_id);
        self.execute_with_routing(input).await
    }

    fn is_suitable_for(&self, _task_analysis: &TaskAnalysis) -> bool {
        // Routing is suitable for all tasks as it's an optimization pattern
        // It's particularly beneficial when:
        // - Multiple providers/models are available
        // - Cost or performance optimization is important
        // - Task complexity varies significantly
        true
    }

    fn estimate_execution_time(&self, input: &WorkflowInput) -> Duration {
        // Add routing overhead to base execution time
        let base_time = Duration::from_secs(if input.prompt.len() > 1000 { 60 } else { 30 });
        let routing_overhead = Duration::from_secs(5);
        base_time + routing_overhead
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_config_default() {
        let config = RouteConfig::default();
        assert!(config.enable_cost_optimization);
        assert!(config.enable_performance_routing);
        assert!(config.enable_domain_routing);
        assert!(config.fallback_enabled);
        assert_eq!(config.routing_timeout, Duration::from_secs(10));
    }

    #[test]
    fn test_route_score_calculation() {
        let config = RouteConfig::default();
        let router = TaskRouter::new(config);

        let route = Route {
            route_id: "test".to_string(),
            provider: "test".to_string(),
            model: "test".to_string(),
            reasoning: "test".to_string(),
            confidence: 0.9,
            estimated_cost: 0.1,
            estimated_time: Duration::from_secs(30),
        };

        let score = router.calculate_route_score(&route);
        assert!(score > 0.0 && score <= 1.0);
    }
}
