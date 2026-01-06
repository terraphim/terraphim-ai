# terraphim-cli Documentation

## Overview

**terraphim-cli** is the automation-focused command-line interface for Terraphim AI, designed for scripted workflows, CI/CD pipelines, and headless environments. It provides structured JSON output suitable for parsing by other tools.

### Quick Facts

- **Binary Size**: 15 MB
- **Type**: Command-Line Interface
- **Default Output**: JSON
- **Version**: 1.0.0

## Installation

terraphim-cli is included in the Terraphim AI distribution. For installation details, see the [Installation Guide](./installation.md).

```bash
# Verify installation
terraphim-cli --version
# Output: terraphim-cli 1.0.0
```

## Usage

### Basic Syntax

```bash
terraphim-cli [OPTIONS] <COMMAND>
```

### Global Options

| Option | Description |
|--------|-------------|
| `--format <FORMAT>` | Output format: json, json-pretty, text |
| `--quiet` | Suppress non-JSON output (errors, warnings) |
| `-h, --help` | Print help |
| `-V, --version` | Print version |

### Output Formats

#### JSON (Default)

Structured JSON output for programmatic parsing:

```bash
terraphim-cli search "performance patterns"
```

**Output:**
```json
{"results":[{"title":"Pattern Name","score":0.95,"role":"Engineer"}],"metadata":{"query":"performance patterns","count":1}}
```

#### JSON-Pretty

Formatted JSON for readability:

```bash
terraphim-cli search "performance patterns" --format json-pretty
```

**Output:**
```json
{
  "results": [
    {
      "title": "Pattern Name",
      "score": 0.95,
      "role": "Engineer"
    }
  ],
  "metadata": {
    "query": "performance patterns",
    "count": 1
  }
}
```

#### Text

Human-readable text output:

```bash
terraphim-cli roles --format text
```

**Output:**
```
Engineer - Default engineering role
Default - General-purpose role
```

#### Quiet Mode

Suppress all non-JSON output:

```bash
# Normal output
terraphim-cli search "patterns"
# Output: {"results":[...],"metadata":{...}}

# With quiet mode (suppresses warnings)
terraphim-cli search "patterns" --quiet
# Output: {"results":[...],"metadata":{...}}
```

## Commands

### 1. search

Search the knowledge graph for matching documents.

```bash
terraphim-cli search <QUERY>
terraphim-cli search "machine learning patterns"
terraphim-cli search "API design" --format json
```

**Options:**
- `--format <FORMAT>`: Output format (default: json)
- `--quiet`: Suppress warnings

**Example Output (JSON):**

```json
{
  "results": [
    {
      "title": "Async Rust Patterns",
      "score": 0.95,
      "snippet": "Best practices for async/await in Rust...",
      "role": "Engineer",
      "url": "https://example.com/async-rust"
    },
    {
      "title": "Error Handling Patterns",
      "score": 0.89,
      "snippet": "Comprehensive guide to error handling...",
      "role": "Engineer",
      "url": "https://example.com/error-handling"
    }
  ],
  "metadata": {
    "query": "async patterns",
    "total_results": 2,
    "timestamp": "2026-01-06T12:00:00Z",
    "role": "Engineer"
  }
}
```

**Example Output (Text):**

```
Search Results for: "async patterns"

1. Async Rust Patterns (score: 0.95)
   Role: Engineer
   URL: https://example.com/async-rust

2. Error Handling Patterns (score: 0.89)
   Role: Engineer
   URL: https://example.com/error-handling
```

### 2. config

Display current configuration.

```bash
# Show full configuration (JSON)
terraphim-cli config

# Text format
terraphim-cli config --format text
```

**Example Output (JSON):**

```json
{
  "config": {
    "id": "Desktop",
    "global_shortcut": "Ctrl+X",
    "default_role": "Engineer",
    "roles": {
      "Engineer": {
        "shortname": "Engineer",
        "name": "Engineer",
        "relevance_function": "title-scorer"
      }
    }
  }
}
```

### 3. roles

List available roles.

```bash
# List roles (JSON)
terraphim-cli roles

# Text format
terraphim-cli roles --format text
```

**Example Output (JSON):**

```json
{
  "roles": [
    {
      "name": "Engineer",
      "shortname": "Engineer",
      "description": "Default engineering role with local knowledge graph"
    },
    {
      "name": "Default",
      "shortname": "Default",
      "description": "General-purpose role"
    }
  ],
  "default_role": "Engineer"
}
```

### 4. graph

Show top concepts from knowledge graph.

```bash
# Show graph (JSON)
terraphim-cli graph

# Text format
terraphim-cli graph --format text
```

**Example Output (JSON):**

```json
{
  "concepts": [
    {
      "name": "async",
      "score": 0.95,
      "connections": ["await", "tokio", "async-std"]
    },
    {
      "name": "error handling",
      "score": 0.89,
      "connections": ["Result", "?", "thiserror"]
    }
  ],
  "metadata": {
    "total_concepts": 50,
    "role": "Engineer"
  }
}
```

