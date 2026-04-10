# terraphim_service - Main Service Layer

## Overview

`terraphim_service` is the primary service layer for the Terraphim AI system. It orchestrates search operations, manages LLM interactions, and provides the main API functionality. This crate coordinates between configuration, middleware, persistence, and external services.

## Domain Model

### Core Concepts

#### TerraphimService
Main service coordinator that manages configuration and orchestrates operations.

```rust
pub struct TerraphimService {
    config_state: ConfigState,
}
```

**Key Responsibilities:**
- Manage role and haystack configuration
- Coordinate search operations
- Handle LLM integration
- Provide service lifecycle management

#### ConfigState
Shared state containing all configuration and runtime data.

```rust
pub struct ConfigState {
    pub config: Arc<Mutex<TerraphimConfig>>,
    pub roles: AHashMap<RoleName, RoleGraphSync>,
    pub llm_client: Arc<Mutex<Option<LlmClient>>>,
}
```

**Key Responsibilities:**
- Store global configuration
- Cache role graphs
- Manage LLM client lifecycle
- Thread-safe access to shared state

### Search Operations

#### Query
Search query structure with scoring capabilities.

```rust
pub struct Query {
    pub search_term: NormalisedTermValue,
    pub search_terms: Option<Vec<NormalisedTermValue>>,
    pub operator: Option<LogicalOperator>,
    pub skip: Option<u64>,
    pub limit: Option<u64>,
    pub role: Option<RoleName>,
    pub layer: Layer,
}
```

**Key Responsibilities:**
- Define search parameters
- Support complex queries
- Enable pagination
- Role-scoped searches

### LLM Integration

#### LlmClient
Generic LLM client supporting multiple providers.

```rust
pub struct LlmClient {
    pub provider: LlmProvider,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub model: String,
}
```

**Key Responsibilities:**
- Unified interface for multiple LLM providers
- Manage API authentication
- Handle provider-specific configuration
- Standardise request/response formats

#### LlmProvider
Supported LLM providers.

```rust
pub enum LlmProvider {
    OpenRouter,
    OpenAI,
    Anthropic,
    Ollama,
    Custom(String),
}
```

**Key Responsibilities:**
- Identify provider type
- Enable provider-specific logic
- Support custom providers

### Context Management

#### ContextItem
Fragment of context for LLM requests.

```rust
pub struct ContextItem {
    pub content: String,
    pub source: String,
    pub relevance: f64,
}
```

**Key Responsibilities:**
- Store context fragments
- Track source attribution
- Maintain relevance scores

#### ConversationManager
Manage chat conversations and context.

```rust
pub struct ConversationManager {
    pub conversations: AHashMap<String, Conversation>,
}
```

**Key Responsibilities:**
- Store conversation history
- Manage context windows
- Handle conversation lifecycle

### Summarisation

#### SummarisationRequest
Request for document summarisation.

```rust
pub struct SummarisationRequest {
    pub documents: Vec<Document>,
    pub role: RoleName,
    pub max_length: Option<usize>,
}
```

**Key Responsibilities:**
- Specify summarisation parameters
- Provide documents to summarise
- Set output constraints

#### SummarisationResult
Result of summarisation operation.

```rust
pub struct SummarisationResult {
    pub summary: String,
    pub documents_processed: usize,
    pub model_used: String,
}
```

**Key Responsibilities:**
- Store generated summary
- Track processing statistics
- Identify model used

## Data Models

### Service Types

#### RelevanceFunction
Algorithm for ranking search results.

```rust
pub enum RelevanceFunction {
    TitleScorer,
    BM25,
    BM25F,
    BM25Plus,
    TerraphimGraph,
}
```

**Use Cases:**
- `TitleScorer`: Simple title matching
- `BM25`: Okapi BM25 algorithm
- `BM25F`: Field-length normalised BM25
- `BM25Plus`: BM25 with additional features
- `TerraphimGraph`: Knowledge graph-based ranking

#### SearchQuery
Search request structure.

```rust
pub struct SearchQuery {
    pub search_term: NormalisedTermValue,
    pub search_terms: Option<Vec<NormalisedTermValue>>,
    pub operator: Option<LogicalOperator>,
    pub skip: Option<u64>,
    pub limit: Option<u64>,
    pub role: Option<RoleName>,
    pub layer: Layer,
    pub include_pinned: bool,
}
```

**Use Cases:**
- Single-term search
- Multi-term boolean search
- Role-scoped queries
- Layer-aware search
- Pagination

#### LogicalOperator
Boolean operators for combining search terms.

```rust
pub enum LogicalOperator {
    And,
    Or,
    Not,
}
```

**Use Cases:**
- Combining search criteria
- Excluding terms
- Building complex queries

### LLM Data Models

#### LlmMessage
Message in LLM conversation.

```rust
pub struct LlmMessage {
    pub role: String,
    pub content: String,
}
```

**Use Cases:**
- User messages
- Assistant messages
- System prompts

