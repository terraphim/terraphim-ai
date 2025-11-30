# Context Collections Management

Guide to organizing, managing, and switching between context collections in Terraphim for optimal AI-assisted development.

## What are Context Collections?

Context collections are named sets of documents, vibe-rules, and configuration organized for specific development contexts. Think of them as workspaces or profiles that include:

- **Documents**: Reference materials, architecture docs, API documentation
- **Vibe-Rules**: Coding standards, patterns, and best practices
- **Configuration**: LLM settings, haystack configurations, relevance functions
- **Knowledge Graph**: Pre-built semantic relationships between concepts

Collections enable quick context switching without losing accumulated knowledge.

## Collection Types

### 1. Domain Collections

Organized by technical domain or area of expertise.

**Examples**:
- `web-backend`: API design, database patterns, authentication
- `cli-tools`: Argument parsing, POSIX conventions, error messages
- `wasm`: Memory management, JS interop, size optimization
- `embedded`: Hardware constraints, real-time, power management
- `data-science`: Algorithm notebooks, ML patterns, visualization

### 2. Project Collections

Project-specific knowledge and conventions.

**Examples**:
- `project-alpha`: Alpha project architecture, API contracts, team conventions
- `client-beta`: Client-specific patterns, business rules, integration docs
- `internal-tools`: Company-wide standards, authentication patterns, logging

### 3. Language Collections

Language-specific patterns and idioms.

**Examples**:
- `rust-advanced`: Async patterns, unsafe code, FFI, macros
- `python-data`: NumPy patterns, Pandas idioms, ML pipelines
- `typescript-react`: Component patterns, hooks, state management

### 4. Learning Collections

Educational materials and tutorials.

**Examples**:
- `rust-beginner`: Ownership basics, borrowing, lifetimes
- `tokio-tutorial`: Async fundamentals, runtime, channels
- `kubernetes-ops`: Deployment patterns, monitoring, debugging

---

## Directory Structure

```
docs/
├── context-collections/
│   ├── web-backend/
│   │   ├── collection.json          # Collection metadata
│   │   ├── architecture/
│   │   │   ├── api-design.md
│   │   │   ├── auth-patterns.md
│   │   │   └── database-access.md
│   │   ├── vibe-rules/
│   │   │   ├── rest-api-conventions.md
│   │   │   ├── error-responses.md
│   │   │   └── rate-limiting.md
│   │   └── references/
│   │       ├── http-status-codes.md
│   │       └── jwt-claims.md
│   │
│   ├── cli-tools/
│   │   ├── collection.json
│   │   ├── patterns/
│   │   │   ├── argument-parsing.md
│   │   │   ├── subcommands.md
│   │   │   └── configuration.md
│   │   └── vibe-rules/
│   │       ├── error-messages.md
│   │       ├── output-formatting.md
│   │       └── exit-codes.md
│   │
│   └── wasm/
│       ├── collection.json
│       ├── architecture/
│       │   ├── memory-management.md
│       │   └── js-interop.md
│       └── vibe-rules/
│           ├── wasm-bindgen-patterns.md
│           └── performance-tips.md
```

---

## Collection Metadata

Each collection has a `collection.json` file:

```json
{
  "name": "Web Backend Development",
  "shortname": "web-backend",
  "description": "Context for building web backend services with REST APIs, databases, and authentication",
  "version": "1.0.0",
  "tags": ["web", "backend", "api", "database"],
  "languages": ["rust", "python", "typescript"],
  "frameworks": ["axum", "salvo", "fastapi", "express"],
  "created": "2025-01-15",
  "updated": "2025-01-20",
  "author": "Engineering Team",
  "token_estimate": 15000,
  "dependencies": [
    "global/naming-conventions",
    "global/documentation-standards"
  ],
  "recommended_llm": {
    "provider": "ollama",
    "model": "llama3.2:3b",
    "temperature": 0.7,
    "system_prompt_template": "web-backend-engineer"
  },
  "haystacks": [
    {
      "location": "docs/context-collections/web-backend",
      "service": "Ripgrep",
      "priority": 1
    },
    {
      "location": "docs/vibe-rules/global",
      "service": "Ripgrep",
      "priority": 2
    }
  ]
}
```

