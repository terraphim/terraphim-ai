# API Reference Snippets

Generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)

## terraphim_orchestrator

### AgentOrchestrator
```rust
pub struct AgentOrchestrator {
    // Core orchestrator for multi-agent coordination
}

impl AgentOrchestrator {
    pub async fn new(config: OrchestratorConfig) -> Result<Self, OrchestratorError>;
    pub async fn run(&mut self) -> Result<(), OrchestratorError>;
    pub async fn handle_dispatch(&mut self, 
        task: DispatchTask, 
        ctx: SpawnContext
    ) -> Result<AgentRunRecord, OrchestratorError>;
}
```

### PrDispatchConfig
```rust
pub struct PrDispatchConfig {
    pub project: String,
    pub agent_name: String,
    pub on_event: WebhookEvent,
    pub path_filter: Option<PathFilter>,
}
```

## terraphim_agent (Robot Mode)

### RobotResponse
```rust
pub struct RobotResponse<T: Serialize> {
    pub success: bool,
    pub meta: ResponseMeta,
    pub data: Option<T>,
    pub errors: Vec<RobotError>,
}
```

### ResponseMeta
```rust
pub struct ResponseMeta {
    pub command: String,
    pub elapsed_ms: u64,
    pub timestamp: DateTime<Utc>,
    pub version: String,
    pub query: Option<String>,
    pub role: Option<String>,
}
```

## terraphim_types

### Document
```rust
pub struct Document {
    pub id: String,
    pub title: String,
    pub body: String,
    pub rank: Option<u32>,
    pub source: Option<String>,
    pub fields: HashMap<String, String>,
}
```

### SearchQuery
```rust
pub struct SearchQuery {
    pub query: String,
    pub role: Option<String>,
    pub limit: Option<usize>,
}
```

## terraphim_spawner

### SpawnContext
```rust
pub struct SpawnContext {
    pub agent_name: String,
    pub task: String,
    pub worktree: Option<PathBuf>,
    pub env: HashMap<String, String>,
}
```
