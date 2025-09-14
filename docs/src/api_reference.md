# Terraphim AI Agent Evolution System - API Reference

## Overview

This document provides comprehensive API reference for the Terraphim AI Agent Evolution System. The API is designed around trait-based abstractions that provide flexibility and extensibility while maintaining type safety.

## Core Types and Traits

### Basic Types

```rust
pub type AgentId = String;
pub type TaskId = String;
pub type MemoryId = String;
pub type LessonId = String;
pub type EvolutionResult<T> = Result<T, EvolutionError>;
```

### Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum EvolutionError {
    #[error("Memory operation error: {0}")]
    MemoryError(String),
    
    #[error("Task operation error: {0}")]
    TaskError(String),
    
    #[error("Lesson operation error: {0}")]
    LessonError(String),
    
    #[error("LLM operation error: {0}")]
    LlmError(String),
    
    #[error("Workflow execution error: {0}")]
    WorkflowError(String),
    
    #[error("Persistence error: {0}")]
    PersistenceError(String),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
}
```

## Agent Evolution System

### AgentEvolutionSystem

Central coordinator for tracking agent development over time.

```rust
pub struct AgentEvolutionSystem {
    pub agent_id: AgentId,
    pub memory_evolution: VersionedMemory,
    pub tasks_evolution: VersionedTaskList,
    pub lessons_evolution: VersionedLessons,
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
}

impl AgentEvolutionSystem {
    /// Create a new evolution system for an agent
    pub fn new(agent_id: AgentId) -> Self;
    
    /// Create a snapshot of current agent state
    pub async fn create_snapshot(&self, description: String) -> EvolutionResult<()>;
    
    /// Get agent snapshots within a time range
    pub async fn get_snapshots_in_range(
        &self, 
        start: DateTime<Utc>, 
        end: DateTime<Utc>
    ) -> EvolutionResult<Vec<AgentSnapshot>>;
    
    /// Calculate goal alignment score
    pub async fn calculate_goal_alignment(&self, goal: &str) -> EvolutionResult<f64>;
}
```

### AgentSnapshot

```rust
pub struct AgentSnapshot {
    pub snapshot_id: String,
    pub agent_id: AgentId,
    pub timestamp: DateTime<Utc>,
    pub memory_state: MemoryState,
    pub tasks_state: TasksState,
    pub lessons_state: LessonsState,
    pub metadata: SnapshotMetadata,
}

pub struct SnapshotMetadata {
    pub description: String,
    pub created_by: String,
    pub tags: Vec<String>,
    pub quality_metrics: Option<QualityMetrics>,
}
```

## Memory Evolution

### VersionedMemory

Time-based memory state tracking with different memory types.

```rust
pub struct VersionedMemory {
    agent_id: AgentId,
    current_state: MemoryState,
    snapshots: Vec<MemorySnapshot>,
    created_at: DateTime<Utc>,
    last_updated: DateTime<Utc>,
}

impl VersionedMemory {
    /// Create new versioned memory for an agent
    pub fn new(agent_id: AgentId) -> Self;
    
    /// Add short-term memory entry
    pub fn add_short_term_memory(
        &mut self, 
        memory_id: MemoryId, 
        content: String, 
        context: String, 
        tags: Vec<String>
    ) -> EvolutionResult<()>;
    
    /// Promote short-term memory to long-term
    pub fn promote_to_long_term(
        &mut self, 
        memory_id: &MemoryId, 
        consolidation_reason: String
    ) -> EvolutionResult<()>;
    
    /// Add episodic memory entry
    pub fn add_episodic_memory(
        &mut self, 
        memory_id: MemoryId, 
        event_description: String, 
        event_sequence: Vec<String>, 
        outcome: String,
        tags: Vec<String>
    ) -> EvolutionResult<()>;
    
    /// Search memories by content or tags
    pub fn search_memories(
        &self, 
        query: &str, 
        memory_types: Option<Vec<MemoryType>>
    ) -> Vec<&MemoryEntry>;
    
