# terraphim-cli

[![Crates.io](https://img.shields.io/crates/v/terraphim-cli.svg)](https://crates.io/crates/terraphim-cli)
[![License](https://img.shields.io/crates/l/terraphim-cli.svg)](https://github.com/terraphim/terraphim-ai/blob/main/LICENSE-Apache-2.0)

**Automation-friendly CLI for semantic knowledge graph search with JSON output.**

## Overview

`terraphim-cli` is a non-interactive command-line tool designed for scripting and automation. It provides the same semantic search capabilities as `terraphim-repl` but optimized for:

- **JSON Output**: Machine-readable output for scripts and pipelines
- **Exit Codes**: Proper exit codes (0 = success, 1 = error) for automation
- **Shell Completions**: Auto-completion for bash, zsh, and fish
- **Piping**: Works seamlessly in Unix pipelines

## Installation

### From crates.io

```bash
cargo install terraphim-cli
```

### From Source

```bash
git clone https://github.com/terraphim/terraphim-ai
cd terraphim-ai
cargo build --release -p terraphim-cli
./target/release/terraphim-cli --help
```

## Quick Start

### Basic Search

```bash
# Search with JSON output
terraphim-cli search "rust async programming"
```

**Output:**
```json
{
  "query": "rust async programming",
  "role": "Default",
  "results": [
    {
      "id": "doc1",
      "title": "Async Programming in Rust",
      "url": "https://rust-lang.github.io/async-book/",
      "rank": 0.95
    }
  ],
  "count": 1
}
```

### Pretty JSON

```bash
terraphim-cli --format json-pretty search "tokio"
```

### Pipe to jq

```bash
terraphim-cli search "rust" | jq '.results[] | .title'
```

## Commands

### search - Search Documents

```bash
terraphim-cli search <QUERY> [OPTIONS]

Options:
  --role <ROLE>      Role to use for search
  -n, --limit <N>    Maximum number of results
```

**Examples:**
```bash
# Basic search
terraphim-cli search "knowledge graph"

# With role and limit
terraphim-cli search "async" --role Engineer --limit 5

# Extract titles only
terraphim-cli search "rust" | jq -r '.results[].title'
```

---

### config - Show Configuration

```bash
terraphim-cli config
```

**Output:**
```json
{
  "selected_role": "Default",
  "roles": ["Default", "Engineer"]
}
```

---

### roles - List Available Roles

```bash
terraphim-cli roles
```

**Output:**
```json
["Default", "Engineer", "SystemOps"]
```

---

### graph - Show Top Concepts

```bash
terraphim-cli graph [OPTIONS]

Options:
  -k, --top-k <K>    Number of concepts [default: 10]
  --role <ROLE>      Role to use
```

**Example:**
```bash
terraphim-cli graph --top-k 20 --role Engineer
```

**Output:**
```json
{
  "role": "Engineer",
  "top_k": 20,
  "concepts": [
    "rust programming language",
    "async programming",
    "tokio runtime",
    ...
  ]
}
```

---

### replace - Replace Terms with Links

```bash
terraphim-cli replace <TEXT> [OPTIONS]

Options:
  --format <FORMAT>  Output format: markdown, html, wiki, plain [default: markdown]
  --role <ROLE>      Role to use
```

**Examples:**
```bash
# Markdown links (default)
terraphim-cli replace "check out rust async programming"

# HTML links
terraphim-cli replace "rust and tokio" --format html

# Wiki links
terraphim-cli replace "learn rust" --format wiki
```

**Output:**
```json
{
  "original": "check out rust async programming",
  "replaced": "check out [rust](https://rust-lang.org) [async](https://rust-lang.github.io/async-book/) programming",
  "format": "markdown"
}
```

---

### find - Find Matched Terms

```bash
terraphim-cli find <TEXT> [OPTIONS]

Options:
  --role <ROLE>  Role to use
```

**Example:**
```bash
terraphim-cli find "rust async and tokio are great"
```

**Output:**
```json
{
  "text": "rust async and tokio are great",
  "matches": [
    {
      "term": "rust",
      "position": [0, 4],
      "normalized": "rust programming language"
    },
    {
      "term": "async",
      "position": [5, 10],
      "normalized": "asynchronous programming"
    },
    {
      "term": "tokio",
      "position": [15, 20],
      "normalized": "tokio async runtime"
    }
  ],
  "count": 3
}
```

---

### thesaurus - Show Knowledge Graph Terms

```bash
terraphim-cli thesaurus [OPTIONS]

Options:
  --role <ROLE>      Role to use
  --limit <LIMIT>    Maximum terms to show [default: 50]
```

**Example:**
```bash
terraphim-cli thesaurus --role Engineer --limit 10
```

**Output:**
```json
{
  "role": "Engineer",
  "name": "engineer_thesaurus",
  "terms": [
    {
      "id": 1,
      "term": "rust",
      "normalized": "rust programming language",
      "url": "https://rust-lang.org"
    },
    ...
  ],
  "total_count": 150,
  "shown_count": 10
}
```

---

### completions - Generate Shell Completions

```bash
terraphim-cli completions <SHELL>

Shells: bash, zsh, fish, powershell
```

**Install Completions:**

**Bash:**
```bash
terraphim-cli completions bash > ~/.local/share/bash-completion/completions/terraphim-cli
```

**Zsh:**
```bash
terraphim-cli completions zsh > ~/.zfunc/_terraphim-cli
```

**Fish:**
```bash
terraphim-cli completions fish > ~/.config/fish/completions/terraphim-cli.fish
```

---

## Global Options

```bash
--format <FORMAT>   Output format: json, json-pretty, text [default: json]
--quiet             Suppress non-JSON output (errors, warnings)
--help              Print help
--version           Print version
```

## Exit Codes

- `0` - Success
- `1` - Error (invalid input, service failure, etc.)

## Scripting Examples

### Search and Extract URLs

```bash
terraphim-cli search "rust documentation" | jq -r '.results[].url'
```

### Count Results

```bash
terraphim-cli search "async" | jq '.count'
```

### Filter by Rank

```bash
terraphim-cli search "rust" | jq '.results[] | select(.rank > 0.8)'
```

### Loop Through Results

```bash
terraphim-cli search "tokio" | jq -r '.results[] | "\(.title): \(.url)"' | while read line; do
  echo "Found: $line"
done
```

### Replace Text in Files

```bash
cat input.md | while read line; do
  terraphim-cli replace "$line" --format markdown | jq -r '.replaced'
done > output.md
```

### Check if Terms Exist

```bash
if terraphim-cli find "rust tokio" | jq '.count > 0'; then
  echo "Found rust and tokio in knowledge graph"
fi
```

## CI/CD Integration

### GitHub Actions

```yaml
- name: Search Knowledge Graph
  run: |
    cargo install terraphim-cli
    terraphim-cli search "deployment" --limit 10 > results.json

- name: Validate Results
  run: |
    COUNT=$(jq '.count' results.json)
    if [ "$COUNT" -eq 0 ]; then
      echo "No results found"
      exit 1
    fi
```

### Shell Scripts

```bash
#!/bin/bash
set -e

# Search for specific terms
RESULTS=$(terraphim-cli search "api documentation" --limit 5)

# Check if we got results
if [ "$(echo $RESULTS | jq '.count')" -eq 0 ]; then
  echo "Error: No documentation found"
  exit 1
fi

# Extract URLs and fetch them
echo $RESULTS | jq -r '.results[].url' | xargs -I {} curl -s {}
```

## Differences from terraphim-repl

| Feature | terraphim-cli | terraphim-repl |
|---------|---------------|----------------|
| **Mode** | Non-interactive | Interactive |
| **Output** | JSON | Pretty tables + colored |
| **Use Case** | Automation/scripts | Human interaction |
| **Exit Codes** | Proper (0/1) | N/A |
| **Completions** | Yes (bash/zsh/fish) | Command completion in REPL |
| **Piping** | Designed for it | N/A |
| **History** | No | Yes |

Use `terraphim-cli` when:
- Writing scripts or automation
- Integrating with other tools via JSON
- CI/CD pipelines
- Batch processing
- Need machine-readable output

Use `terraphim-repl` when:
- Interactive exploration
- Learning the system
- Ad-hoc queries
- Human-readable output preferred

## Configuration

Uses the same configuration as `terraphim-repl`:
- `~/.terraphim/config.json` - Main configuration
- Supports role-based search
- Works offline with embedded defaults

## System Requirements

### Minimum (Measured)
- **RAM**: 20 MB (typical: 15 MB)
- **Disk**: 15 MB
- **OS**: Linux, macOS, or Windows
- **Rust**: 1.70+ (for installation)

### Performance
- **Startup**: <200ms
- **Search**: 50-180ms
- **Replace/Find**: <10ms
- **Memory scaling**: ~1MB per 1000 thesaurus terms

**Note**: Actual measurements show 8-18 MB RAM usage, making this tool suitable for containers, VMs, and embedded systems.

## Troubleshooting

### Command Not Found

```bash
# Make sure cargo bin is in PATH
export PATH="$HOME/.cargo/bin:$PATH"
```

### JSON Parsing Errors

```bash
# Use --quiet to suppress non-JSON output
terraphim-cli --quiet search "query" | jq '.'
```

### Completions Not Working

```bash
# Bash: Check completion directory
ls ~/.local/share/bash-completion/completions/

# Zsh: Check fpath includes ~/.zfunc
echo $fpath

# Fish: Check completions directory
ls ~/.config/fish/completions/
```

## Building from Source

```bash
# Debug build
cargo build -p terraphim-cli

# Release build (optimized)
cargo build --release -p terraphim-cli

# Run tests
cargo test -p terraphim-cli

# Generate docs
cargo doc -p terraphim-cli --open
```

## Related Projects

- **[terraphim-repl](../terraphim_repl)**: Interactive REPL interface
- **[terraphim_types](../terraphim_types)**: Core type definitions
- **[terraphim_automata](../terraphim_automata)**: Text matching engine
- **[terraphim_rolegraph](../terraphim_rolegraph)**: Knowledge graph implementation

## Support

- **Discord**: https://discord.gg/VPJXB6BGuY
- **Discourse**: https://terraphim.discourse.group
- **Issues**: https://github.com/terraphim/terraphim-ai/issues

## License

Licensed under Apache-2.0. See [LICENSE](../../LICENSE-Apache-2.0) for details.

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for version history.
