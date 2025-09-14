# Terraphim AI Agent Workflow Patterns Guide

## Introduction

This guide provides comprehensive documentation for the 5 core workflow patterns implemented in the Terraphim AI Agent Evolution System. Each pattern is designed for specific use cases and execution scenarios, providing reliable and optimized AI agent orchestration.

## Pattern Overview

| Pattern | Primary Use Case | Execution Model | Best For |
|---------|------------------|-----------------|----------|
| **Prompt Chaining** | Step-by-step processing | Serial | Complex analysis, quality-critical tasks |
| **Routing** | Cost/performance optimization | Single path | Varying complexity tasks, resource optimization |
| **Parallelization** | Independent subtasks | Concurrent | Multi-perspective analysis, large data processing |
| **Orchestrator-Workers** | Complex coordination | Hierarchical | Multi-step projects, specialized expertise |
| **Evaluator-Optimizer** | Quality improvement | Iterative | Creative tasks, accuracy-critical outputs |

## 1. Prompt Chaining Pattern

### Overview

Prompt Chaining executes tasks through a series of connected steps, where each step's output becomes the next step's input. This creates a reliable pipeline for complex tasks that require step-by-step processing.

```rust
use terraphim_agent_evolution::workflows::prompt_chaining::*;
use terraphim_agent_evolution::*;

// Basic usage
let adapter = LlmAdapterFactory::create_mock("test");
let chaining = PromptChaining::new(adapter);

let workflow_input = WorkflowInput {
    task_id: "analysis_task".to_string(),
    agent_id: "analyst_agent".to_string(),
    prompt: "Analyze the market trends in renewable energy".to_string(),
    context: None,
    parameters: WorkflowParameters::default(),
    timestamp: Utc::now(),
};

let result = chaining.execute(workflow_input).await?;
```

### Configuration Options

```rust
let chain_config = ChainConfig {
    max_steps: 5,
    preserve_context: true,
    quality_check: true,
    timeout_per_step: Duration::from_secs(60),
    context_window: 2000,
};

let chaining = PromptChaining::with_config(adapter, chain_config);
```

### Step Types

#### Analysis Chain
- **Extract Information**: Pull key data from input
- **Identify Patterns**: Find relationships and trends  
- **Synthesize Analysis**: Combine insights into conclusions

#### Generation Chain
- **Brainstorm Ideas**: Generate initial concepts
- **Develop Content**: Expand ideas into full content
- **Refine Output**: Polish and improve final result

#### Problem-Solving Chain
- **Understand Problem**: Break down the core issue
- **Generate Solutions**: Create multiple solution approaches
- **Evaluate Options**: Assess feasibility and effectiveness
- **Recommend Action**: Provide final recommendation

### Best Practices

1. **Keep Steps Focused**: Each step should have a single, clear purpose
2. **Preserve Context**: Essential information should flow between steps
3. **Add Quality Gates**: Validate outputs at critical steps
4. **Handle Failures**: Implement retry logic for failed steps

### Example: Document Analysis Chain

```rust
// Custom analysis chain for legal document review
let legal_analysis_steps = vec![
    ChainStep {
        step_id: "extract_clauses".to_string(),
        prompt_template: "Extract all key clauses from this legal document: {input}".to_string(),
        expected_output: "structured_list".to_string(),
        validation_criteria: vec!["completeness".to_string()],
    },
    ChainStep {
        step_id: "assess_risks".to_string(),
        prompt_template: "Assess legal risks in these clauses: {input}".to_string(),
        expected_output: "risk_assessment".to_string(),
        validation_criteria: vec!["thoroughness".to_string()],
    },
    ChainStep {
        step_id: "provide_recommendations".to_string(),
        prompt_template: "Provide recommendations based on this risk assessment: {input}".to_string(),
        expected_output: "action_items".to_string(),
        validation_criteria: vec!["actionability".to_string()],
    },
];
```

## 2. Routing Pattern

### Overview

The Routing pattern intelligently directs tasks to the most appropriate execution path based on multiple criteria including cost, performance, and task complexity.

```rust
use terraphim_agent_evolution::workflows::routing::*;

let primary_adapter = LlmAdapterFactory::create_mock("gpt-4");
let routing = Routing::new(primary_adapter);

// Add alternative routes
let routing = routing
    .add_route("fast", LlmAdapterFactory::create_mock("gpt-3.5"), 0.1, 0.9)
    .add_route("precise", LlmAdapterFactory::create_mock("claude-3"), 0.3, 0.95);

let result = routing.execute(workflow_input).await?;
```

