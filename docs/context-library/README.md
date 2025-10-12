# Context Library

This directory contains reusable context for AI-assisted development. Documents here are indexed into the knowledge graph and made available for semantic search.

## Directory Structure

```
context-library/
├── architecture/       # System architecture documentation
├── patterns/          # Design patterns and best practices
├── rules/            # Coding rules and standards
└── references/       # Library and tool references
```

## Usage

### Adding Context

1. Create markdown files in the appropriate subdirectory
2. Use hashtags to tag concepts: `#async #rust #patterns`
3. Cross-reference related documents: `See [[async-patterns]]`
4. Include code examples where relevant

### Searching Context

With Terraphim running:

```bash
# Search semantically
curl -X POST http://localhost:PORT/documents/search \
  -H "Content-Type: application/json" \
  -d '{"search_term": "async patterns", "role": "Context Engineer"}'

# Via MCP in Claude Desktop
# Claude will automatically search this context when relevant
```

### Organizing Content

#### architecture/
System design, data flow, API contracts, and high-level architecture decisions.

Example topics:
- System design documents
- Architecture decision records (ADRs)
- Data flow diagrams
- API contracts and schemas
- Component interactions

#### patterns/
Reusable design patterns, best practices, and problem-solving approaches.

Example topics:
- Async/concurrent programming patterns
- Error handling strategies
- Testing approaches
- Performance optimization techniques
- Security patterns

#### rules/
Coding standards, conventions, and project-specific guidelines.

Example topics:
- Language-specific coding standards
- Code review checklists
- Security guidelines
- Performance requirements
- Documentation requirements

#### references/
Quick reference for libraries, tools, and frameworks used in the project.

Example topics:
- Library API quick references
- Tool usage guides
- Framework conventions
- Common commands and snippets
- Troubleshooting guides

## Best Practices

### Document Structure

```markdown
# Title

Brief description of the topic.

## Problem
What problem does this solve?

## Solution
How is it solved?

## Example
\`\`\`rust
// Working code example
\`\`\`

## Trade-offs
What are the pros and cons?

## Related
- [[other-pattern]]
- [[related-concept]]
```

### Tagging Strategy

Use consistent hashtags across documents:

- **Languages**: `#rust`, `#python`, `#typescript`
- **Categories**: `#async`, `#error`, `#testing`, `#security`
- **Frameworks**: `#tokio`, `#fastapi`, `#react`
- **Patterns**: `#pattern`, `#antipattern`, `#refactoring`

### Cross-References

Link related documents using `[[wiki-style]]` links:

```markdown
## See Also

- [[async-patterns]] - Async programming patterns
- [[error-handling]] - Error handling strategies
- [[testing]] - Testing approaches
```

The knowledge graph will create edges between linked concepts.

## Integration with Vibe-Rules

This context library complements vibe-rules:

- **Context Library**: Background knowledge, architecture, references
- **Vibe-Rules**: Actionable coding rules and patterns

Both are indexed together into the knowledge graph, allowing semantic search across all content.

## Token Management

To track token usage:

1. Add metadata to document frontmatter:

```markdown
---
tokens: 1500
source: OpenAI API docs
last_updated: 2025-01-20
---

# Document content...
```

2. Query document stats:

```bash
# Get indexing statistics
curl http://localhost:PORT/config | jq '.roles["Context Engineer"].kg'
```

## Maintenance

### Regular Updates

- Review and update documents quarterly
- Remove obsolete information
- Add new patterns as they emerge
- Update examples to match current best practices

### Quality Checks

- Ensure all code examples compile/run
- Verify cross-references are valid
- Check that hashtags are consistent
- Validate document structure

### Version Control

```bash
# Track changes
git add docs/context-library/
git commit -m "docs: add async cancellation pattern"

# Share with team
git push origin main
```

## Examples

See existing context in:
- `../vibe-rules/` - Coding rules and patterns
- `../src/` - Project-specific documentation

## Contributing

1. Follow the document structure template
2. Include working code examples
3. Use consistent hashtags
4. Cross-reference related documents
5. Test that examples work

## See Also

- [Conare Comparison](../conare-comparison.md) - Feature comparison with Conare AI
- [Knowledge Graph System](../src/kg/knowledge-graph-system.md) - How indexing works
- [Vibe Rules](../vibe-rules/README.md) - Actionable coding rules
