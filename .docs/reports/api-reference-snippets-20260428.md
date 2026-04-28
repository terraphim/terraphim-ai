# API Reference Snippets

**Generated:** 2026-04-28
**Agent:** documentation-generator (Ferrox)

These snippets represent key public APIs requiring documentation. They are extracted from source and serve as templates for doc comment insertion.

---

## terraphim_agent

### robot::budget

```rust
/// Results from a budget-constrained operation.
pub struct BudgetedResults {
    pub total_tokens: usize,
    pub items: Vec<BudgetedItem>,
}

/// Errors that can occur during budget calculation.
pub enum BudgetError {
    Exceeded,
    InvalidInput,
}

/// Engine for computing token budgets across operations.
pub struct BudgetEngine {
    config: BudgetConfig,
}
```

### robot::docs (Self-Documentation API)

```rust
/// Runtime self-documentation capabilities for agent introspection.
pub struct SelfDocumentation {
    capabilities: Capabilities,
}

/// Describes a documented CLI command.
pub struct CommandDoc {
    pub name: String,
    pub description: String,
    pub arguments: Vec<ArgumentDoc>,
    pub flags: Vec<FlagDoc>,
    pub examples: Vec<ExampleDoc>,
}

/// Describes a positional argument.
pub struct ArgumentDoc {
    pub name: String,
    pub description: String,
    pub required: bool,
}

/// Describes a command-line flag.
pub struct FlagDoc {
    pub name: String,
    pub description: String,
    pub short: Option<char>,
    pub long: String,
}

/// Usage example for a command.
pub struct ExampleDoc {
    pub description: String,
    pub command: String,
}
```

### robot::output

```rust
/// Output format for robot responses.
pub enum OutputFormat {
    Json,
    Yaml,
    Table,
    Plain,
}

/// Field display mode for tabular output.
pub enum FieldMode {
    Full,
    Compact,
    Truncate(usize),
}

/// Configuration for robot formatter.
pub struct RobotConfig {
    pub format: OutputFormat,
    pub field_mode: FieldMode,
    pub color: bool,
}

/// Formats structured data for terminal output.
pub struct RobotFormatter {
    config: RobotConfig,
}
```

### robot::schema

```rust
/// Wrapper for robot API responses with metadata.
pub struct RobotResponse<T: Serialize> {
    pub data: T,
    pub meta: ResponseMeta,
}

/// Metadata attached to every robot response.
pub struct ResponseMeta {
    pub query: String,
    pub role: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub duration_ms: u64,
}

/// Auto-correction suggestion for typos.
pub struct AutoCorrection {
    pub original: String,
    pub suggestion: String,
    pub confidence: f64,
}

/// Pagination parameters for paginated responses.
pub struct Pagination {
    pub page: usize,
    pub per_page: usize,
    pub total: usize,
}

/// Token budget for a specific operation.
pub struct TokenBudget {
    pub allocated: usize,
    pub consumed: usize,
    pub remaining: usize,
}

/// Structured error response from robot API.
pub struct RobotError {
    pub code: String,
    pub message: String,
    pub details: Option<Value>,
}
```

### repl::commands

```rust
/// Available REPL commands for interactive agent control.
pub enum ReplCommand {
    Query { text: String },
    Learn { path: PathBuf },
    Search { term: String },
    Status,
    Quit,
}

/// Parsed REPL command with arguments.
pub struct ParsedCommand {
    pub command: ReplCommand,
    pub args: Vec<String>,
}
```

---

## terraphim_server

### api

