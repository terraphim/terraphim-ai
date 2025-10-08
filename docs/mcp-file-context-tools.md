# MCP File Context Tools

Enhanced Model Context Protocol (MCP) tools for file-based context management, providing line-numbered references and semantic code search.

## Overview

Terraphim's MCP server exposes powerful tools for working with file context, similar to Conare AI's "@" file referencing but with additional semantic capabilities through knowledge graph integration.

## Available Tools

### 1. `extract_paragraphs_from_automata`

Extract paragraphs from text starting at matched terms, with line numbers.

**Purpose**: Get context around specific concepts or code elements with precise line references.

**Parameters**:
- `text` (string): The text content to search
- `term` (string): The term to find and extract context around
- `max_paragraphs` (number, optional): Maximum paragraphs to return (default: 3)

**Returns**:
```json
{
  "paragraphs": [
    {
      "content": "paragraph text...",
      "start_line": 42,
      "end_line": 48,
      "term_position": 42
    }
  ]
}
```

**Example Usage**:
```typescript
// Via MCP in Claude Desktop
const result = await mcp.callTool("extract_paragraphs_from_automata", {
  text: fileContents,
  term: "async fn",
  max_paragraphs: 2
});

// Returns paragraphs containing "async fn" with line numbers
```

**Use Cases**:
- Extract function definitions with line references
- Get context around specific patterns or concepts
- Reference code snippets in documentation with accurate line numbers

---

### 2. `search` (Enhanced with File Context)

Search knowledge graph with document content and line numbers.

**Purpose**: Semantic search that returns full document context with file paths.

**Parameters**:
- `query` (string): Search term or phrase
- `role` (string, optional): Role context for search
- `limit` (number, optional): Maximum results (default: 10)
- `skip` (number, optional): Results to skip for pagination

**Returns**:
```json
{
  "documents": [
    {
      "id": "doc-123",
      "url": "file:///path/to/file.rs",
      "body": "full file contents...",
      "description": "Brief summary",
      "rank": 0.95,
      "line_count": 150
    }
  ]
}
```

**Enhanced Features**:
- Returns `url` field with file paths
- Includes full `body` content for local files
- Provides `rank` for relevance sorting
- Adds `line_count` for context size estimation

**Example Usage**:
```typescript
const results = await mcp.callTool("search", {
  query: "async cancellation pattern",
  role: "Context Engineer",
  limit: 5
});

// Returns documents about async cancellation with file paths
// Claude can then extract specific lines or functions
```

---

### 3. `autocomplete_terms`

Autocomplete search terms from knowledge graph.

**Purpose**: Discover related concepts and get term suggestions.

**Parameters**:
- `query` (string): Prefix or term to autocomplete
- `limit` (number, optional): Maximum suggestions (default: 10)
- `role` (string, optional): Role context

**Returns**:
```json
{
  "suggestions": [
    {
      "term": "async-patterns",
      "normalized_term": "async patterns",
      "id": 12345,
      "url": "file:///docs/vibe-rules/rust/async-patterns.md",
      "score": 0.98
    }
  ]
}
```

**Use Cases**:
- Discover related concepts while typing
- Find synonyms and related terms
- Navigate knowledge graph interactively

---

### 4. `autocomplete_with_snippets`

Autocomplete with code/documentation snippets.

**Purpose**: Get term suggestions with preview snippets.

**Parameters**:
- `query` (string): Search prefix
- `limit` (number, optional): Maximum results
- `role` (string, optional): Role context

**Returns**:
```json
{
  "suggestions": [
    {
      "term": "tokio::spawn",
      "snippet": "async fn example() {\n    tokio::spawn(async { ... });\n}",
      "url": "file:///docs/vibe-rules/rust/async-patterns.md",
      "start_line": 42
    }
  ]
}
```

**Use Cases**:
- Preview code patterns before inserting
- See usage examples during autocomplete
- Learn API signatures interactively

---

### 5. `find_matches`

Find all concept matches in text with positions.

**Purpose**: Identify concepts/terms in code or documentation.

**Parameters**:
- `text` (string): Text to analyze
- `role` (string, optional): Role for knowledge graph context

**Returns**:
```json
{
  "matches": [
    {
      "term": "tokio",
      "normalized_term": "tokio",
      "start_position": 150,
      "end_position": 155,
      "line_number": 12,
      "concept_id": 42
    }
  ]
}
```

**Use Cases**:
- Analyze code for known patterns
- Tag documentation with concepts
- Build code-to-concept mappings

---

### 6. `is_all_terms_connected_by_path`

Check if terms are related in the knowledge graph.

**Purpose**: Verify semantic relationships between concepts.

**Parameters**:
- `terms` (array of strings): Terms to check connectivity

**Returns**:
```json
{
  "connected": true,
  "path": ["tokio", "async", "spawn"],
  "path_length": 2
}
```

