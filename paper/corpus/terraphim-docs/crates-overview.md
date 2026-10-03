# Crate Reference

Terraphim AI is a modular Rust workspace comprising 52 crates. Each crate has a single responsibility and can be used independently or composed into larger systems. All crates are available in the [terraphim-ai](https://github.com/terraphim/terraphim-ai) monorepo.

## Core Engine

The foundational crates that power Terraphim's deterministic knowledge graph search.

| Crate | Description |
|-------|-------------|
| **terraphim_automata** | Aho-Corasick automata for searching and processing knowledge graphs. The core matching engine. |
| **terraphim_rolegraph** | Role-based knowledge graph module. Maps search roles to domain-specific graph views. |
| **terraphim_types** | Core types crate shared across the entire workspace. |
| **terraphim_config** | Configuration loading and management for all Terraphim components. |
| **terraphim_settings** | Settings handling library for runtime preferences and defaults. |
| **terraphim_service** | Service layer handling user requests and responses for the Terraphim core. |
| **terraphim_middleware** | Middleware for searching haystacks (pluggable data source backends). |
| **terraphim-markdown-parser** | Markdown parser for extracting structured content from knowledge base files. |
| **terraphim_persistence** | Persistence layer with Persistable trait and DeviceStorage backends (memory, SQLite, redb). |
| **terraphim_build_args** | Build argument management for compile-time feature configuration. |
| **terraphim_test_utils** | Shared test utilities and fixtures for all Terraphim crates. |

## Binaries and CLIs

User-facing executables and command-line tools.

| Crate | Description |
|-------|-------------|
| **terraphim_agent** | Terraphim AI Agent CLI with interactive REPL, session search, learning capture, and ASCII graph visualisation. |
| **terraphim-cli** | CLI tool for semantic knowledge graph search with JSON output for automation and scripting. |
| **terraphim_server** | HTTP server handling the core logic of Terraphim AI. Provides REST API and knowledge graph backend. |
| **terraphim_update** | Shared auto-update functionality for all Terraphim AI binaries. |
| **terraphim_validation** | Release validation system ensuring binary and asset integrity before publishing. |

## Agent Orchestration (AI Dark Factory)

OTP-inspired agent management system for running autonomous AI coding agents.

| Crate | Description |
|-------|-------------|
| **terraphim_orchestrator** | AI Dark Factory orchestrator wiring spawner, router, and supervisor into a reconciliation loop. |
| **terraphim_spawner** | Agent spawner with health checking, output capture, and lifecycle management. |
| **terraphim_router** | Unified routing engine for LLM and agent providers (keyword routing, tier selection). |
| **terraphim_agent_supervisor** | OTP-inspired supervision trees for fault-tolerant AI agent management. |
| **terraphim_agent_application** | OTP-style application behaviour for the Terraphim agent system. |
| **terraphim_agent_messaging** | Erlang-style asynchronous message passing system for AI agents. |
| **terraphim_agent_registry** | Knowledge graph-based agent registry for intelligent agent discovery and capability matching. |
| **terraphim_agent_evolution** | Agent evolution and self-improvement tracking. |
| **terraphim_workspace** | Workspace management for agent execution including lifecycle, hooks, and isolation. |
| **terraphim_multi_agent** | Multi-agent system built on roles with rust-genai integration. |

## Knowledge Graph Intelligence

Advanced crates for KG-powered reasoning, task planning, and goal management.

| Crate | Description |
|-------|-------------|
| **terraphim_kg_orchestration** | Knowledge graph-based agent orchestration engine for coordinating multi-agent workflows. |
| **terraphim_kg_agents** | Specialised knowledge graph-based agent implementations. |
| **terraphim_kg_linter** | Linter for markdown-based Terraphim KG schemas (commands, types, permissions). |
| **terraphim_goal_alignment** | Knowledge graph-based goal alignment system for multi-level goal management and conflict resolution. |
| **terraphim_task_decomposition** | Knowledge graph-based task decomposition for intelligent task analysis and execution planning. |
| **terraphim_rlm** | Recursive Language Model (RLM) orchestration for structured reasoning chains. |
| **terraphim_hooks** | Unified hooks infrastructure for knowledge graph-based text replacement and validation. |
| **terraphim_file_search** | Knowledge-graph scored file search integration. |

## Haystack Integrations

Pluggable data source connectors for searching external systems.

| Crate | Description |
|-------|-------------|
| **haystack_core** | Core traits and types for all Terraphim haystack integrations. |
| **haystack_atlassian** | Atlassian (Confluence, Jira) integration for searching enterprise knowledge bases. |
| **haystack_discourse** | Discourse forum integration for fetching posts and messages. |
| **haystack_grepapp** | Grep.app integration for searching code across GitHub repositories. |
| **haystack_jmap** | JMAP email protocol integration for searching email (Fastmail, etc.). |

## Session and Usage Analytics

Tools for analysing AI coding assistant sessions and tracking usage.

| Crate | Description |
|-------|-------------|
| **terraphim_sessions** | Session management for AI coding assistant history. Search across Claude Code, Cursor, and Aider sessions. |
| **terraphim-session-analyzer** | Analyse AI coding assistant session logs to identify agent usage patterns. |
| **terraphim_ccusage** | Claude Code usage tracking and cost analysis. |
| **terraphim_usage** | General usage telemetry and analytics. |

## DevOps and Infrastructure

Deployment, CI/CD, and infrastructure management.

| Crate | Description |
|-------|-------------|
| **terraphim_symphony** | Symphony orchestration service. Reads issues from trackers and dispatches coding agent sessions. |
| **terraphim_tracker** | Issue tracker abstraction for Gitea and Linear with PageRank-based prioritisation. |
| **terraphim_github_runner** | GitHub Actions runner with Firecracker sandbox integration. |
| **terraphim_github_runner_server** | HTTP server for the GitHub Actions runner service. |
| **terraphim-firecracker** | Sub-2-second VM boot optimisation system for sandboxed agent execution. |
| **terraphim_mcp_server** | Model Context Protocol (MCP) server exposing Terraphim tools to AI assistants. |
| **terraphim_onepassword_cli** | 1Password CLI integration for secret management. |

## Chat and Assistants

Multi-channel AI assistant interfaces.

| Crate | Description |
|-------|-------------|
| **terraphim_tinyclaw** | Multi-channel AI assistant for Telegram, Discord, and CLI. |

## Language Bindings

Cross-language bindings for using Terraphim from Python, Node.js, and WebAssembly.

| Crate | Description |
|-------|-------------|
| **terraphim_automata_py** | Python (PyO3) bindings for terraphim_automata. Fast autocomplete and text processing for knowledge graphs. |
| **terraphim_rolegraph_py** | Python bindings for terraphim_rolegraph. Knowledge graph operations for AI agents. |
| **terraphim-automata-node-rs** | Node.js (NAPI) bindings for Terraphim's Aho-Corasick matcher. |
| **terraphim-automata-wasm** | WebAssembly bindings for terraphim_automata. Runs in the browser. |

## Browser Extensions

| Crate | Description |
|-------|-------------|
| **terrraphim-automata-wasm** (extension) | WASM core for the Terraphim browser extensions (Sidebar and Autocomplete). |

---

## Quick Install

```bash
# Install the agent (interactive REPL + session search)
cargo install terraphim-agent

# Install the CLI (JSON output for automation)
cargo install terraphim-cli
```

Or use the universal installer:

```bash
curl -fsSL https://raw.githubusercontent.com/terraphim/terraphim-ai/main/scripts/install.sh | bash
```

## Architecture

The crate dependency graph follows a layered architecture:

1. **Types and Config** (bottom): `terraphim_types`, `terraphim_config`, `terraphim_settings`
2. **Core Engine**: `terraphim_automata`, `terraphim_rolegraph`, `terraphim_persistence`
3. **Service Layer**: `terraphim_service`, `terraphim_middleware`, haystack integrations
4. **Agent System**: `terraphim_spawner`, `terraphim_router`, `terraphim_agent_supervisor`
5. **Orchestration**: `terraphim_orchestrator`, `terraphim_kg_orchestration`, `terraphim_symphony`
6. **User Interfaces** (top): `terraphim_agent`, `terraphim-cli`, `terraphim_server`, `terraphim_tinyclaw`

## Contributing

Each crate has its own `README.md` with specific build instructions and examples. See the [Contribution Guide](./CONTRIBUTE.md) for the overall workflow.

Source: [github.com/terraphim/terraphim-ai](https://github.com/terraphim/terraphim-ai)
