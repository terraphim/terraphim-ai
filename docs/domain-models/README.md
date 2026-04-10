# Terraphim AI Domain and Data Models

This directory contains comprehensive documentation of domain models and data structures for all crates in the Terraphim AI system.

## Overview

Terraphim AI is organised as a Cargo workspace with multiple specialised crates. Each crate implements specific functionality with well-defined domain concepts and data models.

## Crate Documentation

### Core System Crates

- [terraphim_types](./terraphim_types.md) - Core type definitions shared across the system
- [terraphim_service](./terraphim_service.md) - Main service layer for search and AI operations
- [terraphim_rolegraph](./terraphim_rolegraph.md) - Knowledge graph implementation
- [terraphim_automata](./terraphim_automata.md) - Text matching and autocomplete engine
- [terraphim_persistence](./terraphim_persistence.md) - Multi-backend storage abstraction
- [terraphim_config](./terraphim_config.md) - Configuration management
- [terraphim_middleware](./terraphim_middleware.md) - Haystack indexing and orchestration

### Agent System Crates

- [terraphim_agent_supervisor](./terraphim_agent_supervisor.md) - Agent lifecycle management
- [terraphim_agent_registry](./terraphim_agent_registry.md) - Agent discovery and registration
- [terraphim_agent_messaging](./terraphim_agent_messaging.md) - Inter-agent communication
- [terraphim_agent_evolution](./terraphim_agent_evolution.md) - Agent learning and adaptation
- [terraphim_goal_alignment](./terraphim_goal_alignment.md) - Goal-driven orchestration
- [terraphim_task_decomposition](./terraphim_task_decomposition.md) - Task breakdown
- [terraphim_multi_agent](./terraphim_multi_agent.md) - Multi-agent coordination
- [terraphim_kg_agents](./terraphim_kg_agents.md) - Knowledge graph agents
- [terraphim_kg_orchestration](./terraphim_kg_orchestration.md) - Knowledge graph workflows

### Haystack Integration Crates

- [haystack_core](./haystack_core.md) - Core haystack abstraction
- [haystack_atlassian](./haystack_atlassian.md) - Confluence and Jira integration
- [haystack_discourse](./haystack_discourse.md) - Discourse forum integration
- [haystack_jmap](./haystack_jmap.md) - Email integration via JMAP

### Supporting Crates

- [terraphim_settings](./terraphim_settings.md) - Device and server settings
- [terraphim_mcp_server](./terraphim_mcp_server.md) - MCP server for AI tools
- [terraphim_tui](./terraphim_tui.md) - Terminal UI implementation
- [terraphim_atomic_client](./terraphim_atomic_client.md) - Atomic Data integration
- [terraphim_onepassword_cli](./terraphim_onepassword_cli.md) - 1Password CLI integration
- [terraphim-markdown-parser](./terraphim-markdown-parser.md) - Markdown parsing utilities
- [terraphim_orchestrator](./terraphim_orchestrator.md) - Workflow orchestration

## Domain Model Concepts

### Core Concepts

1. **Role** - User profile with specific knowledge domains and preferences
2. **Haystack** - Data source containing searchable documents
3. **Document** - Indexed content with metadata and knowledge graph connections
4. **Thesaurus** - Normalised terms with concept mappings
5. **Knowledge Graph** - Graph of connected concepts and documents
6. **Node** - Concept entity in the knowledge graph
7. **Edge** - Relationship between nodes
8. **Agent** - Autonomous AI entity with specialised capabilities

### Search and Retrieval

1. **SearchQuery** - Query structure with terms and operators
2. **RelevanceFunction** - Algorithm for ranking results (BM25, TitleScorer, TerraphimGraph)
3. **IndexedDocument** - Document with search indexes and concept links
4. **TriggerIndex** - TF-IDF index for semantic fallback search

### LLM Integration

1. **Conversation** - Chat context with messages
2. **ChatMessage** - Individual message in conversation
3. **ContextItem** - Context fragment for LLM requests
4. **RoutingRule** - Rule-based LLM provider selection
5. **RoutingDecision** - Result of routing logic

## Data Model Patterns

### Immutable Data

- All domain types implement `Clone` and `Serialize`
- Use `Arc` for shared immutable state
- Builder patterns for complex construction

### Persistence

- `Persistable` trait for save/load operations
- Multi-backend storage with automatic fallback
- Cache warm-up for performance

### Error Handling

- `thiserror` for comprehensive error types
- `Result<T, E>` throughout
- Categorised errors for handling strategies

## Naming Conventions

- **Structs**: PascalCase (e.g., `Document`, `RoleGraph`)
- **Enums**: PascalCase (e.g., `ServiceType`, `RelevanceFunction`)
- **Fields**: snake_case (e.g., `search_term`, `llm_api_key`)
- **Modules**: snake_case (e.g., `terraphim_service`, `haystack_atlassian`)

## Future Documentation

Additional crate documentation will be added as the system evolves:
- terraphim_firecracker
- terraphim_github_runner
- terraphim_symphony
- terraphim_router
- terraphim_validation
- And other specialised crates

## Maintenance

When adding new crates or modifying domain models:
1. Update this README with the new crate
2. Create a dedicated crate documentation file
3. Document all public types and their relationships
4. Include examples of common usage patterns
5. Keep diagrams and examples up to date
