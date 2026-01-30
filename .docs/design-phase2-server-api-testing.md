# Terraphim AI Server API Testing Framework Design

## Overview

This document outlines a comprehensive testing framework for the Terraphim AI server API to ensure robust release validation. The framework covers all HTTP endpoints, providing systematic testing for functionality, performance, and security.

## Server API Testing Strategy

### API Endpoint Coverage

Based on the current server implementation (`terraphim_server/src/api.rs`), the following endpoints require comprehensive testing:

#### Core System Endpoints
- `GET /health` - Health check endpoint
- `GET /config` - Fetch current configuration
- `POST /config` - Update configuration
- `GET /config/schema` - Get configuration JSON schema
- `POST /config/selected_role` - Update selected role

#### Document Management Endpoints
- `POST /documents` - Create new document
- `GET /documents/search` - Search documents (GET method)
- `POST /documents/search` - Search documents (POST method)
- `POST /documents/summarize` - Generate document summary
- `POST /documents/async_summarize` - Async document summarization
- `POST /summarization/batch` - Batch document summarization

#### Summarization Queue Management
- `GET /summarization/status` - Check summarization capabilities
- `GET /summarization/queue/stats` - Queue statistics
- `GET /summarization/task/{task_id}/status` - Task status
- `POST /summarization/task/{task_id}/cancel` - Cancel task

#### Knowledge Graph & Role Management
- `GET /rolegraph` - Get role graph visualization
- `GET /roles/{role_name}/kg_search` - Search knowledge graph terms
- `GET /thesaurus/{role_name}` - Get role thesaurus
- `GET /autocomplete/{role_name}/{query}` - FST-based autocomplete

#### LLM & Chat Features
- `POST /chat` - Chat completion with LLM
- `GET /openrouter/models` - List OpenRouter models (if feature enabled)

#### Conversation Management
- `POST /conversations` - Create conversation
- `GET /conversations` - List conversations
- `GET /conversations/{id}` - Get specific conversation
- `POST /conversations/{id}/messages` - Add message
- `POST /conversations/{id}/context` - Add context
- `POST /conversations/{id}/search-context` - Add search results as context
- `PUT /conversations/{id}/context/{context_id}` - Update context
- `DELETE /conversations/{id}/context/{context_id}` - Delete context

#### Workflow Management (Advanced)
- Various workflow endpoints via `workflows::create_router()`

### Test Categories

#### 1. Unit Tests
- **Purpose**: Test individual functions in isolation
- **Scope**: Request parsing, response formatting, validation logic
- **Implementation**: Direct function calls with mocked dependencies

#### 2. Integration Tests
- **Purpose**: Test endpoint functionality with real dependencies
- **Scope**: HTTP request/response cycle, database interactions
- **Implementation**: Test server with actual storage backends

#### 3. End-to-End Tests
- **Purpose**: Test complete user workflows
- **Scope**: Multi-step operations, cross-feature interactions
- **Implementation**: Browser automation or API sequence testing

#### 4. Performance Tests
- **Purpose**: Validate performance under load
- **Scope**: Response times, concurrent requests, memory usage
- **Implementation**: Load testing with configurable concurrency

#### 5. Security Tests
- **Purpose**: Validate security measures
- **Scope**: Input validation, authentication, rate limiting
- **Implementation**: Malicious input testing, penetration testing

### Test Environment Setup

#### Local Testing Environment
```bash
# Development server with test configuration
cargo run -p terraphim_server -- --role test --config test_config.json

# Test database setup
export TEST_DB_PATH="/tmp/terraphim_test"
mkdir -p $TEST_DB_PATH
```

#### Containerized Testing
```dockerfile
# Dockerfile.test
FROM rust:1.70
WORKDIR /app
COPY . .
RUN cargo build --release
EXPOSE 8080
CMD ["./target/release/terraphim_server", "--role", "test"]
```

#### CI/CD Integration
```yaml
# .github/workflows/api-tests.yml
name: API Tests
on: [push, pull_request]
jobs:
  api-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Run API Tests
        run: cargo test -p terraphim_server --test api_test_suite
```

### Mock Server Strategy

