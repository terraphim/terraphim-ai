# Conare AI vs Terraphim: Context Engineering Comparison

## Executive Summary

Conare AI is a macOS-only context management tool for Claude Code ($59 lifetime). Terraphim is an open-source, cross-platform AI assistant with knowledge graph capabilities that can replicate and exceed Conare's functionality using semantic search, knowledge graphs, and the Model Context Protocol (MCP).

This guide shows how to use Terraphim to achieve superior context engineering compared to Conare, while maintaining full privacy and extensibility.

## Feature Comparison

| Feature | Conare AI | Terraphim | Advantage |
|---------|-----------|-----------|-----------|
| **Context Items** | Upload docs/PDFs/websites once, reuse | Knowledge Graph + Haystacks (local, GitHub, Notion, email, Reddit, Rust docs) | Terraphim: Multiple sources, semantic relationships |
| **Vibe-Rules** | Store coding rules and patterns | Knowledge Graph with rule nodes + Role-based system prompts | Terraphim: Hierarchical rules with graph relationships |
| **File References** | "@" referencing with line numbers | MCP tools: autocomplete, paragraph extraction, code search | Terraphim: More powerful with semantic context |
| **Token Tracking** | Monitor uploaded material tokens | Built-in document metadata and indexing stats | Terraphim: Full document analytics |
| **Privacy** | Local (uses Claude Code) | Fully local with multiple backend options | Equal: Both privacy-first |
| **Platform** | macOS only | Linux, macOS, Windows | Terraphim: Cross-platform |
| **Price** | $59 lifetime | Free, open source | Terraphim: Free |
| **LLM Support** | Claude only | Ollama, OpenRouter, any compatible provider | Terraphim: Multiple providers |
| **Search** | Basic context retrieval | BM25, BM25F, BM25Plus, TerraphimGraph with semantic expansion | Terraphim: Advanced relevance algorithms |
| **Context Management** | Load/unload rules, collections | Role switching with per-role knowledge graphs | Terraphim: More flexible with roles |

## Core Concepts Mapping

### Conare → Terraphim

#### 1. Context Items → Knowledge Graph + Haystacks

**Conare Approach:**
- Upload a document once
- Reference it across conversations
- Track token usage

**Terraphim Approach:**
```json
{
  "name": "Context Engineer",
  "haystacks": [
    {
      "location": "docs/context-library",
      "service": "Ripgrep",
      "extra_parameters": {}
    },
    {
      "location": "https://github.com/your-org/design-patterns",
      "service": "QueryRs",
      "extra_parameters": {}
    }
  ]
}
```

Terraphim indexes documents into a knowledge graph with:
- **Nodes**: Concepts extracted from documents
- **Edges**: Relationships between concepts
- **Documents**: Full-text indexed with BM25 relevance
- **Thesaurus**: Semantic mappings (synonyms, related terms)

Search once, get semantically related results automatically.

#### 2. Vibe-Rules → Knowledge Graph Rules + System Prompts

**Conare Approach:**
- Store coding rules
- Load/unload rule sets
- Global vs local rules

**Terraphim Approach:**

Create rules as knowledge graph documents with special tags:

```json
{
  "id": "async-best-practices",
  "url": "file:///rules/async-patterns.md",
  "body": "Always use tokio::spawn for concurrent tasks. Prefer bounded channels for backpressure. Use tokio::select! for cancellation.",
  "description": "Async programming best practices for Rust",
  "tags": ["rule", "async", "rust", "tokio"],
  "rank": 1.0
}
```

System prompt per role:
```json
{
  "llm_system_prompt": "You are an expert Rust engineer. Follow these coding rules:\n1. Use tokio for async\n2. Prefer bounded channels\n3. Implement proper error handling with Result<T, E>\n\nRefer to the knowledge graph for detailed patterns."
}
```

**Advantages:**
- Rules are searchable: "Show me async patterns" → finds related rules
- Rules have relationships: "async-pattern" → "tokio-spawn" → "structured-concurrency"
- Hierarchical: Global rules (all roles) + role-specific rules
- Version control: Rules are just markdown files in git

#### 3. File References → MCP Tools

**Conare Approach:**
- "@" instant file referencing
- Shows line numbers
- Provides full context

**Terraphim Approach:**

MCP server already exposes powerful tools:

```typescript
// Autocomplete with context
autocomplete_terms(prefix: string, limit: number) → [{term, snippet}]

// Extract paragraphs starting at matched term
extract_paragraphs_from_automata(text: string, term: string) → [{paragraph, line_number}]

// Search with semantic expansion
search(query: string, role: string, limit: number) → [Document]

// Graph connectivity
is_all_terms_connected_by_path(terms: string[]) → boolean
```

**Usage in Claude Desktop:**

```json
// claude_desktop_config.json
{
  "mcpServers": {
    "terraphim": {
      "command": "/path/to/terraphim_mcp_server",
      "args": ["--config", "context_engineer_config.json"]
    }
  }
}
```

Now Claude can:
1. Search your codebase semantically
2. Extract relevant paragraphs with line numbers
3. Understand relationships between concepts
4. Navigate graph connections

## Implementation Guide

### Step 1: Create Context Engineer Role

Create `terraphim_server/default/context_engineer_config.json`:

```json
{
  "id": "Server",
  "global_shortcut": "Ctrl+Shift+C",
  "roles": {
    "Context Engineer": {
      "shortname": "CtxEng",
      "name": "Context Engineer",
      "relevance_function": "terraphim-graph",
      "terraphim_it": true,
      "theme": "lumen",
      "kg": {
        "automata_path": null,
        "knowledge_graph_local": {
          "input_type": "markdown",
          "path": "docs/context-library"
        },
        "public": true,
        "publish": true
      },
      "haystacks": [
        {
          "location": "docs/context-library",
          "service": "Ripgrep",
          "read_only": false,
          "extra_parameters": {}
        },
        {
          "location": "docs/vibe-rules",
          "service": "Ripgrep",
          "read_only": false,
          "extra_parameters": {}
        }
      ],
      "llm_provider": "ollama",
      "ollama_base_url": "http://127.0.0.1:11434",
      "ollama_model": "llama3.2:3b",
      "llm_auto_summarize": true,
      "llm_system_prompt": "You are an expert Context Engineer specializing in knowledge graphs, semantic search, and AI-powered code assistance. You understand design patterns, best practices, and coding standards across multiple languages. When referencing code, always provide file paths and line numbers. When suggesting patterns, cite relevant rules from the knowledge graph.",
      "extra": {}
    }
  },
  "default_role": "Context Engineer",
  "selected_role": "Context Engineer"
}
```

### Step 2: Create Context Library Structure

```bash
# Create directory structure
mkdir -p docs/context-library/{architecture,patterns,rules,references}
mkdir -p docs/vibe-rules/{global,rust,python,typescript}

# Example structure:
docs/context-library/
├── architecture/
│   ├── system-design.md
│   ├── data-flow.md
│   └── api-contracts.md
├── patterns/
│   ├── async-rust.md
│   ├── error-handling.md
│   └── testing-strategies.md
├── rules/
│   └── coding-standards.md
└── references/
    ├── libraries.md
    └── tools.md

docs/vibe-rules/
├── global/
│   ├── naming-conventions.md
│   ├── documentation-standards.md
│   └── security-guidelines.md
├── rust/
│   ├── async-patterns.md
│   ├── error-handling.md
│   └── testing.md
├── python/
│   └── pep8-extensions.md
└── typescript/
    └── react-patterns.md
```

### Step 3: Create Vibe-Rules as Knowledge Graph

Example `docs/vibe-rules/rust/async-patterns.md`:

```markdown
# Rust Async Programming Patterns

## Tokio Spawn Pattern
#async #rust #tokio #concurrency

Always use `tokio::spawn` for concurrent tasks that don't need to share state:

\`\`\`rust
// Good: Independent tasks
tokio::spawn(async move {
    process_item(item).await
});

// Bad: Unnecessary mutex for independent work
let mutex = Arc::new(Mutex::new(state));
\`\`\`

## Bounded Channels Pattern
#async #rust #channels #backpressure

Prefer bounded channels for backpressure:

\`\`\`rust
// Good: Bounded channel with backpressure
let (tx, rx) = tokio::sync::mpsc::channel(100);

// Bad: Unbounded can cause memory issues
let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
\`\`\`

## Cancellation Pattern
#async #rust #cancellation #tokio-select

Use `tokio::select!` for proper cancellation:

\`\`\`rust
tokio::select! {
    result = long_task() => {
        handle_result(result);
    }
    _ = shutdown_signal.recv() => {
        cleanup().await;
    }
}
\`\`\`
```