**Use Cases**:
- Verify that code uses related concepts
- Find semantic gaps in documentation
- Validate tag consistency

---

## Workflow Examples

### Example 1: Find and Reference Code Pattern

**User**: "Show me how to handle async cancellation in Rust"

**Claude's Workflow**:
```typescript
// 1. Search for relevant documents
const searchResults = await mcp.callTool("search", {
  query: "async cancellation",
  role: "Context Engineer",
  limit: 3
});

// 2. Extract specific pattern with line numbers
const doc = searchResults.documents[0];
const paragraphs = await mcp.callTool("extract_paragraphs_from_automata", {
  text: doc.body,
  term: "tokio::select",
  max_paragraphs: 1
});

// 3. Return with precise reference
console.log(`Found pattern at ${doc.url}:${paragraphs[0].start_line}`);
```

**Claude's Response**:
> Here's the recommended async cancellation pattern from `docs/vibe-rules/rust/async-patterns.md:42-48`:
> 
> ```rust
> tokio::select! {
>     result = long_task() => {
>         handle_result(result);
>     }
>     _ = shutdown.recv() => {
>         cleanup().await;
>     }
> }
> ```

### Example 2: Interactive Code Completion

**User**: Typing "async" in editor

**Claude's Workflow**:
```typescript
// 1. Get autocomplete suggestions with snippets
const suggestions = await mcp.callTool("autocomplete_with_snippets", {
  query: "async",
  limit: 5
});

// 2. Show suggestions to user
// User selects "async-cancellation"

// 3. Get full context
const context = await mcp.callTool("search", {
  query: "async cancellation pattern",
  limit: 1
});
```

**Result**: Full pattern with explanation inserted into editor.

### Example 3: Code Review with Concept Analysis

**User**: "Review this code for async best practices"

**Claude's Workflow**:
```typescript
// 1. Find all async-related concepts in code
const matches = await mcp.callTool("find_matches", {
  text: userCode,
  role: "Context Engineer"
});

// 2. For each match, check if it follows patterns
for (const match of matches.matches) {
  // 3. Search for related best practices
  const practices = await mcp.callTool("search", {
    query: match.term,
    role: "Context Engineer"
  });
  
  // 4. Compare code to best practice
  // Report violations or confirm compliance
}
```

**Claude's Response**:
> I found 3 async patterns in your code:
> 
> 1. Line 42: `tokio::spawn` - ✅ Follows best practice (see `async-patterns.md:15`)
> 2. Line 67: Unbounded channel - ⚠️ Consider using bounded channel (see `async-patterns.md:45`)
> 3. Line 89: No cancellation handling - ❌ Missing cancellation (see `async-patterns.md:78`)

---

## Implementation Details

### Line Number Tracking

Line numbers are calculated during paragraph extraction:

```rust
pub struct ParagraphResult {
    pub content: String,
    pub start_line: usize,
    pub end_line: usize,
    pub term_position: usize,
}

pub fn extract_paragraphs_from_automata(
    text: &str,
    term: &str,
    max_paragraphs: usize,
) -> Vec<ParagraphResult> {
    let lines: Vec<&str> = text.lines().collect();
    let mut results = Vec::new();
    
    for (line_num, line) in lines.iter().enumerate() {
        if line.contains(term) {
            // Extract paragraph around match
            let start = line_num.saturating_sub(2);
            let end = (line_num + 3).min(lines.len());
            
            results.push(ParagraphResult {
                content: lines[start..end].join("\n"),
                start_line: start + 1,  // 1-indexed
                end_line: end,
                term_position: line_num + 1,
            });
        }
    }
    
    results
}
```

### File Path Resolution

Documents include `url` field with file:// URLs:

```rust
pub struct IndexedDocument {
    pub id: String,
    pub url: String,  // "file:///path/to/file.rs"
    pub body: String,
    pub description: String,
    pub rank: f64,
}
```

URLs are resolved to absolute paths:
- Local files: `file:///absolute/path/to/file.rs`
- Remote URLs: `https://example.com/doc.html`
- Relative paths: Resolved relative to workspace root

---

## Configuration

Enable file context tools in MCP server:

```json
{
  "mcpServers": {
    "terraphim": {
      "command": "/path/to/terraphim_mcp_server",
      "args": ["--config", "context_engineer_config.json"],
      "env": {
        "RUST_LOG": "info",
        "ENABLE_FILE_CONTEXT": "true"
      }
    }
  }
}
```

### Role Configuration

Configure Context Engineer role with appropriate haystacks:

```json
{
  "haystacks": [
    {
      "location": "docs/context-library",
      "service": "Ripgrep",
      "read_only": false
    },
    {
      "location": "docs/vibe-rules",
      "service": "Ripgrep",
      "read_only": false
    },
    {
      "location": "src",
      "service": "Ripgrep",
      "read_only": true
    }
  ]
}
```

---