### Route Configuration

```rust
let route_config = RouteConfig {
    cost_weight: 0.4,      // 40% weight on cost optimization
    performance_weight: 0.3, // 30% weight on speed
    quality_weight: 0.3,    // 30% weight on output quality
    fallback_strategy: FallbackStrategy::BestAvailable,
    max_retries: 3,
};
```

### Route Selection Criteria

#### Task Complexity Assessment
- **Simple**: Single-step, clear instructions, basic responses
- **Moderate**: Multi-step, some analysis required, structured output
- **Complex**: Deep analysis, creative thinking, specialized knowledge
- **Expert**: Domain-specific expertise, high accuracy requirements

#### Cost Optimization
```rust
// Example cost-performance matrix
let routes = vec![
    Route {
        name: "budget".to_string(),
        adapter: cheap_adapter,
        cost_score: 0.1,        // Very low cost
        performance_score: 0.7, // Moderate performance
        quality_score: 0.6,     // Basic quality
    },
    Route {
        name: "balanced".to_string(),
        adapter: mid_tier_adapter,
        cost_score: 0.3,        // Medium cost
        performance_score: 0.8, // Good performance
        quality_score: 0.8,     // Good quality
    },
    Route {
        name: "premium".to_string(),
        adapter: high_end_adapter,
        cost_score: 0.8,        // High cost
        performance_score: 0.9, // Excellent performance
        quality_score: 0.95,    // Excellent quality
    },
];
```

### Dynamic Route Selection

```rust
impl TaskRouter {
    fn select_optimal_route(&self, analysis: &TaskAnalysis) -> Route {
        let routes = self.available_routes();
        let mut best_route = None;
        let mut best_score = 0.0;
        
        for route in routes {
            let score = self.calculate_route_score(route, analysis);
            if score > best_score {
                best_score = score;
                best_route = Some(route);
            }
        }
        
        best_route.unwrap_or(self.default_route())
    }
}
```

## 3. Parallelization Pattern

### Overview

The Parallelization pattern executes multiple independent tasks concurrently and intelligently aggregates their results, significantly reducing execution time while potentially improving output quality through multiple perspectives.

```rust
use terraphim_agent_evolution::workflows::parallelization::*;

let parallel_config = ParallelConfig {
    max_parallel_tasks: 4,
    task_timeout: Duration::from_secs(120),
    aggregation_strategy: AggregationStrategy::Synthesis,
    failure_threshold: 0.5, // 50% of tasks must succeed
    retry_failed_tasks: false,
};

let parallelization = Parallelization::with_config(adapter, parallel_config);
let result = parallelization.execute(workflow_input).await?;
```

### Task Decomposition Strategies

#### Comparison Tasks
```rust
// Automatically creates comparison-focused parallel tasks
let comparison_tasks = vec![
    ParallelTask {
        task_id: "comparison_analysis".to_string(),
        prompt: "Analyze the key aspects and criteria for comparison".to_string(),
        description: "Identify comparison criteria".to_string(),
        priority: TaskPriority::High,
        expected_output_type: "analysis".to_string(),
    },
    ParallelTask {
        task_id: "pros_cons".to_string(),
        prompt: "List the pros and cons for each option".to_string(),
        description: "Evaluate advantages and disadvantages".to_string(),
        priority: TaskPriority::High,
        expected_output_type: "evaluation".to_string(),
    },
];
```

#### Research Tasks
```rust
let research_tasks = vec![
    ParallelTask {
        task_id: "background_research".to_string(),
        prompt: "Research the background and context".to_string(),
        description: "Gather background information".to_string(),
        priority: TaskPriority::High,
        expected_output_type: "background".to_string(),
    },
    ParallelTask {
        task_id: "current_state".to_string(),
        prompt: "Analyze current developments".to_string(),
        description: "Current state analysis".to_string(),
        priority: TaskPriority::High,
        expected_output_type: "analysis".to_string(),
    },
    ParallelTask {
        task_id: "implications".to_string(),
        prompt: "Identify implications and impacts".to_string(),
        description: "Impact analysis".to_string(),
        priority: TaskPriority::Normal,
        expected_output_type: "implications".to_string(),
    },
];
```

### Aggregation Strategies

#### 1. Concatenation
Simple merging of all results:
```rust
AggregationStrategy::Concatenation
// Output: "## Result 1\n[content]\n\n## Result 2\n[content]..."
```