    /// Get memory evolution timeline
    pub fn get_memory_timeline(&self) -> Vec<MemoryEvolution>;
    
    /// Create memory snapshot
    pub async fn create_snapshot(&mut self, description: String) -> EvolutionResult<()>;
}
```

### Memory Types

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryType {
    ShortTerm,
    LongTerm,
    Episodic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub memory_id: MemoryId,
    pub memory_type: MemoryType,
    pub content: String,
    pub context: String,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,
    pub access_count: usize,
    pub importance_score: f64,
    pub associated_tasks: Vec<TaskId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryState {
    pub short_term_memories: HashMap<MemoryId, MemoryEntry>,
    pub long_term_memories: HashMap<MemoryId, MemoryEntry>,
    pub episodic_memories: HashMap<MemoryId, MemoryEntry>,
    pub metadata: MemoryMetadata,
}
```

## Task Evolution

### VersionedTaskList

Complete task lifecycle tracking from creation to completion.

```rust
pub struct VersionedTaskList {
    agent_id: AgentId,
    current_state: TasksState,
    snapshots: Vec<TaskSnapshot>,
    created_at: DateTime<Utc>,
    last_updated: DateTime<Utc>,
}

impl VersionedTaskList {
    /// Create new versioned task list for an agent
    pub fn new(agent_id: AgentId) -> Self;
    
    /// Add a new task
    pub fn add_task(
        &mut self,
        task_id: TaskId,
        description: String,
        priority: TaskPriority,
        estimated_duration: Option<Duration>
    ) -> EvolutionResult<()>;
    
    /// Start task execution
    pub fn start_task(&mut self, task_id: &TaskId) -> EvolutionResult<TaskExecution>;
    
    /// Complete a task
    pub fn complete_task(
        &mut self, 
        task_id: &TaskId, 
        result: String
    ) -> EvolutionResult<TaskCompletion>;
    
    /// Cancel a task
    pub fn cancel_task(
        &mut self, 
        task_id: &TaskId, 
        reason: String
    ) -> EvolutionResult<()>;
    
    /// Update task progress
    pub fn update_task_progress(
        &mut self, 
        task_id: &TaskId, 
        progress: f64, 
        notes: Option<String>
    ) -> EvolutionResult<()>;
    
    /// Add task dependency
    pub fn add_dependency(
        &mut self, 
        task_id: &TaskId, 
        depends_on: &TaskId
    ) -> EvolutionResult<()>;
    
    /// Get tasks ready for execution
    pub fn get_ready_tasks(&self) -> Vec<&Task>;
    
    /// Get task execution history
    pub fn get_task_history(&self, task_id: &TaskId) -> Option<&TaskHistory>;
    
    /// Create task snapshot
    pub async fn create_snapshot(&mut self, description: String) -> EvolutionResult<()>;
}
```

### Task Types

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Blocked,
    Completed,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub task_id: TaskId,
    pub description: String,
    pub priority: TaskPriority,
    pub status: TaskStatus,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub estimated_duration: Option<Duration>,
    pub actual_duration: Option<Duration>,
    pub dependencies: Vec<TaskId>,
    pub tags: Vec<String>,
    pub metadata: TaskMetadata,
}
```

## Lessons Evolution

### VersionedLessons

Learning system that tracks success patterns and failure analysis.

```rust
pub struct VersionedLessons {
    agent_id: AgentId,
    current_state: LessonsState,
    snapshots: Vec<LessonSnapshot>,
    created_at: DateTime<Utc>,
    last_updated: DateTime<Utc>,
}

impl VersionedLessons {
    /// Create new versioned lessons for an agent
    pub fn new(agent_id: AgentId) -> Self;
    
    /// Learn from a successful experience
    pub fn learn_from_success(
        &mut self,
        lesson_id: LessonId,
        description: String,
        context: String,
        success_factors: Vec<String>,
        confidence: f64
    ) -> EvolutionResult<()>;
    