## Performance Considerations

### Caching

MCP server caches:
- Autocomplete indices (rebuilt when role changes)
- Knowledge graph automata (loaded once per role)
- Document content (read from disk on demand)

### Latency

Typical latency:
- `autocomplete_terms`: 5-20ms (in-memory FST lookup)
- `search`: 50-200ms (knowledge graph traversal + file I/O)
- `extract_paragraphs_from_automata`: 10-50ms (linear scan + extraction)
- `find_matches`: 20-100ms (Aho-Corasick matching)

### Memory Usage

Memory per role:
- Autocomplete index: ~5-10MB (depends on thesaurus size)
- Knowledge graph: ~20-50MB (nodes + edges + documents)
- Document cache: ~0MB (not cached by default)

Total: ~25-60MB per active role.

---

## Comparison with Conare AI

| Feature | Conare AI | Terraphim MCP |
|---------|-----------|---------------|
| **File References** | "@" instant referencing | `search` + `extract_paragraphs` tools |
| **Line Numbers** | Automatic | Returned with paragraph extraction |
| **Context Window** | Full file | Configurable paragraphs or full file |
| **Semantic Search** | No | Yes (knowledge graph expansion) |
| **Concept Matching** | No | Yes (`find_matches`) |
| **Autocomplete** | Basic | Fuzzy + semantic expansion |
| **Token Tracking** | Built-in UI | Via document metadata |
| **Cross-References** | Manual | Automatic via knowledge graph |

**Advantages of Terraphim**:
1. **Semantic Understanding**: Finds related concepts, not just keyword matches
2. **Knowledge Graph**: Understands relationships between concepts
3. **Flexible Extraction**: Get exactly the context you need (paragraph, function, etc.)
4. **Multi-Source**: Search across local files, URLs, APIs simultaneously
5. **Extensible**: Add custom tools via MCP protocol

---

## Best Practices

### 1. Use Specific Search Terms

**Good**:
```typescript
search({ query: "tokio::select cancellation", role: "Context Engineer" })
```

**Bad**:
```typescript
search({ query: "async", role: "Context Engineer" })  // Too broad
```

### 2. Extract Minimal Context

**Good**:
```typescript
extract_paragraphs_from_automata({
  text: doc.body,
  term: "tokio::spawn",
  max_paragraphs: 1  // Only the immediate context
})
```

**Bad**:
```typescript
// Returning entire file when only one function needed
search({ query: "tokio", limit: 1 })
```

### 3. Combine Tools for Rich Context

```typescript
// 1. Find relevant documents
const docs = await search({ query: "async patterns" });

// 2. Extract specific examples
const examples = await extract_paragraphs({
  text: docs[0].body,
  term: "tokio::select"
});

// 3. Find related concepts
const related = await autocomplete_terms({
  query: "tokio::select"
});

// Result: Full context with examples and related concepts
```

### 4. Cache Autocomplete Index

```typescript
// Build once per role
await mcp.callTool("build_autocomplete_index", {
  role: "Context Engineer"
});

// Then use autocomplete freely
const suggestions = await mcp.callTool("autocomplete_terms", {
  query: "async"
});
```

---

## Troubleshooting

### MCP Tool Not Found

```bash
# Verify MCP server is running
ps aux | grep terraphim_mcp_server

# Check Claude Desktop logs
tail -f ~/Library/Logs/Claude/mcp*.log

# Test MCP server directly
cd crates/terraphim_mcp_server
./start_local_dev.sh
```

### Empty Results

```bash
# Verify knowledge graph is built
curl http://localhost:PORT/config | jq '.roles["Context Engineer"].kg'

# Check haystack configuration
curl http://localhost:PORT/config | jq '.roles["Context Engineer"].haystacks'

# Rebuild if needed
cargo run -- --config context_engineer_config.json
```

### Line Numbers Incorrect

Line numbers are 1-indexed (first line is line 1, not line 0).

If line numbers seem off:
1. Check file encoding (must be UTF-8)
2. Verify line endings (LF vs CRLF)
3. Ensure no binary content in text files

---

## Future Enhancements

Planned improvements to file context tools:

1. **Streaming Results**: Stream large file contents to avoid memory issues
2. **Syntax-Aware Extraction**: Extract complete functions/classes using AST
3. **Diff-Based Context**: Show changes between versions with line references
4. **Multi-File Context**: Extract related code across multiple files
5. **Token Budget Management**: Automatic context truncation based on LLM limits
6. **IDE Integration**: Direct jump-to-definition from MCP responses

---

## See Also

- [Conare Comparison](./conare-comparison.md) - Full feature comparison
- [MCP Integration](./src/ClaudeDesktop.md) - Claude Desktop setup
- [Knowledge Graph System](./src/kg/knowledge-graph-system.md) - How indexing works
- [Vibe Rules](./vibe-rules/README.md) - Coding rules and patterns
