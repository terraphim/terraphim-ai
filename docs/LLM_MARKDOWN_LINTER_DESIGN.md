# LLM Markdown Linter Design for Terraphim KG Schemas

## Overview

This document specifies the design for an LLM-focused markdown linter that validates markdown-based Terraphim Knowledge Graph schemas. The linter provides AI agents with predefined commands, data type definitions in markdown-like KG structures, and enforces security permissions.

## Goals

1. **Command Validation**: Validate markdown files containing AI agent command definitions
2. **Schema Validation**: Ensure KG schema definitions (nodes, edges, concepts) are well-formed
3. **Security Enforcement**: Validate permissions, risk levels, and execution modes
4. **Type Safety**: Check data type definitions and relationships
5. **Graph Integrity**: Validate graph connectivity and term relationships
6. **LLM-Friendly**: Provide clear, actionable error messages for AI agents

## Architecture

### Core Components

```
┌─────────────────────────────────────────────────────────────┐
│                    LLM Markdown Linter                      │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐    │
│  │   Parser     │  │  Validator   │  │   Reporter   │    │
│  │   Layer      │→ │    Layer     │→ │    Layer     │    │
│  └──────────────┘  └──────────────┘  └──────────────┘    │
│         │                  │                  │            │
│         ↓                  ↓                  ↓            │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐    │
│  │ YAML Front   │  │ KG Schema    │  │ JSON/Text    │    │
│  │ matter       │  │ Checker      │  │ Output       │    │
│  └──────────────┘  └──────────────┘  └──────────────┘    │
│         │                  │                              │
│         ↓                  ↓                              │
│  ┌──────────────┐  ┌──────────────┐                      │
│  │ Command Def  │  │ Automata     │                      │
│  │ Validator    │  │ Integration  │                      │
│  └──────────────┘  └──────────────┘                      │
│                           │                              │
│                           ↓                              │
│                   ┌──────────────┐                      │
│                   │ RoleGraph    │                      │
│                   │ Validation   │                      │
│                   └──────────────┘                      │
└─────────────────────────────────────────────────────────────┘
```

## Markdown Schema Format

### 1. Command Definition Schema

Commands are defined in markdown files with YAML frontmatter:

```markdown
---
name: "semantic-search"
description: "Execute semantic search across knowledge graph"
version: "1.0.0"
execution_mode: "hybrid"
risk_level: "low"
category: "search"

parameters:
  - name: "query"
    type: "string"
    required: true
    description: "Search query text"
    validation:
      min_length: 1
      max_length: 500
      pattern: "^[a-zA-Z0-9\\s\\-_]+$"

  - name: "limit"
    type: "number"
    required: false
    default_value: 10
    validation:
      min: 1
      max: 100

permissions:
  - "kg:read"
  - "search:execute"

knowledge_graph_required:
  - "search algorithms"
  - "knowledge graph"
  - "semantic matching"

resource_limits:
  max_memory_mb: 512
  max_cpu_time: 30
  network_access: false

timeout: 60
---

# Semantic Search Command

This command executes semantic search across the knowledge graph using
the Terraphim automata and graph embedding systems.

## Usage

```bash
/semantic-search query="machine learning" limit=20
```

## Implementation Details

The command leverages:
- Aho-Corasick automata for fast term matching
- Graph embeddings for semantic similarity
- BM25 ranking for relevance scoring
```

### 2. Knowledge Graph Schema

KG schemas define nodes, edges, and relationships:

