# Context Engineering with Terraphim: Quick Start

Get started with context engineering using Terraphim as a superior alternative to Conare AI in 10 minutes.

## Why Terraphim?

Terraphim provides all of Conare AI's features plus:
- ✅ Open source (no $59 fee)
- ✅ Cross-platform (not just macOS)
- ✅ Knowledge graphs for semantic search
- ✅ Multiple LLM providers (Ollama, OpenRouter, custom)
- ✅ Version-controlled context (git)
- ✅ Advanced MCP integration

## Quick Start (10 minutes)

### 1. Install Terraphim

```bash
# Clone repository
git clone https://github.com/terraphim/terraphim-ai
cd terraphim-ai

# Build
cargo build --release

# Verify installation
./target/release/terraphim_server --version
```

### 2. Create Context Directories

```bash
# Create context structure
mkdir -p docs/context-library/{architecture,patterns,rules,references}
mkdir -p docs/vibe-rules/{global,rust,python,typescript}

# Example vibe-rule
cat > docs/vibe-rules/rust/async-patterns.md <<'EOF'
# Rust Async Patterns

## Tokio Spawn Pattern
#rust #async #tokio #spawn

Use `tokio::spawn` for concurrent tasks:

\`\`\`rust
tokio::spawn(async move {
    process_item(item).await;
});
\`\`\`

## See Also
- [[error-handling]]
- [[testing]]
EOF
```

### 3. Configure Context Engineer Role

Use the provided configuration:

```bash
# Copy example config
cp terraphim_server/default/context_engineer_config.json my_config.json

# Or create from scratch
cat > my_config.json <<'EOF'
{
  "id": "Server",
  "roles": {
    "Context Engineer": {
      "shortname": "CtxEng",
      "name": "Context Engineer",
      "relevance_function": "terraphim-graph",
      "kg": {
        "knowledge_graph_local": {
          "input_type": "markdown",
          "path": "docs/context-library"
        }
      },
      "haystacks": [
        {
          "location": "docs/context-library",
          "service": "Ripgrep"
        },
        {
          "location": "docs/vibe-rules",
          "service": "Ripgrep"
        }
      ],
      "llm_provider": "ollama",
      "ollama_base_url": "http://127.0.0.1:11434",
      "ollama_model": "llama3.2:3b"
    }
  },
  "default_role": "Context Engineer",
  "selected_role": "Context Engineer"
}
EOF
```

### 4. Start Terraphim Server

```bash
# Start Ollama (if using local LLM)
ollama serve

# Start Terraphim
cargo run --release -- --config my_config.json

# Server will:
# 1. Index all markdown files in context-library and vibe-rules
# 2. Build knowledge graph with nodes and edges
# 3. Create automata for fast searching
# 4. Start HTTP API on dynamic port (check logs for port)
```

### 5. Setup MCP Server for Claude Desktop

```bash
# Start MCP server (separate terminal)
cd crates/terraphim_mcp_server
./start_local_dev.sh

# Or build and run manually
cargo build --release
./target/release/terraphim_mcp_server --config ../../my_config.json
```

### 6. Configure Claude Desktop

Edit `~/Library/Application Support/Claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "terraphim": {
      "command": "/full/path/to/terraphim-ai/target/release/terraphim_mcp_server",
      "args": ["--config", "/full/path/to/terraphim-ai/my_config.json"],
      "env": {
        "RUST_LOG": "info"
      }
    }
  }
}
```

Restart Claude Desktop.

### 7. Test Context Engineering

In Claude Desktop:

```
You: "Show me async patterns"

Claude: [Uses terraphim search tool]
I found several async patterns in the knowledge graph:

1. **Tokio Spawn Pattern** (docs/vibe-rules/rust/async-patterns.md:5)
   Use `tokio::spawn` for concurrent tasks that don't share state.

   Example:
   ```rust
   tokio::spawn(async move {
       process_item(item).await;
   });
   ```

2. **Bounded Channels** (docs/vibe-rules/rust/async-patterns.md:45)
   [...]
```

---

## Usage Examples

### Example 1: Add New Vibe-Rule

