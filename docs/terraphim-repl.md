# terraphim-repl Documentation

## Overview

**terraphim-repl** is the interactive Read-Eval-Print Loop (REPL) shell for Terraphim AI, designed for rapid exploration, testing, and learning. It provides a lightweight alternative to the full TUI interface with quick command execution.

### Quick Facts

- **Binary Size**: 15 MB
- **Type**: Interactive REPL Shell
- **Mode**: Offline by default
- **Version**: 1.0.0

## Installation

terraphim-repl is included in the Terraphim AI distribution. For installation details, see the [Installation Guide](./installation.md).

```bash
# Verify installation
terraphim-repl --version
# Output: terraphim-repl 1.0.0
```

## Getting Started

### Starting the REPL

```bash
terraphim-repl
```

**Output:**
```
============================================================
Terraphim REPL v1.0.0
============================================================
Type /help for help, /quit to exit
Mode: Offline Mode | Current Role: Engineer

Available commands:
  /search <query> - Search documents
  /config show - Display configuration
  /role [list|select] - Manage roles
  /graph - Show knowledge graph
  /replace <text> - Replace terms with links
  /find <text> - Find matched terms
  /thesaurus - View thesaurus
  /help [command] - Show help
  /quit - Exit REPL

>
```

### Exiting the REPL

```bash
> /quit
Goodbye! 👋
```

Or press `Ctrl+D` (EOF).

## REPL Commands

### 1. /search

Search the knowledge graph.

```bash
> /search <QUERY>
> /search async rust patterns
> /search "machine learning"
```

**Examples:**

```
> /search error handling
Found 5 results:
1. Error Handling in Rust (score: 0.95)
2. Result and Option Types (score: 0.89)
3. Custom Error Types (score: 0.87)
...

> /search "async patterns"
Searching for: "async patterns"
Results:
- Async/Await Best Practices
- Error Handling in Async Code
- Performance Optimization Patterns
```

### 2. /config

Display or modify configuration.

```bash
# Show current configuration
> /config show

# Set configuration value
> /config set <KEY> <VALUE>
> /config set role Engineer

# Reset to defaults
> /config reset
```

**Example:**

```
> /config show
Current Configuration:
- Role: Engineer
- Format: human
- Server: localhost:8000
- Timeout: 30s
```

### 3. /role

Manage available roles.

```bash
# List all roles
> /role list

# Select a role
> /role select <NAME>
> /role select Engineer

# Show current role
> /role current
```

**Example Session:**

```
> /role list
Available Roles:
1. Engineer (default)
2. Default
3. Reviewer
4. Security

> /role select Reviewer
Role changed to: Reviewer

> /role current
Current Role: Reviewer
```

### 4. /graph

Display knowledge graph top concepts.

```bash
> /graph
> /graph --top 20
```

**Example:**

```
> /graph
Top Concepts:
1. async (score: 0.95) → await, tokio, async-std
2. error handling (score: 0.89) → Result, ?, thiserror
3. performance (score: 0.87) → optimization, speed, profiling
4. testing (score: 0.85) → mocks, benches, proptest
5. lifetimes (score: 0.83) → 'static, borrowing, ownership
```

### 5. /replace

Replace matched terms with links.

```bash
> /replace <TEXT>
> /replace Using async patterns for better performance
```

**Example:**

```
> /replace Error handling in async Rust
Original: Error handling in async Rust
Replaced: [Error handling](/concepts/error-handling) in [async Rust](/concepts/async)

Matches:
- "Error handling" → exists (score: 0.95)
- "async Rust" → exists (score: 0.92)
```

### 6. /find

Find matched terms in text.

```bash
> /find <TEXT>
> /find This code uses async/await for concurrency
```

**Example:**

```
> /find Error handling with Result types in async code
Text: Error handling with Result types in async code

Matches:
✓ Error handling (score: 0.95)
✓ Result types (score: 0.91)
✓ async code (score: 0.88)
✓ concurrency (score: 0.85)

Found: 4 | Missing: 0
```

### 7. /thesaurus

View thesaurus terms.

```bash
# Show all terms
> /thesaurus

# Search for a term
> /thesaurus async
> /thesaurus performance
```

**Example:**

```
> /thesaurus async
Thesaurus for "async":
- concurrent
- parallel
- non-blocking
- asynchronous

Related terms:
- await
- future
- task
```

### 8. /help

Get help on commands.

```bash
# General help
> /help

# Help for specific command
> /help search
> /help role
> /help graph
```

**Example:**

```
> /help search
/search - Search the knowledge graph

Usage:
  /search <query>

Options:
  --json    Output in JSON format
  --limit N Limit results to N items

Examples:
  /search async patterns
  /search "error handling" --json --limit 10

> /help
Available Commands:
  /search <query>    - Search documents
  /config show       - Display configuration
  /role [list|select] - Manage roles
  /graph             - Show knowledge graph
  /replace <text>    - Replace terms with links
  /find <text>       - Find matched terms
  /thesaurus         - View thesaurus
  /help [command]    - Show help
  /quit              - Exit REPL
```

### 9. /quit

Exit the REPL.

```bash
> /quit
Goodbye! 👋
```

## Interactive Features

### Command History

The REPL maintains command history across sessions:

```bash
> /search pattern1
> /search pattern2
# Press Up arrow to see previous commands
```

**Navigation:**
- `Up/Down arrows`: Navigate history
- `Ctrl+R`: Reverse search history
- `Ctrl+C`: Cancel current input

### Tab Completion

Commands support tab completion:

```bash
> /se<TAB>     # Completes to /search
> /role li<TAB> # Completes to /role list
```

### Multi-line Input

For complex queries, use multi-line input:

```bash
> /search "async rust"
  --limit 10
  --format json
```

### Output Formatting

**Human Format (Default):**

```
> /search async patterns
Found 5 results:
1. Async Rust Patterns (score: 0.95)
   Snippet: Best practices for async/await...
   Role: Engineer

2. Error Handling in Async (score: 0.89)
   Snippet: Managing errors in async code...
   Role: Engineer
```

**JSON Format:**

```
> /search async patterns --json
{
  "results": [...],
  "metadata": {...}
}
```

## Use Cases

### 1. Quick Exploration

```bash
terraphim-repl

> /role list
> /search "performance optimization"
> /graph
> /thesaurus async
> /quit
```

### 2. Learning Terraphim

```bash
terraphim-repl

> /help
> /help search
> /search "basic patterns"
> /find "test code"
> /graph
> /quit
```

### 3. Testing Search Patterns

```bash
terraphim-repl

> /search "error handling"
> /search "Error handling"
> /search "errors"
> /search "exception"
> /find "This function returns a Result type"
> /quit
```

### 4. Documentation Validation

```bash
terraphim-repl

> /find "The async pattern requires proper error handling"
> /replace "Using Result types for error handling in async code"
> /graph
> /quit
```

### 5. Role Comparison

```bash
terraphim-repl

> /role select Engineer
> /search "API design"
> /role select Reviewer
> /search "API design"
> /role select Security
> /search "API design"
> /quit
```

## Configuration

### Default Configuration

On first run, terraphim-repl creates a default configuration:

```json
{
  "role": "Engineer",
  "format": "human",
  "server_url": "http://localhost:8000",
  "history_size": 1000,
  "colors": true,
  "prompt": "> "
}
```

### Setting Configuration

```bash
> /config set role Engineer
> /config set format json
> /config set colors false
```

### Environment Variables

```bash
# Default role
TERRAPHIM_DEFAULT_ROLE=Engineer

# Server URL
TERRAPHIM_SERVER_URL=http://localhost:8000

# History file
TERRAPHIM_HISTORY=~/.terraphim/repl-history

# Log level
RUST_LOG=info
```

## Tips and Tricks

### 1. Quick Searches

```bash
# Single-word searches
> /search async

# Phrase searches
> /search "async rust"

# Multiple words
> /search async rust patterns
```

### 2. Output Redirection

```bash
# Save results to file
> /search "patterns" > results.txt

# Save JSON output
> /search "patterns" --json > results.json

# Pipe to other tools
> /graph | grep async
```

### 3. Aliases

Create shell aliases for common commands:

```bash
# ~/.bashrc or ~/.zshrc
alias ts='terraphim-repl'
alias tsq='terraphim-repl -c "search" -q'
```

### 4. Script Mode

Run scripts non-interactively:

```bash
terraphim-repl <<EOF
/search "patterns"
/graph
/quit
EOF
```

### 5. Batch Mode

Execute multiple commands:

```bash
terraphim-repl -c "/search pattern1; /search pattern2; /quit"
```

## Troubleshooting

### Common Issues

**1. Connection Refused**

```bash
# Error: Could not connect to server
# Solution: Use offline mode or check server
terraphim-repl
> /config show
# Check server_url setting
```

**2. Empty Results**

```bash
# No results found
# Solution: Check role configuration
> /role list
> /role current
> /config show
```

**3. Slow Performance**

```bash
# REPL is slow
# Solution: Use offline mode
> /config set server_url ""
```

### Debug Mode

```bash
# Enable debug logging
RUST_LOG=debug terraphim-repl

# Save logs
RUST_LOG=trace terraphim-repl 2>&1 | tee repl-debug.log
```

### Reset Configuration

```bash
# Reset to defaults
> /config reset

# Or delete config file
rm ~/.terraphim/repl-config.json
terraphim-repl
```

## Comparison with Other Tools

| Feature | terraphim-repl | terraphim-agent | terraphim-cli |
|---------|----------------|-----------------|---------------|
| Interactivity | High | Very High | None |
| Learning curve | Low | Medium | Low |
| Speed | Fast | Medium | Very Fast |
| Feature set | Core | Full | Full |
| Scriptability | Low | Medium | High |
| Best for | Learning/Exploration | Daily use | Automation |

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+C` | Cancel input |
| `Ctrl+D` | Exit (EOF) |
| `Ctrl+L` | Clear screen |
| `Up/Down` | History navigation |
| `Tab` | Autocomplete |
| `Ctrl+R` | Reverse search |

## See Also

- [CLI Tools Overview](./cli-tools-overview.md)
- [terraphim-agent Documentation](./terraphim-agent.md)
- [terraphim-cli Documentation](./terraphim-cli.md)
- [Installation Guide](./installation.md)
- [Knowledge Graph Documentation](./knowledge-graph.md)
