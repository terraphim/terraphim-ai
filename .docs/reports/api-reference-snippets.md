# API Reference Snippets

Generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)

## terraphim_types

Core domain types for the Terraphim knowledge graph system.

```rust
use terraphim_types::{NormalizedTerm, Role, RankingMethod};

/// A normalized term with knowledge graph metadata
pub struct NormalizedTerm {
    pub term: String,
    pub role: Role,
    pub score: f64,
}

/// Ranking strategies for search results
pub enum RankingMethod {
    BM25,
    TfIdf,
    GraphDistance,
}
```

## terraphim_config

Configuration management for roles, profiles, and devices.

```rust
use terraphim_config::{Config, Role, Profile};

/// Load configuration from filesystem
pub async fn load_config() -> Result<Config>;

/// Save configuration back to disk
pub async fn save_config(config: &Config) -> Result<()>;
```

## terraphim_service

Business logic layer for search and knowledge graph operations.

```rust
use terraphim_service::{Service, SearchQuery, SearchResults};

/// Execute a ranked search across configured haystacks
pub async fn search(&self, query: SearchQuery) -> Result<SearchResults>;
```

## terraphim_server

HTTP server with Axum providing REST API and WebSocket support.

```rust
use terraphim_server::{start_server, ServerConfig};

/// Start the Terraphim server with given configuration
pub async fn start_server(config: ServerConfig) -> Result<()>;
```

## terraphim_agent

CLI agent with REPL, robot mode, and learning capture.

```rust
use terraphim_agent::{Agent, Command};

/// Process a command in robot mode
pub async fn run_robot(command: Command) -> Result<String>;
```

## terraphim_orchestrator

Autonomous Development Flow orchestrator for multi-agent coordination.

```rust
use terraphim_orchestrator::{Orchestrator, FlowConfig};

/// Spawn agents for a PR lifecycle workflow
pub async fn dispatch_pr_workflow(&self, pr_number: u64) -> Result<RunId>;
```
