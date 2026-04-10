# terraphim_middleware - Haystack Indexing and Orchestration

## Overview

`terraphim_middleware` provides indexing and search orchestration for various haystacks (data sources). It coordinates between different haystack types and the main service layer, providing a unified interface for document indexing and search operations.

## Domain Model

### Core Concepts

#### HaystackIndexer
Trait for indexing documents from different haystack types.

```rust
pub trait HaystackIndexer: Send + Sync {
    async fn index_documents(
        &mut self,
        haystack: &Haystack,
        search_query: &SearchQuery
    ) -> Result<Vec<Document>>;
}
```

**Key Responsibilities:**
- Define indexing interface
- Support multiple haystack types
- Return indexed documents
- Handle haystack-specific logic

### Implementations

#### RipgrepIndexer
Indexer for local filesystem search using ripgrep.

```rust
pub struct RipgrepIndexer {
    // Internal state
}

impl HaystackIndexer for RipgrepIndexer {
    async fn index_documents(
        &mut self,
        haystack: &Haystack,
        search_query: &SearchQuery
    ) -> Result<Vec<Document>> {
        // Use ripgrep to index and search
    }
}
```

**Key Responsibilities:**
- Execute ripgrep commands
- Parse ripgrep output
- Return formatted documents

#### QueryRsHaystackIndexer
Indexer for QueryRs (Reddit + Rust docs) search.

```rust
pub struct QueryRsHaystackIndexer {
    // Internal state
}

impl HaystackIndexer for QueryRsHaystackIndexer {
    async fn index_documents(
        &mut self,
        haystack: &Haystack,
        search_query: &SearchQuery
    ) -> Result<Vec<Document>> {
        // Query QueryRs API
        // Parse results
        // Return documents
    }
}
```

**Key Responsibilities:**
- Call QueryRs API endpoints
- Handle pagination
- Parse response formats

## Data Models

### Search Operations

#### SearchQuery
Unified search query structure.

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
- Pagination support

### Haystack Configuration

#### Haystack
Data source configuration.

```rust
pub struct Haystack {
    pub location: String,
    pub service: ServiceType,
    pub read_only: bool,
    pub fetch_content: bool,
    pub atomic_server_secret: Option<String>,
    pub extra_parameters: std::collections::HashMap<String, String>,
}
```

**Use Cases:**
- Define source location
- Specify service type
- Control access behaviour
- Pass service-specific parameters

#### ServiceType
Supported haystack services.

```rust
pub enum ServiceType {
    Ripgrep,
    Atomic,
    QueryRs,
    ClickUp,
    Mcp,
    Perplexity,
    GrepApp,
    AiAssistant,
    Quickwit,
    Jmap,
}
```

**Use Cases:**
- Service routing
- Indexer selection
- Configuration validation

## Implementation Patterns

### Haystack Search

#### Unified Search Interface
```rust
pub async fn search_haystacks(
    config_state: &mut ConfigState,
    search_query: &SearchQuery
) -> Result<Vec<Document>> {
    let config = config_state.config.lock().await;
    let role = config.get_role(&search_query.role)
        .ok_or_else(|| Error::RoleNotFound(...))?;

    let mut all_documents = Vec::new();

    for haystack in &role.haystacks {
        let documents = match haystack.service {
            ServiceType::Ripgrep => {
                let mut indexer = RipgrepIndexer::new();
                indexer.index_documents(haystack, search_query).await?
            }
            ServiceType::QueryRs => {
                let mut indexer = QueryRsHaystackIndexer::new();
                indexer.index_documents(haystack, search_query).await?
            }
            ServiceType::Atomic => {
                let mut indexer = AtomicHaystackIndexer::new();
                indexer.index_documents(haystack, search_query).await?
            }
            // ... other service types
            _ => continue,
        };

        all_documents.extend(documents);
    }

    Ok(all_documents)
}
```

**Pattern:**
- Lock config for read
- Get role configuration
- Iterate over haystacks
- Match on service type
- Aggregate results
- Handle errors gracefully

### Thesaurus Building

#### Thesaurus from Haystack
```rust
pub async fn build_thesaurus_from_haystack(
    config_state: &mut ConfigState,
    search_query: &SearchQuery
) -> Result<Thesaurus> {
    let config = config_state.config.lock().await;
    let role = config.get_role(&search_query.role)
        .ok_or_else(|| Error::RoleNotFound(...))?;

    let mut thesaurus = Thesaurus::new(role.name.as_str().to_string());

    for haystack in &role.haystacks {
        let documents = search_haystacks(config_state, search_query).await?;

        for document in documents {
            // Extract terms from document
            // Build concept mappings
            // Insert into thesaurus
        }
    }

    Ok(thesaurus)
}
```