#### LlmResponse
Response from LLM provider.

```rust
pub struct LlmResponse {
    pub content: String,
    pub model: String,
    pub tokens_used: Option<u32>,
}
```

**Use Cases:**
- Store response content
- Track model used
- Monitor token usage

#### RoutingDecision
Decision on which LLM provider to use.

```rust
pub struct RoutingDecision {
    pub provider: LlmProvider,
    pub model: String,
    pub confidence: f64,
}
```

**Use Cases:**
- Provider selection
- Model choice
- Confidence tracking

### Thesaurus Management

#### ThesaurusBuildRequest
Request to build thesaurus from haystack.

```rust
pub struct ThesaurusBuildRequest {
    pub search_query: SearchQuery,
    pub role: RoleName,
}
```

**Use Cases:**
- Trigger thesaurus build
- Specify role context
- Define build parameters

#### ThesaurusBuildResult
Result of thesaurus build operation.

```rust
pub struct ThesaurusBuildResult {
    pub thesaurus: Thesaurus,
    pub documents_processed: usize,
    pub build_time_ms: u64,
}
```

**Use Cases:**
- Return built thesaurus
- Report statistics
- Performance metrics

## Implementation Patterns

### Service Lifecycle

#### Initialisation
```rust
impl TerraphimService {
    pub fn new(config_state: ConfigState) -> Self {
        Self { config_state }
    }
}
```

**Pattern:**
- Accept pre-initialised config state
- Store reference for lifecycle
- Minimal construction logic

#### Configuration Loading
```rust
impl TerraphimService {
    pub async fn ensure_thesaurus_loaded(
        &mut self,
        role_name: &RoleName
    ) -> Result<Thesaurus> {
        // Load from automata path or build from local KG
        // Save to persistence for future access
        // Return cached thesaurus
    }
}
```

**Pattern:**
- Multi-source loading with fallback
- Persistence caching
- Graceful degradation

### Search Operations

#### Query Execution
```rust
impl TerraphimService {
    pub async fn search(&self, query: &SearchQuery) -> Result<Vec<Document>> {
        // 1. Resolve role configuration
        // 2. Build/load thesaurus
        // 3. Execute search via middleware
        // 4. Apply relevance scoring
        // 5. Return ranked results
    }
}
```

**Pattern:**
- Async/await throughout
- Multi-step pipeline
- Error propagation
- Result aggregation

#### Thesaurus Building
```rust
impl TerraphimService {
    pub async fn build_thesaurus(
        &mut self,
        search_query: &SearchQuery
    ) -> Result<()> {
        Ok(build_thesaurus_from_haystack(
            &mut self.config_state,
            search_query
        ).await?)
    }
}
```

**Pattern:**
- Delegate to middleware
- Shared config state mutation
- Error wrapper for consistent API

### LLM Integration

#### Client Management
```rust
impl TerraphimService {
    pub async fn get_llm_client(&self) -> Result<LlmClient> {
        let config_guard = self.config_state.config.lock().await;
        let role = config_guard.get_role(&query.role)?;

        // Initialise LLM client if needed
        // Return cached or new client
    }
}
```

**Pattern:**
- Lazy initialisation
- Caching for performance
- Configuration-driven creation

#### Summarisation
```rust
impl TerraphimService {
    pub async fn summarise_documents(
        &self,
        request: SummarisationRequest
    ) -> Result<SummarisationResult> {
        let client = self.get_llm_client().await?;

        // Build context from documents
        // Call LLM for summarisation
        // Parse and return result
    }
}
```

**Pattern:**
- Client retrieval
- Context building
- Provider-agnostic operation
- Error handling

## Error Handling

### ServiceError
Comprehensive error type for service operations.

```rust
pub enum ServiceError {
    #[error("Middleware error: {0}")]
    Middleware(#[from] terraphim_middleware::Error),

    #[error("OpenDal error: {0}")]
    OpenDal(Box<opendal::Error>),

    #[error("Persistence error: {0}")]
    Persistence(#[from] terraphim_persistence::Error),

    #[error("Config error: {0}")]
    Config(String),

    #[cfg(feature = "openrouter")]
    #[error("OpenRouter error: {0}")]
    OpenRouter(#[from] crate::openrouter::OpenRouterError),

    #[error("Common error: {0}")]
    Common(#[from] crate::error::CommonError),
}
```

**Categories:**
- **Integration**: Middleware, OpenRouter errors
- **Storage**: OpenDal, Persistence errors
- **Configuration**: Config errors
- **Common**: Shared error types

### Error Handling Patterns

```rust
pub async fn example_operation(&self) -> Result<()> {
    // Try-fast with early error returns
    let config = self.config_state.config.lock().await;
    let role = config.get_role(&role_name)
        .ok_or_else(|| ServiceError::Config(...))?;

    // Error conversion with ?
    let result = risky_operation().await?;

    // Contextual error wrapping
    let result = another_operation()
        .await
        .map_err(|e| ServiceError::Middleware(...))?;

    Ok(())
}
```