    /// Learn from a failure
    pub fn learn_from_failure(
        &mut self,
        lesson_id: LessonId,
        description: String,
        context: String,
        failure_causes: Vec<String>,
        prevention_strategies: Vec<String>
    ) -> EvolutionResult<()>;
    
    /// Learn from general experience
    pub fn learn_from_experience(
        &mut self,
        lesson_type: String,
        content: String,
        domain: String,
        confidence: f64
    ) -> EvolutionResult<()>;
    
    /// Apply a lesson to current situation
    pub fn apply_lesson(
        &mut self,
        lesson_id: &LessonId,
        application_context: String
    ) -> EvolutionResult<LessonApplication>;
    
    /// Update lesson based on application results
    pub fn update_lesson_effectiveness(
        &mut self,
        lesson_id: &LessonId,
        effectiveness_score: f64,
        feedback: String
    ) -> EvolutionResult<()>;
    
    /// Search lessons by content or domain
    pub fn search_lessons(&self, query: &str, domain: Option<&str>) -> Vec<&Lesson>;
    
    /// Get most applicable lessons for current context
    pub fn get_applicable_lessons(&self, context: &str) -> Vec<&Lesson>;
    
    /// Create lesson snapshot
    pub async fn create_snapshot(&mut self, description: String) -> EvolutionResult<()>;
}
```

### Lesson Types

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lesson {
    pub lesson_id: LessonId,
    pub lesson_type: String,
    pub content: String,
    pub domain: String,
    pub confidence: f64,
    pub created_at: DateTime<Utc>,
    pub last_applied: Option<DateTime<Utc>>,
    pub applied_count: usize,
    pub effectiveness_score: f64,
    pub tags: Vec<String>,
    pub metadata: LessonMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LessonsState {
    pub technical_lessons: HashMap<LessonId, Lesson>,
    pub process_lessons: HashMap<LessonId, Lesson>,
    pub domain_lessons: HashMap<LessonId, Lesson>,
    pub failure_lessons: HashMap<LessonId, Lesson>,
    pub success_patterns: HashMap<LessonId, Lesson>,
    pub metadata: LessonsMetadata,
}
```

## Workflow Patterns

### WorkflowPattern Trait

Base trait for all workflow patterns.

```rust
#[async_trait]
pub trait WorkflowPattern: Send + Sync {
    /// Get the name of this pattern
    fn pattern_name(&self) -> &'static str;
    
    /// Execute the workflow pattern
    async fn execute(&self, input: WorkflowInput) -> EvolutionResult<WorkflowOutput>;
    
    /// Check if this pattern is suitable for the given task analysis
    fn is_suitable_for(&self, task_analysis: &TaskAnalysis) -> bool;
    
    /// Estimate execution time for this pattern with given input
    fn estimate_execution_time(&self, input: &WorkflowInput) -> Duration;
}
```