```markdown
---
schema_type: "knowledge_graph"
schema_name: "engineering_concepts"
version: "1.0.0"
namespace: "terraphim.kg.engineering"

node_types:
  - name: "Concept"
    properties:
      - name: "id"
        type: "u64"
        required: true
        unique: true
      - name: "value"
        type: "NormalizedTermValue"
        required: true
      - name: "rank"
        type: "u64"
        default: 0
      - name: "metadata"
        type: "object"
        required: false

  - name: "Document"
    properties:
      - name: "id"
        type: "string"
        required: true
        unique: true
      - name: "url"
        type: "string"
        required: true
        validation:
          pattern: "^https?://"
      - name: "body"
        type: "string"
        required: true

edge_types:
  - name: "RelatedTo"
    from: "Concept"
    to: "Concept"
    properties:
      - name: "rank"
        type: "u64"
        default: 1
      - name: "doc_hash"
        type: "HashMap<String, u64>"

relationships:
  - type: "path_connectivity"
    description: "All matched terms must be connected by a single path"
    validator: "is_all_terms_connected_by_path"

  - type: "bidirectional"
    description: "Edges are bidirectional in the graph"
    constraint: "symmetric"

security:
  read_permissions:
    - "kg:read"
    - "concept:view"
  write_permissions:
    - "kg:write"
    - "concept:modify"
  delete_permissions:
    - "kg:admin"
    - "concept:delete"
---

# Engineering Concepts Knowledge Graph

This knowledge graph represents engineering concepts and their relationships,
optimized for semantic search and autocomplete functionality.

## Graph Structure

- **Nodes**: Represent individual concepts with normalized terms
- **Edges**: Represent relationships between concepts with co-occurrence counts
- **Documents**: Source documents that contain the concepts

## Validation Rules

1. **Node Uniqueness**: Each concept must have a unique ID
2. **Edge Connectivity**: Edges must reference valid node IDs
3. **Path Connectivity**: Terms in queries should form connected paths
4. **Normalized Terms**: All terms must be lowercase and trimmed
```

### 3. Thesaurus Schema

Thesaurus definitions for term normalization:

```markdown
---
schema_type: "thesaurus"
thesaurus_name: "Default"
version: "1.0.0"
case_sensitive: false
match_mode: "leftmost_longest"

term_structure:
  - name: "id"
    type: "u64"
    description: "Unique identifier for the normalized term"
    required: true

  - name: "nterm"
    type: "string"
    description: "Normalized term value (lowercase, trimmed)"
    required: true
    validation:
      pattern: "^[a-z0-9\\s\\-_]+$"

  - name: "url"
    type: "string"
    description: "Optional URL for the concept"
    required: false
    validation:
      pattern: "^https?://"

automata_config:
  match_kind: "LeftmostLongest"
  ascii_case_insensitive: true
  enable_fuzzy: true
  fuzzy_algorithm: "jaro_winkler"
  fuzzy_threshold: 0.85

validation_rules:
  - rule: "unique_ids"
    description: "All term IDs must be unique"
    severity: "error"

  - rule: "normalized_format"
    description: "All nterm values must be lowercase"
    severity: "error"

  - rule: "valid_urls"
    description: "URLs must be valid HTTP/HTTPS"
    severity: "warning"
---

# Default Thesaurus

This thesaurus provides term normalization and synonym mapping for
the Terraphim knowledge graph system.

## Format

```json
{
  "name": "Default",
  "data": {
    "term": {
      "id": 1,
      "nterm": "normalized term",
      "url": "https://example.com/term"
    }
  }
}
```
```

## Validation Rules

### 1. Frontmatter Validation

| Rule | Severity | Description |
|------|----------|-------------|
| Valid YAML | Error | Frontmatter must be valid YAML |
| Required Fields | Error | `name`, `description` must be present |
| Valid Execution Mode | Error | Must be `local`, `firecracker`, or `hybrid` |
| Valid Risk Level | Error | Must be `low`, `medium`, `high`, or `critical` |
| Parameter Types | Error | Must be `string`, `number`, `boolean`, `array`, `object` |
| Command Name Format | Error | Must start with letter, alphanumeric + `-_` only |
| Version Format | Warning | Should follow semver (e.g., "1.0.0") |
| Unique Parameters | Error | Parameter names must be unique |
| Required Without Default | Error | Required parameters cannot have default values |

### 2. Knowledge Graph Validation

