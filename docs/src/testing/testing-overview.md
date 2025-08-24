# Testing Overview

Terraphim employs a comprehensive testing strategy to ensure code quality, reliability, and maintainability across all components and platforms.

## Testing Strategy

### Multi-Level Testing
Terraphim implements a layered testing approach:

1. **Unit Tests**: Individual function and module testing
2. **Integration Tests**: Cross-component functionality testing
3. **End-to-End Tests**: Full system workflow testing
4. **Performance Tests**: Benchmarking and performance validation
5. **WASM Tests**: WebAssembly compatibility testing

### Testing Tools

#### Rust Testing
- **cargo test**: Standard Rust testing framework
- **tokio::test**: Async test support
- **criterion**: Performance benchmarking
- **mockall**: Mocking for unit tests

#### Frontend Testing
- **Playwright**: E2E testing for desktop and web applications
- **Vitest**: Unit testing for Svelte components
- **TypeScript**: Type checking and validation

#### Integration Testing
- **Atomic Server**: Real Atomic Data server integration
- **Haystack**: Document indexing and search validation
- **Knowledge Graph**: Graph construction and traversal testing

## Unit Testing

### Rust Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_basic_functionality() {
        // Test implementation
    }
    
    #[tokio::test]
    async fn test_async_functionality() {
        // Async test implementation
    }
}
```

### Test Organization
```
crates/terraphim_service/
├── src/
│   ├── lib.rs
│   └── score/
│       ├── mod.rs
│       ├── bm25.rs
│       └── bm25_test.rs
└── tests/
    ├── integration_test.rs
    └── e2e_test.rs
```

### BM25 Testing
Comprehensive tests for all BM25 variants:

```rust
#[test]
fn test_bm25_scorer_basic_functionality() {
    let documents = create_test_documents();
    let mut bm25_scorer = OkapiBM25Scorer::new();
    bm25_scorer.initialize(&documents);
    
    let query = "rust programming";
    let scores: Vec<f64> = documents.iter()
        .map(|doc| bm25_scorer.score(query, doc))
        .collect();
    
    assert!(scores.iter().all(|&score| score >= 0.0));
}
```

## Integration Testing

### Service Integration Tests
```rust
#[tokio::test]
async fn test_search_integration() {
    let config_state = create_test_config_state();
    let mut service = TerraphimService::new(config_state);
    
    let search_query = SearchQuery {
        search_term: NormalizedTermValue::from("test"),
        skip: None,
        limit: Some(10),
        role: None,
    };
    
    let results = service.search(&search_query).await?;
    assert!(!results.is_empty());
}
```

### Atomic Data Integration
```rust
#[tokio::test]
async fn test_atomic_data_integration() {
    let client = AtomicClient::new("http://localhost:9883")?;
    
    // Test resource creation
    let resource = create_test_resource();
    let created = client.create_resource(&resource).await?;
    assert_eq!(created.subject, resource.subject);
    
    // Test search functionality
    let results = client.search("test").await?;
    assert!(!results.is_empty());
}
```

### Knowledge Graph Integration
```rust
#[test]
fn test_knowledge_graph_construction() {
    let documents = load_test_documents();
    let graph = build_knowledge_graph(&documents)?;
    
    assert!(graph.nodes.len() > 0);
    assert!(graph.edges.len() > 0);
}
```

### MCP Server Integration Testing
The MCP (Model Context Protocol) server testing validates comprehensive functionality including knowledge graph integration:

```rust
#[tokio::test]
async fn test_bug_report_extraction_with_kg_terms() -> Result<()> {
    // Connect to MCP server
    let transport = TokioChildProcess::new(cmd)?;
    let service = ().serve(transport).await?;
    
    // Build autocomplete index for Terraphim Engineer role
    let build_result = service
        .call_tool(CallToolRequestParam {
            name: "build_autocomplete_index".into(),
            arguments: json!({"role": "Terraphim Engineer"}),
        })
        .await?;
    
    // Test comprehensive bug report extraction
    let extract_result = service
        .call_tool(CallToolRequestParam {
            name: "extract_paragraphs_from_automata".into(),
            arguments: json!({
                "text": comprehensive_bug_report,
                "include_term": true,
                "role": "Terraphim Engineer"
            }),
        })
        .await?;
    
    // Validates extraction of 2,615+ paragraphs from bug reports
    assert!(extract_result.is_ok());
}
```

### Knowledge Graph Term Verification
```rust
#[tokio::test]
async fn test_kg_bug_reporting_terms_available() -> Result<()> {
    // Test autocomplete for domain-specific terms
    let payroll_autocomplete = service
        .call_tool(CallToolRequestParam {
            name: "autocomplete_terms".into(),
            arguments: json!({
                "query": "payroll",
                "limit": 10,
                "role": "Terraphim Engineer"
            }),
        })
        .await?;
    
    // Validates payroll terms: 3 suggestions
    // data consistency terms: 9 suggestions  
    // quality assurance terms: 9 suggestions
    assert!(payroll_autocomplete.is_ok());
}
```

## End-to-End Testing

### Playwright Tests
```typescript
import { test, expect } from '@playwright/test';