## Concurrency Patterns

### Shared State

#### Mutex Protection
```rust
pub struct ConfigState {
    pub config: Arc<Mutex<TerraphimConfig>>,
    pub roles: AHashMap<RoleName, RoleGraphSync>,
    pub llm_client: Arc<Mutex<Option<LlmClient>>>,
}
```

**Pattern:**
- `Arc` for shared ownership
- `Mutex` for exclusive access
- Minimal lock duration
- Lock order consistency

#### Async Locking
```rust
pub async fn get_config(&self) -> Result<Role> {
    let config_guard = self.config_state.config.lock().await;
    config_guard.get_role(&role_name)
        .cloned()
        .ok_or_else(|| ServiceError::Config(...))
}
```

**Pattern:**
- `lock().await` for async mutex
- Minimise lock duration
- Clone before releasing
- Avoid deadlock with consistent ordering

## Performance Optimisations

### Caching

#### Role Graph Caching
```rust
impl ConfigState {
    pub async fn get_or_build_role_graph(
        &self,
        role_name: &RoleName
    ) -> Result<RoleGraphSync> {
        // Check cache first
        if let Some(cached) = self.roles.get(role_name) {
            return Ok(cached.clone());
        }

        // Build and cache
        let role_graph = self.build_role_graph(role_name).await?;
        self.roles.insert(role_name.clone(), role_graph.clone());
        Ok(role_graph)
    }
}
```

**Pattern:**
- Check cache before building
- Clone for thread safety
- Populate cache on miss
- Return cached value

#### LLM Client Caching
```rust
impl TerraphimService {
    pub async fn get_llm_client(&self) -> Result<LlmClient> {
        let client_guard = self.config_state.llm_client.lock().await;

        if let Some(client) = client_guard.as_ref() {
            return Ok(client.clone());
        }

        // Initialise and cache
        let client = LlmClient::new(...)?;
        *client_guard = Some(client.clone());
        Ok(client)
    }
}
```

**Pattern:**
- Check initialised state
- Initialise on first use
- Cache for subsequent calls
- Clone for thread safety

### Batch Operations

#### Bulk Document Processing
```rust
impl TerraphimService {
    pub async fn summarise_documents_batch(
        &self,
        documents: Vec<Document>
    ) -> Result<Vec<SummarisationResult>> {
        let results = tokio::try_join_all(
            documents.into_iter()
                .map(|doc| self.summarise_document(doc))
                .collect()
        ).await;

        results.into_iter().collect()
    }
}
```

**Pattern:**
- `try_join_all` for concurrent execution
- Preserve error information
- Collect all results
- Handle partial failures

## Testing Patterns

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_search_simple() {
        let service = create_test_service().await;
        let query = create_test_query();

        let results = service.search(&query).await.unwrap();
        assert!(!results.is_empty());
    }

    #[tokio::test]
    async fn test_thesaurus_building() {
        let mut service = create_test_service().await;
        let query = create_test_query();

        service.build_thesaurus(&query).await.unwrap();

        let role = service.get_role("test").await.unwrap();
        assert!(!role.thesaurus.is_empty());
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_end_to_end_search() {
    // 1. Setup test configuration
    // 2. Create service with real dependencies
    // 3. Execute search through full pipeline
    // 4. Verify results
    // 5. Cleanup
}
```

## Future Enhancements

### Planned Features

#### Streaming Responses
```rust
impl TerraphimService {
    pub async fn stream_llm_response(
        &self,
        messages: Vec<LlmMessage>
    ) -> Result<Pin<Box<dyn Stream<Item = String> + Send>>> {
        // Return streaming response
    }
}
```

#### Caching Layer
```rust
impl TerraphimService {
    pub async fn search_with_cache(
        &self,
        query: &SearchQuery,
        ttl: Duration
    ) -> Result<Vec<Document>> {
        // Check cache
        // Return cached or compute
    }
}
```

#### Rate Limiting
```rust
impl TerraphimService {
    pub async fn search_rate_limited(
        &self,
        query: &SearchQuery
    ) -> Result<Vec<Document>> {
        // Acquire rate limit
        // Execute search
        // Release rate limit
    }
}
```

## Best Practices

### Service Design

- Keep services thin and focused
- Delegate to specialised crates
- Use async throughout
- Handle errors gracefully

### Configuration

- Load configuration once
- Cache initialised objects
- Support runtime updates
- Validate at load time

### Error Handling

- Use `Result<T>` consistently
- Provide context in errors
- Categorise error types
- Support graceful degradation

### Concurrency

- Minimise lock duration
- Use `Arc` for sharing
- Avoid nested locks
- Respect async boundaries

## References

- [Tokio documentation](https://tokio.rs/)
- [OpenDAL for storage](https://opendal.org/)
- [Serde for serialisation](https://serde.rs/)
- [ThisError for error handling](https://docs.rs/thiserror/)