| Rule | Severity | Description |
|------|----------|-------------|
| Node ID Uniqueness | Error | All node IDs must be unique within graph |
| Edge References | Error | Edge IDs must reference valid nodes |
| Type Consistency | Error | Property types must match schema |
| Path Connectivity | Warning | Recommended: matched terms should form paths |
| Normalized Terms | Error | All terms must be lowercase, trimmed |
| Symmetric Edges | Error | Bidirectional edges must have reverse edges |
| Orphan Nodes | Warning | Nodes should have at least one edge |
| Document References | Error | Document IDs in edges must exist |

### 3. Security Validation

| Rule | Severity | Description |
|------|----------|-------------|
| Permission Format | Error | Permissions must follow `resource:action` format |
| Valid Permissions | Error | Permissions must be in allowed list |
| Risk/Mode Match | Warning | High-risk commands should use Firecracker mode |
| Resource Limits | Error | Limits must be positive integers |
| Network Access | Warning | Network access requires justification |
| KG Concepts Exist | Error | Required KG concepts must exist in graph |
| Permission Hierarchy | Error | Write requires read, delete requires write |

### 4. Type System Validation

| Rule | Severity | Description |
|------|----------|-------------|
| Rust Type Mapping | Error | Types must map to valid Rust types |
| Generic Constraints | Error | Generic types must specify constraints |
| Option/Result Usage | Warning | Nullable fields should use Option<T> |
| Collection Types | Error | Collections must specify element types |
| Custom Type Refs | Error | Custom types must be defined in schema |
| Enum Values | Error | Enum values must be explicitly listed |

## Implementation Plan

### Crate Structure

```
crates/terraphim_kg_linter/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Public API
│   ├── parser/
│   │   ├── mod.rs          # Parser orchestration
│   │   ├── frontmatter.rs  # YAML frontmatter parser
│   │   ├── command.rs      # Command definition parser
│   │   ├── schema.rs       # KG schema parser
│   │   └── thesaurus.rs    # Thesaurus schema parser
│   ├── validator/
│   │   ├── mod.rs          # Validation orchestration
│   │   ├── command.rs      # Command validation rules
│   │   ├── schema.rs       # Schema validation rules
│   │   ├── security.rs     # Security validation rules
│   │   ├── graph.rs        # Graph integrity validation
│   │   └── types.rs        # Type system validation
│   ├── analyzer/
│   │   ├── mod.rs          # Analysis engine
│   │   ├── automata.rs     # Automata integration
│   │   ├── graph.rs        # RoleGraph integration
│   │   └── embeddings.rs   # Graph embeddings analysis
│   ├── reporter/
│   │   ├── mod.rs          # Report generation
│   │   ├── json.rs         # JSON output
│   │   ├── text.rs         # Human-readable output
│   │   └── llm.rs          # LLM-friendly output
│   ├── types.rs            # Type definitions
│   └── error.rs            # Error types
├── tests/
│   ├── command_validation_tests.rs
│   ├── schema_validation_tests.rs
│   ├── security_tests.rs
│   └── integration_tests.rs
└── examples/
    ├── valid_command.md
    ├── valid_schema.md
    ├── invalid_examples.md
    └── linter_usage.rs
```

### Key Dependencies

```toml
[dependencies]
# Existing Terraphim crates
terraphim_types = { path = "../terraphim_types" }
terraphim_automata = { path = "../terraphim_automata" }
terraphim_rolegraph = { path = "../terraphim_rolegraph" }

# Parsing and validation
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_yaml = "0.9"
regex = "1.10"

# Error handling
thiserror = "1.0"

# Async support
tokio = { version = "1.0", features = ["full"] }

# Logging
tracing = "0.1"
```

### Public API

