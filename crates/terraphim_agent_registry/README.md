# Terraphim Agent Registry

Knowledge graph-based agent registry for intelligent agent discovery and capability matching in the Terraphim AI ecosystem.

## Overview

The `terraphim_agent_registry` crate provides a sophisticated agent registry that leverages Terraphim's knowledge graph infrastructure to enable intelligent agent discovery, capability matching, and role-based specialization. It integrates seamlessly with the existing automata and role graph systems to provide context-aware agent management.

## Key Features

- **Knowledge Graph Integration**: Uses existing `extract_paragraphs_from_automata` and `is_all_terms_connected_by_path` for intelligent agent discovery
- **Role-Based Specialization**: Leverages `terraphim_rolegraph` for agent role management and hierarchy
- **Capability Matching**: Semantic matching of agent capabilities to task requirements
- **Agent Metadata**: Rich metadata storage with knowledge graph context
- **Dynamic Discovery**: Real-time agent discovery based on evolving requirements
- **Performance Optimization**: Efficient indexing and caching for fast lookups
- **Multiple Discovery Algorithms**: Exact match, fuzzy match, semantic match, and hybrid approaches

## Architecture

```
┌─────────────────────┐    ┌──────────────────────┐    ┌─────────────────────┐
│   Agent Registry    │    │  Knowledge Graph     │    │  Role Graph         │
│   (Core)            │◄──►│  Integration         │◄──►│  Integration        │
└─────────────────────┘    └──────────────────────┘    └─────────────────────┘
         │                           │                           │
         ▼                           ▼                           ▼
┌─────────────────────┐    ┌──────────────────────┐    ┌─────────────────────┐
│   Agent Metadata    │    │  Discovery Engine    │    │  Capability         │
│   Management        │    │  (Multiple Algos)    │    │  Registry           │
└─────────────────────┘    └──────────────────────┘    └─────────────────────┘
```

## Core Concepts

### Agent Roles

Every agent in the registry has a **primary role** and can have multiple **secondary roles**. Roles are integrated with the `terraphim_rolegraph` system:

```rust
use terraphim_agent_registry::{AgentRole, AgentMetadata};

let role = AgentRole::new(
    "planner".to_string(),
    "Planning Agent".to_string(),
    "Responsible for task planning and coordination".to_string(),
);

// Roles support hierarchy and specialization
role.hierarchy_level = 2;
role.parent_roles = vec!["coordinator".to_string()];
role.child_roles = vec!["task_planner".to_string(), "resource_planner".to_string()];
role.knowledge_domains = vec!["project_management".to_string(), "scheduling".to_string()];
```

### Agent Capabilities

Agents have well-defined capabilities with performance metrics and resource requirements:

```rust
use terraphim_agent_registry::{AgentCapability, CapabilityMetrics, ResourceUsage};

let capability = AgentCapability {
    capability_id: "task_planning".to_string(),
    name: "Task Planning".to_string(),
    description: "Plan and organize complex tasks".to_string(),
    category: "planning".to_string(),
    required_domains: vec!["project_management".to_string()],
    input_types: vec!["requirements".to_string(), "constraints".to_string()],
    output_types: vec!["plan".to_string(), "timeline".to_string()],
    performance_metrics: CapabilityMetrics {
        avg_execution_time: Duration::from_secs(30),
        success_rate: 0.95,
        quality_score: 0.9,
        resource_usage: ResourceUsage {
            memory_mb: 256.0,
            cpu_percent: 15.0,
            network_kbps: 5.0,
            storage_mb: 100.0,
        },
        last_updated: Utc::now(),
    },
    dependencies: vec!["basic_reasoning".to_string()],
};
```

### Knowledge Graph Context

Agents operate within knowledge graph contexts that define their understanding:

```rust
use terraphim_agent_registry::KnowledgeContext;

let context = KnowledgeContext {
    domains: vec!["software_engineering".to_string(), "project_management".to_string()],
    concepts: vec!["agile".to_string(), "scrum".to_string(), "kanban".to_string()],
    relationships: vec!["implements".to_string(), "depends_on".to_string()],
    extraction_patterns: vec!["task_.*".to_string(), "requirement_.*".to_string()],
    similarity_thresholds: {
        let mut thresholds = HashMap::new();
        thresholds.insert("concept_similarity".to_string(), 0.8);
        thresholds.insert("domain_similarity".to_string(), 0.7);
        thresholds
    },
};
```

## Quick Start

### 1. Create a Registry

```rust
use std::sync::Arc;
use terraphim_agent_registry::{RegistryBuilder, RegistryConfig};
use terraphim_rolegraph::RoleGraph;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create role graph (integrate with existing Terraphim role graph)
    let role_graph = Arc::new(RoleGraph::new());

    // Configure registry
    let config = RegistryConfig {
        max_agents: 1000,
        auto_cleanup: true,
        cleanup_interval_secs: 300,
        enable_monitoring: true,
        discovery_cache_ttl_secs: 3600,
    };

    // Build registry
    let registry = RegistryBuilder::new()
        .with_role_graph(role_graph)
        .with_config(config)
        .build()?;

    // Start background tasks
    registry.start_background_tasks().await?;

    Ok(())
}
```