test('search functionality', async ({ page }) => {
  await page.goto('http://localhost:5173');
  
  // Test search input
  await page.fill('[data-testid="search-input"]', 'rust programming');
  await page.click('[data-testid="search-button"]');
  
  // Verify results
  const results = await page.locator('[data-testid="search-result"]');
  await expect(results).toHaveCount(3);
});
```

### Desktop Application Testing
```typescript
test('desktop search with BM25', async ({ page }) => {
  await page.goto('http://localhost:5174');
  
  // Select BM25 scorer
  await page.selectOption('[data-testid="scorer-select"]', 'bm25f');
  
  // Perform search
  await page.fill('[data-testid="search-input"]', 'test query');
  await page.click('[data-testid="search-button"]');
  
  // Verify BM25 results
  const results = await page.locator('[data-testid="search-result"]');
  await expect(results).toHaveCount(5);
});
```

### Atomic Server Integration
```typescript
test('atomic server haystack integration', async ({ page }) => {
  // Setup atomic server
  await setupAtomicServer();
  
  // Test document creation
  await createTestDocument();
  
  // Test search through atomic server
  await page.goto('http://localhost:5173');
  await page.fill('[data-testid="search-input"]', 'ATOMIC-test');
  await page.click('[data-testid="search-button"]');
  
  // Verify atomic document results
  const atomicResults = await page.locator('[data-testid="atomic-result"]');
  await expect(atomicResults).toHaveCount(1);
});
```

## Performance Testing

### Benchmarking
```rust
use criterion::{criterion_group, criterion_main, Criterion};

fn benchmark_bm25_scoring(c: &mut Criterion) {
    let documents = create_large_test_dataset();
    let mut bm25_scorer = BM25FScorer::new();
    bm25_scorer.initialize(&documents);
    
    c.bench_function("bm25f_scoring", |b| {
        b.iter(|| {
            bm25_scorer.score("rust programming", &documents[0])
        })
    });
}

criterion_group!(benches, benchmark_bm25_scoring);
criterion_main!(benches);
```

### Throughput Testing
```rust
#[test]
fn test_automata_throughput() {
    let terms = generate_large_term_set(10000);
    let automata = Automata::new(terms)?;
    
    let start = std::time::Instant::now();
    for _ in 0..1000 {
        let _results = automata.prefix_search("rust")?;
    }
    let duration = start.elapsed();
    
    assert!(duration.as_millis() < 100); // Should complete in under 100ms
}
```

## WASM Testing

### WebAssembly Compatibility
```rust
#[cfg(target_arch = "wasm32")]
#[test]
fn test_wasm_automata() {
    let terms = vec!["rust".to_string(), "programming".to_string()];
    let automata = Automata::new(terms)?;
    
    let results = automata.prefix_search("ru")?;
    assert_eq!(results, vec!["rust"]);
}
```

### Browser Testing
```typescript
test('WASM autocomplete in browser', async ({ page }) => {
  await page.goto('http://localhost:5173');
  
  // Test WASM autocomplete
  await page.fill('[data-testid="autocomplete-input"]', 'ru');
  
  const suggestions = await page.locator('[data-testid="autocomplete-suggestion"]');
  await expect(suggestions).toHaveCount(1);
  await expect(suggestions.first()).toHaveText('rust');
});
```

## Test Data Management

### Test Fixtures
```rust
fn create_test_documents() -> Vec<Document> {
    vec![
        Document {
            id: "doc1".to_string(),
            title: "Introduction to Rust Programming".to_string(),
            body: "Rust is a systems programming language...".to_string(),
            description: Some("A comprehensive guide to Rust".to_string()),
            tags: Some(vec!["programming".to_string(), "rust".to_string()]),
            rank: None,
            stub: None,
            url: "https://example.com/doc1".to_string(),
        },
        // More test documents...
    ]
}
```

### Test Configuration
```rust
fn create_test_config_state() -> ConfigState {
    ConfigState {
        roles: HashMap::new(),
        selected_role: RoleName::from("Engineer"),
        // Test configuration...
    }
}
```

## CI/CD Testing

### GitHub Actions
```yaml
name: Tests
on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      
      - name: Run unit tests
        run: cargo test --all
      
      - name: Run integration tests
        run: cargo test --test integration
      
      - name: Run E2E tests
        run: yarn test:e2e
      
      - name: Run performance tests
        run: cargo bench
```

### Test Coverage
```bash
# Generate coverage report
cargo tarpaulin --out Html --output-dir coverage

# View coverage in browser
open coverage/tarpaulin-report.html
```

## Testing Best Practices

### Test Organization
1. **Unit tests**: In same file as implementation
2. **Integration tests**: In `tests/` directory
3. **E2E tests**: In `desktop/tests/` directory
4. **Performance tests**: In `benches/` directory

### Test Naming
```rust
#[test]
fn test_bm25_scorer_basic_functionality() { }
#[test]
fn test_bm25f_scorer_field_weights() { }
#[test]
fn test_bm25plus_scorer_enhanced_parameters() { }
```

### Async Testing
```rust
#[tokio::test]
async fn test_async_operation() {
    let result = async_function().await?;
    assert!(result.is_some());
}
```

### Error Testing
```rust
#[test]
fn test_error_handling() {
    let result = function_that_may_fail();
    assert!(result.is_err());
    
    if let Err(ExpectedError::SpecificError) = result {
        // Test specific error case
    }
}
```

### Mock Testing
```rust
use mockall::predicate::*;

#[test]
fn test_with_mocks() {
    let mut mock_service = MockService::new();
    mock_service
        .expect_search()
        .with(eq("test query"))
        .returning(|_| Ok(vec![test_document()]));
    
    let result = mock_service.search("test query")?;
    assert!(!result.is_empty());
}
```

## Test Maintenance

### Regular Updates
- Update test data when APIs change
- Maintain test coverage above 80%
- Review and update integration tests quarterly
- Performance test updates with major releases

### Test Documentation
- Document complex test scenarios
- Explain test data requirements
- Document test environment setup
- Maintain test troubleshooting guides

## Future Testing Enhancements

### Planned Improvements
1. **Property-based testing**: Using proptest for generative testing
2. **Fuzzing**: Automated input testing for edge cases
3. **Load testing**: High-volume performance validation
4. **Security testing**: Vulnerability assessment
5. **Accessibility testing**: UI accessibility validation 