**Pattern:**
- Search haystacks for documents
- Process documents to extract terms
- Build thesaurus structure
- Return populated thesaurus

## Error Handling

### Error Types

```rust
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Serde deserialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Role not found: {0}")]
    RoleNotFound(String),

    #[error("Indexing error: {0}")]
    Indexation(String),

    #[error("Config error: {0}")]
    Config(#[from] TerraphimConfigError),

    #[error("Persistence error: {0}")]
    Persistence(#[from] terraphim_persistence::Error),

    #[error("Builder error: {0}")]
    Builder(#[from] BuilderError),

    #[error("HTTP request error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Validation error: {0}")]
    Validation(String),
}
```

**Categories:**
- **Serialisation**: JSON parsing errors
- **I/O**: File system errors
- **Configuration**: Role/config errors
- **Integration**: Builder and HTTP errors
- **Validation**: Input validation errors

## Performance Optimisations

### Parallel Haystack Search

```rust
pub async fn search_haystacks_parallel(
    config_state: &mut ConfigState,
    search_query: &SearchQuery
) -> Result<Vec<Document>> {
    let config = config_state.config.lock().await;
    let role = config.get_role(&search_query.role)
        .ok_or_else(|| Error::RoleNotFound(...))?;

    let search_tasks: Vec<_> = role.haystacks
        .iter()
        .map(|haystack| {
            let haystack = haystack.clone();
            let config_state = config_state.clone();
            async move {
                // Search individual haystack
            }
        })
        .collect();

    let results = tokio::try_join_all(search_tasks).await;
    // Aggregate results
}
```

**Pattern:**
- Create async task per haystack
- Execute searches in parallel
- Wait for all to complete
- Aggregate results

### Caching

#### Document Cache
```rust
struct DocumentCache {
    cache: AHashMap<String, Vec<Document>>,
    ttl: Duration,
}

impl DocumentCache {
    pub async fn get_or_search(
        &mut self,
        key: &str,
        search_fn: impl Fn() -> Vec<Document>
    ) -> Vec<Document> {
        if let Some(cached) = self.cache.get(key) {
            return cached.clone();
        }

        let documents = search_fn();
        self.cache.insert(key.to_string(), documents.clone());
        documents
    }
}
```

**Pattern:**
- Check cache first
- Execute search on miss
- Update cache
- Apply TTL for invalidation

## Testing Patterns

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_type_serialisation() {
        let service = ServiceType::QueryRs;
        let json = serde_json::to_string(&service).unwrap();
        let deserialised: ServiceType = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialised, service);
    }

    #[test]
    fn test_haystack_serialisation() {
        let mut haystack = Haystack::new(
            "/path".to_string(),
            ServiceType::Ripgrep,
            false
        );
        haystack.extra_parameters.insert("key".to_string(), "value".to_string());

        let json = serde_json::to_string(&haystack).unwrap();
        let deserialised: Haystack = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialised.location, haystack.location);
        assert_eq!(deserialised.extra_parameters, haystack.extra_parameters);
    }
}
```

## Best Practices

### Indexer Design

- Implement `HaystackIndexer` trait
- Handle service-specific logic
- Return consistent `Document` format
- Support cancellation

### Error Handling

- Provide context in errors
- Categorise error types
- Support graceful degradation
- Log at appropriate levels

### Performance

- Use async throughout
- Support parallel searches
- Implement caching
- Minimise allocations

## Future Enhancements

### Planned Features

#### Streaming Indexing
```rust
pub trait StreamingHaystackIndexer: HaystackIndexer {
    async fn index_documents_stream(
        &mut self,
        haystack: &Haystack,
        search_query: &SearchQuery
    ) -> Result<Pin<Box<dyn Stream<Item = Document> + Send>>>;
}
```

#### Indexing Progress
```rust
pub struct IndexingProgress {
    pub total: usize,
    pub processed: usize,
    pub current_file: String,
}

pub trait ProgressReportingHaystackIndexer: HaystackIndexer {
    async fn index_documents_with_progress(
        &mut self,
        haystack: &Haystack,
        search_query: &SearchQuery,
        progress_callback: impl Fn(IndexingProgress)
    ) -> Result<Vec<Document>>;
}
```

#### Incremental Indexing
```rust
pub trait IncrementalHaystackIndexer: HaystackIndexer {
    async fn index_documents_incremental(
        &mut self,
        haystack: &Haystack,
        since: DateTime<Utc>
    ) -> Result<Vec<Document>>;
}
```

## References

- [Ripgrep documentation](https://github.com/BurntSushi/ripgrep)
- [reqwest for HTTP](https://docs.rs/reqwest/)
- [ThisError for error handling](https://docs.rs/thiserror/)
- [Tokio for async](https://tokio.rs/)