### 2. Register Agents

```rust
use terraphim_agent_registry::{AgentRegistry, AgentMetadata, AgentRole};

// Create agent metadata
let agent_id = AgentPid::new();
let supervisor_id = SupervisorId::new();

let primary_role = AgentRole::new(
    "data_analyst".to_string(),
    "Data Analysis Agent".to_string(),
    "Specializes in data analysis and visualization".to_string(),
);

let mut metadata = AgentMetadata::new(agent_id.clone(), supervisor_id, primary_role);

// Add capabilities
let analysis_capability = AgentCapability {
    capability_id: "data_analysis".to_string(),
    name: "Data Analysis".to_string(),
    description: "Analyze datasets and generate insights".to_string(),
    category: "analysis".to_string(),
    required_domains: vec!["statistics".to_string(), "data_science".to_string()],
    input_types: vec!["csv".to_string(), "json".to_string()],
    output_types: vec!["report".to_string(), "visualization".to_string()],
    performance_metrics: CapabilityMetrics::default(),
    dependencies: Vec::new(),
};

metadata.add_capability(analysis_capability)?;

// Register with registry
registry.register_agent(metadata).await?;
```

### 3. Discover Agents

```rust
use terraphim_agent_registry::AgentDiscoveryQuery;

// Create discovery query
let query = AgentDiscoveryQuery {
    required_roles: vec!["data_analyst".to_string()],
    required_capabilities: vec!["data_analysis".to_string()],
    required_domains: vec!["statistics".to_string()],
    task_description: Some("Analyze customer behavior data and generate insights".to_string()),
    min_success_rate: Some(0.8),
    max_resource_usage: Some(ResourceUsage {
        memory_mb: 1024.0,
        cpu_percent: 50.0,
        network_kbps: 100.0,
        storage_mb: 500.0,
    }),
    preferred_tags: vec!["experienced".to_string()],
};

// Discover matching agents
let result = registry.discover_agents(query).await?;

println!("Found {} matching agents", result.matches.len());
for agent_match in result.matches {
    println!(
        "Agent: {} (Score: {:.2}) - {}",
        agent_match.agent.agent_id,
        agent_match.match_score,
        agent_match.explanation
    );
}
```

## Discovery Algorithms

The registry supports multiple discovery algorithms:

### Exact Match
```rust
use terraphim_agent_registry::{DiscoveryEngine, DiscoveryContext, DiscoveryAlgorithm};

let context = DiscoveryContext {
    algorithm: DiscoveryAlgorithm::ExactMatch,
    ..Default::default()
};
```

### Fuzzy Match
```rust
let context = DiscoveryContext {
    algorithm: DiscoveryAlgorithm::FuzzyMatch,
    ..Default::default()
};
```

### Semantic Match (Knowledge Graph)
```rust
let context = DiscoveryContext {
    algorithm: DiscoveryAlgorithm::SemanticMatch,
    ..Default::default()
};
```

### Hybrid Approach
```rust
let context = DiscoveryContext {
    algorithm: DiscoveryAlgorithm::Hybrid(vec![
        DiscoveryAlgorithm::ExactMatch,
        DiscoveryAlgorithm::FuzzyMatch,
        DiscoveryAlgorithm::SemanticMatch,
    ]),
    ..Default::default()
};
```

## Knowledge Graph Integration

The registry integrates deeply with Terraphim's knowledge graph infrastructure:

### Concept Extraction
```rust
// Uses extract_paragraphs_from_automata for intelligent context analysis
let task_description = "Plan a software development project using agile methodology";
let extracted_concepts = kg_integration.extract_concepts_from_text(task_description).await?;
// Returns: ["software", "development", "project", "agile", "methodology"]
```

### Connectivity Analysis
```rust
// Uses is_all_terms_connected_by_path for requirement validation
let requirements = vec!["planning", "agile", "software_development"];
let connectivity = kg_integration.analyze_term_connectivity(&requirements).await?;

if connectivity.all_connected {
    println!("All requirements are connected in the knowledge graph");
} else {
    println!("Disconnected terms: {:?}", connectivity.disconnected);
}
```

### Role Hierarchy Navigation
```rust
// Leverages terraphim_rolegraph for role-based discovery
let related_roles = kg_integration.find_related_roles("senior_developer").await?;
// Returns parent roles, child roles, and sibling roles
```

## Advanced Features