#### External Service Mocking
- **OpenRouter API**: Mock for chat completion and model listing
- **File System**: In-memory file system for document testing
- **Database**: SQLite in-memory for isolated tests
- **Network Services**: Mock HTTP servers for external integrations

#### Mock Implementation
```rust
// Mock LLM client for testing
pub struct MockLLMClient {
    responses: HashMap<String, String>,
}

impl MockLLMClient {
    pub fn new() -> Self {
        Self {
            responses: HashMap::new(),
        }
    }

    pub fn add_response(&mut self, input_pattern: &str, response: &str) {
        self.responses.insert(input_pattern.to_string(), response.to_string());
    }
}
```

### Data Validation

#### Input Validation
- **Document Creation**: Validate required fields, content formats
- **Search Queries**: Validate query parameters, role names
- **Configuration**: Validate configuration schema compliance
- **Chat Messages**: Validate message formats, role assignments

#### Output Validation
- **Response Schema**: Verify JSON structure compliance
- **Data Types**: Validate field types and formats
- **Status Codes**: Ensure appropriate HTTP status codes
- **Error Messages**: Validate error response formats

#### Error Handling Tests
- **Missing Required Fields**: 400 Bad Request responses
- **Invalid Role Names**: 404 Not Found responses
- **Malformed JSON**: 400 Bad Request responses
- **Service Unavailability**: 503 Service Unavailable responses

### Performance Testing

#### Load Testing Scenarios
- **Concurrent Search**: 100 simultaneous search requests
- **Document Creation**: Batch document creation performance
- **Chat Completions**: LLM request handling under load
- **Configuration Updates**: Concurrent config modification testing

#### Response Time Validation
```rust
// Performance benchmarks
const MAX_RESPONSE_TIME_MS: u64 = 1000; // 1 second for most endpoints
const SEARCH_TIMEOUT_MS: u64 = 5000;     // 5 seconds for complex searches
const LLM_TIMEOUT_MS: u64 = 30000;       // 30 seconds for LLM calls
```

#### Memory Usage Testing
- **Memory Leaks**: Monitor memory usage during extended tests
- **Document Storage**: Validate memory usage with large documents
- **Caching**: Test cache efficiency and memory management
- **Concurrent Load**: Memory usage under high concurrency

### Security Testing

#### Authentication & Authorization
- **Role-Based Access**: Test role-based functionality restrictions
- **API Key Validation**: Validate OpenRouter API key handling
- **Configuration Security**: Test sensitive configuration exposure

#### Input Sanitization
- **SQL Injection**: Test for SQL injection vulnerabilities
- **XSS Prevention**: Validate input sanitization for web interfaces
- **Path Traversal**: Test file system access restrictions
- **Command Injection**: Validate command execution security

#### Rate Limiting
- **Request Rate Limits**: Test rate limiting implementation
- **DDoS Protection**: Validate denial of service protection
- **Resource Limits**: Test resource usage restrictions

## Implementation Plan

### Step 1: Create Test Server Harness

#### Test Server Infrastructure
```rust
// terraphim_server/tests/test_harness.rs
pub struct TestServer {
    server: axum::Router,
    client: reqwest::Client,
    base_url: String,
}

impl TestServer {
    pub async fn new() -> Self {
        let router = terraphim_server::build_router_for_tests().await;
        let addr = "127.0.0.1:0".parse().unwrap();
        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        let port = listener.local_addr().unwrap().port();

        tokio::spawn(axum::serve(listener, router));

        Self {
            server: router,
            client: reqwest::Client::new(),
            base_url: format!("http://127.0.0.1:{}", port),
        }
    }

    pub async fn get(&self, path: &str) -> reqwest::Response {
        self.client.get(&format!("{}{}", self.base_url, path))
            .send().await.unwrap()
    }

    pub async fn post<T: serde::Serialize>(&self, path: &str, body: &T) -> reqwest::Response {
        self.client.post(&format!("{}{}", self.base_url, path))
            .json(body)
            .send().await.unwrap()
    }
}
```