### 5. replace

Replace matched terms with links.

```bash
terraphim-cli replace <TEXT>
terraphim-cli replace "Using async Rust patterns for better performance"
```

**Example Output (JSON):**

```json
{
  "original": "Using async Rust patterns for better performance",
  "replaced": "Using [async Rust](/concepts/async) patterns for better [performance](/concepts/performance)",
  "matches": [
    {
      "original": "async Rust",
      "concept": "async",
      "link": "/concepts/async"
    },
    {
      "original": "performance",
      "concept": "performance",
      "link": "/concepts/performance"
    }
  ]
}
```

### 6. find

Find matched terms in text.

```bash
terraphim-cli find <TEXT>
terraphim-cli find "Error handling in async Rust requires proper Result types"
```

**Example Output (JSON):**

```json
{
  "text": "Error handling in async Rust requires proper Result types",
  "matches": [
    {
      "term": "Error handling",
      "score": 0.95,
      "exists": true
    },
    {
      "term": "async Rust",
      "score": 0.92,
      "exists": true
    },
    {
      "term": "Result types",
      "score": 0.88,
      "exists": false,
      "suggestions": ["Result", "error types"]
    }
  ],
  "found_count": 2,
  "missing_count": 1
}
```

### 7. thesaurus

Show thesaurus terms.

```bash
# Show thesaurus
terraphim-cli thesaurus

# Search thesaurus
echo "async" | terraphim-cli thesaurus -
```

**Example Output (JSON):**

```json
{
  "thesaurus": {
    "async": ["concurrent", "parallel", "non-blocking"],
    "performance": ["efficiency", "speed", "optimization"],
    "error": ["failure", "exception", "problem"]
  }
}
```

### 8. completions

Generate shell completions.

```bash
# Generate bash completions
terraphim-cli completions bash > terraphim-cli.bash

# Generate zsh completions
terraphim-cli completions zsh > _terraphim-cli

# Fish completions
terraphim-cli completions fish > terraphim-cli.fish
```

**Installation:**

```bash
# Bash
echo "source /path/to/terraphim-cli.bash" >> ~/.bashrc

# Zsh
echo "source /path/to/_terraphim-cli" >> ~/.zshrc

# Fish
echo "source /path/to/terraphim-cli.fish" >> ~/.config/fish/config.fish
```

## Automation Examples

### CI/CD Pipeline Integration

```yaml
# .github/workflows/code-quality.yml
name: Knowledge Graph Validation

on:
  push:
    branches: [main]
  pull_request:

jobs:
  validate-patterns:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install terraphim-cli
        run: |
          curl -L https://terraphim.ai/install.sh | sh
          echo "$HOME/.terraphim/bin" >> $GITHUB_PATH
      
      - name: Check for deprecated patterns
        run: |
          result=$(terraphim-cli search "deprecated patterns" --format json)
          count=$(echo "$result" | jq '.metadata.total_results')
          
          if [ "$count" -gt 0 ]; then
            echo "Found $count deprecated patterns"
            echo "$result" | jq '.results[].title'
            exit 1
          fi
      
      - name: Validate documentation links
        run: |
          result=$(terraphim-cli find "documentation content" --format json)
          missing=$(echo "$result" | jq '.missing_count')
          
          if [ "$missing" -gt 0 ]; then
            echo "::warning::Found $missing missing concepts"
          fi
```

### Scripted Knowledge Updates

```bash
#!/bin/bash
# scripts/check-knowledge-base.sh

set -e

TERRAPHIM="terraphim-cli --quiet"

echo "Checking knowledge base for updates..."

# Check for new patterns
new_patterns=$($TERRAPHIM search "new patterns" --format json)
pattern_count=$(echo "$new_patterns" | jq '.metadata.total_results')

if [ "$pattern_count" -gt 0 ]; then
    echo "Found $pattern_count new patterns"
    echo "$new_patterns" | jq -r '.results[].title' >> new-patterns.txt
fi

# Validate existing documentation
doc_validation=$($TERRAPHIM find "$(cat doc-content.txt)" --format json)
broken_links=$(echo "$doc_validation" | jq '[.matches[] | select(.exists == false)] | length')

if [ "$broken_links" -gt 0 ]; then
    echo "::warning::Found $broken_links broken knowledge links"
    echo "$doc_validation" | jq -r '.matches[] | select(.exists == false) | .term' >> broken-links.txt
fi

echo "Knowledge base check complete"
```

### Makefile Integration