### Workflow Input/Output

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowInput {
    pub task_id: String,
    pub agent_id: AgentId,
    pub prompt: String,
    pub context: Option<String>,
    pub parameters: WorkflowParameters,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowOutput {
    pub task_id: String,
    pub agent_id: AgentId,
    pub result: String,
    pub metadata: WorkflowMetadata,
    pub execution_trace: Vec<ExecutionStep>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowMetadata {
    pub pattern_used: String,
    pub execution_time: Duration,
    pub steps_executed: usize,
    pub success: bool,
    pub quality_score: Option<f64>,
    pub resources_used: ResourceUsage,
}
```

### Task Analysis

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskAnalysis {
    pub complexity: TaskComplexity,
    pub domain: String,
    pub requires_decomposition: bool,
    pub suitable_for_parallel: bool,
    pub quality_critical: bool,
    pub estimated_steps: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskComplexity {
    Simple,
    Moderate,
    Complex,
    VeryComplex,
}
```

## Specific Workflow Patterns

### 1. Prompt Chaining

```rust
pub struct PromptChaining {
    llm_adapter: Arc<dyn LlmAdapter>,
    chain_config: ChainConfig,
}

impl PromptChaining {
    pub fn new(llm_adapter: Arc<dyn LlmAdapter>) -> Self;
    pub fn with_config(llm_adapter: Arc<dyn LlmAdapter>, config: ChainConfig) -> Self;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainConfig {
    pub max_steps: usize,
    pub preserve_context: bool,
    pub quality_check: bool,
    pub timeout_per_step: Duration,
    pub context_window: usize,
}
```

### 2. Routing

```rust
pub struct Routing {
    primary_adapter: Arc<dyn LlmAdapter>,
    route_config: RouteConfig,
    alternative_adapters: HashMap<String, Arc<dyn LlmAdapter>>,
}

impl Routing {
    pub fn new(primary_adapter: Arc<dyn LlmAdapter>) -> Self;
    pub fn with_config(primary_adapter: Arc<dyn LlmAdapter>, config: RouteConfig) -> Self;
    pub fn add_route(
        self, 
        name: &str, 
        adapter: Arc<dyn LlmAdapter>, 
        cost: f64, 
        performance: f64
    ) -> Self;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteConfig {
    pub cost_weight: f64,
    pub performance_weight: f64,
    pub quality_weight: f64,
    pub fallback_strategy: FallbackStrategy,
    pub max_retries: usize,
}
```

### 3. Parallelization

```rust
pub struct Parallelization {
    llm_adapter: Arc<dyn LlmAdapter>,
    parallel_config: ParallelConfig,
}

impl Parallelization {
    pub fn new(llm_adapter: Arc<dyn LlmAdapter>) -> Self;
    pub fn with_config(llm_adapter: Arc<dyn LlmAdapter>, config: ParallelConfig) -> Self;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelConfig {
    pub max_parallel_tasks: usize,
    pub task_timeout: Duration,
    pub aggregation_strategy: AggregationStrategy,
    pub failure_threshold: f64,
    pub retry_failed_tasks: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AggregationStrategy {
    Concatenation,
    BestResult,
    Synthesis,
    MajorityVote,
    StructuredCombination,
}
```

### 4. Orchestrator-Workers

```rust
pub struct OrchestratorWorkers {
    orchestrator_adapter: Arc<dyn LlmAdapter>,
    worker_adapters: HashMap<WorkerRole, Arc<dyn LlmAdapter>>,
    orchestration_config: OrchestrationConfig,
}

impl OrchestratorWorkers {
    pub fn new(orchestrator_adapter: Arc<dyn LlmAdapter>) -> Self;
    pub fn with_config(
        orchestrator_adapter: Arc<dyn LlmAdapter>, 
        config: OrchestrationConfig
    ) -> Self;
    pub fn add_worker(self, role: WorkerRole, adapter: Arc<dyn LlmAdapter>) -> Self;
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkerRole {
    Analyst,
    Researcher,
    Writer,
    Reviewer,
    Validator,
    Synthesizer,
}
```

### 5. Evaluator-Optimizer

```rust
pub struct EvaluatorOptimizer {
    generator_adapter: Arc<dyn LlmAdapter>,
    evaluator_adapter: Arc<dyn LlmAdapter>,
    optimizer_adapter: Arc<dyn LlmAdapter>,
    optimization_config: OptimizationConfig,
}

impl EvaluatorOptimizer {
    pub fn new(llm_adapter: Arc<dyn LlmAdapter>) -> Self;
    pub fn with_config(llm_adapter: Arc<dyn LlmAdapter>, config: OptimizationConfig) -> Self;
    pub fn with_specialized_adapters(
        generator: Arc<dyn LlmAdapter>,
        evaluator: Arc<dyn LlmAdapter>,
        optimizer: Arc<dyn LlmAdapter>,
    ) -> Self;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationConfig {
    pub max_iterations: usize,
    pub quality_threshold: f64,
    pub improvement_threshold: f64,
    pub evaluation_criteria: Vec<EvaluationCriterion>,
    pub optimization_strategy: OptimizationStrategy,
    pub early_stopping: bool,
}
```

## LLM Integration

### LlmAdapter Trait

Unified interface for LLM providers.

```rust
#[async_trait]
pub trait LlmAdapter: Send + Sync {
    /// Get the provider name
    fn provider_name(&self) -> &'static str;
    
    /// Create a completion
    async fn complete(&self, prompt: &str, options: CompletionOptions) -> EvolutionResult<String>;
    
    /// Create a chat completion with multiple messages
    async fn chat_complete(&self, messages: Vec<Value>, options: CompletionOptions) -> EvolutionResult<String>;
    
    /// List available models for this provider
    async fn list_models(&self) -> EvolutionResult<Vec<String>>;
}

#[derive(Clone, Debug)]
pub struct CompletionOptions {
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub model: Option<String>,
}
```

### LlmAdapterFactory

Factory for creating LLM adapters.

```rust
pub struct LlmAdapterFactory;

impl LlmAdapterFactory {
    /// Create a mock adapter for testing
    pub fn create_mock(provider: &str) -> Arc<dyn LlmAdapter>;
    
    /// Create an adapter from configuration
    pub fn from_config(
        provider: &str, 
        model: &str, 
        config: Option<Value>
    ) -> EvolutionResult<Arc<dyn LlmAdapter>>;
    
    /// Create an adapter with a specific role/persona
    pub fn create_specialized_agent(
        provider: &str,
        model: &str,
        preamble: &str,
    ) -> EvolutionResult<Arc<dyn LlmAdapter>>;
}
```

## Integration Management

### EvolutionWorkflowManager

Main integration point between workflows and evolution tracking.

```rust
pub struct EvolutionWorkflowManager {
    evolution_system: AgentEvolutionSystem,
    default_llm_adapter: Arc<dyn LlmAdapter>,
}

impl EvolutionWorkflowManager {
    /// Create a new evolution workflow manager
    pub fn new(agent_id: AgentId) -> Self;
    
    /// Create with custom LLM adapter
    pub fn with_adapter(agent_id: AgentId, adapter: Arc<dyn LlmAdapter>) -> Self;
    
    /// Execute a task using the most appropriate workflow pattern
    pub async fn execute_task(
        &mut self,
        task_id: String,
        prompt: String,
        context: Option<String>,
    ) -> EvolutionResult<String>;
    
    /// Execute a task with a specific workflow pattern
    pub async fn execute_with_pattern(
        &mut self,
        task_id: String,
        prompt: String,
        context: Option<String>,
        pattern_name: &str,
    ) -> EvolutionResult<String>;
    
    /// Get the agent evolution system for direct access
    pub fn evolution_system(&self) -> &AgentEvolutionSystem;
    
    /// Get mutable access to the evolution system
    pub fn evolution_system_mut(&mut self) -> &mut AgentEvolutionSystem;
    
    /// Save the current evolution state
    pub async fn save_evolution_state(&self) -> EvolutionResult<()>;
}
```

### WorkflowFactory

Factory for creating and selecting workflow patterns.

```rust
pub struct WorkflowFactory;

impl WorkflowFactory {
    /// Create a workflow pattern for a specific task analysis
    pub fn create_for_task(
        analysis: &TaskAnalysis, 
        adapter: Arc<dyn LlmAdapter>
    ) -> Box<dyn WorkflowPattern>;
    
    /// Create a workflow pattern by name
    pub fn create_by_name(
        pattern_name: &str, 
        adapter: Arc<dyn LlmAdapter>
    ) -> EvolutionResult<Box<dyn WorkflowPattern>>;
    
    /// Get available pattern names
    pub fn available_patterns() -> Vec<&'static str>;
    
    /// Analyze task and recommend best pattern
    pub fn recommend_pattern(analysis: &TaskAnalysis) -> &'static str;
}
```

## Evolution Viewing

### MemoryEvolutionViewer

Visualization and analysis of agent memory evolution.

```rust
pub struct MemoryEvolutionViewer {
    agent_id: AgentId,
}

impl MemoryEvolutionViewer {
    pub fn new(agent_id: AgentId) -> Self;
    
    /// Get evolution timeline for memory
    pub async fn get_evolution_timeline(
        &self, 
        start: DateTime<Utc>, 
        end: DateTime<Utc>
    ) -> EvolutionResult<Vec<MemoryEvolution>>;
    
    /// Compare memory states between two points in time
    pub async fn compare_memory_states(
        &self,
        earlier: DateTime<Utc>,
        later: DateTime<Utc>,
    ) -> EvolutionResult<MemoryComparison>;
    
    /// Get memory insights and trends
    pub async fn get_memory_insights(
        &self,
        time_range: (DateTime<Utc>, DateTime<Utc>),
    ) -> EvolutionResult<Vec<MemoryInsight>>;
}
```

## Performance and Quality Metrics

### QualityMetrics

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMetrics {
    pub overall_score: f64,
    pub accuracy_score: f64,
    pub completeness_score: f64,
    pub clarity_score: f64,
    pub relevance_score: f64,
    pub coherence_score: f64,
    pub efficiency_score: f64,
}
```

### ResourceUsage

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResourceUsage {
    pub llm_calls: usize,
    pub tokens_consumed: usize,
    pub parallel_tasks: usize,
    pub memory_peak_mb: f64,
}
```

### PerformanceMetrics

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub execution_time: Duration,
    pub success_rate: f64,
    pub error_rate: f64,
    pub average_quality_score: f64,
    pub resource_efficiency: f64,
    pub cost_per_execution: f64,
}
```

## Usage Examples

### Basic Agent Evolution Setup

```rust
use terraphim_agent_evolution::*;

// Create evolution system for an agent
let mut evolution_system = AgentEvolutionSystem::new("agent_001".to_string());

// Add some initial memory
evolution_system.memory_evolution.add_short_term_memory(
    "mem_001".to_string(),
    "User preferences analysis".to_string(),
    "User prefers concise responses".to_string(),
    vec!["user_preference".to_string()],
)?;

// Create a task
evolution_system.tasks_evolution.add_task(
    "task_001".to_string(),
    "Analyze quarterly sales data".to_string(),
    TaskPriority::High,
    Some(Duration::from_secs(300)),
)?;

// Learn from success
evolution_system.lessons_evolution.learn_from_success(
    "lesson_001".to_string(),
    "Structured approach works well for data analysis".to_string(),
    "Quarterly sales analysis task".to_string(),
    vec!["step_by_step_analysis".to_string(), "clear_visualizations".to_string()],
    0.9,
)?;
```

### Workflow Execution with Evolution Tracking

```rust
// Create integrated workflow manager
let mut manager = EvolutionWorkflowManager::new("agent_001".to_string());

// Execute task with automatic pattern selection
let result = manager.execute_task(
    "analysis_task".to_string(),
    "Analyze the impact of AI on software development".to_string(),
    Some("Focus on productivity and code quality aspects".to_string()),
).await?;

println!("Result: {}", result);

// The system automatically:
// 1. Analyzed the task to select best pattern
// 2. Executed the chosen workflow pattern
// 3. Updated memory, tasks, and lessons
// 4. Created evolution snapshots
```

### Custom Workflow Pattern Usage

```rust
// Create specific workflow pattern
let adapter = LlmAdapterFactory::create_mock("gpt-4");
let chaining = PromptChaining::new(adapter);

let workflow_input = WorkflowInput {
    task_id: "custom_analysis".to_string(),
    agent_id: "agent_001".to_string(),
    prompt: "Perform comprehensive market analysis for electric vehicles".to_string(),
    context: None,
    parameters: WorkflowParameters::default(),
    timestamp: Utc::now(),
};

let output = chaining.execute(workflow_input).await?;

println!("Pattern used: {}", output.metadata.pattern_used);
println!("Quality score: {:?}", output.metadata.quality_score);
println!("Execution time: {:?}", output.metadata.execution_time);
```

This API reference provides comprehensive coverage of all public interfaces in the Terraphim AI Agent Evolution System, enabling developers to effectively integrate and extend the system for their specific use cases.