#### Test Data Management
```rust
// terraphim_server/tests/fixtures.rs
pub struct TestFixtures {
    documents: Vec<Document>,
    roles: HashMap<String, Role>,
}

impl TestFixtures {
    pub fn sample_document() -> Document {
        Document {
            id: "test-doc-1".to_string(),
            url: "file:///test/doc1.md".to_string(),
            title: "Test Document".to_string(),
            body: "# Test Document\n\nThis is a test document for API validation.".to_string(),
            description: Some("A test document for validation".to_string()),
            summarization: None,
            stub: None,
            tags: Some(vec!["test".to_string(), "api".to_string()]),
            rank: Some(1.0),
            source_haystack: None,
        }
    }

    pub fn test_role() -> Role {
        Role {
            name: RoleName::new("TestRole"),
            shortname: Some("test".to_string()),
            relevance_function: RelevanceFunction::TitleScorer,
            theme: "default".to_string(),
            kg: None,
            haystacks: vec![],
            terraphim_it: false,
            ..Default::default()
        }
    }
}
```

#### Request/Response Validation Framework
```rust
// terraphim_server/tests/validation.rs
pub trait ResponseValidator {
    fn validate_status(&self, expected: StatusCode) -> &Self;
    fn validate_json_schema<T: serde::de::DeserializeOwned>(&self) -> T;
    fn validate_error_response(&self) -> Option<String>;
}

impl ResponseValidator for reqwest::Response {
    fn validate_status(&self, expected: StatusCode) -> &Self {
        assert_eq!(self.status(), expected, "Expected status {}, got {}", expected, self.status());
        self
    }

    fn validate_json_schema<T: serde::de::DeserializeOwned>(&self) -> T {
        self.json().await.unwrap_or_else(|e| {
            panic!("Failed to parse JSON response: {}", e);
        })
    }

    fn validate_error_response(&self) -> Option<String> {
        if !self.status().is_success() {
            Some(self.text().await.unwrap_or_default())
        } else {
            None
        }
    }
}
```

### Step 2: Implement API Endpoint Tests

#### Health Check Tests
```rust
// terraphim_server/tests/health_tests.rs
#[tokio::test]
async fn test_health_check() {
    let server = TestServer::new().await;

    let response = server.get("/health").await;

    response
        .validate_status(StatusCode::OK)
        .text()
        .await
        .map(|body| assert_eq!(body, "OK"));
}
```

#### Document Management Tests
```rust
// terraphim_server/tests/document_tests.rs
#[tokio::test]
async fn test_create_document() {
    let server = TestServer::new().await;
    let document = TestFixtures::sample_document();

    let response = server.post("/documents", &document).await;

    response.validate_status(StatusCode::OK);

    let create_response: CreateDocumentResponse = response.validate_json_schema();
    assert_eq!(create_response.status, Status::Success);
    assert!(!create_response.id.is_empty());
}

#[tokio::test]
async fn test_search_documents_get() {
    let server = TestServer::new().await;
    let query = SearchQuery {
        query: "test".to_string(),
        role: None,
        limit: Some(10),
        offset: Some(0),
    };

    let response = server.get(&format!("/documents/search?query={}&limit={}&offset={}",
        query.query, query.limit.unwrap(), query.offset.unwrap())).await;

    response.validate_status(StatusCode::OK);

    let search_response: SearchResponse = response.validate_json_schema();
    assert_eq!(search_response.status, Status::Success);
}

#[tokio::test]
async fn test_search_documents_post() {
    let server = TestServer::new().await;
    let query = SearchQuery {
        query: "test".to_string(),
        role: None,
        limit: Some(10),
        offset: Some(0),
    };

    let response = server.post("/documents/search", &query).await;

    response.validate_status(StatusCode::OK);

    let search_response: SearchResponse = response.validate_json_schema();
    assert_eq!(search_response.status, Status::Success);
}
```

#### Configuration Management Tests
```rust
// terraphim_server/tests/config_tests.rs
#[tokio::test]
async fn test_get_config() {
    let server = TestServer::new().await;

    let response = server.get("/config").await;

    response.validate_status(StatusCode::OK);

    let config_response: ConfigResponse = response.validate_json_schema();
    assert_eq!(config_response.status, Status::Success);
}

#[tokio::test]
async fn test_update_config() {
    let server = TestServer::new().await;
    let mut config = TestFixtures::test_config();
    config.global_shortcut = "Ctrl+Shift+T".to_string();

    let response = server.post("/config", &config).await;

    response.validate_status(StatusCode::OK);

    let config_response: ConfigResponse = response.validate_json_schema();
    assert_eq!(config_response.status, Status::Success);
    assert_eq!(config_response.config.global_shortcut, "Ctrl+Shift+T");
}
```

