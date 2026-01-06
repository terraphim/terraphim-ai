# CLI Tools Documentation

This section provides comprehensive documentation for all Terraphim AI command-line tools.

## Overview

Terraphim AI provides three primary CLI tools, each designed for different use cases:

1. **[CLI Tools Overview](./cli-tools-overview.md)** - Quick reference guide for choosing the right tool
2. **[terraphim-agent](./terraphim-agent.md)** - Full-featured TUI interface for interactive use
3. **[terraphim-cli](./terraphim-cli.md)** - Automation-focused CLI for scripts and CI/CD
4. **[terraphim-repl](./terraphim-repl.md)** - Interactive REPL for quick exploration

## Quick Start

### Choose Your Tool

```
Need quick answers?      → Use terraphim-repl
Daily knowledge work?    → Use terraphim-agent  
Automation/scripts?      → Use terraphim-cli
```

### Installation Verification

```bash
# Check all tools are installed
terraphim-agent --version  # v1.0.0
terraphim-cli --version    # v1.0.0
terraphim-repl --version   # v1.0.0
```

### First Commands

```bash
# Interactive exploration
terraphim-repl
> /search "patterns"
> /graph
> /quit

# Quick search
terraphim-cli search "async rust"

# Full TUI
terraphim-agent search "async rust"
```

## Tool Comparison

| Aspect | terraphim-agent | terraphim-cli | terraphim-repl |
|--------|----------------|---------------|----------------|
| **Type** | Full TUI | CLI | REPL Shell |
| **Interactivity** | High | None | High |
| **Automation** | Medium | High | Low |
| **Output Formats** | Human, JSON | JSON, Text | Human, JSON |
| **Best For** | Daily use | Scripts, CI/CD | Learning, testing |
| **Size** | 17 MB | 15 MB | 15 MB |

## Common Workflows

### 1. Knowledge Search

**Interactive (Recommended):**
```bash
terraphim-agent search "machine learning patterns"
```

**Automation:**
```bash
results=$(terraphim-cli search "patterns" --format json)
echo "$results" | jq '.results | length'
```

**Quick Check:**
```bash
terraphim-repl
> /search "patterns"
> /quit
```

### 2. Role Management

**Full Control:**
```bash
terraphim-agent roles
terraphim-agent config
```

**Scripted:**
```bash
terraphim-cli roles --format json | jq '.roles[].name'
```

### 3. Documentation Validation

**Manual:**
```bash
terraphim-repl
> /find "This code implements async patterns"
> /replace "Using async Rust for concurrency"
> /quit
```

**Automated:**
```bash
terraphim-cli find "documentation text" --format json | jq '.missing_count'
```

### 4. Integration with AI Agents

**Robot Mode:**
```bash
terraphim-agent --robot --format json search "patterns"
```

**Claude Code Hooks:**
```bash
terraphim-agent hook --operation search --input "query"
```

## Documentation Structure

```
docs/
├── cli-tools-overview.md      # This overview
├── terraphim-agent.md         # Full TUI documentation
├── terrraphim-cli.md          # CLI documentation
├── terraphim-repl.md          # REPL documentation
└── installation.md            # Installation guide
```

## Key Features

### terraphim-agent Features
- Full terminal user interface (TUI)
- 14+ commands including search, roles, graph, chat
- Robot mode for AI agent integration
- Claude Code hooks support
- Safety guard for dangerous commands
- Interactive graph visualization
- Multiple output formats

### terraphim-cli Features
- Clean JSON output for parsing
- Quiet mode for scripts
- Shell completion generation
- CI/CD pipeline integration
- Headless server operation
- Fast execution

### terraphim-repl Features
- Interactive command history
- Tab completion
- Quick exploration mode
- Learning-friendly interface
- Multi-line input support
- Colorized output

## Configuration

### Environment Variables

```bash
# All tools
TERRAPHIM_SERVER_URL=http://localhost:8000
TERRAPHIM_API_KEY=your-api-key
TERRAPHIM_DATA_DIR=~/.terraphim
RUST_LOG=info
```

