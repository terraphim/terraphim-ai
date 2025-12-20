# Summary: Crates Overview

## Core Service Layer
- **terraphim_server**: Main HTTP API server binary (default workspace member)
- **terraphim_service**: Search, document management, AI integration
- **terraphim_middleware**: Haystack indexing, document processing, search orchestration
- **terraphim_config**: Configuration management, role-based settings
- **terraphim_persistence**: Document storage abstraction layer
- **terraphim_types**: Shared type definitions
- **terraphim_settings**: Device and server settings

## Knowledge Graph
- **terraphim_rolegraph**: Knowledge graph with node/edge relationships
- **terraphim_automata**: Text matching, autocomplete, thesaurus building (WASM-capable)
- **terraphim_kg_agents**: Knowledge graph-specific agent implementations
- **terraphim_kg_orchestration**: Knowledge graph workflow orchestration
- **terraphim_kg_linter**: Knowledge graph linting tools

## Agent System
- **terraphim_agent**: Main agent implementation
- **terraphim_agent_supervisor**: Agent lifecycle management
- **terraphim_agent_registry**: Agent discovery and registration
- **terraphim_agent_messaging**: Inter-agent communication
- **terraphim_agent_evolution**: Agent learning and adaptation
- **terraphim_multi_agent**: Multi-agent coordination
- **terraphim_goal_alignment**: Goal-driven agent orchestration
- **terraphim_task_decomposition**: Breaking complex tasks into subtasks

## Haystack Integrations
- **haystack_core**: Core haystack abstraction
- **haystack_atlassian**: Confluence and Jira
- **haystack_discourse**: Discourse forum
- **haystack_jmap**: Email via JMAP protocol
- **haystack_grepapp**: Grep.app search

## User Interfaces
- **terraphim_repl**: Interactive REPL (11 commands)
- **terraphim_cli**: Automation CLI (8 commands)
- **terraphim_mcp_server**: MCP server for AI tool integration
- **desktop/src-tauri**: Tauri desktop application

## Supporting
- **terraphim_atomic_client**: Atomic Data integration
- **terraphim_onepassword_cli**: 1Password CLI integration
- **terraphim-markdown-parser**: Markdown parsing utilities
- **terraphim_build_args**: Build-time argument handling
- **terraphim_update**: Self-update functionality