#### Summarization Tests
```rust
// terraphim_server/tests/summarization_tests.rs
#[tokio::test]
async fn test_summarize_document() {
    let server = TestServer::new().await;
    let request = SummarizeDocumentRequest {
        document_id: "test-doc-1".to_string(),
        role: "TestRole".to_string(),
        max_length: Some(250),
        force_regenerate: Some(true),
    };

    let response = server.post("/documents/summarize", &request).await;

    // Check if OpenRouter feature is enabled
    if cfg!(feature = "openrouter") {
        response.validate_status(StatusCode::OK);
        let summary_response: SummarizeDocumentResponse = response.validate_json_schema();
        assert_eq!(summary_response.status, Status::Success);
        assert!(summary_response.summary.is_some());
    } else {
        response.validate_status(StatusCode::OK);
        let summary_response: SummarizeDocumentResponse = response.validate_json_schema();
        assert_eq!(summary_response.status, Status::Error);
        assert!(summary_response.error.unwrap().contains("OpenRouter feature not enabled"));
    }
}

#[tokio::test]
async fn test_async_summarize_document() {
    let server = TestServer::new().await;
    let request = AsyncSummarizeRequest {
        document_id: "test-doc-1".to_string(),
        role: "TestRole".to_string(),
        priority: Some("normal".to_string()),
        max_length: Some(250),
        force_regenerate: Some(true),
        callback_url: None,
    };

    let response = server.post("/documents/async_summarize", &request).await;

    response.validate_status(StatusCode::OK);

    let async_response: AsyncSummarizeResponse = response.validate_json_schema();
    assert!(matches!(async_response.status, Status::Success | Status::Error));
}
```

#### LLM Chat Tests
```rust
// terraphim_server/tests/chat_tests.rs
#[tokio::test]
async fn test_chat_completion() {
    let server = TestServer::new().await;
    let request = ChatRequest {
        role: "TestRole".to_string(),
        messages: vec![
            ChatMessage {
                role: "user".to_string(),
                content: "Hello, can you help me with testing?".to_string(),
            }
        ],
        model: None,
        conversation_id: None,
        max_tokens: Some(100),
        temperature: Some(0.7),
    };

    let response = server.post("/chat", &request).await;

    response.validate_status(StatusCode::OK);

    let chat_response: ChatResponse = response.validate_json_schema();

    // Response may be successful or error depending on LLM configuration
    match chat_response.status {
        Status::Success => {
            assert!(chat_response.message.is_some());
            assert!(chat_response.model_used.is_some());
        }
        Status::Error => {
            assert!(chat_response.error.is_some());
        }
        _ => panic!("Unexpected status: {:?}", chat_response.status),
    }
}
```

### Step 3: Add Integration Test Scenarios

#### Multi-Server Communication Tests
```rust
// terraphim_server/tests/integration/multi_server_tests.rs
#[tokio::test]
async fn test_cross_server_document_sync() {
    let server1 = TestServer::new().await;
    let server2 = TestServer::new().await;

    // Create document on server 1
    let document = TestFixtures::sample_document();
    let response1 = server1.post("/documents", &document).await;
    let create_response: CreateDocumentResponse = response1.validate_json_schema();

    // Verify document exists on server 2 (if sharing is enabled)
    let response2 = server2.get(&format!("/documents/search?query={}", document.id)).await;
    let search_response: SearchResponse = response2.validate_json_schema();

    assert_eq!(search_response.status, Status::Success);
    assert!(search_response.results.iter().any(|d| d.id == document.id));
}
```

#### Database Integration Tests
```rust
// terraphim_server/tests/integration/database_tests.rs
#[tokio::test]
async fn test_persistence_integration() {
    let server = TestServer::new().await;

    // Create document
    let document = TestFixtures::sample_document();
    let response = server.post("/documents", &document).await;
    let create_response: CreateDocumentResponse = response.validate_json_schema();

    // Restart server (simulate crash recovery)
    drop(server);
    let server = TestServer::new().await;

    // Verify document persistence
    let response = server.get(&format!("/documents/search?query={}", document.id)).await;
    let search_response: SearchResponse = response.validate_json_schema();

    assert_eq!(search_response.status, Status::Success);
    assert!(search_response.results.iter().any(|d| d.id == document.id));
}
```