### Capability Dependencies
```rust
let mut capability_registry = CapabilityRegistry::new();

// Register capabilities with dependencies
let advanced_planning = AgentCapability {
    capability_id: "advanced_planning".to_string(),
    dependencies: vec!["basic_planning".to_string(), "risk_assessment".to_string()],
    // ... other fields
};

capability_registry.register_capability(advanced_planning)?;

// Check if agent has all required dependencies
let agent_capabilities = vec!["basic_planning".to_string(), "risk_assessment".to_string()];
let can_use = capability_registry.check_dependencies("advanced_planning", &agent_capabilities);
```

### Performance Monitoring
```rust
// Agents track performance metrics automatically
agent_metadata.record_task_completion(Duration::from_secs(45), true);
agent_metadata.record_resource_usage(ResourceUsage {
    memory_mb: 512.0,
    cpu_percent: 25.0,
    network_kbps: 20.0,
    storage_mb: 200.0,
});

let success_rate = agent_metadata.get_success_rate(); // 0.0 to 1.0
```

### Dynamic Role Assignment
```rust
// Agents can assume multiple roles
let secondary_role = AgentRole::new(
    "code_reviewer".to_string(),
    "Code Review Specialist".to_string(),
    "Reviews code for quality and standards".to_string(),
);

agent_metadata.add_secondary_role(secondary_role)?;

// Check if agent can fulfill a role
if agent_metadata.has_role("code_reviewer") {
    println!("Agent can perform code reviews");
}
```

## Integration with Terraphim Ecosystem

### With Agent Supervisor
```rust
use terraphim_agent_supervisor::{Supervisor, AgentSpec};

// Registry integrates with supervision system
let supervisor = Supervisor::new(supervisor_id, RestartStrategy::OneForOne);

// Agents found through registry can be supervised
for agent_match in discovery_result.matches {
    let agent_spec = AgentSpec::new(
        agent_match.agent.agent_id,
        agent_match.agent.primary_role.role_id,
        serde_json::json!({}),
    );
    supervisor.start_agent(agent_spec).await?;
}
```

### With Messaging System
```rust
use terraphim_agent_messaging::{MessageSystem, AgentMessage};

// Discovered agents can communicate through messaging system
let message_system = MessageSystem::new();
for agent_match in discovery_result.matches {
    let message = AgentMessage::new(
        "task_assignment".to_string(),
        serde_json::json!({"task": "analyze_data"}),
    );
    message_system.send_message(agent_match.agent.agent_id, message).await?;
}
```

### With GenAgent Framework
```rust
use terraphim_gen_agent::{GenAgentFactory, GenAgentRuntime};

// Registry works with GenAgent runtime system
let factory = GenAgentFactory::new(state_manager, runtime_config);

for agent_match in discovery_result.matches {
    // Create runtime for discovered agents
    let runtime = factory.get_runtime(&agent_match.agent.agent_id).await;
    // ... interact with agent through runtime
}
```

## Configuration

### Registry Configuration
```rust
let config = RegistryConfig {
    max_agents: 10000,              // Maximum agents to register
    auto_cleanup: true,             // Automatically remove terminated agents
    cleanup_interval_secs: 300,     // Cleanup every 5 minutes
    enable_monitoring: true,        // Enable performance monitoring
    discovery_cache_ttl_secs: 3600, // Cache discovery results for 1 hour
};
```

### Knowledge Graph Configuration
```rust
let automata_config = AutomataConfig {
    min_confidence: 0.7,           // Minimum confidence for concept extraction
    max_paragraphs: 10,            // Maximum paragraphs to extract
    context_window: 512,           // Context window size
    language_models: vec!["default".to_string()],
};

let similarity_thresholds = SimilarityThresholds {
    role_similarity: 0.8,          // Role matching threshold
    capability_similarity: 0.75,   // Capability matching threshold
    domain_similarity: 0.7,        // Domain matching threshold
    concept_similarity: 0.65,      // Concept matching threshold
};
```

## Performance

The registry is optimized for high performance:

- **Efficient Indexing**: Agents indexed by role, capability, and domain
- **Caching**: Query results cached with configurable TTL
- **Background Processing**: Automatic cleanup and monitoring
- **Concurrent Access**: Thread-safe operations with minimal locking

### Benchmarks

Run benchmarks to see performance characteristics:

```bash
cargo bench --features benchmarks
```

## Testing

Run the comprehensive test suite:

```bash
# Unit tests
cargo test

# Integration tests
cargo test --test integration_tests

# All tests with logging
RUST_LOG=debug cargo test
```

## Examples

See the `tests/` directory for comprehensive examples:

- Basic agent registration and discovery
- Knowledge graph integration
- Role-based specialization
- Capability matching
- Performance monitoring
- Multi-algorithm discovery

## Contributing

Contributions are welcome! Please see the main Terraphim repository for contribution guidelines.

## License

This project is licensed under the Apache License 2.0 - see the LICENSE file for details.
