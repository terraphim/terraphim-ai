# Terraphim Goal Alignment System

Knowledge graph-based goal alignment system for multi-level goal management and conflict resolution in the Terraphim AI ecosystem.

## Overview

The `terraphim_goal_alignment` crate provides a sophisticated goal alignment system that leverages Terraphim's knowledge graph infrastructure to ensure goal hierarchy consistency, detect conflicts, and propagate goals through role hierarchies. It integrates seamlessly with the agent registry and role graph systems to provide context-aware goal management.

## Key Features

- **Multi-level Goal Management**: Global, high-level, and local goal alignment with hierarchy validation
- **Knowledge Graph Integration**: Uses existing `extract_paragraphs_from_automata` and `is_all_terms_connected_by_path` for intelligent goal analysis
- **Conflict Detection**: Semantic, resource, temporal, and priority conflict detection with resolution strategies
- **Goal Propagation**: Intelligent goal distribution through role hierarchies with automatic agent assignment
- **Dynamic Alignment**: Real-time goal alignment as system state changes with incremental updates
- **Performance Optimization**: Efficient caching, incremental updates, and background processing

## Architecture

```
┌─────────────────────┐    ┌──────────────────────┐    ┌─────────────────────┐
│   Goal Aligner      │    │  Knowledge Graph     │    │  Conflict Detector  │
│   (Core Engine)     │◄──►│  Analyzer            │◄──►│  & Resolver         │
└─────────────────────┘    └──────────────────────┘    └─────────────────────┘
         │                           │                           │
         ▼                           ▼                           ▼
┌─────────────────────┐    ┌──────────────────────┐    ┌─────────────────────┐
│   Goal Hierarchy    │    │  Goal Propagation    │    │  Agent Registry     │
│   Management        │    │  Engine              │    │  Integration        │
└─────────────────────┘    └──────────────────────┘    └─────────────────────┘
```

## Core Concepts

### Goal Hierarchy

Goals are organized in a three-level hierarchy:

```rust
use terraphim_goal_alignment::{Goal, GoalLevel};

// Global strategic objectives
let global_goal = Goal::new(
    "increase_revenue".to_string(),
    GoalLevel::Global,
    "Increase company revenue by 20% this year".to_string(),
    1, // High priority
);

// High-level departmental objectives
let high_level_goal = Goal::new(
    "improve_product_quality".to_string(),
    GoalLevel::HighLevel,
    "Reduce product defects by 50%".to_string(),
    2,
);

// Local task-level objectives
let local_goal = Goal::new(
    "implement_testing".to_string(),
    GoalLevel::Local,
    "Implement automated testing for core modules".to_string(),
    3,
);
```

### Knowledge Graph Context

Goals operate within rich knowledge graph contexts:

```rust
use terraphim_goal_alignment::GoalKnowledgeContext;

let mut context = GoalKnowledgeContext::default();
context.domains = vec!["software_engineering".to_string(), "quality_assurance".to_string()];
context.concepts = vec!["testing".to_string(), "automation".to_string(), "quality".to_string()];
context.relationships = vec!["implements".to_string(), "improves".to_string()];
context.keywords = vec!["unit_test".to_string(), "integration_test".to_string()];

goal.knowledge_context = context;
```

### Goal Constraints

Goals can have various types of constraints:

```rust
use terraphim_goal_alignment::{GoalConstraint, ConstraintType};

let temporal_constraint = GoalConstraint {
    constraint_type: ConstraintType::Temporal,
    description: "Must complete by end of quarter".to_string(),
    parameters: {
        let mut params = HashMap::new();
        params.insert("deadline".to_string(), serde_json::json!("2024-03-31"));
        params
    },
    is_hard: true,
    priority: 1,
};

let resource_constraint = GoalConstraint {
    constraint_type: ConstraintType::Resource,
    description: "Requires 2 senior developers".to_string(),
    parameters: {
        let mut params = HashMap::new();
        params.insert("resource_type".to_string(), serde_json::json!("senior_developer"));
        params.insert("amount".to_string(), serde_json::json!(2));
        params
    },
    is_hard: true,
    priority: 2,
};

goal.add_constraint(temporal_constraint)?;
goal.add_constraint(resource_constraint)?;
```

## Quick Start

### 1. Create Goal Aligner

```rust
use std::sync::Arc;
use terraphim_goal_alignment::{
    KnowledgeGraphGoalAligner, KnowledgeGraphGoalAnalyzer,
    AutomataConfig, SimilarityThresholds, AlignmentConfig,
};
use terraphim_rolegraph::RoleGraph;
use terraphim_agent_registry::AgentRegistry;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create knowledge graph analyzer
    let role_graph = Arc::new(RoleGraph::new());
    let kg_analyzer = Arc::new(KnowledgeGraphGoalAnalyzer::new(
        role_graph.clone(),
        AutomataConfig::default(),
        SimilarityThresholds::default(),
    ));

    // Create goal aligner
    let agent_registry = Arc::new(your_agent_registry);
    let config = AlignmentConfig::default();

    let aligner = KnowledgeGraphGoalAligner::new(
        kg_analyzer,
        agent_registry,
        role_graph,
        config,
    );

    Ok(())
}
```