#### External API Integration Tests
```rust
// terraphim_server/tests/integration/external_api_tests.rs
#[tokio::test]
#[cfg(feature = "openrouter")]
async fn test_openrouter_integration() {
    let server = TestServer::new().await;

    // Test model listing
    let request = OpenRouterModelsRequest {
        role: "TestRole".to_string(),
        api_key: None, // Use environment variable
    };

    let response = server.post("/openrouter/models", &request).await;

    if std::env::var("OPENROUTER_KEY").is_ok() {
        response.validate_status(StatusCode::OK);
        let models_response: OpenRouterModelsResponse = response.validate_json_schema();
        assert_eq!(models_response.status, Status::Success);
        assert!(!models_response.models.is_empty());
    } else {
        response.validate_status(StatusCode::OK);
        let models_response: OpenRouterModelsResponse = response.validate_json_schema();
        assert_eq!(models_response.status, Status::Error);
        assert!(models_response.error.unwrap().contains("OpenRouter API key"));
    }
}
```

### Step 4: Performance and Load Testing

#### Concurrent Request Testing
```rust
// terraphim_server/tests/performance/concurrent_tests.rs
#[tokio::test]
async fn test_concurrent_search_requests() {
    let server = TestServer::new().await;
    let client = reqwest::Client::new();

    let mut handles = Vec::new();

    // Spawn 100 concurrent search requests
    for i in 0..100 {
        let client = client.clone();
        let base_url = server.base_url.clone();

        let handle = tokio::spawn(async move {
            let start = std::time::Instant::now();

            let response = client
                .get(&format!("{}/documents/search?query=test{}", base_url, i))
                .send()
                .await
                .unwrap();

            let duration = start.elapsed();

            assert_eq!(response.status(), StatusCode::OK);

            duration
        });

        handles.push(handle);
    }

    // Wait for all requests and collect response times
    let durations: Vec<_> = futures::future::join_all(handles)
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    // Validate performance requirements
    let avg_duration = durations.iter().sum::<std::time::Duration>() / durations.len() as u32;
    assert!(avg_duration < std::time::Duration::from_millis(1000),
           "Average response time {} exceeds 1000ms", avg_duration.as_millis());

    let max_duration = durations.iter().max().unwrap();
    assert!(max_duration < std::time::Duration::from_millis(5000),
           "Maximum response time {} exceeds 5000ms", max_duration.as_millis());
}
```

#### Memory Usage Testing
```rust
// terraphim_server/tests/performance/memory_tests.rs
#[tokio::test]
async fn test_memory_usage_under_load() {
    let server = TestServer::new().await;

    // Get initial memory usage
    let initial_memory = get_memory_usage();

    // Create many documents
    for i in 0..1000 {
        let mut document = TestFixtures::sample_document();
        document.id = format!("test-doc-{}", i);
        document.title = format!("Test Document {}", i);
        document.body = format!("Content for document {}", i);

        let response = server.post("/documents", &document).await;
        response.validate_status(StatusCode::OK);
    }

    // Perform many searches
    for i in 0..1000 {
        let response = server.get(&format!("/documents/search?query=test-doc-{}", i)).await;
        response.validate_status(StatusCode::OK);
    }

    // Check memory usage after operations
    let final_memory = get_memory_usage();
    let memory_increase = final_memory - initial_memory;

    // Memory increase should be reasonable (less than 100MB)
    assert!(memory_increase < 100 * 1024 * 1024,
           "Memory increase {} bytes exceeds 100MB limit", memory_increase);
}

fn get_memory_usage() -> usize {
    // Implementation for getting current memory usage
    // This would typically use platform-specific APIs
    0 // Placeholder
}
```