```rust
// lib.rs

/// Main linter interface
pub struct KgLinter {
    config: LinterConfig,
    automata_cache: Option<AutocompleteIndex>,
    graph_cache: Option<RoleGraph>,
}

impl KgLinter {
    /// Create a new linter with configuration
    pub fn new(config: LinterConfig) -> Self;

    /// Lint a single markdown file
    pub async fn lint_file(&self, path: &Path) -> LintResult;

    /// Lint all markdown files in a directory
    pub async fn lint_directory(&self, path: &Path) -> Vec<LintResult>;

    /// Lint markdown content directly
    pub fn lint_content(&self, content: &str, schema_type: SchemaType) -> LintResult;

    /// Validate against knowledge graph
    pub async fn validate_with_graph(
        &self,
        content: &str,
        graph: &RoleGraph
    ) -> ValidationReport;
}

/// Linter configuration
pub struct LinterConfig {
    pub strict_mode: bool,
    pub enable_graph_validation: bool,
    pub enable_automata_validation: bool,
    pub max_errors: Option<usize>,
    pub severity_threshold: Severity,
    pub allowed_permissions: Vec<String>,
}

/// Schema type discriminator
pub enum SchemaType {
    Command,
    KnowledgeGraph,
    Thesaurus,
    Auto, // Auto-detect from frontmatter
}

/// Lint result
pub struct LintResult {
    pub file_path: Option<PathBuf>,
    pub schema_type: SchemaType,
    pub diagnostics: Vec<Diagnostic>,
    pub is_valid: bool,
    pub metadata: LintMetadata,
}

/// Individual diagnostic
pub struct Diagnostic {
    pub severity: Severity,
    pub rule: String,
    pub message: String,
    pub location: Location,
    pub suggestion: Option<String>,
    pub llm_hint: Option<String>,
}

/// Diagnostic severity
pub enum Severity {
    Error,
    Warning,
    Info,
    Hint,
}

/// Location in file
pub struct Location {
    pub line: usize,
    pub column: usize,
    pub span: Option<(usize, usize)>,
}

/// Validation report with graph analysis
pub struct ValidationReport {
    pub lint_result: LintResult,
    pub graph_analysis: GraphAnalysis,
    pub automata_analysis: AutomataAnalysis,
}

/// Graph analysis results
pub struct GraphAnalysis {
    pub node_coverage: f64,
    pub edge_connectivity: bool,
    pub orphan_nodes: Vec<u64>,
    pub missing_concepts: Vec<String>,
    pub path_analysis: Vec<PathResult>,
}

/// Automata analysis results
pub struct AutomataAnalysis {
    pub term_matches: Vec<TermMatch>,
    pub fuzzy_suggestions: Vec<FuzzySuggestion>,
    pub coverage_percentage: f64,
}
```

## Validation Algorithms

### 1. Command Validation Pipeline

```rust
async fn validate_command(
    command: &CommandDefinition,
    graph: Option<&RoleGraph>
) -> Result<Vec<Diagnostic>, LinterError> {
    let mut diagnostics = Vec::new();

    // 1. Frontmatter validation
    diagnostics.extend(validate_frontmatter(command)?);

    // 2. Parameter validation
    diagnostics.extend(validate_parameters(&command.parameters)?);

    // 3. Security validation
    diagnostics.extend(validate_security(command)?);

    // 4. Knowledge graph validation (if graph provided)
    if let Some(graph) = graph {
        diagnostics.extend(validate_kg_requirements(command, graph).await?);
    }

    // 5. Execution mode validation
    diagnostics.extend(validate_execution_mode(command)?);

    Ok(diagnostics)
}
```

### 2. Knowledge Graph Schema Validation

```rust
async fn validate_kg_schema(
    schema: &KgSchema,
    automata: Option<&AutocompleteIndex>
) -> Result<Vec<Diagnostic>, LinterError> {
    let mut diagnostics = Vec::new();

    // 1. Node type validation
    diagnostics.extend(validate_node_types(&schema.node_types)?);

    // 2. Edge type validation
    diagnostics.extend(validate_edge_types(&schema.edge_types)?);

    // 3. Relationship validation
    diagnostics.extend(validate_relationships(&schema.relationships)?);

    // 4. Type system validation
    diagnostics.extend(validate_type_system(schema)?);

    // 5. Automata integration (if provided)
    if let Some(automata) = automata {
        diagnostics.extend(validate_with_automata(schema, automata).await?);
    }

    Ok(diagnostics)
}
```