### Configuration Files

```bash
# terraphim-agent
~/.terraphim/config.json

# terraphim-cli  
~/.terraphim/cli-config.json

# terraphim-repl
~/.terraphim/repl-config.json
```

## Examples

### Bash Script Integration

```bash
#!/bin/bash
# check-patterns.sh

TERRAPHIM="terraphim-cli --quiet"

# Search for deprecated patterns
result=$($TERRAPHIM search "deprecated" --format json)
count=$(echo "$result" | jq '.metadata.total_results')

if [ "$count" -gt 0 ]; then
    echo "Found $count deprecated patterns:"
    echo "$result" | jq -r '.results[].title'
    exit 1
fi

echo "No deprecated patterns found"
```

### Git Hook Integration

```bash
#!/bin/bash
# .git/hooks/pre-commit

# Check for known anti-patterns
result=$(terraphim-cli --quiet search "anti-pattern" --format json)
count=$(echo "$result" | jq '.metadata.total_results')

if [ "$count" -gt 0 ]; then
    echo "Warning: Found $count potential anti-patterns in knowledge base"
    echo "$result" | jq -r '.results[].title' | head -5
fi
```

### Makefile Integration

```makefile
check-docs:
	@terraphim-cli find "$$(cat docs/content.txt)" --format json | \
	jq -e '[.matches[] | select(.exists == false)] | length == 0' && \
	echo "Documentation check passed" || \
	echo "Missing concepts found"
```

## Troubleshooting

### Common Issues

**Connection Refused:**
```bash
# Check if server is running
terraphim-cli --server search "patterns"

# Or use offline mode
terraphim-agent search "patterns"  # Works offline
```

**No Results:**
```bash
# Check role configuration
terraphim-cli roles

# Verify knowledge graph
terraphim-repl
> /graph
```

**Performance Issues:**
```bash
# Use lighter tools
terraphim-cli search "patterns"  # Faster than agent

# Use quiet mode
terraphim-cli --quiet search "patterns"
```

### Getting Help

```bash
# General help
terraphim-agent --help
terraphim-cli --help
terraphim-repl --help

# Command-specific help
terraphim-agent search --help
terraphim-cli search --help

# In REPL
terraphim-repl
> /help
> /help search
```

## Advanced Topics

### Output Format Selection

**Human-Readable:**
```bash
terraphim-agent search "patterns"                    # Default
terraphim-cli search "patterns" --format text
```

**JSON for Processing:**
```bash
terraphim-agent --format json search "patterns"
terraphim-cli search "patterns"                       # Default
```

**Compact JSON for Piping:**
```bash
terraphim-agent --format json-compact search "patterns"
```

### Server vs Offline Mode

**Offline Mode (Default):**
```bash
# No server required
terraphim-agent search "patterns"
terraphim-cli search "patterns"
terraphim-repl
```

**Server Mode:**
```bash
# Requires running server
terraphim-agent --server search "patterns"
terraphim-cli --server search "patterns"
```

### Shell Completions

```bash
# Generate completions
terraphim-cli completions bash > ~/.bash_completion
terraphim-agent completions zsh > ~/.zsh/completion

# Source completions
source ~/.bash_completion
```

## Related Documentation

- [Installation Guide](./installation.md)
- [Configuration Reference](./configuration.md)
- [Knowledge Graph Documentation](./knowledge-graph.md)
- [API Documentation](../api/README.md)
- [CI/CD Integration](./ci-cd-migration.md)

## Version Information

- **terraphim-agent**: 1.0.0
- **terraphim-cli**: 1.0.0
- **terraphim-repl**: 1.0.0
- **Documentation Version**: 1.0.0

## Support

- **Issues**: https://github.com/terraphim/terraphim-ai/issues
- **Documentation**: https://terraphim.ai/docs
- **Discord**: https://terraphim.ai/discord