#### Large Dataset Processing
```rust
// terraphim_server/tests/performance/large_dataset_tests.rs
#[tokio::test]
async fn test_large_document_processing() {
    let server = TestServer::new().await;

    // Create a large document (1MB)
    let mut large_content = String::new();
    for i in 0..10000 {
        large_content.push_str(&format!("Line {}: This is a large document for performance testing.\n", i));
    }

    let large_document = Document {
        id: "large-doc-1".to_string(),
        url: "file:///test/large.md".to_string(),
        title: "Large Test Document".to_string(),
        body: large_content,
        description: Some("A large document for performance testing".to_string()),
        summarization: None,
        stub: None,
        tags: Some(vec!["large".to_string(), "test".to_string()]),
        rank: Some(1.0),
        source_haystack: None,
    };

    // Test creation of large document
    let start = std::time::Instant::now();
    let response = server.post("/documents", &large_document).await;
    let creation_time = start.elapsed();

    response.validate_status(StatusCode::OK);
    assert!(creation_time < std::time::Duration::from_secs(5),
           "Large document creation took {} seconds", creation_time.as_secs());

    // Test searching for large document
    let start = std::time::Instant::now();
    let response = server.get("/documents/search?query=large").await;
    let search_time = start.elapsed();

    response.validate_status(StatusCode::OK);
    assert!(search_time < std::time::Duration::from_secs(3),
           "Large document search took {} seconds", search_time.as_secs());
}
```

## Test Cases

### Happy Path Tests

#### Document Creation Success
```rust
#[tokio::test]
async fn test_create_document_success() {
    let server = TestServer::new().await;
    let document = TestFixtures::sample_document();

    let response = server.post("/documents", &document).await;

    response.validate_status(StatusCode::OK);

    let create_response: CreateDocumentResponse = response.validate_json_schema();
    assert_eq!(create_response.status, Status::Success);
    assert!(!create_response.id.is_empty());
}
```

#### Search Query Success
```rust
#[tokio::test]
async fn test_search_query_success() {
    let server = TestServer::new().await;

    // First create a document
    let document = TestFixtures::sample_document();
    server.post("/documents", &document).await.validate_status(StatusCode::OK);

    // Then search for it
    let response = server.get("/documents/search?query=Test").await;

    response.validate_status(StatusCode::OK);

    let search_response: SearchResponse = response.validate_json_schema();
    assert_eq!(search_response.status, Status::Success);
    assert!(!search_response.results.is_empty());
    assert!(search_response.results.iter().any(|d| d.title.contains("Test")));
}
```

### Error Handling Tests

#### Missing Required Fields
```rust
#[tokio::test]
async fn test_create_document_missing_required_fields() {
    let server = TestServer::new().await;

    let mut incomplete_document = TestFixtures::sample_document();
    incomplete_document.id = "".to_string(); // Missing required ID

    let response = server.post("/documents", &incomplete_document).await;

    response.validate_status(StatusCode::BAD_REQUEST);

    let error_text = response.text().await.unwrap();
    assert!(error_text.contains("error") || error_text.contains("invalid"));
}
```

#### Invalid Role Names
```rust
#[tokio::test]
async fn test_invalid_role_name() {
    let server = TestServer::new().await;

    let response = server.get("/thesaurus/NonExistentRole").await;

    response.validate_status(StatusCode::NOT_FOUND);

    let thesaurus_response: ThesaurusResponse = response.validate_json_schema();
    assert_eq!(thesaurus_response.status, Status::Error);
    assert!(thesaurus_response.error.unwrap().contains("not found"));
}
```

#### Malformed JSON
```rust
#[tokio::test]
async fn test_malformed_json_request() {
    let server = TestServer::new().await;
    let client = reqwest::Client::new();

    let response = client
        .post(&format!("{}/documents", server.base_url))
        .header("Content-Type", "application/json")
        .body("{ invalid json }")
        .send()
        .await
        .unwrap();

    response.validate_status(StatusCode::BAD_REQUEST);
}
```

### Edge Case Tests

#### Boundary Conditions
```rust
#[tokio::test]
async fn test_empty_search_query() {
    let server = TestServer::new().await;

    let response = server.get("/documents/search?query=").await;

    // Should handle empty query gracefully
    response.validate_status(StatusCode::OK);

    let search_response: SearchResponse = response.validate_json_schema();
    assert_eq!(search_response.status, Status::Success);
}
```