### 3. Graph Connectivity Validation

Leverages existing `is_all_terms_connected_by_path` from terraphim_rolegraph:

```rust
fn validate_path_connectivity(
    required_concepts: &[String],
    graph: &RoleGraph
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Create a query text from required concepts
    let query = required_concepts.join(" ");

    // Check if all terms are connected by a path
    if !graph.is_all_terms_connected_by_path(&query) {
        diagnostics.push(Diagnostic {
            severity: Severity::Warning,
            rule: "path_connectivity".to_string(),
            message: format!(
                "Required concepts {:?} are not connected by a single path in the knowledge graph",
                required_concepts
            ),
            location: Location::default(),
            suggestion: Some(
                "Consider adding intermediate concepts to connect these terms, \
                 or verify that all concepts exist in the graph".to_string()
            ),
            llm_hint: Some(
                "The knowledge graph analysis shows these concepts are disconnected. \
                 This may indicate missing relationships or orphaned concepts.".to_string()
            ),
        });
    }

    diagnostics
}
```

### 4. Automata-Based Term Validation

```rust
async fn validate_with_automata(
    schema: &KgSchema,
    automata: &AutocompleteIndex
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Extract all term references from schema
    let terms = extract_terms_from_schema(schema);

    for term in terms {
        // Use autocomplete to find matches
        let matches = autocomplete_search(automata, &term, 5);

        if matches.is_empty() {
            diagnostics.push(Diagnostic {
                severity: Severity::Warning,
                rule: "term_not_in_automata".to_string(),
                message: format!("Term '{}' not found in automata index", term),
                location: Location::from_term(&term),
                suggestion: Some(
                    "Add this term to the thesaurus or check for typos".to_string()
                ),
                llm_hint: Some(format!(
                    "This term is not recognized in the current knowledge base. \
                     Similar terms: {:?}",
                    fuzzy_autocomplete_search(automata, &term, 3)
                )),
            });
        }
    }

    diagnostics
}
```

## LLM-Friendly Output Format

The linter provides specialized output for LLM consumption:

```json
{
  "schema_type": "command",
  "file_path": "commands/semantic-search.md",
  "validation_status": "failed",
  "summary": {
    "total_diagnostics": 5,
    "errors": 2,
    "warnings": 2,
    "hints": 1
  },
  "diagnostics": [
    {
      "severity": "error",
      "rule": "missing_parameter",
      "message": "Required parameter 'query' is missing type definition",
      "location": {
        "line": 12,
        "column": 5,
        "snippet": "  - name: \"query\""
      },
      "suggestion": "Add 'type: \"string\"' to the parameter definition",
      "llm_hint": "Parameters must specify their data type. Common types are: string, number, boolean, array, object. For this query parameter, 'string' is most appropriate.",
      "fix": {
        "type": "add_field",
        "line": 12,
        "field": "type",
        "value": "string"
      }
    }
  ],
  "graph_analysis": {
    "concepts_validated": true,
    "required_concepts": ["semantic search", "knowledge graph"],
    "concepts_found": ["semantic search"],
    "concepts_missing": ["knowledge graph"],
    "connectivity": "disconnected",
    "suggestions": [
      "Add 'knowledge graph' to the thesaurus",
      "Create edges connecting 'semantic search' to 'knowledge graph'"
    ]
  },
  "automata_analysis": {
    "terms_checked": 15,
    "terms_matched": 12,
    "fuzzy_suggestions": [
      {
        "term": "knowlege graph",
        "suggestion": "knowledge graph",
        "confidence": 0.92
      }
    ]
  }
}
```

## Integration with Existing Components

### 1. terraphim_automata Integration

