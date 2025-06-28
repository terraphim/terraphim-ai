# Document Import Test for Terraphim Atomic Server

This test demonstrates the complete end-to-end workflow of importing documents from the filesystem into Atomic Server and then searching over those imported documents.

## Overview

The test suite includes three main tests:

1. **`test_document_import_and_search`** - Main test that imports documents from `/docs/src` path and searches them
2. **`test_single_document_import_and_search`** - Tests importing a single document with specific content
3. **`test_document_import_edge_cases`** - Tests various edge cases like special characters, unicode, etc.

## Prerequisites

### 1. Atomic Server
You need a running Atomic Server instance. Install and start it:

```bash
# Install atomic-server (if not already installed)
cargo install atomic-server

# Start the server
atomic-server --port 9883
```

### 2. Environment Configuration
Create a `.env` file in the project root with:

```env
ATOMIC_SERVER_URL=http://localhost:9883
ATOMIC_SERVER_SECRET=your_secret_here
```

To get your secret, visit `http://localhost:9883/setup` in your browser and follow the setup process.

### 3. Source Documents
The test expects markdown files in the `/docs/src` directory. If you don't have any, the test will skip gracefully.

## Running the Tests

### Option 1: Using the provided script
```bash
cd crates/terraphim_middleware/tests
./run_document_import_test.sh
```

### Option 2: Running individual tests
```bash
# Run the main test
cargo test --package terraphim_middleware test_document_import_and_search -- --nocapture

# Run the single document test
cargo test --package terraphim_middleware test_single_document_import_and_search -- --nocapture

# Run the edge cases test
cargo test --package terraphim_middleware test_document_import_edge_cases -- --nocapture
```

### Option 3: Run all tests
```bash
cargo test --package terraphim_middleware -- --nocapture
```

## What the Test Does

### Main Test (`test_document_import_and_search`)

1. **Creates a parent collection** in Atomic Server for organizing imported documents
2. **Scans the `/docs/src` directory** for markdown files (`.md` extension)
3. **Imports each file** as a Document resource with:
   - Title extracted from first heading or filename
   - Full content in the `body` property
   - Source file path for reference
   - Proper metadata and relationships
4. **Searches the imported documents** using various Rust-related terms:
   - `function`, `struct`, `impl`, `async`, `test`, `use`, `mod`, `pub`, `fn`, `let`
5. **Verifies search results** contain the imported documents
6. **Cleans up** by deleting all imported documents and the parent collection

### Single Document Test (`test_single_document_import_and_search`)

Creates a single test document with specific content and verifies that searches for terms like "Rust", "async", "function", and "test" return the document.

### Edge Cases Test (`test_document_import_edge_cases`)

Tests various edge cases:
- Documents with special characters (`& < > " '`)
- Documents with unicode characters (α β γ δ ε) and emojis (🦀 🚀 💻)
- Documents with very long titles
- Documents with code blocks

## Expected Output

When running successfully, you should see output like:

```
🚀 Terraphim Document Import Test
==================================
✅ Atomic Server is running
✅ .env file found
✅ src directory found
📄 Found 15 markdown files in docs/src directory

Running document import test...
This test will:
  1. Import up to 10 markdown files from docs/src/ into Atomic Server
  2. Search the imported documents
  3. Verify search results
  4. Clean up imported documents

Imported document 1: Introduction
Imported document 2: Architecture
Imported document 3: Contributing
...
Successfully imported 10 documents
Searching for: 'function'
  Found 8 results on attempt 1
  Matching imported documents: ["Introduction", "Architecture"]
...
✅ Document import test completed!
```

## Troubleshooting

### Atomic Server not running
```
❌ Atomic Server is not running on http://localhost:9883
Please start Atomic Server first:
  atomic-server --port 9883
```

### Missing .env file
```
❌ .env file not found in project root
Please create a .env file with:
  ATOMIC_SERVER_URL=http://localhost:9883
  ATOMIC_SERVER_SECRET=your_secret_here
```

### No markdown files found
```
Warning: /docs/src directory not found, skipping test
```
or
```
No documents were imported, skipping search test
```

### Authentication errors
Make sure your `ATOMIC_SERVER_SECRET` is correct. You can get it from the Atomic Server setup page at `http://localhost:9883/setup`.

### Search not finding documents
The test includes retry logic and waits for indexing. If searches consistently fail, check:
1. Atomic Server logs for indexing errors
2. That the documents were actually created (check the Atomic Server web interface)
3. That the search terms are present in the imported documents

## Integration with Terraphim

This test validates the integration between:
- **Filesystem scanning** (using `walkdir`)
- **Document parsing** (extracting titles from markdown)
- **Atomic Server integration** (creating and searching documents)
- **Terraphim middleware** (using `AtomicHaystackIndexer`)

The test ensures that the complete pipeline from filesystem to search results works correctly, which is essential for the Terraphim desktop application's ability to import and search local documents through Atomic Server. 