```rust
/// Response payload for document creation.
pub struct CreateDocumentResponse {
    pub id: String,
    pub status: String,
}

/// Response payload for search queries.
pub struct SearchResponse {
    pub results: Vec<IndexedDocument>,
    pub total: usize,
    pub duration_ms: u64,
}

/// Response payload for configuration requests.
pub struct ConfigResponse {
    pub roles: Vec<RoleName>,
    pub selected_role: RoleName,
    pub settings: Settings,
}

/// Request to change the active role.
pub struct SelectedRoleRequest {
    pub role: RoleName,
}

/// DTO for knowledge graph nodes.
pub struct GraphNodeDto {
    pub id: String,
    pub label: String,
    pub node_type: String,
}

/// DTO for knowledge graph edges.
pub struct GraphEdgeDto {
    pub source: String,
    pub target: String,
    pub relation: String,
}

/// Response for role graph queries.
pub struct RoleGraphResponseDto {
    pub nodes: Vec<GraphNodeDto>,
    pub edges: Vec<GraphEdgeDto>,
}

/// Query parameters for role graph searches.
pub struct RoleGraphQuery {
    pub role: Option<RoleName>,
    pub query: String,
}

/// Query parameters for knowledge graph search.
pub struct KgSearchQuery {
    pub term: String,
    pub limit: Option<usize>,
    pub filters: Option<HashMap<String, String>>,
}

/// Request to summarise a document.
pub struct SummarizeDocumentRequest {
    pub document_id: String,
    pub max_length: Option<usize>,
}

/// Response containing document summary.
pub struct SummarizeDocumentResponse {
    pub document_id: String,
    pub summary: String,
    pub tokens_used: usize,
}

/// Request for asynchronous summarisation.
pub struct AsyncSummarizeRequest {
    pub document_id: String,
    pub callback_url: Option<String>,
}

/// Response for async summarisation request.
pub struct AsyncSummarizeResponse {
    pub task_id: String,
    pub status: String,
}

/// Response for task status queries.
pub struct TaskStatusResponse {
    pub task_id: String,
    pub status: String,
    pub progress: Option<f64>,
    pub result: Option<String>,
}

/// Request to cancel a running task.
pub struct CancelTaskRequest {
    pub task_id: String,
}

/// Response for task cancellation.
pub struct CancelTaskResponse {
    pub task_id: String,
    pub cancelled: bool,
}

/// Response containing queue statistics.
pub struct QueueStatsResponse {
    pub pending: usize,
    pub running: usize,
    pub completed: usize,
    pub failed: usize,
}

/// Request for batch summarisation.
pub struct BatchSummarizeRequest {
    pub items: Vec<BatchSummarizeItem>,
}

/// Individual item in a batch summarisation request.
pub struct BatchSummarizeItem {
    pub document_id: String,
    pub max_length: Option<usize>,
}

/// Response for batch summarisation.
pub struct BatchSummarizeResponse {
    pub results: Vec<SummarizeDocumentResponse>,
    pub errors: Vec<String>,
}

/// Query parameters for summarisation status.
pub struct SummarizationStatusQuery {
    pub task_id: String,
}

/// Response for summarisation status.
pub struct SummarizationStatusResponse {
    pub task_id: String,
    pub status: String,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Single chat message in a conversation.
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

/// Request payload for chat completions.
pub struct ChatRequest {
    pub messages: Vec<ChatMessage>,
    pub model: Option<String>,
    pub max_tokens: Option<usize>,
}

/// Response payload for chat completions.
pub struct ChatResponse {
    pub message: ChatMessage,
    pub tokens_used: usize,
    pub model: String,
}

/// Request for OpenRouter model listing.
pub struct OpenRouterModelsRequest {
    pub filter: Option<String>,
}

/// Response containing available OpenRouter models.
pub struct OpenRouterModelsResponse {
    pub models: Vec<ModelInfo>,
}

/// Response for thesaurus queries.
pub struct ThesaurusResponse {
    pub term: String,
    pub synonyms: Vec<String>,
    pub antonyms: Vec<String>,
}

/// Response for autocomplete queries.
pub struct AutocompleteResponse {
    pub suggestions: Vec<String>,
    pub query: String,
}
```

### lib

```rust
/// Shared application state for Axum handlers.
pub struct AppState {
    pub config: ConfigState,
    pub persistence: Arc<dyn Persistence>,
}

/// Starts the Axum HTTP server.
pub async fn axum_server(
    server_hostname: SocketAddr,
    mut config_state: ConfigState,
) -> Result<()>;

/// Builds a router configured for integration tests.
pub async fn build_router_for_tests() -> Router;
```

---

## terraphim_types

```rust
/// Name of a role in the system.
pub struct RoleName {
    pub name: String,
}

/// A normalised term value with associated metadata.
pub struct NormalizedTermValue(String);

/// A term that has been normalised for indexing.
pub struct NormalizedTerm {
    pub value: NormalizedTermValue,
    pub frequency: usize,
    pub sources: Vec<String>,
}

/// A concept node in the knowledge graph.
pub struct Concept {
    pub id: String,
    pub label: String,
    pub description: Option<String>,
}

/// Types of documents that can be indexed.
pub enum DocumentType {
    Markdown,
    PlainText,
    Json,
    Yaml,
}

/// Routing directive for request dispatch.
pub struct RouteDirective {
    pub path: String,
    pub method: String,
    pub handler: String,
}

/// Directives extracted from markdown frontmatter.
pub struct MarkdownDirectives {
    pub route: Option<RouteDirective>,
    pub tags: Vec<String>,
}

/// A document in the index.
pub struct Document {
    pub id: String,
    pub title: String,
    pub content: String,
    pub doc_type: DocumentType,
}

/// An edge in the knowledge graph.
pub struct Edge {
    pub source: String,
    pub target: String,
    pub relation: String,
    pub weight: f64,
}

/// A node in the knowledge graph.
pub struct Node {
    pub id: String,
    pub label: String,
    pub node_type: String,
    pub properties: HashMap<String, Value>,
}

/// A thesaurus entry for term expansion.
pub struct Thesaurus {
    pub term: String,
    pub synonyms: Vec<String>,
    pub domain: Option<String>,
}

/// An index over a collection of documents.
pub struct Index {
    pub name: String,
    pub documents: Vec<Document>,
    pub updated_at: DateTime<Utc>,
}

/// Quality score for indexed content.
pub struct QualityScore {
    pub score: f64,
    pub factors: Vec<String>,
}

/// A document that has been indexed with metadata.
pub struct IndexedDocument {
    pub document: Document,
    pub score: QualityScore,
    pub indexed_at: DateTime<Utc>,
}

/// Logical operators for compound queries.
pub enum LogicalOperator {
    And,
    Or,
    Not,
}
```

---

*This file is auto-generated. Do not edit manually. Regenerate via documentation-generator agent.*
