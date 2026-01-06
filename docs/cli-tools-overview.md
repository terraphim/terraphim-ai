# CLI Tools Overview

Terraphim AI provides a comprehensive suite of command-line tools designed to meet different use cases, from interactive exploration to automated workflows. This guide helps you choose the right tool for your specific needs.

## Tool Comparison Matrix

| Feature | terraphim-agent | terraphim-cli | terraphim-repl |
|---------|----------------|---------------|----------------|
| **Primary Use** | Interactive TUI | Automation | Interactive Shell |
| **Output Formats** | Human, JSON, JSON-compact | JSON, JSON-pretty, Text | Human only |
| **Interactivity** | Full TUI with menus | Non-interactive | Interactive REPL |
| **AI Integration** | Robot mode, Hooks | JSON output | Limited |
| **Scriptability** | Moderate | High | Low |
| **Learning Curve** | Medium | Low | Medium |
| **Best For** | Daily use | CI/CD, scripts | Quick exploration |

## When to Use Each Tool

### Use terraphim-agent When:

- **Daily Knowledge Work**: You need a rich, interactive interface for regular knowledge graph queries
- **Visual Exploration**: You want to browse roles, graphs, and search results visually
- **AI Agent Integration**: You're building AI agents that need structured output with robot mode
- **Safety-Critical Operations**: You need the guard command to prevent destructive operations
- ** Claude Code Integration**: You want to use Terraphim hooks with Claude Code
- **Mixed Workflows**: You switch between exploration and automation

```bash
# Interactive search with full TUI
terraphim-agent search "machine learning patterns"

# Robot mode for AI agents (JSON output)
terraphim-agent --robot --format json search "error patterns"

# Check for dangerous commands
terraphim-agent guard "rm -rf /"

# Use hooks for Claude Code integration
terraphim-agent hook --operation search --input "API design"
```

### Use terraphim-cli When:

- **Automation Pipelines**: You need to integrate Terraphim into CI/CD systems
- **Scripted Workflows**: You're writing shell scripts or Makefiles
- **Structured Data Processing**: You need machine-readable JSON for further processing
- **Headless Environments**: You're running in servers without display
- **Performance Testing**: You want minimal overhead for benchmarking

```bash
# JSON output for scripts
terraphim-cli search "performance optimization" --format json

# Text output for human review
terraphim-cli roles --format text

# Quiet mode for automation (suppress non-JSON output)
terraphim-cli search "error patterns" --quiet
```

### Use terraphim-repl When:

- **Quick Exploration**: You want to test queries rapidly without starting a full TUI
- **Learning Terraphim**: You're new and want to experiment with commands
- **Interactive Debugging**: You're troubleshooting knowledge graph issues
- **Prototype Development**: You're testing search patterns before scripting

```bash
# Start interactive session
terraphim-repl

# Inside REPL:
/search rust async patterns
/roles
/graph
/config show
/quit
```

## Decision Flowchart

```
Start
  |
  v
Are you building an automation script or CI/CD pipeline?
  |
  +--[Yes]--> Use terraphim-cli
  |
  +--[No]--> Are you integrating with AI agents or need Claude Code hooks?
              |
              +--[Yes]--> Use terraphim-agent (robot mode, hooks, guard)
              |
              +--[No]--> Do you need quick interactive exploration?
                        |
                        +--[Yes]--> Use terraphim-repl
                        |
                        +--[No]--> Use terraphim-agent (full TUI experience)
```

## Output Format Comparison

### terraphim-agent Output Formats

```bash
# Human-readable (default)
terraphim-agent search "API design"
# Output: Formatted table with colors and emphasis

# JSON-compact (for piping)
terraphim-agent --format json-compact search "API design"
# Output: {"results":[...],"metadata":{...}}

# JSON (structured)
terraphim-agent --format json search "API design"
# Output: Pretty-printed JSON with indentation

# Robot mode (for AI agents)
terraphim-agent --robot search "API design"
# Output: Machine-readable with exit codes
```

### terraphim-cli Output Formats

```bash
# JSON (default)
terraphim-cli search "API design"
# Output: JSON object

# JSON-pretty
terraphim-cli search "API design" --format json-pretty
# Output: Formatted JSON

# Text
terraphim-cli roles --format text
# Output: Human-readable text
```

## Feature Comparison by Category

### Search Capabilities

| Feature | terraphim-agent | terraphim-cli | terraphim-repl |
|---------|----------------|---------------|----------------|
| Basic search | ✅ | ✅ | ✅ |
| Role-based search | ✅ | ✅ | ✅ |
| Fuzzy matching | ✅ | ✅ | ✅ |
| Result highlighting | ✅ | ❌ | Partial |
| Pagination | ✅ | ✅ | ❌ |

### Knowledge Graph Features

| Feature | terraphim-agent | terraphim-cli | terraphim-repl |
|---------|----------------|---------------|----------------|
| Graph visualization | ✅ | ❌ | ❌ |
| Top concepts | ✅ | ✅ | ✅ |
| Role management | ✅ | ✅ | ✅ |
| Configuration | ✅ | ✅ | ✅ |

### Automation Features

| Feature | terraphim-agent | terraphim-cli | terraphim-repl |
|---------|----------------|---------------|----------------|
| Exit codes | ✅ (robot mode) | ✅ | ❌ |
| JSON output | ✅ | ✅ | ❌ |
| Quiet mode | ❌ | ✅ | ❌ |
| Shell completions | ✅ | ✅ | ❌ |

### Integration Features

| Feature | terraphim-agent | terraphim-cli | terraphim-repl |
|---------|----------------|---------------|----------------|
| Claude Code hooks | ✅ | ❌ | ❌ |
| Safety guard | ✅ | ❌ | ❌ |
| Server mode | ✅ | ❌ | ❌ |
| MCP protocol | ❌ | ❌ | ❌ |

## Quick Reference

### Installation Verification

```bash
# Check versions
terraphim-agent --version    # v1.0.0
terraphim-cli --version      # v1.0.0
terraphim-repl --version     # v1.0.0

# Get help
terraphim-agent --help
terraphim-cli --help
terraphim-repl --help
```

### Common Workflows

**1. Daily Knowledge Search (terraphim-agent)**
```bash
terraphim-agent search "rust async patterns"
terraphim-agent roles
terraphim-agent graph
```

**2. Automated Pipeline (terraphim-cli)**
```bash
#!/bin/bash
results=$(terraphim-cli search "API errors" --format json)
if echo "$results" | jq '.count > 0' | grep -q true; then
    echo "Found issues to fix"
fi
```

**3. Quick Exploration (terraphim-repl)**
```bash
terraphim-repl
# /search error handling
# /graph
# /quit
```

**4. AI Agent Integration (terraphim-agent robot mode)**
```bash
terraphim-agent --robot --format json search "dependency patterns"
# Parse JSON, use results, exit with appropriate code
```

## Environment Variables

All tools support these environment variables:

```bash
# Server configuration
TERRAPHIM_SERVER_URL=http://localhost:8000

# API key for LLM features
TERRAPHIM_API_KEY=your-key-here

# Data directory
TERRAPHIM_DATA_DIR=~/.terraphim

# Log level
RUST_LOG=info
```

## Next Steps

- [terraphim-agent Documentation](./terraphim-agent.md) - Comprehensive guide to the main TUI interface
- [terraphim-cli Documentation](./terraphim-cli.md) - Complete CLI reference for automation
- [terraphim-repl Documentation](./terraphim-repl.md) - Interactive REPL guide
- [Installation Guide](./installation.md) - How to install all tools