#### 2. Best Result Selection
Chooses highest quality output:
```rust
AggregationStrategy::BestResult
// Uses quality scoring to select the single best result
```

#### 3. LLM Synthesis
Intelligent combination using LLM:
```rust
AggregationStrategy::Synthesis
// Creates coherent synthesis of all perspectives
```

#### 4. Majority Vote
Consensus-based selection:
```rust
AggregationStrategy::MajorityVote
// Selects most common result across parallel executions
```

#### 5. Structured Combination
Organized section-based combination:
```rust
AggregationStrategy::StructuredCombination
// Creates structured document with clear sections
```

### Batch Execution Management

```rust
impl Parallelization {
    async fn execute_task_batches(&self, tasks: Vec<ParallelTask>) -> Result<Vec<ParallelTaskResult>> {
        let mut all_results = Vec::new();
        
        // Sort by priority (Critical first)
        tasks.sort_by(|a, b| b.priority.cmp(&a.priority));
        
        // Process in batches to respect max_parallel_tasks limit
        for batch in tasks.chunks(self.parallel_config.max_parallel_tasks) {
            let batch_futures: Vec<_> = batch.iter()
                .map(|task| self.execute_single_task(task.clone()))
                .collect();
                
            let batch_results = join_all(batch_futures).await;
            all_results.extend(batch_results);
            
            // Brief delay between batches
            if batch.len() == self.parallel_config.max_parallel_tasks {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }

        Ok(all_results)
    }
}
```

## 4. Orchestrator-Workers Pattern

### Overview

The Orchestrator-Workers pattern implements hierarchical task execution where an orchestrator agent creates detailed execution plans and coordinates specialized worker agents to execute specific subtasks.

```rust
use terraphim_agent_evolution::workflows::orchestrator_workers::*;

let orchestrator_adapter = LlmAdapterFactory::create_mock("orchestrator");
let orchestrator = OrchestratorWorkers::new(orchestrator_adapter);

// Add specialized workers
let orchestrator = orchestrator
    .add_worker(WorkerRole::Analyst, LlmAdapterFactory::create_mock("analyst"))
    .add_worker(WorkerRole::Writer, LlmAdapterFactory::create_mock("writer"));

let result = orchestrator.execute(workflow_input).await?;
```

### Worker Roles and Specializations

```rust
pub enum WorkerRole {
    Analyst,     // Data analysis and insights
    Researcher,  // Information gathering and validation
    Writer,      // Content creation and documentation
    Reviewer,    // Quality assurance and feedback
    Validator,   // Accuracy and consistency checking
    Synthesizer, // Result integration and final assembly
}
```

#### Analyst Worker
- **Purpose**: Break down complex information and identify patterns
- **Specialization**: Data analysis, trend identification, insight generation
- **Output**: Structured analysis with key findings and recommendations

```rust
let analyst_prompt = format!(
    "You are a skilled analyst. Focus on breaking down complex information, identifying patterns, and providing insights.
    
    Task: {}
    
    Expected deliverable: {}
    
    Quality criteria: {}",
    task.instruction,
    task.expected_deliverable,
    task.quality_criteria.join(", ")
);
```

#### Researcher Worker  
- **Purpose**: Gather comprehensive information and verify facts
- **Specialization**: Information collection, fact checking, source validation
- **Output**: Well-sourced findings with verified information

#### Writer Worker
- **Purpose**: Create clear, engaging, and well-structured content
- **Specialization**: Content creation, documentation, communication
- **Output**: Polished written content that effectively communicates ideas

#### Reviewer Worker
- **Purpose**: Evaluate content quality and provide constructive feedback
- **Specialization**: Quality assessment, improvement suggestions
- **Output**: Detailed review with specific recommendations

### Coordination Strategies

#### Sequential Execution
```rust
CoordinationStrategy::Sequential
// Workers execute one after another with context accumulation
```

#### Parallel Coordinated  
```rust
CoordinationStrategy::ParallelCoordinated
// Workers execute in dependency-based levels, parallel within each level
```

#### Pipeline
```rust
CoordinationStrategy::Pipeline
// Streaming execution where outputs flow directly to next workers
```

#### Dynamic
```rust
CoordinationStrategy::Dynamic
// Adaptive scheduling based on performance and resource availability
```

### Execution Plan Generation