```rust
use terraphim_automata::{
    autocomplete_search,
    fuzzy_autocomplete_search,
    AutocompleteIndex,
};

impl KgLinter {
    pub async fn with_automata(&mut self, index: AutocompleteIndex) -> &mut Self {
        self.automata_cache = Some(index);
        self
    }

    fn validate_terms_with_automata(&self, terms: &[String]) -> Vec<Diagnostic> {
        if let Some(ref automata) = self.automata_cache {
            terms.iter().filter_map(|term| {
                let matches = autocomplete_search(automata, term, 1);
                if matches.is_empty() {
                    Some(Diagnostic::term_not_found(term, automata))
                } else {
                    None
                }
            }).collect()
        } else {
            vec![]
        }
    }
}
```

### 2. terraphim_rolegraph Integration

```rust
use terraphim_rolegraph::{RoleGraph, RoleGraphSync};
use terraphim_types::{Node, Edge, NormalizedTermValue};

impl KgLinter {
    pub async fn with_graph(&mut self, graph: RoleGraph) -> &mut Self {
        self.graph_cache = Some(graph);
        self
    }

    fn validate_graph_connectivity(
        &self,
        required_concepts: &[String]
    ) -> Vec<Diagnostic> {
        if let Some(ref graph) = self.graph_cache {
            let query = required_concepts.join(" ");
            if !graph.is_all_terms_connected_by_path(&query) {
                vec![Diagnostic::disconnected_concepts(required_concepts)]
            } else {
                vec![]
            }
        } else {
            vec![]
        }
    }
}
```

## CLI Tool

```rust
// bin/terraphim-kg-lint.rs

use clap::Parser;
use terraphim_kg_linter::{KgLinter, LinterConfig, SchemaType};

#[derive(Parser)]
#[command(name = "terraphim-kg-lint")]
#[command(about = "Lint Terraphim KG markdown schemas")]
struct Cli {
    /// Path to markdown file or directory
    path: PathBuf,

    /// Schema type (command, kg, thesaurus, auto)
    #[arg(short, long, default_value = "auto")]
    schema_type: String,

    /// Enable strict mode
    #[arg(long)]
    strict: bool,

    /// Path to automata index for validation
    #[arg(long)]
    automata: Option<PathBuf>,

    /// Path to role graph for validation
    #[arg(long)]
    graph: Option<PathBuf>,

    /// Output format (json, text, llm)
    #[arg(short, long, default_value = "text")]
    format: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let config = LinterConfig {
        strict_mode: cli.strict,
        enable_graph_validation: cli.graph.is_some(),
        enable_automata_validation: cli.automata.is_some(),
        ..Default::default()
    };

    let mut linter = KgLinter::new(config);

    // Load automata if provided
    if let Some(automata_path) = cli.automata {
        let index = load_autocomplete_index(&automata_path).await?;
        linter.with_automata(index);
    }

    // Load graph if provided
    if let Some(graph_path) = cli.graph {
        let graph = load_rolegraph(&graph_path).await?;
        linter.with_graph(graph);
    }

    // Lint the file(s)
    let results = if cli.path.is_dir() {
        linter.lint_directory(&cli.path).await
    } else {
        vec![linter.lint_file(&cli.path).await?]
    };

    // Output results
    match cli.format.as_str() {
        "json" => println!("{}", serde_json::to_string_pretty(&results)?),
        "llm" => output_llm_format(&results),
        _ => output_text_format(&results),
    }

    // Exit with error code if any errors found
    let has_errors = results.iter().any(|r| !r.is_valid);
    std::process::exit(if has_errors { 1 } else { 0 });
}
```

## Usage Examples

### 1. Lint a Single Command File

```bash
terraphim-kg-lint commands/semantic-search.md \
  --schema-type command \
  --graph ./data/engineering_graph.json \
  --format llm
```

### 2. Lint All KG Schemas in Directory

```bash
terraphim-kg-lint schemas/ \
  --strict \
  --automata ./data/automata_index.bin \
  --graph ./data/rolegraph.json \
  --format json > lint-report.json
```

### 3. Programmatic Usage