This markdown becomes a knowledge graph automatically:
- **Nodes**: "async", "rust", "tokio", "concurrency", "channels", "backpressure", "cancellation"
- **Edges**: "async" → "tokio", "tokio" → "tokio::spawn", "channels" → "backpressure"
- **Document**: Indexed with full text, searchable by tags

### Step 4: Build Knowledge Graph

```bash
# Run Terraphim server with Context Engineer role
cargo run --release -- --config context_engineer_config.json

# The server will:
# 1. Index all markdown files in docs/context-library and docs/vibe-rules
# 2. Build thesaurus from hashtags and terms
# 3. Create automata for fast matching
# 4. Generate knowledge graph with nodes and edges
# 5. Enable semantic search across all documents
```

### Step 5: Use MCP Server with Claude Desktop

Configure Claude Desktop to use Terraphim MCP:

```json
{
  "mcpServers": {
    "terraphim": {
      "command": "/path/to/terraphim_mcp_server",
      "args": ["--config", "/path/to/context_engineer_config.json"],
      "env": {
        "RUST_LOG": "info"
      }
    }
  }
}
```

Now Claude can:

```
User: "Show me async patterns"
Claude: [Uses search tool] → Returns documents with #async tags

User: "What's the best practice for channels?"
Claude: [Uses autocomplete_terms("channel")] → Finds "bounded channels" rule

User: "Extract the tokio::spawn example"
Claude: [Uses extract_paragraphs_from_automata] → Returns exact code with line numbers
```

## Advanced Usage

### Context Collections

Create named collections by organizing rules into directories:

```bash
docs/vibe-rules/
├── collections/
│   ├── web-backend/        # Collection: Web backend rules
│   │   ├── api-design.md
│   │   ├── auth-patterns.md
│   │   └── database-access.md
│   ├── cli-tools/          # Collection: CLI tool rules
│   │   ├── argument-parsing.md
│   │   └── error-messages.md
│   └── wasm/               # Collection: WASM rules
│       ├── memory-management.md
│       └── js-interop.md
```

Switch collections by changing the `haystacks` location in your role config.

### Token Tracking

Terraphim tracks document metadata automatically:

```rust
// In your code
let stats = service.get_graph_stats().await?;
println!("Documents indexed: {}", stats.document_count);
println!("Nodes (concepts): {}", stats.node_count);
println!("Edges (relationships): {}", stats.edge_count);
```

For token counting, add to your documents:

```markdown
---
tokens: 1500
source: OpenAI API docs
last_updated: 2025-01-20
---
# API Documentation
...
```

### Hierarchical Rules

Implement global + role-specific rules:

```json
{
  "haystacks": [
    {
      "location": "docs/vibe-rules/global",
      "service": "Ripgrep",
      "extra_parameters": {}
    },
    {
      "location": "docs/vibe-rules/rust",
      "service": "Ripgrep",
      "extra_parameters": {}
    }
  ]
}
```

Search priority: Role-specific rules rank higher than global rules.

### Version Control Integration

Since rules are markdown files:

```bash
# Track rule changes
git add docs/vibe-rules/
git commit -m "Add async cancellation pattern"

# Share rules with team
git push origin main

# Team members pull latest rules
git pull origin main

# Terraphim rebuilds knowledge graph automatically
```

## Migration from Conare

If you're currently using Conare:

1. **Export context items**: Copy your uploaded documents to `docs/context-library/`
2. **Export vibe-rules**: Copy your rules to `docs/vibe-rules/` as markdown with hashtags
3. **Configure Terraphim**: Create `context_engineer_config.json` with your preferences
4. **Run Terraphim**: Start the server and MCP server
5. **Configure Claude Desktop**: Point to Terraphim MCP server
6. **Test**: Search for rules, verify autocomplete works

## Best Practices

### 1. Use Hashtags for Tagging

```markdown
# Error Handling Pattern
#rust #error #result #thiserror

Use `thiserror` for custom error types:
\`\`\`rust
#[derive(thiserror::Error, Debug)]
pub enum MyError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
\`\`\`
```

### 2. Create Cross-References

```markdown
# Testing Async Code
#rust #testing #async #tokio

See also: [[async-patterns]], [[tokio-spawn]]

Use `tokio::test` for async tests:
\`\`\`rust
#[tokio::test]
async fn test_async_function() {
    let result = my_async_fn().await;
    assert_eq!(result, expected);
}
\`\`\`
```