```rust
impl OrchestratorWorkers {
    async fn create_execution_plan(&self, prompt: &str, context: &Option<String>) -> Result<ExecutionPlan> {
        let planning_prompt = format!(
            r#"Create a comprehensive execution plan that breaks down this task into specific worker assignments.

Task: {}
Context: {}

Consider:
1. What specialized workers are needed?
2. What are the specific deliverables?
3. What dependencies exist between tasks?
4. What quality criteria should be applied?

Provide a structured plan with clear task assignments."#,
            prompt, 
            context.as_deref().unwrap_or("")
        );

        let planning_result = self.orchestrator_adapter
            .complete(&planning_prompt, CompletionOptions::default())
            .await?;

        // Parse into structured execution plan
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
            estimated_duration: Duration::from_secs(300),
        })
    }
}
```

### Quality Gates and Validation

```rust
impl OrchestratorWorkers {
    async fn evaluate_quality_gate(&self, results: &[WorkerResult]) -> Result<bool> {
        let successful_results: Vec<_> = results.iter()
            .filter(|r| r.success)
            .collect();

        if successful_results.is_empty() {
            return Ok(false);
        }

        let average_quality: f64 = successful_results.iter()
            .map(|r| r.quality_score)
            .sum::<f64>() / successful_results.len() as f64;

        let success_rate = successful_results.len() as f64 / results.len() as f64;

        Ok(average_quality >= self.orchestration_config.quality_gate_threshold 
           && success_rate >= 0.5)
    }
}
```

## 5. Evaluator-Optimizer Pattern

### Overview

The Evaluator-Optimizer pattern implements iterative quality improvement through evaluation and refinement loops, continuously enhancing output quality until it meets specified thresholds.

```rust
use terraphim_agent_evolution::workflows::evaluator_optimizer::*;

let optimization_config = OptimizationConfig {
    max_iterations: 3,
    quality_threshold: 0.85,
    improvement_threshold: 0.05, // 5% minimum improvement
    evaluation_criteria: vec![
        EvaluationCriterion::Accuracy,
        EvaluationCriterion::Completeness,
        EvaluationCriterion::Clarity,
        EvaluationCriterion::Relevance,
    ],
    optimization_strategy: OptimizationStrategy::Adaptive,
    early_stopping: true,
};

let evaluator = EvaluatorOptimizer::with_config(adapter, optimization_config);
let result = evaluator.execute(workflow_input).await?;
```

### Evaluation Criteria

```rust
pub enum EvaluationCriterion {
    Accuracy,    // Factual correctness and precision
    Completeness,// Thorough coverage of all aspects
    Clarity,     // Clear and understandable presentation
    Relevance,   // Direct connection to the request
    Coherence,   // Logical flow and consistency
    Depth,       // Thorough analysis and insight
    Creativity,  // Original thinking and novel approaches
    Conciseness, // Efficient use of language
}
```

### Optimization Strategies

#### Incremental Optimization
```rust
OptimizationStrategy::Incremental
// Makes small improvements while preserving structure
```

#### Selective Optimization
```rust
OptimizationStrategy::Selective
// Regenerates specific sections that need improvement
```

#### Complete Regeneration
```rust
OptimizationStrategy::Complete
// Creates entirely new content with feedback incorporated
```

#### Adaptive Strategy
```rust
OptimizationStrategy::Adaptive
// Chooses strategy based on evaluation results
```

### Evaluation Process

```rust
impl EvaluatorOptimizer {
    async fn evaluate_content(&self, content: &str, original_prompt: &str, iteration: usize) -> Result<Evaluation> {
        let evaluation_prompt = format!(
            r#"Evaluate the following content against the original request and quality criteria:

Original Request: {}

Content to Evaluate:
{}

Evaluation Criteria:
{}

Provide:
1. Overall quality score (0.0 to 1.0)
2. Individual scores for each criterion
3. Key strengths of the content
4. Areas that need improvement
5. Specific suggestions for improvement"#,
            original_prompt,
            content,
            self.get_criteria_descriptions().join("\n")
        );

        let evaluation_response = self.evaluator_adapter
            .complete(&evaluation_prompt, CompletionOptions::default())
            .await?;

        // Parse evaluation response
        let overall_score = self.extract_overall_score(&evaluation_response);
        let criterion_scores = self.extract_criterion_scores(&evaluation_response);
        let (strengths, weaknesses, suggestions) = self.extract_feedback(&evaluation_response);

        Ok(Evaluation {
            iteration,
            overall_score,
            criterion_scores,
            strengths,
            weaknesses,
            improvement_suggestions: suggestions,
            meets_threshold: overall_score >= self.optimization_config.quality_threshold,
        })
    }
}
```