### 2. Add Goals

```rust
use terraphim_goal_alignment::{Goal, GoalLevel};

// Create a strategic goal
let mut strategic_goal = Goal::new(
    "digital_transformation".to_string(),
    GoalLevel::Global,
    "Complete digital transformation initiative".to_string(),
    1,
);

// Set knowledge context
strategic_goal.knowledge_context.domains = vec![
    "digital_transformation".to_string(),
    "technology".to_string(),
    "business_process".to_string(),
];

strategic_goal.knowledge_context.concepts = vec![
    "automation".to_string(),
    "digitization".to_string(),
    "process_improvement".to_string(),
];

// Assign to roles
strategic_goal.assigned_roles = vec![
    "cto".to_string(),
    "digital_transformation_lead".to_string(),
];

// Add to aligner
aligner.add_goal(strategic_goal).await?;
```

### 3. Perform Goal Alignment

```rust
use terraphim_goal_alignment::{GoalAlignmentRequest, AlignmentType};

let request = GoalAlignmentRequest {
    goal_ids: Vec::new(), // All goals
    alignment_type: AlignmentType::FullAlignment,
    force_reanalysis: false,
    context: HashMap::new(),
};

let response = aligner.align_goals(request).await?;

println!("Alignment Score: {:.2}", response.summary.alignment_score_after);
println!("Conflicts Detected: {}", response.summary.conflicts_detected);
println!("Conflicts Resolved: {}", response.summary.conflicts_resolved);
println!("Goals Updated: {}", response.summary.goals_updated);

// Review recommendations
for recommendation in &response.summary.pending_recommendations {
    println!("Recommendation: {}", recommendation.description);
    println!("Priority: {}", recommendation.priority);
}
```

### 4. Propagate Goals

```rust
use terraphim_goal_alignment::{
    GoalPropagationEngine, GoalPropagationRequest, PropagationConfig,
};

let propagation_engine = GoalPropagationEngine::new(
    role_graph,
    agent_registry,
    PropagationConfig::default(),
);

let request = GoalPropagationRequest {
    source_goal: strategic_goal,
    target_roles: vec!["engineering_manager".to_string(), "product_manager".to_string()],
    max_depth: Some(3),
    context: HashMap::new(),
};

let result = propagation_engine.propagate_goal(request).await?;

println!("Goals Created: {}", result.summary.goals_created);
println!("Agents Assigned: {}", result.summary.agents_assigned);
println!("Roles Reached: {}", result.summary.roles_reached);
println!("Success Rate: {:.2}%", result.summary.success_rate * 100.0);
```

## Advanced Features

### Conflict Detection and Resolution

The system automatically detects various types of conflicts:

```rust
use terraphim_goal_alignment::{ConflictDetector, ConflictType};

let detector = ConflictDetector::new();

// Detect all conflicts
let conflicts = detector.detect_all_conflicts(&goals)?;

for conflict in &conflicts {
    println!("Conflict: {} vs {}", conflict.goal1, conflict.goal2);
    println!("Type: {:?}", conflict.conflict_type);
    println!("Severity: {:.2}", conflict.severity);
    println!("Description: {}", conflict.description);

    // Resolve conflict
    let resolution = detector.resolve_conflict(conflict, &mut goals)?;
    if resolution.success {
        println!("Resolution: {}", resolution.description);
    }
}
```

### Knowledge Graph Analysis

Deep integration with Terraphim's knowledge graph:

```rust
// Analyze goal connectivity
let analysis = GoalAlignmentAnalysis {
    goals: vec![goal1, goal2, goal3],
    analysis_type: AnalysisType::ConnectivityValidation,
    context: HashMap::new(),
};

let result = kg_analyzer.analyze_goal_alignment(analysis).await?;

// Check connectivity issues
for issue in &result.connectivity_issues {
    println!("Connectivity Issue: {}", issue.description);
    for fix in &issue.suggested_fixes {
        println!("  Suggested Fix: {}", fix);
    }
}
```

### Custom Propagation Strategies

Implement custom goal propagation logic:

```rust
use terraphim_goal_alignment::{PropagationStrategy, PropagationConfig};

let config = PropagationConfig {
    strategy: PropagationStrategy::SimilarityBased,
    min_role_similarity: 0.8,
    max_depth: 4,
    auto_assign_agents: true,
    max_agents_per_goal: 5,
};

let engine = GoalPropagationEngine::new(role_graph, agent_registry, config);
```