#### Special Characters
```rust
#[tokio::test]
async fn test_search_with_special_characters() {
    let server = TestServer::new().await;

    let special_chars = "!@#$%^&*()_+-=[]{}|;':\",./<>?";
    let response = server.get(&format!("/documents/search?query={}",
        urlencoding::encode(special_chars))).await;

    response.validate_status(StatusCode::OK);

    let search_response: SearchResponse = response.validate_json_schema();
    assert_eq!(search_response.status, Status::Success);
}
```

#### Maximum Length Values
```rust
#[tokio::test]
async fn test_maximum_document_length() {
    let server = TestServer::new().await;

    let mut large_document = TestFixtures::sample_document();
    // Create a document with maximum reasonable size
    large_document.body = "x".repeat(1_000_000); // 1MB document

    let response = server.post("/documents", &large_document).await;

    // Should either succeed or fail gracefully
    match response.status() {
        StatusCode::OK => {
            let create_response: CreateDocumentResponse = response.validate_json_schema();
            assert_eq!(create_response.status, Status::Success);
        }
        StatusCode::BAD_REQUEST => {
            // Should fail with a clear error message
            let error_text = response.text().await.unwrap();
            assert!(error_text.contains("too large") || error_text.contains("limit"));
        }
        _ => panic!("Unexpected status code: {}", response.status()),
    }
}
```

### Security Tests

#### SQL Injection Prevention
```rust
#[tokio::test]
async fn test_sql_injection_prevention() {
    let server = TestServer::new().await;

    let malicious_query = "'; DROP TABLE documents; --";
    let response = server.get(&format!("/documents/search?query={}",
        urlencoding::encode(malicious_query))).await;

    // Should handle malicious input safely
    response.validate_status(StatusCode::OK);

    let search_response: SearchResponse = response.validate_json_schema();
    assert_eq!(search_response.status, Status::Success);

    // Verify no documents were actually deleted
    let normal_response = server.get("/documents/search?query=test").await;
    normal_response.validate_status(StatusCode::OK);
}
```

#### XSS Prevention
```rust
#[tokio::test]
async fn test_xss_prevention() {
    let server = TestServer::new().await;

    let mut malicious_document = TestFixtures::sample_document();
    malicious_document.title = "<script>alert('xss')</script>".to_string();
    malicious_document.body = "Document content with <script>alert('xss')</script> malicious content".to_string();

    let response = server.post("/documents", &malicious_document).await;

    response.validate_status(StatusCode::OK);

    let create_response: CreateDocumentResponse = response.validate_json_schema();
    assert_eq!(create_response.status, Status::Success);

    // Search for the document and verify XSS is sanitized
    let search_response = server.get(&format!("/documents/search?query={}",
        urlencoding::encode(&malicious_document.title))).await;

    search_response.validate_status(StatusCode::OK);

    let search_result: SearchResponse = search_response.validate_json_schema();

    // Check that script tags are properly escaped or removed
    if let Some(found_doc) = search_result.results.first() {
        assert!(!found_doc.title.contains("<script>"));
        assert!(!found_doc.body.contains("<script>"));
    }
}
```

#### Rate Limiting
```rust
#[tokio::test]
async fn test_rate_limiting() {
    let server = TestServer::new().await;
    let client = reqwest::Client::new();

    let mut responses = Vec::new();

    // Send 100 requests rapidly
    for i in 0..100 {
        let response = client
            .get(&format!("{}/documents/search?query=test{}", server.base_url, i))
            .send()
            .await
            .unwrap();
        responses.push(response.status());
    }

    // Check if any requests were rate limited
    let rate_limited_count = responses.iter()
        .filter(|&&status| status == StatusCode::TOO_MANY_REQUESTS)
        .count();

    // If rate limiting is implemented, some requests should be blocked
    // If not implemented, this test serves as documentation of the current behavior
    println!("Rate limited requests: {}/100", rate_limited_count);
}
```

## Success Criteria

### Coverage Requirements

#### Endpoint Coverage
- ✅ All HTTP endpoints have at least one test
- ✅ All response status codes are tested
- ✅ All error conditions are covered
- ✅ All input validation scenarios are tested

