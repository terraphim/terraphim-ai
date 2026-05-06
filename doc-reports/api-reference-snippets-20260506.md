# API Reference Snippets — $(date +%Y-%m-%d)

## terraphim_orchestrator (459 warnings)

Representative missing docs:

```rust
/// Orchestrates autonomous development flow agents.
///
/// Manages agent lifecycle, dispatches tasks based on Gitea webhooks,
/// and coordinates multi-agent workflows.
pub struct Orchestrator {
    /// Gitea instance URL for webhook handling.
    pub gitea_url: String,
    /// Maximum concurrent agents allowed.
    pub max_concurrency: usize,
}

/// Dispatches a task to an available agent.
///
/// # Arguments
/// * `task` - The dispatch task configuration
///
/// # Returns
/// Agent run record tracking execution state.
pub async fn dispatch_task(&self, task: DispatchTask
) -> Result<AgentRunRecord, OrchestratorError>;
```

## terraphim_agent (99 warnings)

```rust
/// Terraphim AI agent runtime.
///
/// Provides CLI, TUI, and REPL interfaces for interacting
/// with the Terraphim knowledge graph and search systems.
pub struct Agent {
    /// Active role defining behaviour and knowledge graph.
    pub role: Role,
    /// Configuration for LLM provider routing.
    pub config: AgentConfig,
}

/// Captures a failed command as a structured learning.
///
/// Automatically redacts secrets before storage.
pub fn capture_failed_command(
    command: &str,
    error_output: &str,
    exit_code: i32,
) -> CapturedLearning;
```

## terraphim_types (128 warnings)

```rust
/// Core types shared across the Terraphim workspace.
///
/// Defines document, role, graph, and search primitives.
pub struct IndexedDocument {
    /// Unique document identifier.
    pub id: String,
    /// BM25 relevance score.
    pub score: f32,
    /// Knowledge graph quality metadata.
    pub quality_score: QualityScore,
}

/// Quality dimensions for knowledge graph scoring.
///
/// Based on Krogstie-Lindland-Sindre (KLS) framework.
pub struct QualityScore {
    /// Semantic correctness (0.0–1.0)
    pub semantic: f64,
    /// Pragmatic utility (0.0–1.0)
    pub pragmatic: f64,
    /// Syntactic validity (0.0–1.0)
    pub syntactic: f64,
}
```

## terraphim_service (115 warnings)

```rust
/// HTTP service layer for Terraphim AI.
///
/// Axum-based REST API exposing search, roles, and graph endpoints.
pub async fn search_handler(
    State(state): State<AppState>,
    Json(req): Json<SearchRequest>,
) -> Result<Json<SearchResponse>>;

/// Application state shared across handlers.
pub struct AppState {
    /// Role graph for query routing.
    pub rolegraph: Arc<RoleGraph>,
    /// Session store for multi-turn conversations.
    pub sessions: SessionStore,
}
```

## terraphim_server (138 warnings)

```rust
/// Terraphim server binary entrypoint.
///
/// Configures tracing, loads roles, and binds the Axum router.
pub async fn run_server(config: ServerConfig) -> Result<(), ServerError>;

/// Server configuration from TOML or environment.
pub struct ServerConfig {
    /// Listen address (default: 127.0.0.1:8000)
    pub bind_addr: SocketAddr,
    /// Path to roles configuration directory.
    pub roles_dir: PathBuf,
}
```
