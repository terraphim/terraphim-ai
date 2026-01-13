# Terraphim Session Analyzer (tsa/cla)

A powerful Rust-based CLI tool for analyzing AI coding assistant session logs to identify agent usage patterns, tool chains, and development insights. Supports Claude Code, Cursor, Aider, Codex, and OpenCode.

## Features

- **Tool Usage Analysis**: Track which tools are used most frequently across sessions
- **Agent-Tool Correlation**: Understand which AI agents use which tools
- **Tool Chain Detection**: Identify common sequences of tools used together
- **Knowledge Graph Search**: Semantic search across tool usage patterns
- **Multiple Export Formats**: JSON, CSV, Markdown, HTML, and terminal output
- **Timeline Visualization**: Interactive HTML timeline of tool usage
- **Real-time Monitoring**: Watch for new sessions as they're created
- **Self-Analysis**: Analyze how projects were built using Claude Code

## Quick Start

### Installation

```bash
# Install from crates.io
cargo install terraphim-session-analyzer

# Or build from source
git clone https://github.com/terraphim/terraphim-ai.git
cd terraphim-ai

# Build the release binary
cargo build --release --features terraphim

# Run the demo
./demo.sh
```

### Basic Usage

```bash
# Analyze all sessions
./target/release/cla tools

# Show tool chains
./target/release/cla tools --show-chains

# Show agent-tool correlation
./target/release/cla tools --show-correlation

# Filter by specific tool
./target/release/cla tools --tool Bash

# Export to JSON
./target/release/cla tools --format json -o output.json

# Generate timeline
./target/release/cla timeline --output timeline.html

# Watch for new sessions
./target/release/cla watch
```

## Demo Script

Run the automated demonstration to see all features in action:

```bash
./demo.sh
```

This will:
1. Build the release binary (if needed)
2. Analyze your Claude sessions
3. Generate outputs in all supported formats
4. Create an interactive timeline
5. Show tool chains and correlations
6. Save all results to `examples/` directory

See [DEMO.md](DEMO.md) for detailed information about the demo script.

## Commands

### `analyze`
Analyze sessions to identify agent usage patterns.

```bash
cla analyze                                          # Analyze all sessions
cla analyze --target "STATUS_IMPLEMENTATION.md"     # Find specific file
cla analyze --format json --output report.json     # Export to JSON
```

### `list`
List all available Claude sessions.

```bash
cla list                                            # List all sessions
cla list --verbose                                  # Show detailed info
```

### `summary`
Show summary statistics across all sessions.

```bash
cla summary                                         # Overall summary
cla summary --format json                          # JSON export
```

### `timeline`
Generate an interactive HTML timeline visualization.

```bash
cla timeline --output timeline.html                # Generate timeline
```

### `watch`
Monitor for new sessions in real-time.

```bash
cla watch                                          # Watch default directory
cla watch --session-dir /custom/path              # Watch custom directory
```

### `tools`
Analyze tool usage patterns (most powerful command).

```bash
# Basic analysis
cla tools                                          # All tools
cla tools --tool Bash                             # Filter by tool
cla tools --agent "code-editor"                   # Filter by agent

# Advanced analysis
cla tools --show-chains                           # Tool sequences
cla tools --show-correlation                      # Agent-tool matrix
cla tools --kg-search "deploy OR test"           # Knowledge graph search

# Sorting and filtering
cla tools --sort-by recent                        # Most recent first
cla tools --sort-by frequency                     # Most used first
cla tools --min-usage 10                         # Minimum usage count

# Export formats
cla tools --format json -o tools.json            # JSON
cla tools --format csv -o tools.csv              # CSV
cla tools --format markdown -o tools.md          # Markdown
cla tools --format html -o tools.html            # HTML

# Analyze specific session
cla tools ~/.claude/projects/YOUR-PROJECT-DIR/
```

## Output Formats

### Terminal (Default)
Colorized, human-readable output with tables and statistics.

### JSON
Machine-readable format for programmatic analysis:
```json
{
  "tools": [
    {
      "name": "Bash",
      "count": 1234,
      "percentage": 23.4,
      "agents": ["code-editor", "debugger"]
    }
  ]
}
```

### CSV
Spreadsheet-compatible format:
```csv
Tool,Count,Percentage,Agents
Bash,1234,23.4,"code-editor,debugger"
```

### Markdown
Documentation-friendly format with tables and summaries.

### HTML
Interactive timeline with charts and filtering capabilities.

## Configuration

### Environment Variables

- `CLAUDE_SESSION_DIR`: Override default session directory (default: `~/.claude/projects`)

### Command-line Options

- `--verbose, -v`: Enable verbose logging
- `--no-color`: Disable colored output
- `--session-dir, -d`: Specify custom session directory

## Use Cases

### 1. Understanding Tool Usage Patterns
```bash
cla tools --show-chains --min-usage 10
```
Identify which tools are commonly used together.

### 2. Agent Performance Analysis
```bash
cla tools --show-correlation
```
See which agents are most effective with different tools.

### 3. Project Development Insights
```bash
cla tools ~/.claude/projects/YOUR-PROJECT/ --show-chains
```
Analyze how a specific project was built.

### 4. Deployment Pattern Discovery
```bash
cla tools --kg-search "deploy OR publish OR release"
```
Find all deployment-related tool usage.

### 5. Continuous Monitoring
```bash
cla watch
```
Monitor Claude usage in real-time.

### 6. Reporting and Documentation
```bash
cla tools --format markdown -o report.md
cla timeline --output timeline.html
```
Generate reports for team sharing.

## Technical Details

### Session Storage

Claude Code stores sessions in `~/.claude/projects/`, with each project having:
- `session.jsonl`: Main session log
- `artifacts/`: Generated files
- Other metadata files

### Analysis Approach

The tool uses:
1. **Streaming JSON parsing** for memory efficiency
2. **Pattern matching** for tool chain detection
3. **Statistical analysis** for correlation matrices
4. **Knowledge graph** for semantic search (with `terraphim` feature)

### Features

- **terraphim**: Enable knowledge graph capabilities (optional)

Build without knowledge graph:
```bash
cargo build --release
```

Build with knowledge graph:
```bash
cargo build --release --features terraphim
```

## Development

### Running Tests

```bash
cargo test                                         # Run all tests
cargo test --features terraphim                   # With knowledge graph
```

### Code Quality

```bash
cargo clippy                                      # Linting
cargo fmt                                         # Format code
```

## Contributing

Contributions are welcome! Please:
1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run tests and linting
5. Submit a pull request

## License

See [LICENSE](LICENSE) for details.

## Related Projects

- **Terraphim**: Knowledge graph integration (optional feature)
- See [README_TERRAPHIM.md](README_TERRAPHIM.md) for details

## Support

For issues and questions:
- GitHub Issues: [https://github.com/zestic-ai/claude-log-analyzer/issues](https://github.com/zestic-ai/claude-log-analyzer/issues)
- Documentation: See `DEMO.md` and inline `--help` commands

## Acknowledgments

Built with:
- **Rust** - Systems programming language
- **serde_json** - JSON parsing
- **clap** - CLI argument parsing
- **comfy-table** - Terminal tables
- **terraphim** - Knowledge graph (optional)
