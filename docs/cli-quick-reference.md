# CLI Tools Quick Reference

## Command Quick Reference

### terraphim-agent

| Command | Description | Example |
|---------|-------------|---------|
| `search <query>` | Search knowledge graph | `terraphim-agent search "async rust"` |
| `roles` | List available roles | `terraphim-agent roles` |
| `config` | Show configuration | `terraphim-agent config` |
| `graph` | Display knowledge graph | `terraphim-agent graph` |
| `chat` | Start LLM chat | `terraphim-agent chat` |
| `extract <text>` | Extract entities | `terraphim-agent extract "text"` |
| `replace <text>` | Replace terms with links | `terraphim-agent replace "text"` |
| `validate <text>` | Validate against KG | `terraphim-agent validate "text"` |
| `suggest <term>` | Fuzzy match suggestions | `terraphim-agent suggest "asynch"` |
| `hook` | Claude Code integration | `terraphim-agent hook --operation search` |
| `guard <cmd>` | Safety check command | `terraphim-agent guard "rm -rf /"` |
| `interactive` | Full TUI mode | `terraphim-agent interactive` |
| `check-update` | Check for updates | `terraphim-agent check-update` |
| `update` | Update application | `terraphim-agent update` |

### terraphim-cli

| Command | Description | Example |
|---------|-------------|---------|
| `search <query>` | Search knowledge graph | `terraphim-cli search "patterns"` |
| `config` | Show configuration | `terraphim-cli config` |
| `roles` | List available roles | `terraphim-cli roles` |
| `graph` | Display knowledge graph | `terraphim-cli graph` |
| `replace <text>` | Replace terms with links | `terraphim-cli replace "text"` |
| `find <text>` | Find matched terms | `terraphim-cli find "text"` |
| `thesaurus` | Show thesaurus | `terraphim-cli thesaurus` |
| `completions` | Generate shell completions | `terraphim-cli completions bash` |

### terraphim-repl

| Command | Description | Example |
|---------|-------------|---------|
| `/search <query>` | Search knowledge graph | `/search async patterns` |
| `/config show` | Display configuration | `/config show` |
| `/role list` | List available roles | `/role list` |
| `/role select <name>` | Switch role | `/role select Engineer` |
| `/graph` | Display knowledge graph | `/graph` |
| `/replace <text>` | Replace terms with links | `/replace text` |
| `/find <text>` | Find matched terms | `/find text` |
| `/thesaurus` | Show thesaurus | `/thesaurus` |
| `/help [cmd]` | Get help | `/help search` |
| `/quit` | Exit REPL | `/quit` |

## Option Quick Reference

### Global Options

| Option | terraphim-agent | terraphim-cli | terraphim-repl |
|--------|----------------|---------------|----------------|
| `--help, -h` | ✅ | ✅ | ✅ |
| `--version, -V` | ✅ | ✅ | ❌ |
| `--format <fmt>` | ✅ | ✅ | ✅ |
| `--quiet` | ❌ | ✅ | ❌ |
| `--server` | ✅ | ❌ | ❌ |
| `--server-url <url>` | ✅ | ❌ | ❌ |
| `--transparent` | ✅ | ❌ | ❌ |
| `--robot` | ✅ | ❌ | ❌ |

### Output Formats

| Format | Description | Example |
|--------|-------------|---------|
| `human` | Human-readable (default for agent, repl) | `--format human` |
| `json` | Pretty JSON | `--format json` |
| `json-compact` | Compact JSON | `--format json-compact` |
| `text` | Plain text | `--format text` |

## Exit Codes

| Code | Meaning | terraphim-agent | terraphim-cli |
|------|---------|-----------------|---------------|
| 0 | Success | ✅ | ✅ |
| 1 | No results | ✅ (robot mode) | ✅ |
| 2 | Error | ✅ (robot mode) | ✅ |
| 3 | Invalid arguments | ✅ (robot mode) | ❌ |

## Environment Variables

```bash
# All tools
TERRAPHIM_SERVER_URL     # Server URL (default: http://localhost:8000)
TERRAPHIM_API_KEY        # API key for LLM features
TERRAPHIM_DATA_DIR       # Data directory (default: ~/.terraphim)
RUST_LOG                 # Log level (info, debug, trace)
```

## Configuration Files

```bash
~/.terraphim/
├── config.json          # terraphim-agent config
├── cli-config.json      # terraphim-cli config
├── repl-config.json     # terraphim-repl config
└── default_thesaurus.json
```

## Keyboard Shortcuts (REPL)

| Shortcut | Action |
|----------|--------|
| `Up/Down` | Navigate history |
| `Tab` | Autocomplete |
| `Ctrl+C` | Cancel input |
| `Ctrl+D` | Exit (EOF) |
| `Ctrl+L` | Clear screen |
| `Ctrl+R` | Reverse search |

## Keyboard Shortcuts (TUI)

| Shortcut | Action |
|----------|--------|
| `/` | Start search |
| `Ctrl+R` | Switch role |
| `Ctrl+G` | Show graph |
| `Ctrl+C` | Quit |
| `Esc` | Cancel/Back |

## Common Workflows

### Quick Search
```bash
# Most concise
terraphim-cli search "patterns"

# With JSON output
terraphim-cli search "patterns" --format json
```

### Interactive Exploration
```bash
# Start REPL
terraphim-repl

# Inside REPL:
/search patterns
/graph
/quit
```

### Full TUI Experience
```bash
# Start full interface
terraphim-agent interactive

# Or single command
terraphim-agent search "patterns"
```

### Automation Script
```bash
#!/bin/bash
result=$(terraphim-cli --quiet search "patterns" --format json)
count=$(echo "$result" | jq '.metadata.total_results')
echo "Found $count patterns"
```

### Safety Check
```bash
# Check if command is safe
terraphim-agent guard "rm -rf node_modules"

# In script
if terraphim-agent guard "dangerous command"; then
    echo "Safe to proceed"
else
    echo "Blocked by safety guard"
fi
```

## Size Comparison

| Tool | Binary Size | Startup Time | Memory Usage |
|------|-------------|--------------|--------------|
| terraphim-agent | 17 MB | ~1s | 100-200 MB |
| terraphim-cli | 15 MB | ~0.5s | 50-100 MB |
| terraphim-repl | 15 MB | ~0.5s | 50-100 MB |

## Performance Tips

1. **Use terraphim-cli for scripts** - Fastest startup, lowest overhead
2. **Use --quiet mode** - Reduces output processing
3. **Use JSON-compact for piping** - Smaller output
4. **Use offline mode** - No network latency
5. **Limit results** - Pipe to `head` or use `--limit`

## File Locations

```bash
# Linux/macOS
~/.terraphim/config.json

# Windows
%USERPROFILE%\.terraphim\config.json

# Data directory
~/.terraphim/data/
```

## Getting Help

```bash
# General help
tool --help

# Command help
tool command --help

# In REPL
> /help
> /help command
```

## Related Documentation

- [CLI Tools Overview](./cli-tools-overview.md)
- [terraphim-agent Documentation](./terraphim-agent.md)
- [terraphim-cli Documentation](./terraphim-cli.md)
- [terraphim-repl Documentation](./terraphim-repl.md)
- [Installation Guide](./installation.md)
