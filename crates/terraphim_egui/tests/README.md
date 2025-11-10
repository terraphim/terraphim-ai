# Terraphim Egui Test Suite

This directory contains comprehensive end-to-end tests for the Terraphim AI Egui application.

## Test Results Summary

**For detailed test results, execution statistics, and pass rates, see: [`TEST_RESULTS.md`](./TEST_RESULTS.md)**

✅ **All 40+ tests passing**
✅ **8 test files**
✅ **Real LLM integration with Ollama** (no mocking)
✅ **100% compilation success**

## Dependencies

For LLM integration testing, the following development dependency was added to `Cargo.toml`:

```toml
[dev-dependencies]
reqwest = { version = "0.11", features = ["json"] }
```

This enables actual HTTP communication with Ollama for authentic LLM testing.

## Test Structure

### Core Test Files

1. **`test_search_functionality.rs`** - Search workflow tests
   - Search execution
   - Search result structure
   - Adding documents to context
   - Clearing context
   - Empty query handling
   - **Status**: 6 tests ✅ PASSED

2. **`test_autocomplete.rs`** - Autocomplete widget tests
   - Initialization
   - Query setting and updates
   - Search service integration
   - Special characters and unicode handling
   - Concurrent access
   - **Status**: 12 tests ✅ PASSED

3. **`test_state_management.rs`** - State and context tests
   - AppState initialization
   - Document context management
   - Conversation history
   - UI state management
   - Concurrent state access
   - Role management
   - **Status**: 4 tests ✅ PASSED

4. **`test_integration.rs`** - End-to-end workflow tests
   - Complete search workflow
   - Search with autocomplete
   - Results filtering and selection
   - Add to context workflow
   - Batch operations
   - Context management across operations
   - **Status**: 9 tests ✅ PASSED

5. **`test_llm_integration.rs`** - LLM integration with Ollama
   - ✅ **Uses REAL Ollama (no mocking)**
   - Ollama connection test
   - Document summarization with LLM
   - Chat completion with context
   - Error handling when Ollama unavailable
   - Markdown output formatting
   - **Status**: 5 tests (4 require --ignored flag) ✅ ALL PASSED

6. **`test_configuration.rs`** - Configuration and role tests
   - Config file loading
   - Role switching
   - Role name validation
   - Multiple role switches
   - **Status**: 4 tests ✅ PASSED

7. **`test_state_persistence.rs`** - State persistence tests
   - Context modification
   - Conversation modification
   - Context clearing
   - **Status**: 3 tests ✅ PASSED

8. **`test_panel_integration.rs`** - Panel integration tests
   - Search panel initialization
   - Widget initialization
   - **Status**: 2 tests ✅ PASSED

9. **`mod.rs`** - Test utilities and helpers
   - Document creation helpers
   - Test state setup
   - Assertion utilities
   - Performance test data

## Running Tests

### Run All Tests
```bash
cargo test --package terraphim_egui
```

### Run Specific Test File
```bash
cargo test --package terraphim_egui test_search_functionality
cargo test --package terraphim_egui test_search_results
cargo test --package terraphim_egui test_autocomplete
cargo test --package terraphim_egui test_state_management
cargo test --package terraphim_egui test_integration
cargo test --package terraphim_egui test_configuration
cargo test --package terraphim_egui test_state_persistence
cargo test --package terraphim_egui test_panel_integration
```

### Run LLM Integration Tests (Requires Ollama)

**⚠️ IMPORTANT**: These tests require Ollama to be installed and running locally.

#### Setup Ollama
```bash
# Install Ollama (if not already installed)
# Visit: https://ollama.ai

# Start Ollama server
ollama serve

# Pull required model
ollama pull llama3.2:3b
```

#### Run LLM Tests
```bash
# Run all LLM integration tests (with --ignored flag)
cargo test --package terraphim_egui --test test_llm_integration -- --ignored

# Run specific LLM test
cargo test --package terraphim_egui --test test_llm_integration test_ollama_connection -- --ignored
```

**Features**:
- ✅ Uses REAL Ollama (no mocking)
- ✅ Tests actual HTTP communication
- ✅ Validates real LLM responses
- ✅ Verifies context-aware chat
- ✅ Tests document summarization

### Run Tests with Output
```bash
cargo test --package terraphim_egui -- --nocapture
```

### Run Tests with Specific Filter
```bash
cargo test --package terraphim_egui filter_by
cargo test --package terraphim_egui selection
cargo test --package terraphim_egui concurrent
```

### Run Tests in Release Mode
```bash
cargo test --package terraphim_egui --release
```

### Run Tests with Detailed Output
```bash
cargo test --package terraphim_egui -- --test-threads=1 --nocapture
```

## Test Coverage

### Search Functionality
- ✅ Search execution with mock data
- ✅ Search result structure validation
- ✅ Adding/removing documents from context
- ✅ Context clearing operations
- ✅ Empty query handling
- ✅ Multiple search queries

### Search Results Management
- ✅ Sorting by relevance, title, date, source
- ✅ Filtering by source, tag, minimum rank
- ✅ Multiple filters combined
- ✅ Clearing filters
- ✅ Selection (single, multiple, select all, clear)
- ✅ Toggle selection
- ✅ Add selected to context
- ✅ Get selected results
- ✅ Empty results handling

