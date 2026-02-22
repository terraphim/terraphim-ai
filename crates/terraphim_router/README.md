# Unified Routing for Terraphim

This directory contains the unified routing system for Terraphim AI, providing capability-based routing for both LLM providers and spawned agents.

## Crates

### terraphim_router
Capability-based routing engine that routes tasks to the best provider based on:
- Keyword extraction from prompts
- Provider capabilities
- Routing strategies (cost, latency, capability match)

### terraphim_spawner
Agent process spawner with:
- CLI validation
- API key checking
- Health monitoring (30s heartbeat)
- Output capture with @mention detection
- Auto-restart on failure

### terraphim_types (updated)
Added capability types:
- `Capability` enum (DeepThinking, CodeGeneration, etc.)
- `Provider` struct (unified LLM/Agent)
- `ProviderType` enum (Llm vs Agent)
- `CostLevel` and `Latency` enums

## Quick Start

```rust
use terraphim_router::{Router, RoutingContext};
use terraphim_types::capability::{
    Capability, Provider, ProviderType, CostLevel, Latency
};
use std::path::PathBuf;

// Create router
let mut router = Router::new();

// Register LLM provider
router.add_provider(Provider::new(
    "claude-opus",
    "Claude Opus",
    ProviderType::Llm {
        model_id: "claude-3-opus-20240229".to_string(),
        api_endpoint: "https://api.anthropic.com/v1".to_string(),
    },
    vec![Capability::DeepThinking, Capability::CodeGeneration],
));

// Register agent provider
router.add_provider(Provider::new(
    "@codex",
    "Codex Agent",
    ProviderType::Agent {
        agent_id: "@codex".to_string(),
        cli_command: "opencode".to_string(),
        working_dir: PathBuf::from("/workspace"),
    },
    vec![Capability::CodeGeneration],
));

// Route a task
let decision = router.route(
    "Implement a function to parse JSON",
    &RoutingContext::default(),
)?;

println!("Routed to: {}", decision.provider.name);
```

## Provider Configuration

Providers can be configured via markdown files with YAML frontmatter:

```markdown
---
id: "claude-opus"
name: "Claude Opus"
type: "llm"
model_id: "claude-3-opus-20240229"
api_endpoint: "https://api.anthropic.com/v1"
capabilities:
  - deep-thinking
  - code-generation
cost: expensive
latency: slow
keywords:
  - think
  - reason
---

# Claude Opus

Anthropic's most capable model.
```

## Routing Strategies

- **CostOptimized**: Select cheapest provider
- **LatencyOptimized**: Select fastest provider
- **CapabilityFirst**: Select provider with most capabilities
- **RoundRobin**: Distribute load evenly

## Architecture

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│   User Prompt   │────▶│  KeywordRouter   │────▶│  Capabilities   │
└─────────────────┘     └──────────────────┘     └─────────────────┘
                                                          │
                                                          ▼
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│  LLM Provider   │◀────│  RoutingEngine   │◀────│  ProviderRegistry│
│  (API call)     │     │  (Strategy)      │     │  (Filtered)      │
└─────────────────┘     └──────────────────┘     └─────────────────┘
                              │
                              ▼
┌─────────────────┐     ┌──────────────────┐
│  Agent Process  │◀────│  AgentSpawner    │
│  (Spawned)      │     │  (Health + I/O)  │
└─────────────────┘     └──────────────────┘
```

## Testing

```bash
cargo test -p terraphim_router
cargo test -p terraphim_spawner
```

## Examples

See `terraphim_router/examples/unified_routing.rs` for a complete example.

## License

Apache-2.0
