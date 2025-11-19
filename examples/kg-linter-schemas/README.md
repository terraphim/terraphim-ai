# Knowledge Graph Linter Schema Examples

This directory contains example markdown schemas for the Terraphim KG linter.

## File Overview

### Valid Schemas

1. **valid-command.md**
   - Well-formed command definition
   - Demonstrates all features: parameters, validation, permissions, KG requirements
   - Uses `local` execution mode with `low` risk level
   - Includes comprehensive documentation

2. **valid-kg-schema.md**
   - Complete knowledge graph schema definition
   - Defines node types (Concept, Document) and edge types (RelatedTo, ContainedIn)
   - Includes relationship constraints and validation rules
   - Demonstrates automata configuration and security permissions

3. **valid-thesaurus-schema.md**
   - Thesaurus schema with term normalization rules
   - Includes synonym groups and fuzzy matching configuration
   - Demonstrates metadata and licensing information
   - Shows integration with Rust standard library documentation

### Invalid Schemas

4. **invalid-command.md**
   - Intentionally contains 12+ validation errors
   - Used for testing linter error detection
   - Demonstrates common mistakes and edge cases

## Using These Schemas

### Running the Linter

```bash
# Lint a single file
terraphim-kg-lint valid-command.md --format text

# Lint with graph validation
terraphim-kg-lint valid-command.md \
  --graph ../../data/rolegraph.json \
  --automata ../../data/automata_index.bin

# Lint entire directory
terraphim-kg-lint . --format json > lint-report.json

# Strict mode (warnings as errors)
terraphim-kg-lint valid-command.md --strict
```

### Expected Outputs

#### valid-command.md
```
✓ valid-command.md
  Schema type: command
  Status: PASSED
  0 errors, 0 warnings
```

#### invalid-command.md
```
✗ invalid-command.md
  Schema type: command
  Status: FAILED
  12 errors, 3 warnings

  Error (line 2): Invalid command name '123-invalid-name'
    Must start with letter and contain only alphanumeric characters, hyphens, and underscores
    Suggestion: Use 'invalid-name' or 'test-invalid-name'

  Error (line 4): Invalid version format 'not-semver'
    Must follow semantic versioning (e.g., '1.0.0')
    Suggestion: Use '1.0.0'

  ... (10 more errors)
```

## Schema Structure

All schemas follow this pattern:

```markdown
---
# YAML frontmatter with schema definition
schema_type: "command|knowledge_graph|thesaurus"
name: "schema-name"
version: "1.0.0"
# ... specific fields for each schema type
---

# Markdown documentation

Human-readable description and examples
```

## Validation Rules

### Command Schemas

- ✓ Name: alphanumeric + hyphens/underscores, starts with letter
- ✓ Version: semver format (major.minor.patch)
- ✓ Execution mode: local|firecracker|hybrid
- ✓ Risk level: low|medium|high|critical
- ✓ Parameter types: string|number|boolean|array|object
- ✓ Permissions: "resource:action" format
- ✓ No required parameters with default values
- ✓ Unique parameter names
- ✓ Resource limits > 0
- ✓ Timeout > 0

### KG Schemas

- ✓ Node types define properties with types
- ✓ Edge types reference valid node types
- ✓ Relationships specify validators
- ✓ Security permissions follow hierarchy
- ✓ Validation rules have severity levels

### Thesaurus Schemas

- ✓ Term structure defines id, nterm, url
- ✓ Automata config specifies match algorithm
- ✓ Synonym groups have canonical terms
- ✓ Normalized terms are lowercase
- ✓ URLs are valid HTTP/HTTPS

## Integration with Terraphim

These schemas integrate with:

- **terraphim_automata**: Builds Aho-Corasick automata from thesaurus
- **terraphim_rolegraph**: Constructs knowledge graphs from schemas
- **terraphim_tui**: Executes commands with validation
- **terraphim_mcp_server**: Exposes tools to AI agents

## Testing the Linter

```bash
# Run linter tests with these examples
cargo test -p terraphim_kg_linter

# Specific test for valid schemas
cargo test -p terraphim_kg_linter test_valid_schemas

# Test error detection
cargo test -p terraphim_kg_linter test_invalid_command_detection

# Integration test with real graph
cargo test -p terraphim_kg_linter test_graph_validation -- --ignored
```

## Contributing

When adding new schema examples:

1. Add YAML frontmatter with complete metadata
2. Include comprehensive markdown documentation
3. Demonstrate specific features or edge cases
4. Add corresponding test case in linter tests
5. Update this README with the new example
