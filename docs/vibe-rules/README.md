# Vibe-Rules

Actionable coding rules, patterns, and best practices organized by language and framework. These rules are indexed into the knowledge graph and made available for semantic search during development.

## What are Vibe-Rules?

Vibe-rules are your team's coding standards and patterns encoded as searchable knowledge:

- **Searchable**: Tag rules with hashtags for semantic search
- **Contextual**: Link rules to related concepts in the knowledge graph
- **Versioned**: Track rule changes in git
- **Shareable**: Export/import collections across projects
- **Hierarchical**: Global rules + language-specific + project-specific

Think of vibe-rules as executable knowledge that AI assistants can reference when suggesting code.

## Directory Structure

```
vibe-rules/
├── global/           # Universal rules (all languages)
│   ├── naming-conventions.md
│   ├── documentation-standards.md
│   └── security-guidelines.md
├── rust/            # Rust-specific rules
│   ├── async-patterns.md
│   ├── error-handling.md
│   └── testing.md
├── python/          # Python-specific rules
│   └── pep8-extensions.md
├── typescript/      # TypeScript-specific rules
│   └── react-patterns.md
└── collections/     # Named rule sets
    ├── web-backend/
    ├── cli-tools/
    └── wasm/
```

## Quick Start

### 1. Search Rules

With Terraphim running:

```bash
# Find async patterns
curl -X POST http://localhost:PORT/documents/search \
  -d '{"search_term": "#async", "role": "Context Engineer"}'

# Via Claude Desktop MCP
# Ask: "Show me async patterns"
# Claude will search vibe-rules automatically
```

### 2. Add New Rule

Create `vibe-rules/rust/new-pattern.md`:

```markdown
# Pattern Name

## When to Use
#rust #pattern-category #specific-tag

Brief description of when this pattern applies.

## Example

\`\`\`rust
// Good example
fn example() {
    // Code here
}
\`\`\`

## Rationale

Why this pattern is recommended.

## Trade-offs

Pros and cons.

## Related
- [[other-pattern]]
```

### 3. Rebuild Knowledge Graph

```bash
# Terraphim automatically detects new files and rebuilds
# Just restart the server or reload configuration
cargo run -- --config context_engineer_config.json
```

## Rule Categories

### Global Rules (All Languages)

**Naming Conventions** (`global/naming-conventions.md`)
- Variable naming
- Function naming
- Type naming
- Boolean naming
- Constants

**Documentation Standards** (`global/documentation-standards.md`)
- When to comment
- Function documentation
- Type documentation
- README structure
- Changelog format

**Security Guidelines** (`global/security-guidelines.md`)
- Input validation
- Secret management
- Authentication patterns
- Authorization patterns
- Secure communication

### Rust Rules

**Async Patterns** (`rust/async-patterns.md`)
- `tokio::spawn` pattern
- Bounded channels
- Cancellation with `tokio::select!`
- Error propagation in async
- Structured concurrency

**Error Handling** (`rust/error-handling.md`)
- `thiserror` for custom errors
- `?` operator usage
- Error recovery patterns
- Early return pattern
- Logging errors

**Testing** (`rust/testing.md`)
- Unit test structure
- Integration tests
- Async testing with `tokio::test`
- Property-based testing
- Test organization

### Python Rules

**PEP 8 Extensions** (`python/pep8-extensions.md`)
- Type hints usage
- Async patterns with `asyncio`
- Error handling
- Testing with `pytest`

### TypeScript Rules

**React Patterns** (`typescript/react-patterns.md`)
- Component structure
- State management
- Error boundaries
- Performance optimization

## Rule Structure

Every rule follows this template:

```markdown
# Rule Title
#primary-tag #secondary-tag #language

## When to Use

Brief description of the scenario where this rule applies.

## Good Example

\`\`\`language
// Code that follows the rule
\`\`\`

## Bad Example

\`\`\`language
// Code that violates the rule
\`\`\`

## Rationale

Explanation of why this rule exists. Include:
- Performance implications
- Maintainability benefits
- Security considerations
- Team preferences

## Trade-offs

Honest discussion of pros and cons:
- **Pros**: Benefits of following this rule
- **Cons**: Situations where the rule may not apply
- **Alternatives**: Other approaches and when to use them

## Exceptions

When it's okay to break this rule:
- Specific scenarios
- Performance constraints
- External API requirements

## Related Rules

- [[related-rule-1]] - Cross-reference
- [[related-rule-2]] - Cross-reference

## References

- [External docs](https://example.com)
- Issue #123 in tracker
```

## Collections

Collections are named sets of rules for specific project types:

### web-backend/
Rules for web backend services:
- API design patterns
- Authentication/authorization
- Database access patterns
- Caching strategies
- Error handling for HTTP

### cli-tools/
Rules for command-line applications:
- Argument parsing conventions
- Error message formatting
- Output formatting
- Exit code conventions
- Configuration file handling

### wasm/
Rules for WebAssembly development:
- Memory management in WASM
- JavaScript interop patterns
- Performance optimization
- Size optimization
- Error handling across WASM boundary

### Creating Collections

```bash
# Create collection directory
mkdir -p vibe-rules/collections/my-project

# Add rules (or symlink existing ones)
ln -s ../../rust/async-patterns.md vibe-rules/collections/my-project/
ln -s ../../global/security-guidelines.md vibe-rules/collections/my-project/

# Configure role to use collection
# In context_engineer_config.json:
{
  "haystacks": [
    {
      "location": "docs/vibe-rules/collections/my-project",
      "service": "Ripgrep"
    }
  ]
}
```