---

## Creating Collections

### Method 1: Directory-Based (Simple)

```bash
# Create collection directory
mkdir -p docs/context-collections/my-collection/{architecture,patterns,vibe-rules,references}

# Add documents
cat > docs/context-collections/my-collection/patterns/example-pattern.md <<EOF
# Example Pattern
#my-tag #pattern

Pattern description here...
EOF

# Create collection metadata
cat > docs/context-collections/my-collection/collection.json <<EOF
{
  "name": "My Collection",
  "shortname": "my-collection",
  "description": "Description here",
  "tags": ["example"]
}
EOF
```

### Method 2: Symlink-Based (Reuse Existing)

```bash
# Create collection directory
mkdir -p docs/context-collections/my-collection

# Symlink existing vibe-rules
ln -s ../../../vibe-rules/rust/async-patterns.md \
  docs/context-collections/my-collection/

ln -s ../../../vibe-rules/global/naming-conventions.md \
  docs/context-collections/my-collection/

# Create collection metadata
cat > docs/context-collections/my-collection/collection.json <<EOF
{
  "name": "My Collection",
  "shortname": "my-collection",
  "description": "Curated rules from global and rust collections",
  "tags": ["rust", "async"]
}
EOF
```

### Method 3: CLI Tool (Planned)

```bash
# Create collection from template
terraphim collection create web-backend \
  --template=web-backend \
  --languages=rust,python \
  --frameworks=axum,fastapi

# Add documents
terraphim collection add web-backend \
  --path=docs/api-design.md

# Import vibe-rules
terraphim collection import web-backend \
  --rules=rust/async-patterns,global/naming-conventions
```

---

## Switching Collections

### Using Role Configuration

Create a role per collection:

```json
{
  "roles": {
    "Web Backend Engineer": {
      "name": "Web Backend Engineer",
      "relevance_function": "terraphim-graph",
      "kg": {
        "knowledge_graph_local": {
          "input_type": "markdown",
          "path": "docs/context-collections/web-backend"
        }
      },
      "haystacks": [
        {
          "location": "docs/context-collections/web-backend",
          "service": "Ripgrep"
        },
        {
          "location": "docs/vibe-rules/global",
          "service": "Ripgrep"
        }
      ],
      "llm_system_prompt": "You are an expert web backend engineer..."
    },
    "CLI Tools Developer": {
      "name": "CLI Tools Developer",
      "relevance_function": "terraphim-graph",
      "kg": {
        "knowledge_graph_local": {
          "input_type": "markdown",
          "path": "docs/context-collections/cli-tools"
        }
      },
      "haystacks": [
        {
          "location": "docs/context-collections/cli-tools",
          "service": "Ripgrep"
        }
      ],
      "llm_system_prompt": "You are an expert CLI tool developer..."
    }
  }
}
```

Switch between collections by changing the selected role:

```bash
# Via API
curl -X POST http://localhost:PORT/config \
  -d '{"selected_role": "Web Backend Engineer"}'

# Via TUI
terraphim-agent roles select "Web Backend Engineer"

# Via desktop UI
# Settings → Roles → Select "Web Backend Engineer"
```

### Dynamic Collection Loading

Load collections on-demand without restarting:

```rust
// Via Terraphim service API
let service = TerraphimService::new(config_state);

// Load new collection
service.load_collection("web-backend").await?;

// Switch active collection
service.activate_collection("web-backend").await?;

// Knowledge graph rebuilds automatically
```

---

## Collection Management

### Listing Collections

```bash
# List available collections
ls docs/context-collections/

# Get collection metadata
cat docs/context-collections/web-backend/collection.json | jq

# Via API
curl http://localhost:PORT/collections
```

### Exporting Collections

Export collection as shareable archive:

```bash
# Create collection archive
tar czf web-backend-collection.tar.gz \
  docs/context-collections/web-backend/

# Include dependencies
tar czf web-backend-full.tar.gz \
  docs/context-collections/web-backend/ \
  docs/vibe-rules/global/

# Share with team
scp web-backend-collection.tar.gz team@server:/collections/
```

### Importing Collections

Import collection from archive:

```bash
# Extract collection
tar xzf web-backend-collection.tar.gz -C docs/context-collections/

# Verify collection
ls docs/context-collections/web-backend/

# Rebuild knowledge graph
cargo run -- --config context_engineer_config.json
```

### Versioning Collections

Use git to version collections:

```bash
# Tag collection version
git tag -a web-backend-v1.0 -m "Web Backend Collection v1.0"

# View collection history
git log docs/context-collections/web-backend/

# Rollback to previous version
git checkout web-backend-v0.9 -- docs/context-collections/web-backend/

# Create branch for experimental changes
git checkout -b web-backend-experiment
```

---

## Collection Composition

### Hierarchical Collections

Collections can inherit from parent collections:

```json
{
  "name": "Project Alpha Backend",
  "shortname": "project-alpha-backend",
  "extends": "web-backend",
  "overrides": {
    "llm_system_prompt": "You are working on Project Alpha backend..."
  },
  "additional_haystacks": [
    {
      "location": "projects/alpha/docs",
      "service": "Ripgrep"
    }
  ]
}
```

Resolution order:
1. Project-specific collection
2. Parent collection (web-backend)
3. Global vibe-rules
4. Language-specific vibe-rules

### Collection Mixing

Combine multiple collections:

```json
{
  "name": "Full Stack Developer",
  "shortname": "fullstack",
  "combines": [
    "web-backend",
    "typescript-react",
    "database-admin"
  ],
  "haystacks": [
    // Haystacks from all three collections merged
  ]
}
```

### Collection Overlays

Temporary modifications without changing base collection:

```json
{
  "name": "Web Backend (Performance Focus)",
  "shortname": "web-backend-perf",
  "base": "web-backend",
  "overlay": {
    "additional_rules": [
      "performance/latency-optimization.md",
      "performance/memory-profiling.md"
    ],
    "llm_temperature": 0.5  // More focused
  }
}
```

---

## Token Management

### Estimating Collection Size

```bash
# Count tokens in collection
find docs/context-collections/web-backend -name "*.md" -exec wc -w {} + | tail -1

# Estimate with metadata
jq '.token_estimate' docs/context-collections/web-backend/collection.json
```

### Optimizing Collection Size

**Strategies**:

1. **Selective Loading**: Only load relevant subcollections
2. **Document Pruning**: Remove obsolete or rarely-used docs
3. **Snippet Extraction**: Store only relevant snippets, not full docs
4. **Lazy Loading**: Load documents on-demand, not upfront

**Example**:
```json
{
  "lazy_loading": {
    "enabled": true,
    "load_threshold": 10000,  // tokens
    "priority_tags": ["critical", "core"]
  }
}
```

### Token Budget Per Context

Configure per-collection token budgets:

```json
{
  "token_budget": {
    "total": 20000,
    "allocation": {
      "system_prompt": 500,
      "vibe_rules": 5000,
      "architecture": 8000,
      "code_examples": 5000,
      "references": 1500
    }
  }
}
```

---

## Best Practices

### 1. Keep Collections Focused

**Good**:
```
web-backend/
├── api-design.md
├── auth-patterns.md
└── database-access.md
```

**Bad**:
```
web-backend/
├── api-design.md
├── mobile-ui-patterns.md  # Wrong domain
├── ml-algorithms.md       # Wrong domain
└── database-access.md
```

### 2. Use Consistent Structure

All collections should follow same structure:
```
collection-name/
├── collection.json
├── architecture/
├── patterns/
├── vibe-rules/
└── references/
```

### 3. Document Dependencies

```json
{
  "dependencies": [
    "global/naming-conventions",
    "global/documentation-standards",
    "rust/error-handling"
  ]
}
```

### 4. Tag Aggressively

```markdown
# API Design Pattern
#api #rest #http #design #web #backend

Pattern description...
```