```bash
# Create rule
cat > docs/vibe-rules/rust/error-handling.md <<'EOF'
# Rust Error Handling

## thiserror Pattern
#rust #error #thiserror

Use `thiserror` for custom error types:

\`\`\`rust
#[derive(Error, Debug)]
pub enum ServiceError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}
\`\`\`
EOF

# Restart Terraphim to rebuild knowledge graph
# Or wait for auto-reload (if enabled)
```

### Example 2: Search Context

```bash
# Via API
curl -X POST http://localhost:PORT/documents/search \
  -H "Content-Type: application/json" \
  -d '{"search_term": "error handling", "role": "Context Engineer"}'

# Via Claude Desktop
# Just ask: "Show me error handling patterns"
```

### Example 3: Autocomplete

```bash
# Via MCP in Claude Desktop
# Start typing "async" and Claude will suggest:
# - async-patterns
# - async-cancellation
# - async-testing
# With snippets and links
```

---

## Common Workflows

### Workflow 1: Learning New Pattern

**Goal**: Learn async cancellation in Rust

1. **Search**: "Show me async cancellation"
2. **Claude finds**: docs/vibe-rules/rust/async-patterns.md
3. **Extract context**: Specific `tokio::select!` example with line numbers
4. **Get related**: "What else should I know about async?"
5. **Claude suggests**: error-handling, testing, structured-concurrency

### Workflow 2: Code Review

**Goal**: Review async code for best practices

1. **Share code**: Paste async function
2. **Ask**: "Review this against async best practices"
3. **Claude searches**: Vibe-rules for #async #rust
4. **Claude analyzes**: Compares code to patterns
5. **Claude reports**: Violations, improvements, links to rules

### Workflow 3: Adding Context

**Goal**: Add new project documentation

1. **Create docs**: Write markdown in `docs/context-library/architecture/`
2. **Tag concepts**: Use `#hashtags` liberally
3. **Cross-reference**: Link with `[[wiki-style]]` links
4. **Restart Terraphim**: Knowledge graph rebuilds automatically
5. **Search available**: New docs now in semantic search

---

## Directory Structure

After setup, you should have:

```
terraphim-ai/
├── docs/
│   ├── context-library/          # Reference documentation
│   │   ├── README.md
│   │   ├── architecture/
│   │   ├── patterns/
│   │   ├── rules/
│   │   └── references/
│   │
│   ├── vibe-rules/               # Actionable coding rules
│   │   ├── README.md
│   │   ├── global/               # Language-agnostic rules
│   │   │   ├── naming-conventions.md
│   │   │   └── documentation-standards.md
│   │   ├── rust/                 # Rust-specific rules
│   │   │   ├── async-patterns.md
│   │   │   └── error-handling.md
│   │   ├── python/
│   │   └── typescript/
│   │
│   ├── conare-comparison.md      # Feature comparison
│   ├── context-collections.md    # Collection management
│   └── mcp-file-context-tools.md # MCP tools reference
│
├── terraphim_server/
│   └── default/
│       └── context_engineer_config.json
│
└── my_config.json                # Your config
```

---

## Comparison: Conare vs Terraphim

| Task | Conare | Terraphim |
|------|--------|-----------|
| **Upload context once** | Manual upload | Create markdown file |
| **Reuse across conversations** | Automatic | Automatic (knowledge graph) |
| **Vibe-rules** | UI-based | Git-versioned markdown |
| **File references** | "@" notation | MCP `search` + `extract_paragraphs` |
| **Token tracking** | Built-in dashboard | Document metadata |
| **Context collections** | Load/unload | Role switching |
| **Price** | $59 | Free |
| **Platform** | macOS only | Linux, macOS, Windows |
| **LLM** | Claude only | Ollama, OpenRouter, custom |

---

## Troubleshooting

### Server Won't Start

```bash
# Check config is valid JSON
cat my_config.json | jq

# Check paths exist
ls docs/context-library/
ls docs/vibe-rules/

# Check port conflicts
lsof -i :8080  # or your configured port
```

### Knowledge Graph Not Building

```bash
# Verify markdown files exist
find docs/context-library -name "*.md"
find docs/vibe-rules -name "*.md"

# Check permissions
ls -la docs/context-library/

# Run with debug logging
RUST_LOG=debug cargo run -- --config my_config.json
```

### MCP Not Connecting