```rust
use terraphim_kg_linter::{KgLinter, LinterConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = LinterConfig::default();
    let linter = KgLinter::new(config);

    let markdown = r#"
---
name: "test-command"
description: "Test command"
execution_mode: "local"
---

# Test Command
    "#;

    let result = linter.lint_content(markdown, SchemaType::Command);

    for diagnostic in result.diagnostics {
        println!("{}: {}", diagnostic.severity, diagnostic.message);
        if let Some(hint) = diagnostic.llm_hint {
            println!("  Hint: {}", hint);
        }
    }

    Ok(())
}
```

## Testing Strategy

### 1. Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_command_definition() {
        let markdown = include_str!("../examples/valid_command.md");
        let linter = KgLinter::new(LinterConfig::default());
        let result = linter.lint_content(markdown, SchemaType::Command);
        assert!(result.is_valid);
    }

    #[test]
    fn test_invalid_parameter_type() {
        let markdown = r#"
---
name: "test"
parameters:
  - name: "param"
    type: "invalid_type"
---
        "#;
        let linter = KgLinter::new(LinterConfig::default());
        let result = linter.lint_content(markdown, SchemaType::Command);
        assert!(!result.is_valid);
        assert!(result.diagnostics.iter().any(|d| d.rule == "invalid_parameter_type"));
    }
}
```

### 2. Integration Tests

```rust
#[tokio::test]
async fn test_with_real_graph() {
    let thesaurus_path = AutomataPath::local_example();
    let thesaurus = load_thesaurus(&thesaurus_path).await.unwrap();
    let graph = RoleGraph::new(RoleName::new("test"), thesaurus).await.unwrap();

    let mut linter = KgLinter::new(LinterConfig::default());
    linter.with_graph(graph);

    let markdown = include_str!("../test-fixtures/command_with_kg_requirements.md");
    let result = linter.lint_content(markdown, SchemaType::Command);

    // Should validate KG requirements
    assert!(result.metadata.graph_validated);
}
```

## Extension Points

### 1. Custom Validation Rules

```rust
pub trait ValidationRule: Send + Sync {
    fn name(&self) -> &str;
    fn validate(&self, schema: &ParsedSchema) -> Vec<Diagnostic>;
}

impl KgLinter {
    pub fn add_custom_rule(&mut self, rule: Box<dyn ValidationRule>) {
        self.custom_rules.push(rule);
    }
}
```

### 2. Custom Reporters

```rust
pub trait Reporter: Send + Sync {
    fn report(&self, results: &[LintResult]) -> String;
}

impl KgLinter {
    pub fn with_reporter(&mut self, reporter: Box<dyn Reporter>) -> &mut Self {
        self.reporter = Some(reporter);
        self
    }
}
```

## Future Enhancements

1. **LSP Integration**: Language Server Protocol for IDE integration
2. **Auto-fix**: Automatic fixes for common issues
3. **Graph Visualization**: Visual representation of connectivity issues
4. **Benchmark Suite**: Performance testing with large schemas
5. **Plugin System**: Dynamic loading of custom validators
6. **CI/CD Integration**: GitHub Actions, GitLab CI pipelines
7. **Schema Evolution**: Validate schema migrations
8. **Embedding Analysis**: Semantic similarity checks using graph embeddings

## Related Work

This linter builds upon and integrates with:

- **PR #277**: Code Assistant with validation pipeline and security model
- **terraphim_automata**: Fast term matching and autocomplete
- **terraphim_rolegraph**: Knowledge graph structure and path connectivity
- **terraphim_tui**: Markdown command parser and execution system
- **Graph Embeddings**: Semantic relationship validation

## References

- [Terraphim KG System Documentation](../docs/src/kg/knowledge-graph-system.md)
- [TUI Command System](../crates/terraphim_tui/commands/README.md)
- [Automata Documentation](../crates/terraphim_automata/README.md)
- [PR #277: Code Assistant Implementation](https://github.com/terraphim/terraphim-ai/pull/277)
