# Rolegraph End-to-End Test Framework

This directory contains comprehensive end-to-end tests that validate search results in the UI exactly like the successful rolegraph and knowledge graph ranking tests, using real `terraphim_server` API without any mocking.

## 🎯 Test Objectives

The tests validate that:

1. **Search Results Appear in UI**: Search results from the backend API are correctly displayed in the Svelte frontend
2. **Role-Based Configuration**: The "Terraphim Engineer" role works correctly with local knowledge graph files
3. **API Integration**: Real HTTP API calls to `localhost:8000` return expected results
4. **Search Terms Validation**: All test search terms ("terraphim-graph", "graph embeddings", etc.) work correctly
5. **UI State Management**: Search input, results display, and error handling work properly

## 🏗️ Architecture

### Test Framework Components

1. **TerraphimServerManager**: Manages the Rust backend server lifecycle
2. **Real API Integration**: Direct HTTP calls to `terraphim_server` endpoints
3. **UI Testing**: Playwright tests for Svelte frontend components
4. **Configuration Management**: Automatic setup of "Terraphim Engineer" role configuration

### Test Data

The tests use the same search terms and expected results as the successful middleware tests:

```typescript
const TEST_SEARCH_TERMS = [
  'terraphim-graph',
  'graph embeddings',
  'graph',
  'knowledge graph based embeddings',
  'terraphim graph scorer'
];

const EXPECTED_RESULTS = {
  'terraphim-graph': { minResults: 1, expectedRank: 34 },
  'graph embeddings': { minResults: 1, expectedRank: 34 },
  'graph': { minResults: 1, expectedRank: 34 },
  'knowledge graph based embeddings': { minResults: 1, expectedRank: 34 },
  'terraphim graph scorer': { minResults: 1, expectedRank: 34 }
};
```

## 🚀 Quick Start

### Prerequisites

- Rust and Cargo installed
- Node.js and Yarn installed
- Knowledge graph files in `docs/src/kg/`

### Running Tests

#### Option 1: Automated Test Runner (Recommended)

```bash
# From the desktop directory
./scripts/run-rolegraph-e2e-tests.sh
```

This script:
- Builds `terraphim_server`
- Sets up test configuration
- Installs dependencies
- Runs all tests with proper environment

#### Option 2: Manual Playwright Test

```bash
# From the desktop directory
yarn e2e tests/e2e/rolegraph-search-validation.spec.ts
```

#### Option 3: Individual Test

```bash
# Run specific test
npx playwright test tests/e2e/rolegraph-search-validation.spec.ts --grep "should validate all test search terms"
```

## 📋 Test Suite

### Core Tests

1. **`should display search input and logo on startup`**
   - Validates basic UI components are visible
   - Checks search input and logo display

2. **`should perform search for terraphim-graph and display results in UI`**
   - Tests search functionality in the UI
   - Validates results appear correctly
   - Checks for expected content

3. **`should validate all test search terms against backend API`**
   - Tests all search terms against the API
   - Validates response structure and content
   - Compares results with expected outcomes

4. **`should perform search in UI and validate results match API`**
   - Compares UI results with API results
   - Ensures consistency between frontend and backend

5. **`should handle role switching and validate search behavior`**
   - Tests role switching functionality
   - Validates search behavior with different roles

6. **`should handle search suggestions and autocomplete`**
   - Tests autocomplete functionality
   - Validates suggestion selection

7. **`should handle error scenarios gracefully`**
   - Tests error handling for edge cases
   - Validates app stability

8. **`should validate search performance and responsiveness`**
   - Tests search performance
   - Validates app responsiveness

## ⚙️ Configuration

### Server Configuration

The tests use a "Terraphim Engineer" configuration that matches the successful middleware tests:

```json
{
  "id": "Desktop",
  "global_shortcut": "Ctrl+Shift+T",
  "roles": {
    "Terraphim Engineer": {
      "shortname": "Terraphim Engineer",
      "name": "Terraphim Engineer",
      "relevance_function": "TerraphimGraph",
      "theme": "lumen",
      "kg": {
        "automata_path": null,
        "knowledge_graph_local": {
          "input_type": "Markdown",
          "path": "./docs/src/kg"
        },
        "public": true,
        "publish": true
      },
      "haystacks": [
        {
          "location": "./docs/src",
          "service": "Ripgrep",
          "read_only": true,
          "atomic_server_secret": null
        }
      ],
      "extra": {}
    }
  },
  "default_role": "Terraphim Engineer",
  "selected_role": "Terraphim Engineer"
}
```

### Environment Variables

- `RUST_LOG=debug`: Enable debug logging for the server
- `CONFIG_PATH`: Path to test configuration file
- `SERVER_PORT=8000`: Port for terraphim_server
- `FRONTEND_URL=http://localhost:1420`: URL for Svelte frontend

## 🔍 Test Validation

### API Validation

Tests validate that the API returns:
- Correct response structure (`status`, `results`, `total`)
- Minimum expected results for each search term
- Content containing search terms or related content
- Proper document structure (`title`, `body`)

### UI Validation

Tests validate that the UI:
- Displays search results correctly
- Shows expected content from API responses
- Handles empty results gracefully
- Maintains search input state
- Responds to user interactions

### Performance Validation

Tests validate:
- Search completion within reasonable time (< 10 seconds)
- App responsiveness during searches
- Error handling without crashes

## 🐛 Troubleshooting

### Common Issues

1. **Server fails to start**
   - Check if `terraphim_server` builds successfully
   - Verify knowledge graph files exist in `docs/src/kg/`
   - Check server logs for configuration errors

2. **Tests fail with timeout**
   - Increase timeout in Playwright config
   - Check if server is responding on port 8000
   - Verify frontend is accessible on port 1420

3. **API calls fail**
   - Check server is running and healthy
   - Verify configuration is correct
   - Check network connectivity

4. **UI elements not found**
   - Check if frontend is built and running
   - Verify CSS selectors match actual UI
   - Check for JavaScript errors in browser console

### Debug Mode

Run tests with debug output:

```bash
# Enable debug logging
export RUST_LOG=debug

# Run with verbose output
npx playwright test --debug tests/e2e/rolegraph-search-validation.spec.ts
```

### Manual Verification

1. **Start server manually**:
   ```bash
   cd ../terraphim_server
   cargo run --bin terraphim_server
   ```

2. **Test API directly**:
   ```bash
   curl "http://localhost:8000/documents/search?search_term=terraphim-graph&limit=5"
   ```

3. **Check frontend**:
   ```bash
   cd desktop
   yarn dev
   # Open http://localhost:1420
   ```

## 📊 Expected Results

### Successful Test Run

When all tests pass, you should see:

```
✅ SUCCESS: All rolegraph end-to-end tests passed
   🎯 Validation: Search results appear correctly in UI
   📄 API Integration: Backend search working with Terraphim Engineer role
   🔧 Configuration: Local KG files properly integrated
   🖥️  UI Testing: Frontend correctly displays search results
```

### API Results

Expected API responses for search terms:
- `terraphim-graph`: 1+ results, rank 34
- `graph embeddings`: 1+ results, rank 34
- `graph`: 1+ results, rank 34
- `knowledge graph based embeddings`: 1+ results, rank 34
- `terraphim graph scorer`: 1+ results, rank 34

## 🔗 Related Tests

- **Middleware Tests**: `crates/terraphim_middleware/tests/rolegraph_knowledge_graph_ranking_test.rs`
- **MCP Server Tests**: `crates/terraphim_mcp_server/tests/mcp_rolegraph_validation_test.rs`
- **Config Tests**: `crates/terraphim_config/tests/desktop_config_validation_test.rs`

## 📝 Contributing

When adding new tests:

1. Follow the existing test structure
2. Use real API calls (no mocking)
3. Validate both API and UI behavior
4. Include proper error handling
5. Add descriptive test names and comments
6. Update this README with new test information
