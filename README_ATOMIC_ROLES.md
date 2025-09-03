# Atomic Server Roles for Terraphim

This document describes the two new atomic server roles created for testing atomic server haystack functionality with different scoring mechanisms.

## Overview

Two new roles have been created to test atomic server haystack integration:

1. **Atomic Title Scorer** - Uses title-based scoring for search results
2. **Atomic Graph Embeddings** - Uses graph embeddings and knowledge graph for semantic search

Both roles use the same markdown documents from `docs/src` to populate the atomic server haystack, but employ different relevance functions for ranking search results.

## Role Configurations

### 1. Atomic Title Scorer Role

**Configuration File:** `atomic_title_scorer_config.json`

**Key Features:**
- **Relevance Function:** `TitleScorer`
- **Theme:** `cerulean`
- **Knowledge Graph:** None (title-based scoring doesn't require KG)
- **Haystack:** Atomic Server at `http://localhost:9883`

**Use Case:** Best for scenarios where document titles are the primary indicator of relevance. This role is faster and simpler since it doesn't require knowledge graph construction.

### 2. Atomic Graph Embeddings Role

**Configuration File:** `atomic_graph_embeddings_config.json`

**Key Features:**
- **Relevance Function:** `TerraphimGraph`
- **Theme:** `superhero`
- **Knowledge Graph:** Built from `docs/src` markdown files
- **Haystack:** Atomic Server at `http://localhost:9883`

**Use Case:** Best for semantic search scenarios where content relationships and graph embeddings provide better search relevance than simple title matching.

## Setup Instructions

### Prerequisites

1. **Atomic Server** must be running at `http://localhost:9883`
2. **Environment Variables** (optional but recommended):
   ```bash
   export ATOMIC_SERVER_URL=http://localhost:9883
   export ATOMIC_SERVER_SECRET=your-base64-secret-here
   ```

### Step 1: Start Atomic Server

```bash
# Install atomic-server if not already installed
cargo install atomic-server

# Start the server
atomic-server start
```

### Step 2: Populate Atomic Server with Test Data

```bash
# Make the script executable (if not already)
chmod +x scripts/populate_atomic_server.sh

# Run the population script
./scripts/populate_atomic_server.sh
```

This script will:
- Create a test collection in Atomic Server
- Import all markdown files from `docs/src`, `docs/src/kg`, and `docs/src/scorers`
- Test basic search functionality
- Provide a summary of created documents

### Step 3: Run End-to-End Tests

```bash
# Make the test runner executable (if not already)
chmod +x scripts/run_atomic_roles_tests.sh

# Run all tests
./scripts/run_atomic_roles_tests.sh
```

## Test Structure

### Configuration Validation Test
- **File:** `crates/terraphim_middleware/tests/atomic_roles_e2e_test.rs`
- **Function:** `test_atomic_roles_config_validation`
- **Purpose:** Validates that both role configurations are properly structured
- **Requirement:** No server needed

### Title Scorer Role Test
- **Function:** `test_atomic_haystack_title_scorer_role`
- **Purpose:** Tests atomic server integration with title-based scoring
- **Features:**
  - Creates test documents with clear titles
  - Tests search terms that should match document titles
  - Verifies title-based ranking behavior
  - Tests integration with terraphim search pipeline

### Graph Embeddings Role Test
- **Function:** `test_atomic_haystack_graph_embeddings_role`
- **Purpose:** Tests atomic server integration with graph-based scoring
- **Features:**
  - Creates test documents with graph-related content
  - Tests semantic search using graph embeddings
  - Verifies graph-based ranking behavior
  - Tests integration with terraphim search pipeline

### Role Comparison Test
- **Function:** `test_atomic_haystack_role_comparison`
- **Purpose:** Compares behavior between title scorer and graph embeddings roles
- **Features:**
  - Creates documents that can be scored differently by each role
  - Compares search results between roles
  - Tests search pipeline integration for both roles
  - Demonstrates the differences in ranking behavior

## Expected Test Results

### Title Scorer Role
- **Search Term:** "Rust" → Should find "Rust Programming Guide"
- **Search Term:** "Terraphim" → Should find "Terraphim Architecture Overview"
- **Search Term:** "Atomic" → Should find "Atomic Server Integration"
- **Ranking:** Based on title matches, exact matches ranked higher

### Graph Embeddings Role
- **Search Term:** "graph" → Should find multiple graph-related documents
- **Search Term:** "embeddings" → Should find embedding-related documents
- **Search Term:** "knowledge" → Should find knowledge graph documents
- **Ranking:** Based on graph embeddings and semantic relationships

## Configuration Details

### Title Scorer Configuration
```json
{
  "relevance_function": "title-scorer",
  "kg": null,
  "haystacks": [
    {
      "location": "http://localhost:9883",
      "service": "Atomic",
      "read_only": true,
      "atomic_server_secret": null
    }
  ]
}
```

### Graph Embeddings Configuration
```json
{
  "relevance_function": "terraphim-graph",
  "kg": {
    "automata_path": null,
    "knowledge_graph_local": {
      "input_type": "markdown",
      "path": "./docs/src"
    },
    "public": true,
    "publish": true
  },
  "haystacks": [
    {
      "location": "http://localhost:9883",
      "service": "Atomic",
      "read_only": true,
      "atomic_server_secret": null
    }
  ]
}
```

## Troubleshooting

### Common Issues

1. **Atomic Server Not Running**
   ```
   ❌ Atomic Server is not running at http://localhost:9883
   ```
   **Solution:** Start atomic-server with `atomic-server start`

2. **Authentication Errors**
   ```
   ❌ Failed to create document: Authentication failed
   ```
   **Solution:** Set `ATOMIC_SERVER_SECRET` environment variable or use anonymous access

3. **Test Failures Due to Indexing Delays**
   ```
   Expected at least 1 results for 'Rust', but got 0
   ```
   **Solution:** Wait longer for indexing or increase retry attempts in tests

4. **Knowledge Graph Build Failures**
   ```
   Failed to build knowledge graph from docs/src
   ```
   **Solution:** Ensure `docs/src` directory exists and contains valid markdown files

### Debug Mode

To run tests with detailed logging:

```bash
export RUST_LOG=terraphim_middleware=debug,terraphim_atomic_client=debug
export RUST_BACKTRACE=1
./scripts/run_atomic_roles_tests.sh
```

## Integration with Existing Tests

These new tests complement the existing atomic server tests:

- **Existing:** `atomic_haystack_config_integration.rs` - Basic atomic server integration
- **New:** `atomic_roles_e2e_test.rs` - Role-specific atomic server testing

The new tests focus on:
- Role-specific behavior differences
- Different relevance functions
- End-to-end search pipeline integration
- Configuration validation

## Performance Considerations

### Title Scorer Role
- **Pros:** Fast, simple, no knowledge graph required
- **Cons:** Limited to title-based matching
- **Best For:** Quick searches, title-focused content

### Graph Embeddings Role
- **Pros:** Semantic search, better relevance for complex queries
- **Cons:** Slower startup (knowledge graph building), more complex
- **Best For:** Semantic search, content with rich relationships

## Future Enhancements

Potential improvements for these roles:

1. **Hybrid Scoring:** Combine title and graph-based scoring
2. **Dynamic Configuration:** Allow runtime switching between roles
3. **Performance Optimization:** Cache knowledge graph builds
4. **Advanced Search:** Add filters, facets, and advanced query syntax
5. **Real-time Updates:** Support for live document updates in atomic server

## Contributing

To add new test cases or modify existing ones:

1. Edit `crates/terraphim_middleware/tests/atomic_roles_e2e_test.rs`
2. Update configuration files if needed
3. Run tests with `./scripts/run_atomic_roles_tests.sh`
4. Ensure all tests pass before submitting changes

## References

- [Atomic Server Documentation](https://docs.atomicdata.dev/)
- [Terraphim Architecture](docs/src/Architecture.md)
- [Existing Atomic Tests](crates/terraphim_middleware/tests/atomic_haystack_config_integration.rs)