## Tagging Strategy

### Primary Tags (Category)
- `#async` - Asynchronous programming
- `#error` - Error handling
- `#testing` - Testing approaches
- `#security` - Security patterns
- `#performance` - Performance optimization
- `#api` - API design
- `#database` - Database access

### Secondary Tags (Specificity)
- `#tokio` - Tokio-specific patterns
- `#channels` - Channel patterns
- `#bounded` - Bounded resources
- `#cancellation` - Cancellation patterns
- `#retry` - Retry logic
- `#timeout` - Timeout handling

### Language Tags
- `#rust` - Rust language
- `#python` - Python language
- `#typescript` - TypeScript language
- `#javascript` - JavaScript language

### Pattern Type Tags
- `#pattern` - Recommended pattern
- `#antipattern` - Pattern to avoid
- `#refactoring` - Refactoring technique
- `#best-practice` - Best practice

## Integration with Claude Desktop

Configure Claude Desktop to use vibe-rules via MCP:

```json
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

```
User: "How should I handle async cancellation in Rust?"

Claude: [Searches vibe-rules for #async #cancellation]
According to the async patterns rule, you should use tokio::select!
for cancellation. Here's the recommended pattern:

[Shows example from async-patterns.md with file path and line numbers]
```

## Version Control

Treat vibe-rules as code:

```bash
# Add new rule
git add docs/vibe-rules/rust/new-pattern.md
git commit -m "docs: add new async pattern for bounded channels"

# Update existing rule
git add docs/vibe-rules/rust/async-patterns.md
git commit -m "docs: update async-patterns with cancellation example"

# Share with team
git push origin main

# Team members pull latest rules
git pull origin main
# Terraphim rebuilds knowledge graph automatically
```

### Rule History

View rule evolution:

```bash
# See who changed what
git log docs/vibe-rules/rust/async-patterns.md

# Compare versions
git diff HEAD~1 docs/vibe-rules/rust/async-patterns.md

# Restore old version
git checkout HEAD~1 -- docs/vibe-rules/rust/async-patterns.md
```

## Token Tracking

Track token usage for LLM context:

```markdown
---
tokens: 800
category: rust-async
priority: high
last_updated: 2025-01-20
---

# Rule content...
```

Query token usage:

```bash
# Count total tokens in vibe-rules
find docs/vibe-rules -name "*.md" -exec wc -w {} + | tail -1
```

## Best Practices

### Writing Rules

1. **Be Specific**: Provide concrete examples, not abstract principles
2. **Show Both**: Include good and bad examples
3. **Explain Why**: Don't just state rules, explain reasoning
4. **Link Related**: Cross-reference related rules
5. **Keep Current**: Update as practices evolve

### Organizing Rules

1. **One Rule Per File**: Easier to search and link
2. **Descriptive Filenames**: `async-patterns.md` not `rules1.md`
3. **Consistent Structure**: Follow the template
4. **Hierarchical**: Global → Language → Framework → Project
5. **Collections for Context**: Group related rules

### Maintaining Rules

1. **Review Quarterly**: Update outdated practices
2. **Deprecate Gracefully**: Mark old rules, don't delete immediately
3. **Get Team Input**: Rules should reflect team consensus
4. **Test Examples**: Ensure code examples compile/run
5. **Measure Impact**: Track which rules help most

## Migration from Other Systems

### From Conare AI

If you're migrating from Conare:

1. Export your vibe-rules to markdown
2. Add hashtags for searching
3. Place in appropriate directory (global/rust/python/etc.)
4. Cross-reference related rules
5. Restart Terraphim to rebuild knowledge graph

### From Team Wiki

Convert wiki pages to vibe-rules:

1. Export wiki pages to markdown
2. Restructure using rule template
3. Add code examples
4. Tag with hashtags
5. Organize by category

## Troubleshooting

### Rules Not Found in Search

```bash
# Verify files exist
ls -la docs/vibe-rules/

# Check config includes path
cat context_engineer_config.json | jq '.roles["Context Engineer"].haystacks'

# Rebuild knowledge graph
cargo run -- --config context_engineer_config.json
```

### Autocomplete Not Working

```bash
# Test MCP server
cd crates/terraphim_mcp_server
./start_local_dev.sh

# Verify Claude Desktop config
cat ~/Library/Application\ Support/Claude/claude_desktop_config.json
```

### Conflicting Rules

When rules conflict:
1. Project-specific overrides framework-specific
2. Framework-specific overrides language-specific
3. Language-specific overrides global
4. Document exceptions in rule

## Examples

See example rules in:
- `global/naming-conventions.md` - Universal naming rules
- `global/documentation-standards.md` - Documentation rules
- `rust/async-patterns.md` - Rust async patterns
- `rust/error-handling.md` - Rust error handling

## Contributing

1. Fork the repository
2. Add your rule following the template
3. Test that examples compile
4. Submit pull request
5. Team reviews and discusses
6. Merge and knowledge graph updates automatically

## See Also

- [Context Library](../context-library/README.md) - Reference documentation
- [Conare Comparison](../conare-comparison.md) - Context engineering with Terraphim
- [Knowledge Graph System](../src/kg/knowledge-graph-system.md) - How rules are indexed
