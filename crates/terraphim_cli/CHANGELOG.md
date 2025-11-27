# Changelog

All notable changes to `terraphim-cli` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.0.0] - 2025-01-25

### Added

#### Core Commands
- **search**: Search documents with JSON output, role selection, and limit
- **config**: Display current configuration with selected role and available roles
- **roles**: List all available roles in JSON format
- **graph**: Show top K concepts from knowledge graph
- **replace**: Replace matched terms with links (markdown/html/wiki/plain formats)
- **find**: Find all matched terms in text with positions and normalized values
- **thesaurus**: Display knowledge graph terms with IDs, URLs, and normalization
- **completions**: Generate shell completions for bash, zsh, fish, powershell

#### Output Formats
- **JSON**: Machine-readable output (default)
- **JSON Pretty**: Human-readable formatted JSON
- **Text**: Simple text output (basic)

#### Global Options
- `--format`: Choose output format (json, json-pretty, text)
- `--quiet`: Suppress non-JSON output for pure machine processing
- Exit codes: 0 for success, 1 for errors

#### Features
- **Non-Interactive**: Single command execution for scripts and automation
- **Pipe-Friendly**: Designed to work with Unix pipes and tools like `jq`
- **Shell Integration**: Auto-completion support for major shells
- **Error Handling**: Proper error messages in JSON format with details
- **Offline Operation**: Works with embedded configuration (no network required)

#### JSON Output Structures

**Search Results:**
```json
{
  "query": "search term",
  "role": "role name",
  "results": [
    {
      "id": "doc_id",
      "title": "Document Title",
      "url": "https://example.com",
      "rank": 0.95
    }
  ],
  "count": 1
}
```

**Configuration:**
```json
{
  "selected_role": "Default",
  "roles": ["Default", "Engineer"]
}
```

**Graph Concepts:**
```json
{
  "role": "Engineer",
  "top_k": 10,
  "concepts": ["concept1", "concept2", ...]
}
```

**Replace Result:**
```json
{
  "original": "text",
  "replaced": "linked text",
  "format": "markdown"
}
```

**Find Matches:**
```json
{
  "text": "input text",
  "matches": [
    {
      "term": "matched",
      "position": [0, 7],
      "normalized": "matched term"
    }
  ],
  "count": 1
}
```

**Thesaurus:**
```json
{
  "role": "Engineer",
  "name": "thesaurus_name",
  "terms": [
    {
      "id": 1,
      "term": "rust",
      "normalized": "rust programming language",
      "url": "https://rust-lang.org"
    }
  ],
  "total_count": 100,
  "shown_count": 50
}
```

**Error:**
```json
{
  "error": "Error message",
  "details": "Detailed error information"
}
```

#### Shell Completions

Generate completions for all major shells:
```bash
terraphim-cli completions bash > terraphim-cli.bash
terraphim-cli completions zsh > _terraphim-cli
terraphim-cli completions fish > terraphim-cli.fish
terraphim-cli completions powershell > _terraphim-cli.ps1
```

#### Use Cases

1. **CI/CD Pipelines**: Validate knowledge graph content in automated builds
2. **Shell Scripts**: Automate document searches and link generation
3. **Data Processing**: Batch process text with knowledge graph enrichment
4. **API Integration**: JSON output integrates with REST APIs and microservices
5. **Report Generation**: Generate reports with semantic search results

#### Dependencies

- `clap 4.5`: Command-line argument parsing with derive macros
- `clap_complete 4.5`: Shell completion generation
- Core terraphim crates: service, config, types, automata, rolegraph
- `serde_json`: JSON serialization
- `tokio`: Async runtime
- `anyhow`: Error handling

#### Build Configuration

- **Optimization**: `opt-level = "z"` (size-optimized)
- **LTO**: Enabled for maximum optimization
- **Strip**: Symbols stripped for smaller binaries
- **Target Size**: <30MB (smaller than REPL due to no rustyline/comfy-table)

### Technical Details

**Architecture:**
- Non-interactive command execution model
- Clap-based argument parsing with derive macros
- Service wrapper (`CliService`) for consistent async operations
- Structured JSON output via serde
- Exit code handling for automation
- Shell completion via clap_complete

**Differences from terraphim-repl:**
- No interactive loop (single command execution)
- No rustyline/comfy-table dependencies
- Pure JSON output (no colored tables)
- Exit codes for success/failure
- Shell completion generation
- Designed for pipes and automation

**Compatibility:**
- Works with terraphim_types v1.0.0
- Works with terraphim_automata v1.0.0
- Works with terraphim_rolegraph v1.0.0
- Works with terraphim_service v1.0.0
- Same configuration as terraphim-repl

### Examples

See [README.md](README.md) for comprehensive examples including:
- Basic search and data extraction
- Piping to jq for JSON processing
- CI/CD integration
- Shell script automation
- Batch text processing

[Unreleased]: https://github.com/terraphim/terraphim-ai/compare/v1.0.0...HEAD
[1.0.0]: https://github.com/terraphim/terraphim-ai/releases/tag/v1.0.0
