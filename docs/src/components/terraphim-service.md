# Terraphim Service

The `terraphim_service` crate is the core service layer that provides document search, ranking, and management capabilities for the Terraphim system.

## Architecture

### Core Components

#### TerraphimService
The main service struct that orchestrates all document operations:

```rust
pub struct TerraphimService {
    config_state: ConfigState,
}
```

**Key Responsibilities**:
- Document search and ranking
- Role-based access control
- Knowledge graph integration
- Document persistence
- AI enhancement integration

#### Search Operations
The service provides multiple search methods:

```rust
impl TerraphimService {
    pub async fn search(&mut self, search_query: &SearchQuery) -> Result<Vec<Document>>
    pub async fn search_documents_selected_role(&mut self, search_term: &NormalizedTermValue) -> Result<Vec<Document>>
}
```

### Scoring System

#### QueryScorer Enum
Defines available scoring algorithms:

```rust
pub enum QueryScorer {
    OkapiBM25,    // Default Okapi BM25
    TFIDF,        // Traditional TF-IDF
    Jaccard,      // Jaccard similarity
    QueryRatio,   // Query term ratio
    BM25,         // BM25 implementation
    BM25F,        // Fielded BM25
    BM25Plus,     // Enhanced BM25
}
```

#### Scorer Implementation
The scoring system supports multiple algorithms:

```rust
pub struct Scorer {
    similarity: Similarity,
    scorer: Option<Box<dyn std::any::Any>>,
}
```

## API Reference

### Document Search

#### Basic Search
```rust
let mut service = TerraphimService::new(config_state);
let search_query = SearchQuery {
    search_term: NormalizedTermValue::from("rust programming"),
    skip: None,
    limit: Some(10),
    role: None,
};

let documents = service.search(&search_query).await?;
```

#### Role-Based Search
```rust
let documents = service.search_documents_selected_role(&NormalizedTermValue::from("search term")).await?;
```

### Document Management

#### Create Document
```rust
let document = Document {
    id: "doc1".to_string(),
    title: "Rust Programming".to_string(),
    body: "Rust is a systems programming language...".to_string(),
    description: Some("Introduction to Rust".to_string()),
    tags: Some(vec!["programming".to_string(), "rust".to_string()]),
    rank: None,
    stub: None,
    url: "https://example.com/doc1".to_string(),
};

let created_doc = service.create_document(document).await?;
```

#### Get Document by ID
```rust
let document = service.get_document_by_id("doc1").await?;
```

### Configuration Management

#### Fetch Configuration
```rust
let config = service.fetch_config().await;
```

#### Update Configuration
```rust
let updated_config = service.update_config(new_config).await?;
```

#### Update Selected Role
```rust
let config = service.update_selected_role(RoleName::from("Engineer")).await?;
```

## Scoring Algorithms

### BM25 Variants

#### Standard BM25
```rust
let query = Query::new("rust programming").name_scorer(Some(QueryScorer::BM25));
let documents = score::sort_documents(&query, documents);
```

#### Fielded BM25 (BM25F)
```rust
let weights = FieldWeights {
    title: 2.0,
    body: 1.0,
    description: 1.5,
    tags: 0.5,
};
```

#### Enhanced BM25 (BM25Plus)
```rust
let params = BM25Params {
    k1: 1.5,
    b: 0.8,
    delta: 1.2,
};
```

### Similarity-Based Scoring

#### Levenshtein Distance
```rust
let query = Query::new("search term").similarity(Similarity::Levenshtein);
```

#### Jaro Distance
```rust
let query = Query::new("search term").similarity(Similarity::Jaro);
```

#### Jaro-Winkler Distance
```rust
let query = Query::new("search term").similarity(Similarity::JaroWinkler);
```

## Knowledge Graph Integration

### Thesaurus Management
```rust
// Build thesaurus for search
service.build_thesaurus(&search_query).await?;

// Ensure thesaurus is loaded for role
let thesaurus = service.ensure_thesaurus_loaded(&role.name).await?;
```

### Document Preprocessing
```rust
// Apply KG preprocessing to documents
if role.terraphim_it {
    documents = service.preprocess_document_content(documents, &role).await?;
}
```

## AI Enhancement

### OpenRouter Integration
```rust
#[cfg(feature = "openrouter")]
if role.has_openrouter_config() {
    documents = service.enhance_descriptions_with_ai(documents, &role).await?;
}
```

### Document Summarization
```rust
let summary = service.generate_document_summary(
    &document,
    &api_key,
    &model,
    max_length
).await?;
```

## Error Handling

The service uses a custom error type:

```rust
pub enum ServiceError {
    Middleware(#[from] terraphim_middleware::Error),
    OpenDal(#[from] opendal::Error),
    Persistence(#[from] terraphim_persistence::Error),
    Config(String),
    #[cfg(feature = "openrouter")]
    OpenRouter(#[from] crate::openrouter::OpenRouterError),
}
```

## Performance Considerations

### Async Operations
All major operations are async to support non-blocking I/O:
- Document search and ranking
- Configuration management
- AI enhancement
- Knowledge graph operations

### Caching
The service implements caching for:
- Configuration state
- Thesaurus data
- Document persistence
- Role-based settings

### Memory Management
- Efficient document indexing
- Minimal memory allocations
- Shared type usage across crates

## Testing

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_search_documents() {
        // Test implementation
    }
}
```

### Integration Tests
```rust
#[test]
fn test_bm25_scorer_basic_functionality() {
    // BM25 test implementation
}
```

## Configuration

### Role-Based Configuration
```json
{
  "name": "Engineer",
  "relevance_function": "title-scorer",
  "query_scorer": "bm25f",
  "terraphim_it": true,
  "kg_path": "./docs/src/kg"
}
```

### Environment Variables
- `TERRAPHIM_CONFIG_PATH`: Configuration file path
- `TERRAPHIM_LOG_LEVEL`: Logging level
- `OPENROUTER_API_KEY`: AI enhancement API key

## Dependencies

### Internal Dependencies
- `terraphim_types`: Core type definitions
- `terraphim_config`: Configuration management
- `terraphim_persistence`: Document storage
- `terraphim_middleware`: Document processing

### External Dependencies
- `tokio`: Async runtime
- `serde`: Serialization
- `log`: Logging
- `reqwest`: HTTP client (for AI integration)

## Best Practices

### Error Handling
```rust
match service.search(&query).await {
    Ok(documents) => {
        // Process documents
    }
    Err(ServiceError::Config(msg)) => {
        // Handle configuration errors
    }
    Err(e) => {
        // Handle other errors
    }
}
```

### Async/Await Usage
```rust
// Always use .await for async operations
let documents = service.search(&query).await?;
let config = service.fetch_config().await;
```

### Resource Management
```rust
// Use RAII for resource management
let service = TerraphimService::new(config_state);
// Service is automatically cleaned up when dropped
```

## Migration Guide

### From Similarity-Based to BM25
1. Update role configuration to use BM25 scorers
2. Test with your document corpus
3. Adjust parameters as needed
4. Monitor performance and relevance

### Adding New Scoring Algorithms
1. Implement the algorithm in the appropriate module
2. Add to `QueryScorer` enum
3. Update scoring logic in `Scorer::score_documents`
4. Add tests for the new algorithm
5. Update documentation
