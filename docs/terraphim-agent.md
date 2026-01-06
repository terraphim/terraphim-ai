# terraphim-agent Documentation

## Overview

**terraphim-agent** is the primary Terraphim AI interface, providing a full-featured Terminal User Interface (TUI) for knowledge graph search and management. With 14+ commands, it supports interactive exploration, AI agent integration, and automated workflows.

### Quick Facts

- **Binary Size**: 17 MB
- **Type**: Interactive TUI Application
- **Default Port**: http://localhost:8000
- **Version**: 1.0.0

## Installation

terraphim-agent is included in the Terraphim AI distribution. For installation details, see the [Installation Guide](./installation.md).

```bash
# Verify installation
terraphim-agent --version
# Output: terraphim-agent 1.0.0
```

## Usage

### Basic Syntax

```bash
terraphim-agent [OPTIONS] [COMMAND]
```

### Global Options

| Option | Description |
|--------|-------------|
| `--server` | Use server API mode instead of offline mode |
| `--server-url <URL>` | Server URL for API mode (default: http://localhost:8000) |
| `--transparent` | Enable transparent background mode |
| `--robot` | Enable robot mode for AI agent integration (JSON output, exit codes) |
| `--format <FORMAT>` | Output format: human, json, json-compact |
| `-h, --help` | Print help summary |
| `-V, --version` | Print version |

### Output Formats

#### Human Format (Default)

Human-readable output with colors, tables, and formatting:

```bash
terraphim-agent search "rust async patterns"
```

#### JSON Format

Machine-readable JSON output:

```bash
terraphim-agent --format json search "rust async patterns"
```

Output structure:
```json
{
  "results": [
    {
      "title": "Pattern Name",
      "score": 0.95,
      "snippet": "...",
      "role": "Engineer"
    }
  ],
  "metadata": {
    "query": "rust async patterns",
    "timestamp": "2026-01-06T12:00:00Z",
    "total_results": 10
  }
}
```

#### JSON-Compact Format

Compact JSON for piping:

```bash
terraphim-agent --format json-compact search "rust async patterns"
```

Output:
```json
{"results":[...],"metadata":{...}}
```

#### Robot Mode

Special mode for AI agents with structured output and exit codes:

```bash
terraphim-agent --robot search "error patterns"
```

**Robot Mode Exit Codes:**
- `0`: Success, results found
- `1`: Success, no results
- `2`: Error occurred
- `3`: Invalid arguments

## Commands

### 1. search

Search the knowledge graph for matching documents.

```bash
terraphim-agent search <QUERY>
terraphim-agent search "machine learning patterns"
terraphim-agent search "API design" --format json
```

**Examples:**

```bash
# Basic search
terraphim-agent search "async rust"

# Search with JSON output
terraphim-agent search "performance optimization" --format json

# Robot mode for scripts
terraphim-agent --robot search "dependency errors"
```

**Output:** Table of matching results with scores, titles, and snippets.

### 2. roles

List and manage available roles.

```bash
# List all roles
terraphim-agent roles

# Show current role configuration
terraphim-agent roles --format json
```

**Available Roles:**
- **Engineer**: Default engineering role with local knowledge graph
- **Default**: General-purpose role
- Custom roles defined in configuration

### 3. config

Display current configuration.

```bash
# Show full configuration
terraphim-agent config

# JSON format
terraphim-agent config --format json
```

**Configuration Sections:**
- Server settings
- Role configurations
- Knowledge graph paths
- LLM settings
- Haystack configurations

### 4. graph

Display knowledge graph top concepts and relationships.

```bash
# Show top concepts
terraphim-agent graph

# JSON for processing
terraphim-agent graph --format json
```

**Output:** Visual representation of connected concepts with relevance scores.

### 5. chat

Enable LLM-powered chat for knowledge exploration (requires LLM configuration).

```bash
terraphim-agent chat
terraphim-agent chat --model gpt-4
```

**Requirements:**
- LLM API key configured
- `llm_enabled: true` in configuration
- Appropriate model specified

### 6. extract

Extract entities and relationships from text.

```bash
terraphim-agent extract <TEXT>
terraphim-agent extract "This Rust code uses async/await patterns"
```

**Output:** Structured entities and relationships.

### 7. replace

Replace matched terms with knowledge graph links.

```bash
terraphim-agent replace <TEXT>
terraphim-agent replace "The async pattern in Rust"
```

**Example:**
```bash
terraphim-agent replace "Error handling in async Rust"
# Output: Error handling in [async Rust](/link/to/concept)
```

### 8. validate

Validate text against the knowledge graph.

```bash
terraphim-agent validate <TEXT>
terraphim-agent validate "Using the new async API"
```

**Output:** Validation results showing which terms exist and which are missing.

### 9. suggest

Suggest similar terms using fuzzy matching.

```bash
terraphim-agent suggest <TERM>
terraphim-agent suggest "asynch"
```

**Output:** List of similar terms with similarity scores.

### 10. hook

Unified hook handler for Claude Code integration.

```bash
terraphim-agent hook --operation <OP> --input <INPUT>
terraphim-agent hook --operation search --input "error handling"
terraphim-agent hook --operation validate --input "async patterns"
```

**Operations:**
- `search`: Execute search query
- `validate`: Validate input text
- `extract`: Extract entities
- `suggest`: Get suggestions

**Claude Code Integration:**

```json
// claude_code.json
{
  "hooks": {
    "PreToolUse": {
      "command": {
        "run": "terraphim-agent hook --operation ${垂_operation} --input \"${垂_input}\""
      }
    }
  }
}
```

### 11. guard

Check commands against safety guard patterns (blocks destructive operations).

```bash
terraphim-agent guard <COMMAND>
terraphim-agent guard "rm -rf /"
terraphim-agent guard "git push --force"
```

**Guard Categories:**
- **文件系统危险操作**: rm, mv (with destructive flags), chmod (with dangerous perms)
- **Git危险操作**: git push --force, git reset --hard
- **网络危险操作**: curl with sensitive headers
- **系统危险操作**: chmod, chown with dangerous permissions

**Exit Codes:**
- `0`: Command is safe
- `1`: Command is potentially dangerous
- `2`: Error checking command

**Example in Scripts:**

```bash
#!/bin/bash
if terraphim-agent guard "rm -rf node_modules"; then
    echo "Command approved"
    rm -rf node_modules
else
    echo "Command blocked by safety guard"
    exit 1
fi
```

### 12. interactive

Start full interactive TUI mode.

```bash
terraphim-agent interactive
```

**TUI Features:**
- Navigation menus
- Search with autocomplete
- Role selection
- Graph visualization
- Keyboard shortcuts

### 13. check-update

Check for updates without installing.

```bash
terraphim-agent check-update
```

**Output:** Current version, latest version, and update availability.

### 14. update

Update to latest version if available.

```bash
terraphim-agent update
```

**Requirements:**
- Appropriate permissions
- Internet connection
- Valid installation

## Server Mode

### Connecting to Server

```bash
# Use default server
terraphim-agent --server search "patterns"

# Use custom server
terraphim-agent --server --server-url http://custom:8000 search "patterns"
```

### Server vs Offline Mode

| Feature | Server Mode | Offline Mode |
|---------|-------------|--------------|
| Knowledge Graph | Remote | Local |
| Performance | Network dependent | Fast |
| Scalability | High | Limited |
| Use Case | Team sharing | Personal use |

## Configuration

### Default Configuration Location

```bash
~/.terraphim/config.json
```

### Example Configuration

```json
{
  "id": "Desktop",
  "global_shortcut": "Ctrl+X",
  "roles": {
    "Engineer": {
      "shortname": "Engineer",
      "name": "Engineer",
      "relevance_function": "title-scorer",
      "terraphim_it": false,
      "theme": "lumen",
      "kg": {
        "automata_path": {
          "Local": "test-fixtures/term_to_id_simple.json"
        },
        "knowledge_graph_local": null,
        "public": false,
        "publish": false
      },
      "haystacks": [
        {
          "location": "localsearch",
          "service": "Ripgrep",
          "read_only": false
        }
      ],
      "llm_enabled": false
    }
  },
  "default_role": "Engineer"
}
```

### Environment Variables

```bash
# Server URL
TERRAPHIM_SERVER_URL=http://localhost:8000

# API Key
TERRAPHIM_API_KEY=your-api-key

# Data Directory
TERRAPHIM_DATA_DIR=~/.terraphim

# Log Level
RUST_LOG=info
```

## Use Cases

### 1. Daily Knowledge Search

```bash
# Quick search for current task
terraphim-agent search "async Rust patterns"

# Explore related concepts
terraphim-agent graph

# Switch roles for different perspectives
terraphim-agent roles
```

### 2. AI Agent Integration

```bash
#!/bin/bash
# AI agent workflow

# Search for patterns
results=$(terraphim-agent --robot --format json search "error handling")
count=$(echo "$results" | jq '.results | length')

if [ "$count" -gt 0 ]; then
    echo "Found relevant patterns"
    echo "$results" | jq '.results[0].title'
fi

# Validate code before commit
if ! terraphim-agent guard "git push --force"; then
    echo "Prevented dangerous operation"
    exit 1
fi
```

### 3. Claude Code Integration

```json
{
  "hooks": {
    "PreToolUse": {
      "command": {
        "run": "terraphim-agent hook --operation ${垂_operation} --input \"${垂_input}\""
      }
    },
    "PostToolUse": {
      "command": {
        "run": "terraphim-agent hook --operation validate --input \"${垂_output}\""
      }
    }
  }
}
```

### 4. Safety-Critical Operations

```bash
#!/bin/bash
# Production deployment script

# Check if command is safe
if ! terraphim-agent guard "kubectl delete --all"; then
    echo "ERROR: Attempted to delete all Kubernetes resources"
    echo "This operation was blocked by safety guard"
    exit 1
fi

# Proceed with safe operations
terraphim-agent guard "kubectl get pods"
```

## Troubleshooting

### Common Issues

**1. Connection Refused**

```bash
# Error: Could not connect to server
# Solution: Start server or use offline mode
terraphim-agent search "patterns"  # Uses offline mode
terraphim-agent --server search "patterns"  # Requires server
```

**2. No Results**

```bash
# Check if knowledge graph is loaded
terraphim-agent config --format json | jq '.roles'

# Verify role configuration
terraphim-agent roles
```

**3. Invalid Output Format**

```bash
# Error: Invalid format specified
# Solution: Use valid format values
terraphim-agent search "patterns" --format human  # Valid
terraphim-agent search "patterns" --format json    # Valid
```

### Log Collection

```bash
# Enable verbose logging
RUST_LOG=debug terraphim-agent search "patterns"

# Save logs to file
RUST_LOG=debug terraphim-agent search "patterns" 2>&1 | tee debug.log
```

## Advanced Features

### Keyboard Shortcuts (Interactive Mode)

| Shortcut | Action |
|----------|--------|
| `/` | Start search |
| `Ctrl+R` | Switch role |
| `Ctrl+G` | Show graph |
| `Ctrl+C` | Quit |
| `Esc` | Cancel/Back |

### Shell Completions

```bash
# Generate completions for bash
terraphim-agent completions bash >> ~/.bashrc

# Generate completions for zsh
terraphim-agent completions zsh >> ~/.zshrc

# Reload shell
source ~/.bashrc
```

## Performance Tips

1. **Use Offline Mode for Personal Knowledge**
   ```bash
   # No network overhead
   terraphim-agent search "local patterns"
   ```

2. **Robot Mode for Scripts**
   ```bash
   # Faster, no TUI overhead
   terraphim-agent --robot --format json search "patterns"
   ```

3. **Limit Results**
   ```bash
   # Reduce output for large datasets
   terraphim-agent search "patterns" | head -20
   ```

## See Also

- [CLI Tools Overview](./cli-tools-overview.md)
- [terraphim-cli Documentation](./terraphim-cli.md)
- [terraphim-repl Documentation](./terraphim-repl.md)
- [Installation Guide](./installation.md)
- [Configuration Reference](./configuration.md)