```makefile
# Makefile

TERRAPHIM := terraphim-cli --quiet

.PHONY: check-docs check-patterns help

check-docs: ## Check documentation for broken links
	@result=$( $(TERRAPHIM) find "$$(cat docs/content.txt)" --format json ); \
	missing=$$(echo "$$result" | jq '.missing_count'); \
	if [ "$$missing" -gt 0 ]; then \
		echo "Found $$missing missing concepts"; \
		echo "$$result" | jq -r '.matches[] | select(.exists == false) | .term'; \
		exit 1; \
	fi
	@echo "Documentation check passed"

check-patterns: ## Check for deprecated patterns
	@result=$( $(TERRAPHIM) search "deprecated" --format json ); \
	count=$$(echo "$$result" | jq '.metadata.total_results'); \
	if [ "$$count" -gt 0 ]; then \
		echo "Found $$count deprecated patterns"; \
		echo "$$result" | jq -r '.results[].title'; \
		exit 1; \
	fi
	@echo "No deprecated patterns found"

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*##' Makefile | sort | awk 'BEGIN {FS=":.*## "}; {printf "%-30s %s\n", $$1, $$2}'
```

### Docker Integration

```dockerfile
# Dockerfile

FROM rust:1.75-slim as builder

WORKDIR /app
COPY . .
RUN cargo build --release --bin terraphim-cli

FROM debian:12-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/terraphim-cli /usr/local/bin/

ENTRYPOINT ["/usr/local/bin/terraphim-cli"]
```

```bash
# Usage in CI
docker run terraphim-cli search "patterns" --format json
```

### Kubernetes Job

```yaml
# k8s-knowledge-check.yaml
apiVersion: batch/v1
kind: Job
metadata:
  name: knowledge-check
spec:
  template:
    spec:
      containers:
      - name: terraphim
        image: terraphim/cli:latest
        command: ["terraphim-cli", "search", "patterns", "--format", "json"]
        env:
        - name: TERRAPHIM_SERVER_URL
          value: "http://terraphim-server:8000"
      restartPolicy: OnFailure
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | No results found |
| 2 | Error occurred |

## Configuration

### Environment Variables

```bash
# Server URL
export TERRAPHIM_SERVER_URL=http://localhost:8000

# API Key
export TERRAPHIM_API_KEY=your-api-key

# Data directory
export TERRAPHIM_DATA_DIR=~/.terraphim

# Log level
export RUST_LOG=info
```

### Configuration File

```json
{
  "server_url": "http://localhost:8000",
  "default_role": "Engineer",
  "output_format": "json",
  "timeout": 30
}
```

## Performance

### Benchmark Results

```bash
# Measure search latency
time terraphim-cli search "patterns" --format json

# Measure throughput
for i in {1..100}; do
    terraphim-cli search "patterns" --format json > /dev/null
done
```

**Typical Performance:**
- Search latency: ~50-200ms (network dependent)
- Cold start: ~1-2 seconds
- Memory usage: ~50-100MB

### Optimization Tips

1. **Use Quiet Mode**
   ```bash
   # Reduces output processing
   terraphim-cli search "patterns" --quiet
   ```

2. **Limit Results**
   ```bash
   # Pipe to head for large result sets
   terraphim-cli search "patterns" | jq '.[:10]'
   ```

3. **Connection Pooling**
   ```bash
   # For multiple requests, use a single session
   export TERRAPHIM_SERVER_URL=http://localhost:8000
   terraphim-cli search "pattern1"
   terraphim-cli search "pattern2"
   ```

## Troubleshooting

### Common Issues

**1. Connection Refused**

```bash
# Error: Could not connect to server
# Solution: Check server URL or use local mode
TERRAPHIM_SERVER_URL=http://localhost:8000 terraphim-cli search "patterns"
```

**2. JSON Parse Error**

```bash
# Error: Invalid JSON in output
# Solution: Check for errors with --quiet flag
terraphim-cli search "patterns" 2>&1 | head -20
```

**3. Timeout**

```bash
# Error: Request timeout
# Solution: Increase timeout or check network
RUST_LOG=debug terraphim-cli search "patterns"
```

### Debug Mode

```bash
# Enable debug logging
RUST_LOG=debug terraphim-cli search "patterns"

# Save full output
RUST_LOG=trace terraphim-cli search "patterns" 2>&1 | tee debug.log
```

## Comparison with Other Tools

| Feature | terraphim-cli | terraphim-agent | grep/ripgrep |
|---------|---------------|-----------------|--------------|
| JSON output | ✅ | ✅ | ❌ |
| Knowledge graph | ✅ | ✅ | ❌ |
| Interactive | ❌ | ✅ | ❌ |
| Speed | Fast | Medium | Very Fast |
| Pattern matching | Semantic | Semantic | Literal |
| Learning curve | Low | Medium | Low |

## See Also

- [CLI Tools Overview](./cli-tools-overview.md)
- [terraphim-agent Documentation](./terraphim-agent.md)
- [terraphim-repl Documentation](./terraphim-repl.md)
- [Installation Guide](./installation.md)
- [API Documentation](../api/README.md)