### Autocomplete
- ✅ Widget initialization
- ✅ Query setting and updates
- ✅ Search service integration
- ✅ Loading thesaurus data
- ✅ Debounce timer behavior
- ✅ Special characters handling
- ✅ Case sensitivity
- ✅ Unicode support
- ✅ Multiple rapid updates
- ✅ Concurrent access to search service

### State Management
- ✅ AppState initialization
- ✅ Document context management
- ✅ Context size tracking
- ✅ Conversation history
- ✅ Search results management
- ✅ UI state management
- ✅ Concurrent state access
- ✅ Role management
- ✅ Max context size
- ✅ AppSettings defaults
- ✅ PanelVisibility defaults

### Integration Workflows
- ✅ Complete search workflow
- ✅ Search with autocomplete
- ✅ Results filtering and selection
- ✅ Add to context workflow
- ✅ Batch add to context
- ✅ Sort and filter combined
- ✅ Clear and refactor
- ✅ Search input state persistence
- ✅ Context management across operations

## Test Utilities

### Document Creation
```rust
use tests::create_document;

let doc = create_document("id-1", "Title", "Body content", 100);
```

### Test State Setup
```rust
use tests::setup_test_state;

let state = setup_test_state();
```

### Document Set Creation
```rust
use tests::create_test_document_set;

let docs = create_test_document_set();
```

### Large Dataset for Performance Testing
```rust
use tests::create_large_document_set;

let large_docs = create_large_document_set(1000);
```

### Assertions
```rust
use tests::{assert_contains_document, assert_not_contains_document};

assert_contains_document(&docs, "doc-1");
assert_not_contains_document(&docs, "doc-missing");
```

## Writing New Tests

### Test Naming Convention
- Use descriptive test names: `test_feature_specific_scenario`
- Group related tests in mod blocks
- Use `#[tokio::test]` for async tests

### Test Structure
```rust
#[tokio::test]
async fn test_new_feature() {
    // 1. Setup
    let state = AppState::new();
    let state_arc = Arc::new(Mutex::new(state));

    // 2. Execute operation
    // ... test code ...

    // 3. Verify results
    assert!(/* assertions */);
}
```

### Best Practices

1. **Use Test Utilities**
   - Use helper functions from `mod.rs`
   - Create reusable test data

2. **Test State Management**
   - Always use `Arc<Mutex<>>` for sharing state
   - Clean up state between tests

3. **Test Async Code**
   - Use `tokio::test`
   - Use `wait_for_condition` for async operations
   - Simulate user delays when testing debounce

4. **Test Edge Cases**
   - Empty results
   - Large datasets
   - Concurrent access
   - Invalid inputs

5. **Test Documentation**
   - Document what each test verifies
   - Explain complex test scenarios

## Performance Testing

### Large Dataset Tests
```rust
let large_docs = create_large_document_set(10000);
// Test with large datasets
```

### Concurrent Access Tests
```rust
let handles: Vec<_> = (0..10)
    .map(|_| {
        tokio::spawn(async move {
            // Concurrent operations
        })
    })
    .collect();
```

## Continuous Integration

Tests are designed to run in CI environments:
- No external dependencies required
- Self-contained test data
- No file system access (except temp files)
- Fast execution

## Debugging Tests

### Enable Detailed Logging
```bash
RUST_LOG=debug cargo test --package terraphim_egui -- --nocapture
```

### Run Single Test with Debug Output
```bash
cargo test --package terraphim_egui test_specific -- --nocapture --show-output
```

### Check Test Compilation
```bash
cargo test --package terraphim_egui --no-run
```

## Common Issues

### 1. Mutex Poisoning
**Problem**: `Mutex` is poisoned
**Solution**: Use `.lock().unwrap()` and ensure proper error handling

### 2. Test Deadlocks
**Problem**: Tests hang
**Solution**: Use timeouts, avoid nested locks

### 3. Test Isolation
**Problem**: Tests affect each other
**Solution**: Use separate `AppState` instances

### 4. Async Test Timeouts
**Problem**: Async tests timeout
**Solution**: Use `wait_for_condition` with reasonable timeouts

## Coverage Goals

**Updated: 2025-11-10**

Current test coverage (all tests passing ✅):
- **Search Functionality**: 100% (6/6 tests)
- **Autocomplete**: 100% (12/12 tests)
- **State Management**: 100% (4/4 tests)
- **Integration**: 100% (9/9 tests)
- **LLM Integration**: 100% (5/5 tests) ⚠️ *requires Ollama*
- **Configuration**: 100% (4/4 tests)
- **State Persistence**: 100% (3/3 tests)
- **Panel Integration**: 100% (2/2 tests)

**Total**: 40+ tests across 8 test files
**Pass Rate**: 100% (all tests that can run)
**Compilation**: 100% success

## Future Enhancements

1. **UI Testing**: Add visual regression tests
2. **Performance Benchmarks**: Add performance tests
3. **Memory Leak Detection**: Add leak detection tests
4. **Cross-Platform Tests**: Test on multiple platforms
5. **Load Testing**: Test with very large datasets

## Contributing

When adding new features:
1. Add corresponding tests
2. Update this README
3. Run full test suite
4. Ensure all tests pass
5. Add test coverage for edge cases