### Real-time Alignment Updates

Enable automatic alignment updates:

```rust
let config = AlignmentConfig {
    real_time_updates: true,
    auto_resolve_conflicts: false, // Manual review required
    max_alignment_iterations: 10,
    convergence_threshold: 0.95,
    ..AlignmentConfig::default()
};

let aligner = KnowledgeGraphGoalAligner::new(
    kg_analyzer,
    agent_registry,
    role_graph,
    config,
);

// Goals will be automatically re-aligned when updated
aligner.update_goal(modified_goal).await?;
```

## Integration with Terraphim Ecosystem

### With Agent Registry

Goals are automatically matched with suitable agents:

```rust
// Goals with assigned roles will automatically discover agents
strategic_goal.assigned_roles = vec!["senior_architect".to_string()];

// The system will find agents with the "senior_architect" role
// and assign them based on capability matching
aligner.add_goal(strategic_goal).await?;
```

### With Role Graph

Goal propagation follows role hierarchies:

```rust
// Goals propagate down the role hierarchy
// Global goals → High-level goals → Local goals
// Executive roles → Manager roles → Worker roles

let propagation_result = engine.propagate_goal(request).await?;

// Review propagation path
for step in &propagation_result.propagation_path {
    println!("Step {}: {} → {} ({})",
        step.step, step.from_role, step.to_role, step.reason);
}
```

### With Knowledge Graph

Semantic analysis guides all operations:

```rust
// Goals are analyzed for semantic consistency
// Concepts are extracted using extract_paragraphs_from_automata
// Connectivity is validated using is_all_terms_connected_by_path

let analysis_result = kg_analyzer.analyze_goal_alignment(analysis).await?;

println!("Overall Alignment Score: {:.2}", analysis_result.overall_alignment_score);

for (goal_id, analysis) in &analysis_result.goal_analyses {
    println!("Goal {}: Connectivity Score {:.2}",
        goal_id, analysis.connectivity.strength_score);
}
```

## Configuration

### Alignment Configuration

```rust
let config = AlignmentConfig {
    auto_resolve_conflicts: false,      // Manual conflict resolution
    max_alignment_iterations: 15,       // Maximum optimization iterations
    convergence_threshold: 0.98,        // High alignment threshold
    real_time_updates: true,            // Automatic re-alignment
    cache_ttl_secs: 3600,              // 1 hour cache TTL
    enable_monitoring: true,            // Performance monitoring
};
```

### Knowledge Graph Configuration

```rust
let automata_config = AutomataConfig {
    min_confidence: 0.8,               // High confidence threshold
    max_paragraphs: 20,                // More context extraction
    context_window: 2048,              // Larger context window
    language_models: vec!["advanced".to_string()],
};

let similarity_thresholds = SimilarityThresholds {
    concept_similarity: 0.85,          // High concept similarity
    domain_similarity: 0.8,            // High domain similarity
    relationship_similarity: 0.75,     // Moderate relationship similarity
    conflict_threshold: 0.6,           // Moderate conflict threshold
};
```

### Propagation Configuration

```rust
let propagation_config = PropagationConfig {
    max_depth: 6,                      // Deep propagation
    min_role_similarity: 0.75,         // Moderate role similarity
    auto_assign_agents: true,          // Automatic agent assignment
    max_agents_per_goal: 8,            // More agents per goal
    strategy: PropagationStrategy::HierarchicalCascade,
};
```

## Performance

The goal alignment system is optimized for performance:

- **Caching**: Analysis results cached with configurable TTL
- **Incremental Updates**: Only re-analyze affected goals
- **Background Processing**: Automatic cleanup and optimization
- **Efficient Algorithms**: Optimized conflict detection and resolution

### Benchmarks

Run benchmarks to see performance characteristics:

```bash
cargo bench --features benchmarks -p terraphim_goal_alignment
```

## Testing

Run the comprehensive test suite:

```bash
# Unit tests
cargo test -p terraphim_goal_alignment

# Integration tests
cargo test --test integration_tests -p terraphim_goal_alignment

# All tests with logging
RUST_LOG=debug cargo test -p terraphim_goal_alignment
```

## Examples

The crate includes comprehensive examples:

- **Basic Goal Management**: Creating, updating, and organizing goals
- **Conflict Detection**: Identifying and resolving goal conflicts
- **Goal Propagation**: Distributing goals through role hierarchies
- **Knowledge Graph Integration**: Semantic analysis and connectivity validation
- **Real-time Alignment**: Dynamic goal alignment with automatic updates

## Contributing

Contributions are welcome! Please see the main Terraphim repository for contribution guidelines.

## License

This project is licensed under the Apache License 2.0 - see the LICENSE file for details.