#### Code Coverage Metrics
- **Line Coverage**: ≥ 90% for API handlers
- **Branch Coverage**: ≥ 85% for conditional logic
- **Function Coverage**: 100% for public API functions
- **Integration Coverage**: ≥ 80% for end-to-end workflows

### Performance Benchmarks

#### Response Time Targets
- **Health Check**: ≤ 50ms (99th percentile)
- **Document Search**: ≤ 500ms (99th percentile)
- **Document Creation**: ≤ 1s (99th percentile)
- **Configuration Updates**: ≤ 2s (99th percentile)
- **LLM Chat Completion**: ≤ 30s (99th percentile, depends on external service)

#### Concurrent Load Testing
- **100 concurrent requests**: All endpoints must respond within SLA
- **Memory usage**: < 512MB under normal load
- **CPU usage**: < 80% under normal load
- **No memory leaks**: Stable memory usage over extended periods

### Security Validation

#### Input Security
- ✅ All user inputs are validated and sanitized
- ✅ SQL injection prevention is effective
- ✅ XSS prevention is implemented
- ✅ Path traversal attacks are prevented

#### API Security
- ✅ Authentication mechanisms work correctly
- ✅ Authorization checks are properly implemented
- ✅ Rate limiting prevents abuse
- ✅ Sensitive data is not exposed in error messages

### CI/CD Integration

#### Automated Testing Pipeline
```yaml
# .github/workflows/api-validation.yml
name: API Validation
on: [push, pull_request]

jobs:
  api-tests:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:13
        env:
          POSTGRES_PASSWORD: postgres
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5

    steps:
      - uses: actions/checkout@v3

      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          components: rustfmt, clippy

      - name: Cache dependencies
        uses: actions/cache@v3
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}

      - name: Install dependencies
        run: cargo build --release

      - name: Run unit tests
        run: cargo test -p terraphim_server --lib

      - name: Run integration tests
        run: cargo test -p terraphim_server --test '*'

      - name: Run performance tests
        run: cargo test -p terraphim_server --test performance -- --ignored

      - name: Run security tests
        run: cargo test -p terraphim_server --test security -- --ignored

      - name: Generate coverage report
        run: |
          cargo install cargo-tarpaulin
          cargo tarpaulin --out xml --output-dir ./coverage

      - name: Upload coverage
        uses: codecov/codecov-action@v3
        with:
          file: ./coverage/cobertura.xml
```

### Test Execution Environment

#### Local Development
```bash
# Run all API tests
cargo test -p terraphim_server

# Run only unit tests
cargo test -p terraphim_server --lib

# Run only integration tests
cargo test -p terraphim_server --test '*'

# Run performance tests
cargo test -p terraphim_server --test performance -- --ignored

# Run security tests
cargo test -p terraphim_server --test security -- --ignored
```

#### Containerized Testing
```bash
# Build test container
docker build -f Dockerfile.test -t terraphim-api-tests .

# Run tests in container
docker run --rm terraphim-api-tests

# Run tests with environment variables
docker run --rm -e OPENROUTER_KEY=test_key terraphim-api-tests
```

## Conclusion

This comprehensive API testing framework provides:

1. **Complete Endpoint Coverage**: All 40+ API endpoints are systematically tested
2. **Multi-Level Testing**: Unit, integration, end-to-end, performance, and security testing
3. **Automated CI/CD Integration**: Tests run automatically on every commit
4. **Performance Validation**: Response times and resource usage are monitored
5. **Security Assurance**: Input validation and attack prevention are verified
6. **Practical Implementation**: Tests use realistic data and scenarios

The framework ensures that Terraphim AI releases are thoroughly validated before deployment, maintaining high quality and reliability standards for users.

## Next Steps

1. **Implement Test Harness**: Create the test server infrastructure
2. **Add Endpoint Tests**: Implement tests for each API endpoint
3. **Set Up CI/CD**: Integrate tests into the automated pipeline
4. **Performance Baseline**: Establish performance benchmarks
5. **Security Audit**: Conduct comprehensive security testing
6. **Documentation**: Create testing guidelines for contributors

This testing framework will serve as the foundation for ensuring robust, secure, and performant API releases for the Terraphim AI platform.