# Claude Log Analyzer - Demo Script

This directory contains an automated demonstration script that showcases all features of the Claude Log Analyzer.

## Quick Start

```bash
# Run the full demo
./demo.sh
```

The script will:
1. Build the release binary (if needed)
2. Create an `examples/` directory for outputs
3. Run through all major features
4. Generate sample outputs in multiple formats

## What Gets Demonstrated

### 1. Basic Tool Analysis
Shows overall tool usage statistics from your Claude sessions.

### 2. Tool-Specific Filtering
Demonstrates filtering by specific tool (e.g., Bash, Read, Edit).

### 3. Export Formats
Generates outputs in:
- **JSON** (`examples/tools.json`) - For programmatic analysis
- **CSV** (`examples/tools.csv`) - For spreadsheet import
- **Markdown** (`examples/tools.md`) - For documentation

### 4. Agent-Tool Correlation Matrix
Shows which AI agents use which tools most frequently.

### 5. Tool Chain Detection
Identifies common sequences of tools used together (e.g., Read → Edit → Write).

### 6. Knowledge Graph Search
Demonstrates semantic search across tool usage (e.g., finding deployment-related tools).

### 7. Sorting Methods
Shows different ways to sort results:
- By frequency (most used)
- By recency (most recent)
- Alphabetically

### 8. Self-Analysis
Analyzes how this very project was built using Claude Code.

### 9. Session Summary
Provides high-level statistics across all sessions.

### 10. Timeline Visualization
Generates an interactive HTML timeline of tool usage.

## Output Directory

All outputs are saved to `examples/`:

```
examples/
├── output-basic.txt           # Basic tool analysis
├── output-tool-filter.txt     # Tool-specific filtering
├── tools.json                 # JSON export
├── tools.csv                  # CSV export
├── tools.md                   # Markdown export
├── output-correlation.txt     # Agent-tool correlation
├── output-chains.txt          # Tool chains
├── output-kg-search.txt       # Knowledge graph search
├── output-sort-recent.txt     # Recent sorting
├── output-self-analysis.txt   # Self-analysis
├── output-summary.txt         # Summary statistics
└── timeline.html             # Interactive timeline
```

## Manual Feature Testing

If you want to test specific features manually:

```bash
# Basic tool analysis
./target/release/cla tools

# Filter by tool
./target/release/cla tools --tool Bash

# Show correlation matrix
./target/release/cla tools --show-correlation

# Show tool chains
./target/release/cla tools --show-chains

# Knowledge graph search
./target/release/cla tools --kg-search "deploy OR test"

# Export to JSON
./target/release/cla tools --format json -o output.json

# Analyze specific session
./target/release/cla tools ~/.claude/projects/YOUR-PROJECT-DIR/

# Generate timeline
./target/release/cla timeline --output timeline.html

# Watch for new sessions
./target/release/cla watch
```

## Requirements

- Rust toolchain (for building)
- Claude Code sessions in `~/.claude/projects/`
- At least one session file to analyze

## Expected Output

The demo takes approximately 10-30 seconds to complete, depending on:
- Number of sessions in your `~/.claude/projects/` directory
- Whether the release binary needs to be built
- System performance

## Troubleshooting

**No sessions found:**
- Ensure you have used Claude Code before
- Check that `~/.claude/projects/` exists and contains session directories

**Build errors:**
- Ensure Rust toolchain is installed: `rustup --version`
- Try cleaning and rebuilding: `cargo clean && cargo build --release --features terraphim`

**Permission errors:**
- Make script executable: `chmod +x demo.sh`
- Check read permissions on `~/.claude/projects/`

## Next Steps

After running the demo:

1. **Explore the JSON output** for programmatic integration:
   ```bash
   jq . examples/tools.json
   ```

2. **View the timeline** in your browser:
   ```bash
   open examples/timeline.html
   ```

3. **Analyze specific patterns** using the correlation matrix:
   ```bash
   cat examples/output-correlation.txt
   ```

4. **Integrate with your workflow** using the CSV export:
   ```bash
   open examples/tools.csv
   ```

## Demo Script Features

The script includes:
- **Color-coded output** for better readability
- **Timing information** for each step
- **Error handling** with graceful failures
- **Automatic directory creation** for outputs
- **Session count verification** before running
- **Progress indicators** throughout execution