More tags = better search and discovery.

### 5. Include Examples

Every pattern should have working examples:

```markdown
# Pattern Name

## Example
\`\`\`rust
// Working code
\`\`\`

## Anti-Example
\`\`\`rust
// What not to do
\`\`\`
```

### 6. Version Control Everything

```bash
# Track all changes
git add docs/context-collections/
git commit -m "docs(web-backend): add rate limiting pattern"

# Tag stable versions
git tag web-backend-v1.1
```

### 7. Measure Usage

Track which collections are most useful:

```bash
# Search frequency per collection
curl http://localhost:PORT/stats/collections

# Most referenced documents
curl http://localhost:PORT/stats/documents?collection=web-backend
```

---

## Integration with Development Tools

### VS Code

Create collection-specific workspaces:

```json
// .vscode/web-backend.code-workspace
{
  "folders": [
    {
      "path": "."
    }
  ],
  "settings": {
    "terraphim.collection": "web-backend",
    "terraphim.autoload": true
  }
}
```

### Claude Desktop

Collection-specific MCP configurations:

```json
{
  "mcpServers": {
    "terraphim-web": {
      "command": "/path/to/terraphim_mcp_server",
      "args": ["--config", "web_backend_config.json"]
    },
    "terraphim-cli": {
      "command": "/path/to/terraphim_mcp_server",
      "args": ["--config", "cli_tools_config.json"]
    }
  }
}
```

### Git Hooks

Auto-rebuild knowledge graph on collection changes:

```bash
#!/bin/bash
# .git/hooks/post-commit

# Check if any collection changed
if git diff-tree -r --name-only --no-commit-id HEAD | grep "context-collections"; then
  echo "Collection changed, rebuilding knowledge graph..."
  cargo run --release -- --rebuild-kg
fi
```

---

## Troubleshooting

### Collection Not Loading

```bash
# Verify collection exists
ls docs/context-collections/my-collection/

# Check collection.json is valid
cat docs/context-collections/my-collection/collection.json | jq

# Verify role configuration
cat config.json | jq '.roles["My Role"].kg.knowledge_graph_local.path'
```

### Search Returns Wrong Results

```bash
# Check active collection
curl http://localhost:PORT/config | jq '.selected_role'

# Verify haystack configuration
curl http://localhost:PORT/config | jq '.roles[] | .haystacks'

# Rebuild knowledge graph
cargo run -- --config config.json --rebuild
```

### Token Budget Exceeded

```bash
# Check collection size
find docs/context-collections/web-backend -name "*.md" -exec wc -w {} +

# Reduce collection:
# 1. Remove rarely-used documents
# 2. Enable lazy loading
# 3. Use document summaries instead of full text
```

---

## Future Enhancements

Planned features:

1. **Collection Marketplace**: Share collections with community
2. **Auto-Generated Collections**: ML-based collection creation from codebases
3. **Collection Analytics**: Track which documents help most
4. **Smart Merging**: Automatically merge overlapping collections
5. **Collection Testing**: Verify collection quality with test queries
6. **Cloud Sync**: Sync collections across devices

---

## Examples

### Web Backend Collection

See: `docs/context-collections/web-backend/`

Includes:
- REST API design patterns
- Authentication/authorization
- Database access patterns
- Error handling conventions
- Rate limiting strategies

### CLI Tools Collection

See: `docs/context-collections/cli-tools/`

Includes:
- Argument parsing patterns
- Subcommand structure
- Error message formatting
- Configuration file handling
- POSIX conventions

### WASM Collection

See: `docs/context-collections/wasm/`

Includes:
- Memory management patterns
- JS interop with wasm-bindgen
- Performance optimization
- Size optimization
- Error handling across boundaries

---

## See Also

- [Conare Comparison](./conare-comparison.md) - Context engineering comparison
- [Vibe Rules](./vibe-rules/README.md) - Coding rules and patterns
- [Context Library](./context-library/README.md) - Reference documentation
- [MCP File Context](./mcp-file-context-tools.md) - File-based context tools