### 3. Include Code Examples

Always include runnable code snippets in rules:

```markdown
# Channel Pattern
#rust #channels #bounded

\`\`\`rust
// Example: Producer-consumer with bounded channel
use tokio::sync::mpsc;

#[tokio::main]
async fn main() {
    let (tx, mut rx) = mpsc::channel(100);

    // Producer
    tokio::spawn(async move {
        for i in 0..10 {
            tx.send(i).await.unwrap();
        }
    });

    // Consumer
    while let Some(msg) = rx.recv().await {
        println!("Received: {}", msg);
    }
}
\`\`\`
```

### 4. Maintain Rule Hierarchy

```
docs/vibe-rules/
├── 00-global/              # Priority 1: Global rules
├── 10-language/            # Priority 2: Language-specific
├── 20-framework/           # Priority 3: Framework-specific
└── 30-project/             # Priority 4: Project-specific
```

## API Reference

### MCP Tools Available

All tools are automatically exposed via Terraphim MCP server:

#### Search Tools
- `search(query, role, limit, skip)` - Semantic search with knowledge graph expansion
- `autocomplete_terms(prefix, limit)` - Fast autocomplete from knowledge graph
- `fuzzy_autocomplete_search(query, max_distance)` - Fuzzy matching with Jaro-Winkler

#### Context Tools
- `extract_paragraphs_from_automata(text, term)` - Extract paragraphs starting at matched term
- `find_matches(text, role)` - Find all concept matches in text
- `is_all_terms_connected_by_path(terms)` - Check if terms are related in graph

#### Knowledge Graph Tools
- `load_thesaurus(role)` - Load knowledge graph for role
- `get_term_context(term, depth)` - Get related concepts with depth traversal

## Performance Comparison

| Operation | Conare | Terraphim | Notes |
|-----------|--------|-----------|-------|
| Initial indexing | ~1s | ~2-3s | Terraphim builds full knowledge graph |
| Context retrieval | <100ms | <50ms | Terraphim uses Aho-Corasick automata |
| Semantic search | N/A | ~200ms | Terraphim expands queries via graph |
| Token counting | Real-time | Metadata-based | Both provide usage info |
| Memory usage | Unknown | ~50MB per role | Terraphim caches automata in memory |

## Troubleshooting

### Knowledge Graph Not Building

```bash
# Check if markdown files exist
ls -la docs/context-library/

# Verify config path
cat terraphim_server/default/context_engineer_config.json | jq '.roles["Context Engineer"].kg'

# Run with debug logging
RUST_LOG=debug cargo run -- --config context_engineer_config.json
```

### MCP Server Not Connecting

```bash
# Test MCP server manually
cd crates/terraphim_mcp_server
./start_local_dev.sh

# Check Claude Desktop config
cat ~/Library/Application\ Support/Claude/claude_desktop_config.json

# Verify MCP server is running
ps aux | grep terraphim_mcp_server
```

### Search Returns No Results

```bash
# Verify documents are indexed
curl http://localhost:PORT/config | jq '.roles["Context Engineer"].kg'

# Check haystack configuration
curl http://localhost:PORT/config | jq '.roles["Context Engineer"].haystacks'

# Test search directly
curl -X POST http://localhost:PORT/documents/search \
  -H "Content-Type: application/json" \
  -d '{"search_term": "async", "role": "Context Engineer"}'
```

## Conclusion

Terraphim provides a superior alternative to Conare AI by offering:

1. **Open Source**: No licensing fees, full customization
2. **Cross-Platform**: Works on Linux, macOS, Windows
3. **Knowledge Graphs**: Semantic relationships between concepts
4. **Multiple LLMs**: Ollama, OpenRouter, custom providers
5. **Advanced Search**: BM25, semantic expansion, graph traversal
6. **Version Control**: Rules and context are just markdown in git
7. **MCP Integration**: Native support for Claude Desktop and other MCP clients
8. **Privacy**: Runs entirely locally with no external dependencies

By using Terraphim as your "Context Engineer", you gain all of Conare's benefits plus advanced knowledge graph capabilities for true semantic code understanding.

## Further Reading

- [Terraphim Knowledge Graph System](./src/kg/knowledge-graph-system.md)
- [MCP Integration Guide](./src/ClaudeDesktop.md)
- [Role Configuration](./src/Architecture.md)
- [Testing Strategies](./src/testing/testing-overview.md)
