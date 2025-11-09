# LLM Markdown Linter Implementation Plan

## Project: terraphim_kg_linter

**Goal**: Implement a comprehensive markdown linter for Terraphim KG schemas that validates commands, knowledge graphs, and thesaurus definitions for LLM agents.

**Related PR**: [#277 - Code Assistant Implementation](https://github.com/terraphim/terraphim-ai/pull/277)

## Implementation Phases

### Phase 1: Foundation (Week 1)

**Deliverables**: Basic crate structure, parser infrastructure, core types

#### Tasks

1. **Create Crate Structure**
   ```bash
   cargo new --lib crates/terraphim_kg_linter
   ```
   - Set up Cargo.toml with dependencies
   - Create module structure (parser/, validator/, reporter/)
   - Add to workspace Cargo.toml

2. **Define Core Types** (`src/types.rs`)
   - `LinterConfig` struct
   - `SchemaType` enum
   - `LintResult` struct
   - `Diagnostic` struct with severity levels
   - `Location` for error reporting
   - `LintMetadata` for statistics

3. **Implement Basic Parser** (`src/parser/`)
   - `frontmatter.rs`: YAML frontmatter extraction (reuse TUI parser logic)
   - `command.rs`: Command definition parsing
   - `mod.rs`: Parser orchestration
   - Error handling with thiserror

4. **Write Unit Tests**
   - Test YAML parsing
   - Test command definition extraction
   - Test error cases (invalid YAML, missing fields)

**Dependencies**:
```toml
[dependencies]
terraphim_types = { path = "../terraphim_types" }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_yaml = "0.9"
regex = "1.10"
thiserror = "1.0"
```

**Success Criteria**:
- ✓ Crate builds successfully
- ✓ Can parse valid command definitions
- ✓ Returns appropriate errors for invalid YAML
- ✓ All unit tests pass

---

### Phase 2: Command Validation (Week 2)

**Deliverables**: Complete command validation with all rules

#### Tasks

1. **Implement Validators** (`src/validator/`)
   - `command.rs`: All command validation rules
     - Frontmatter validation
     - Parameter validation
     - Name/version format checking
     - Type validation
   - `security.rs`: Permission and risk level validation
     - Permission format checking
     - Risk level validation
     - Execution mode validation
   - `types.rs`: Type system validation
     - Valid Rust type mapping
     - Custom type references

2. **Create Reporter** (`src/reporter/`)
   - `text.rs`: Human-readable output
   - `json.rs`: Machine-readable JSON output
   - `llm.rs`: LLM-friendly format with hints and suggestions

3. **Write Comprehensive Tests**
   - Test all validation rules individually
   - Test with valid-command.md example
   - Test with invalid-command.md (should catch all 12 errors)
   - Edge cases and boundary conditions

**Code Example**:
```rust
// src/validator/command.rs
pub fn validate_command_name(name: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let name_regex = regex::Regex::new(r"^[a-zA-Z][a-zA-Z0-9_-]*$").unwrap();
    if !name_regex.is_match(name) {
        diagnostics.push(Diagnostic {
            severity: Severity::Error,
            rule: "invalid_command_name".to_string(),
            message: format!("Invalid command name '{}'", name),
            suggestion: Some("Must start with letter and contain only alphanumeric characters, hyphens, and underscores".to_string()),
            llm_hint: Some("Command names should follow kebab-case or snake_case conventions (e.g., 'semantic-search' or 'kg_query')".to_string()),
            ..Default::default()
        });
    }

    diagnostics
}
```

**Success Criteria**:
- ✓ All validation rules implemented
- ✓ Catches all errors in invalid-command.md
- ✓ Validates valid-command.md successfully
- ✓ Produces LLM-friendly error messages
- ✓ JSON output is well-structured

---

### Phase 3: Knowledge Graph Integration (Week 3)

**Deliverables**: Integration with terraphim_automata and terraphim_rolegraph

#### Tasks

1. **Add Dependencies**
   ```toml
   terraphim_automata = { path = "../terraphim_automata", features = ["remote-loading"] }
   terraphim_rolegraph = { path = "../terraphim_rolegraph" }
   tokio = { version = "1.0", features = ["full"] }
   ```

2. **Implement Automata Integration** (`src/analyzer/automata.rs`)
   - Load autocomplete index
   - Validate terms against automata
   - Fuzzy suggestion generation
   - Term coverage analysis

3. **Implement Graph Integration** (`src/analyzer/graph.rs`)
   - Load RoleGraph
   - Validate KG requirements
   - Path connectivity validation using `is_all_terms_connected_by_path`
   - Node/edge validation

4. **Extend KgLinter API**
   ```rust
   impl KgLinter {
       pub async fn with_automata(&mut self, index: AutocompleteIndex) -> &mut Self;
       pub async fn with_graph(&mut self, graph: RoleGraph) -> &mut Self;
       pub async fn validate_with_graph(&self, content: &str) -> ValidationReport;
   }
   ```

5. **Write Integration Tests**
   - Test with real thesaurus (use AutomataPath::local_example())
   - Test graph connectivity validation
   - Test fuzzy suggestion generation
   - Test with valid-command.md requiring KG concepts

**Success Criteria**:
- ✓ Can load and use automata index
- ✓ Can load and use RoleGraph
- ✓ Validates KG requirements correctly
- ✓ Detects disconnected concepts
- ✓ Provides fuzzy suggestions for typos

---

### Phase 4: KG Schema & Thesaurus Validation (Week 4)

**Deliverables**: Support for KG schema and thesaurus markdown formats

#### Tasks

1. **Extend Parsers**
   - `parser/schema.rs`: Knowledge graph schema parsing
   - `parser/thesaurus.rs`: Thesaurus schema parsing
   - Auto-detection of schema type from frontmatter

2. **Implement Schema Validators**
   - `validator/schema.rs`: KG schema validation
     - Node type validation
     - Edge type validation
     - Relationship validation
     - Type consistency checking
   - Validate with valid-kg-schema.md example

3. **Implement Thesaurus Validators**
   - Thesaurus structure validation
   - Term normalization checking
   - URL validation
   - Synonym group validation
   - Validate with valid-thesaurus-schema.md example

4. **Graph Integrity Checks**
   - Node uniqueness
   - Edge references
   - Orphan node detection
   - Symmetric edge validation

**Success Criteria**:
- ✓ Can parse all three schema types
- ✓ Validates KG schemas correctly
- ✓ Validates thesaurus schemas correctly
- ✓ Detects graph integrity issues
- ✓ All example schemas validate correctly

---

### Phase 5: CLI Tool & Polish (Week 5)

**Deliverables**: Command-line tool, documentation, comprehensive tests

#### Tasks

1. **Create CLI Binary** (`src/bin/terraphim-kg-lint.rs`)
   - Argument parsing with clap
   - File/directory traversal
   - Output formatting
   - Exit codes

2. **Add CLI Dependencies**
   ```toml
   [dependencies]
   clap = { version = "4.0", features = ["derive"] }
   walkdir = "2.0"
   colored = "2.0"
   ```

3. **Documentation**
   - Complete README.md for crate
   - API documentation (rustdoc)
   - Usage examples
   - Integration guide

4. **Comprehensive Testing**
   - Integration tests with all example schemas
   - CLI tests
   - Performance benchmarks
   - Test coverage analysis

5. **CI/CD Integration**
   - Add to GitHub Actions workflows
   - Add to pre-commit hooks
   - Set up automated testing

**CLI Example**:
```bash
# Install
cargo install --path crates/terraphim_kg_linter

# Use
terraphim-kg-lint examples/kg-linter-schemas/valid-command.md
terraphim-kg-lint examples/kg-linter-schemas/ --format json
terraphim-kg-lint . --strict --graph data/rolegraph.json
```

**Success Criteria**:
- ✓ CLI tool works correctly
- ✓ All documentation complete
- ✓ All tests passing
- ✓ CI/CD integration working
- ✓ Ready for production use

---

## Testing Strategy

### Unit Tests (Per Module)

```rust
// validator/command.rs
#[cfg(test)]
mod tests {
    #[test]
    fn test_valid_command_name() { ... }

    #[test]
    fn test_invalid_command_name() { ... }

    #[test]
    fn test_parameter_validation() { ... }
}
```

### Integration Tests

```rust
// tests/command_validation_tests.rs
#[tokio::test]
async fn test_valid_command_passes() {
    let linter = KgLinter::new(LinterConfig::default());
    let content = include_str!("../examples/kg-linter-schemas/valid-command.md");
    let result = linter.lint_content(content, SchemaType::Command);
    assert!(result.is_valid);
    assert_eq!(result.diagnostics.len(), 0);
}

#[tokio::test]
async fn test_invalid_command_catches_errors() {
    let linter = KgLinter::new(LinterConfig::default());
    let content = include_str!("../examples/kg-linter-schemas/invalid-command.md");
    let result = linter.lint_content(content, SchemaType::Command);
    assert!(!result.is_valid);
    assert!(result.diagnostics.len() >= 12);
}

#[tokio::test]
async fn test_with_real_graph() {
    let thesaurus_path = AutomataPath::local_example();
    let thesaurus = load_thesaurus(&thesaurus_path).await.unwrap();
    let graph = RoleGraph::new(RoleName::new("test"), thesaurus).await.unwrap();

    let mut linter = KgLinter::new(LinterConfig::default());
    linter.with_graph(graph);

    let content = include_str!("../examples/kg-linter-schemas/valid-command.md");
    let report = linter.validate_with_graph(content).await.unwrap();

    assert!(report.graph_analysis.edge_connectivity);
}
```

### End-to-End Tests

```bash
#!/bin/bash
# tests/e2e/test_cli.sh

set -e

# Test valid command
terraphim-kg-lint examples/kg-linter-schemas/valid-command.md
[ $? -eq 0 ] || exit 1

# Test invalid command (should fail)
terraphim-kg-lint examples/kg-linter-schemas/invalid-command.md
[ $? -eq 1 ] || exit 1

# Test directory
terraphim-kg-lint examples/kg-linter-schemas/ --format json > /tmp/lint-report.json
grep -q '"is_valid": true' /tmp/lint-report.json

echo "All E2E tests passed!"
```

---

## Performance Targets

| Operation | Target | Notes |
|-----------|--------|-------|
| Parse command | < 1ms | Simple YAML parsing |
| Validate command | < 10ms | Without graph integration |
| Validate with automata | < 50ms | Includes index lookup |
| Validate with graph | < 100ms | Includes connectivity check |
| Lint directory (100 files) | < 5s | Parallel processing |

---

## Code Quality Checklist

- [ ] All public APIs documented with rustdoc
- [ ] Error messages are clear and actionable
- [ ] LLM hints provide context and suggestions
- [ ] Code follows Rust naming conventions
- [ ] No unwrap() in library code (use Result)
- [ ] Async/await used correctly with tokio
- [ ] Tests cover success and error cases
- [ ] Examples compile and run correctly
- [ ] Pre-commit hooks pass
- [ ] clippy warnings resolved
- [ ] Code formatted with rustfmt

---

## Integration Points

### 1. TUI Command System

```rust
// crates/terraphim_tui/src/commands/validator.rs

use terraphim_kg_linter::{KgLinter, SchemaType};

pub async fn validate_command_file(path: &Path) -> Result<()> {
    let linter = KgLinter::new(LinterConfig::default());
    let result = linter.lint_file(path).await?;

    if !result.is_valid {
        for diagnostic in result.diagnostics {
            eprintln!("{}: {}", diagnostic.severity, diagnostic.message);
        }
        return Err(anyhow!("Command validation failed"));
    }

    Ok(())
}
```

### 2. Pre-commit Hook

```bash
#!/bin/bash
# .git/hooks/pre-commit

# Find all markdown command files
for file in $(git diff --cached --name-only --diff-filter=ACM | grep -E '\.md$'); do
    # Skip if not a command file
    grep -q "^---" "$file" || continue

    # Run linter
    if ! terraphim-kg-lint "$file" --strict; then
        echo "Error: $file failed validation"
        exit 1
    fi
done
```

### 3. GitHub Actions

```yaml
# .github/workflows/lint-schemas.yml
name: Lint KG Schemas

on: [push, pull_request]

jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
      - name: Build linter
        run: cargo build -p terraphim_kg_linter --release
      - name: Lint schemas
        run: |
          ./target/release/terraphim-kg-lint \
            examples/kg-linter-schemas/ \
            --format json \
            --strict
```

---

## Future Enhancements (Post v1.0)

1. **LSP Server**
   - Real-time validation in editors
   - Autocomplete for YAML fields
   - Hover documentation

2. **Auto-fix**
   - Automatic formatting
   - Add missing required fields
   - Fix common typos

3. **Graph Visualization**
   - Visual representation of disconnected concepts
   - Interactive graph explorer
   - Mermaid diagram generation

4. **Plugin System**
   - Custom validation rules
   - Domain-specific validators
   - Third-party integrations

5. **Schema Evolution**
   - Version migration tools
   - Breaking change detection
   - Compatibility checking

6. **Web UI**
   - Browser-based linter
   - Visual schema editor
   - Collaborative validation

---

## Dependencies on Other Work

- **PR #277**: Leverages security model and validation pipeline concepts
- **terraphim_automata**: Uses autocomplete and fuzzy search APIs
- **terraphim_rolegraph**: Uses graph connectivity validation
- **terraphim_tui**: Reuses markdown parser patterns

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Code coverage | > 80% | `cargo tarpaulin` |
| Documentation coverage | 100% | `cargo doc` |
| Example schemas validated | 100% | Integration tests |
| Performance targets | 100% met | Benchmarks |
| CI/CD passing | All tests | GitHub Actions |

---

## Development Environment Setup

```bash
# Clone repository
git clone https://github.com/terraphim/terraphim-ai
cd terraphim-ai

# Create feature branch
git checkout -b feature/kg-linter

# Set up development dependencies
cargo build --workspace

# Run existing tests to ensure setup is correct
cargo test --workspace

# Create linter crate
cargo new --lib crates/terraphim_kg_linter

# Start development
cd crates/terraphim_kg_linter
cargo watch -x test
```

---

## Milestone Tracking

### Milestone 1: Foundation (Week 1)
- [ ] Crate structure created
- [ ] Core types defined
- [ ] Basic parser implemented
- [ ] Unit tests passing

### Milestone 2: Command Validation (Week 2)
- [ ] All validation rules implemented
- [ ] Reporter formats working
- [ ] Integration tests passing
- [ ] Example schemas validated

### Milestone 3: KG Integration (Week 3)
- [ ] Automata integration complete
- [ ] Graph validation working
- [ ] Path connectivity validated
- [ ] Integration tests with real data

### Milestone 4: Schema Validation (Week 4)
- [ ] KG schema parsing
- [ ] Thesaurus parsing
- [ ] Graph integrity checks
- [ ] All schema types supported

### Milestone 5: Production Ready (Week 5)
- [ ] CLI tool complete
- [ ] Documentation complete
- [ ] CI/CD integration
- [ ] Performance targets met
- [ ] Ready for v1.0 release

---

## Questions & Decisions

### Open Questions
1. Should we support custom validation plugins?
2. What's the priority for LSP integration?
3. Should we support TOML frontmatter in addition to YAML?
4. How to handle schema versioning and migrations?

### Decisions Made
1. ✓ Use YAML for frontmatter (consistent with existing TUI)
2. ✓ Support async/await for graph operations
3. ✓ LLM-friendly output is a first-class feature
4. ✓ Leverage existing terraphim_automata and terraphim_rolegraph
5. ✓ Start with three schema types: command, kg, thesaurus

---

## Resources

- [Design Document](./LLM_MARKDOWN_LINTER_DESIGN.md)
- [Example Schemas](../examples/kg-linter-schemas/)
- [PR #277](https://github.com/terraphim/terraphim-ai/pull/277)
- [Terraphim Automata](../crates/terraphim_automata/)
- [Terraphim RoleGraph](../crates/terraphim_rolegraph/)
- [TUI Command System](../crates/terraphim_tui/src/commands/)