### Optimization Loop

```rust
impl EvaluatorOptimizer {
    async fn execute_optimization_loop(&self, input: &WorkflowInput) -> Result<WorkflowOutput> {
        // Generate initial content
        let mut current_content = self.generate_initial_content(&input.prompt, &input.context).await?;
        let mut iterations = Vec::new();
        let mut best_score = 0.0;

        for iteration in 1..=self.optimization_config.max_iterations {
            // Evaluate current content
            let evaluation = self.evaluate_content(&current_content, &input.prompt, iteration).await?;

            // Check if quality threshold is met
            if evaluation.meets_threshold && self.optimization_config.early_stopping {
                break;
            }

            // Check for sufficient improvement
            let improvement_delta = evaluation.overall_score - best_score;
            if iteration > 1 && improvement_delta < self.optimization_config.improvement_threshold {
                break;
            }

            best_score = evaluation.overall_score.max(best_score);

            // Generate optimization actions
            let actions = self.generate_optimization_actions(&evaluation).await?;
            
            // Apply optimizations
            current_content = self.apply_optimizations(&current_content, &actions, &input.prompt).await?;

            iterations.push(OptimizationIteration {
                iteration,
                content: current_content.clone(),
                evaluation,
                actions_taken: actions,
                improvement_delta,
                duration: Duration::from_millis(100), // Would track actual time
            });
        }

        // Return final optimized content
        Ok(WorkflowOutput {
            task_id: input.task_id.clone(),
            agent_id: input.agent_id.clone(),
            result: current_content,
            // ... additional metadata
        })
    }
}
```

## Pattern Selection Guidelines

### Decision Matrix

| Task Characteristic | Recommended Pattern |
|---------------------|-------------------|
| **Step-by-step analysis needed** | Prompt Chaining |
| **Cost optimization priority** | Routing |
| **Independent subtasks** | Parallelization |
| **Multiple expertise areas** | Orchestrator-Workers |
| **Quality critical** | Evaluator-Optimizer |
| **Simple single-step** | Routing (fast route) |
| **Complex multi-domain** | Orchestrator-Workers |
| **Creative refinement** | Evaluator-Optimizer |
| **Time-sensitive** | Parallelization or Routing |

### Performance Characteristics

| Pattern | Latency | Resource Usage | Quality | Cost |
|---------|---------|----------------|---------|------|
| Prompt Chaining | Medium | Low | High | Low |
| Routing | Variable | Variable | Variable | Optimized |
| Parallelization | Low | High | High | High |
| Orchestrator-Workers | High | High | Very High | High |
| Evaluator-Optimizer | Very High | Medium | Very High | Medium |

## Integration with Evolution System

All patterns automatically integrate with the Agent Evolution System:

```rust
// Example: Evolution integration happens automatically
let mut manager = EvolutionWorkflowManager::new("agent_001".to_string());

let result = manager.execute_task(
    "analysis_task".to_string(),
    "Analyze market trends in renewable energy".to_string(),
    None,
).await?;

// System automatically:
// 1. Analyzes task to select best pattern
// 2. Executes chosen workflow pattern  
// 3. Updates memory, tasks, and lessons
// 4. Creates evolution snapshots
// 5. Tracks performance metrics
```

## Best Practices

### General Guidelines

1. **Choose the Right Pattern**: Use the decision matrix to select optimal patterns
2. **Configure Appropriately**: Tune parameters for your specific use case
3. **Monitor Performance**: Track execution metrics and quality scores
4. **Handle Failures**: Implement robust error handling and recovery
5. **Quality Gates**: Use thresholds to maintain output standards

### Error Handling

```rust
// Robust error handling example
match workflow.execute(input).await {
    Ok(output) => {
        if output.metadata.quality_score.unwrap_or(0.0) < minimum_quality {
            // Retry with different pattern or parameters
        }
    }
    Err(e) => {
        // Log error and try fallback strategy
        log::error!("Workflow execution failed: {}", e);
    }
}
```

### Performance Optimization

```rust
// Performance monitoring integration
let performance_tracker = PerformanceTracker::new();
let start_time = Instant::now();

let result = workflow.execute(input).await?;

performance_tracker.record_execution(
    workflow.pattern_name(),
    start_time.elapsed(),
    result.metadata.resources_used.clone(),
    result.metadata.quality_score,
);
```

This comprehensive guide provides all the information needed to effectively use and customize the Terraphim AI Agent Evolution System's workflow patterns for your specific use cases.