```bash
# Check MCP server is running
ps aux | grep terraphim_mcp_server

# Test MCP server directly
cd crates/terraphim_mcp_server
cargo test

# Check Claude Desktop config
cat ~/Library/Application\ Support/Claude/claude_desktop_config.json

# View Claude Desktop logs
tail -f ~/Library/Logs/Claude/mcp*.log
```

### Search Returns No Results

```bash
# Verify role is selected
curl http://localhost:PORT/config | jq '.selected_role'

# Check haystacks configured
curl http://localhost:PORT/config | jq '.roles[] | .haystacks'

# Rebuild knowledge graph
cargo run -- --config my_config.json --rebuild
```

---

## Next Steps

### 1. Expand Context Library

Add more documents:
- Architecture decision records (ADRs)
- API documentation
- Design patterns
- Troubleshooting guides

### 2. Create Vibe-Rules

Codify your team's standards:
- Naming conventions
- Error handling patterns
- Testing strategies
- Security guidelines

### 3. Build Collections

Organize by context:
- `web-backend`: API development
- `cli-tools`: Command-line apps
- `data-science`: ML/data pipelines

See: [Context Collections](./context-collections.md)

### 4. Integrate with Workflow

- Git hooks for auto-rebuild
- VS Code workspace integration
- CI/CD documentation validation
- Team knowledge sharing

### 5. Optimize Performance

- Enable autocomplete index caching
- Configure lazy loading for large collections
- Use document summaries instead of full text
- Set up Redis for distributed caching

---

## Resources

### Documentation
- [Conare Comparison](./conare-comparison.md) - Full feature comparison
- [Vibe Rules Guide](./vibe-rules/README.md) - Creating coding rules
- [Context Library](./context-library/README.md) - Reference docs
- [MCP File Context](./mcp-file-context-tools.md) - File-based tools
- [Context Collections](./context-collections.md) - Collection management

### Examples
- `docs/vibe-rules/rust/async-patterns.md` - Async patterns
- `docs/vibe-rules/rust/error-handling.md` - Error handling
- `docs/vibe-rules/global/naming-conventions.md` - Naming rules
- `docs/vibe-rules/global/documentation-standards.md` - Docs standards

### Community
- GitHub Issues: https://github.com/terraphim/terraphim-ai/issues
- Discussions: https://github.com/terraphim/terraphim-ai/discussions
- Documentation: https://terraphim.io/docs

---

## Tips and Tricks

### 1. Use Hashtags Liberally

More tags = better search:
```markdown
# Pattern Name
#rust #async #tokio #concurrency #pattern #spawn
```

### 2. Cross-Reference Everything

Link related concepts:
```markdown
See also:
- [[async-patterns]]
- [[error-handling]]
- [[testing]]
```

### 3. Include Working Examples

Always show working code:
```markdown
\`\`\`rust
// Good: Works and compiles
tokio::spawn(async move { ... });
\`\`\`
```

### 4. Explain Why, Not What

```markdown
// Bad: States the obvious
// Set timeout to 30 seconds
let timeout = Duration::from_secs(30);

// Good: Explains reasoning
// Use 30 second timeout to prevent hanging while
// still allowing large uploads to complete
let timeout = Duration::from_secs(30);
```

### 5. Version Control Everything

```bash
git add docs/
git commit -m "docs: add async cancellation pattern"
git push
```

### 6. Test Your Context

Ask Claude:
- "What async patterns do you know?"
- "Show me error handling examples"
- "How should I structure tests?"

If it can't answer, add more context!

---

## Success Metrics

Track your context engineering success:

1. **Search Success Rate**: Can Claude find what you need?
2. **Code Review Quality**: Does Claude catch pattern violations?
3. **Onboarding Time**: How fast do new developers learn patterns?
4. **Documentation Usage**: Are vibe-rules actually referenced?
5. **Context Growth**: Is the knowledge graph expanding?

---

## Conclusion

You now have a working Terraphim setup for context engineering that rivals Conare AI while being:
- Free and open source
- Cross-platform
- More powerful (knowledge graphs)
- Version-controlled (git)
- Extensible (MCP, custom tools)

Start adding your context, and watch your AI assistant become an expert in your codebase!

---

## Support

Need help?
- Read the [full comparison](./conare-comparison.md)
- Check the [troubleshooting guide](#troubleshooting)
- Open an issue on GitHub
- Ask in community discussions

Happy context engineering! 🚀
