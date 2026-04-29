# Sentrux Quality Gates

## Overview

Terraphim AI now integrates with [Sentrux](https://github.com/sentrux/sentrux) — a Rust-based structural quality analysis engine — to provide automated quality gates for all ADF (AI Dark Factory) projects.

## What is Sentrux?

Sentrux is a code quality sensor that:
- Scans codebases in real-time (52 languages via tree-sitter)
- Computes a quality signal (0-10000) from 5 root cause metrics
- Provides actionable diagnostics to agents via MCP (Model Context Protocol)
- Enforces architectural constraints through a rules engine

## Integration Architecture

```
ADF Agent / CI Pipeline
         |
         v
   [Sentrux Scan]
         |
    +----+----+
    |         |
Quality    Rules
Signal     Check
    |         |
    +----+----+
         |
    [Baseline]
    Comparison
         |
    +----+----+
    |         |
Pass      Fail
    |         |
    v         v
  Merge   Block
```

## Quality Metrics

Sentrux computes a quality signal from 5 root cause metrics:

| Metric | Description | Target |
|--------|-------------|--------|
| **Modularity** | Coupling between modules | Low coupling |
| **Acyclicity** | Circular dependencies | Zero cycles |
| **Depth** | Dependency chain depth | Shallow chains |
| **Equality** | Complexity distribution | Even distribution |
| **Redundancy** | Dead/duplicate code | Minimal redundancy |

Quality Signal = Geometric mean of all 5 metrics (0-10000 scale)

## CI Integration

### Workflow

The `.github/workflows/sentrux-quality-gate.yml` workflow runs on every PR:

1. **Scan** — `sentrux check .` computes quality signal
2. **Baseline** — Fetches baseline from Gitea wiki
3. **Delta** — Compares current vs baseline
4. **Block** — Fails CI if delta < -100 or rule violations exist
5. **Report** — Posts quality report as PR comment

### Configuration

Each project has a `.sentrux/rules.toml` file:

```toml
[constraints]
max_cycles = 5          # Maximum allowed circular dependencies
max_cc = 30             # Maximum cyclomatic complexity
max_file_lines = 500    # Maximum file length
no_god_files = true     # No files with fan-out > 15

[[layers]]
name = "core"
paths = ["src/core/*"]
order = 0

[[layers]]
name = "app"
paths = ["src/app/*"]
order = 2

[[boundaries]]
from = "src/app/*"
to = "src/core/internal/*"
reason = "App must not depend on core internals"
```

## Agent Integration

### Build-Runner

The ADF build-runner now includes quality checks:

```bash
# Before build
sentrux check .  # Establish baseline

# Build steps
cargo test

# After build
sentrux check .  # Measure impact
```

Build status includes quality signal: `fmt+clippy+test pass | quality: 5241`

### MCP Server

Sentrux exposes 9 MCP tools for agent integration:

| Tool | Purpose |
|------|---------|
| `scan` | Initial scan, returns quality_signal |
| `health` | Root cause breakdown |
| `session_start` | Save baseline |
| `session_end` | Compare against baseline |
| `rescan` | Incremental updates |
| `check_rules` | Validate constraints |
| `evolution` | Git history analysis |
| `dsm` | Design Structure Matrix |
| `test_gaps` | Untested file detection |

### Agent Workflow

```
Agent: scan("/path/to/project")
  → { quality_signal: 7342, bottleneck: "modularity" }

Agent: session_start()
  → { status: "Baseline saved", quality_signal: 7342 }

... agent writes code ...

Agent: session_end()
  → { pass: false, signal_before: 7342, signal_after: 6891,
      summary: "Quality degraded during this session" }
```

## Current Baselines

| Project | Language | Quality | Violations |
|---------|----------|---------|------------|
| gitea-robot | Go | **8648** | 2 |
| terraphim-ai | Rust | **5241** | 3 |
| gitea | Go | **3847** | 3 |
| atomic-server | Rust | **3271** | 4 |

## Installation

### Local Development

```bash
# macOS
brew install sentrux/tap/sentrux

# Linux
curl -fsSL https://raw.githubusercontent.com/sentrux/sentrux/main/install.sh | sh
```

### MCP Configuration

**opencode** (`~/.config/opencode/opencode.json`):
```json
"sentrux": {
  "type": "local",
  "command": ["/opt/homebrew/bin/sentrux", "--mcp"]
}
```

**Claude Code** (`~/.claude/settings.local.json`):
```json
"permissions": {
  "allow": ["Bash(sentrux *)"]
}
```

## Usage

### CLI

```bash
# Check quality
sentrux check .

# Save baseline
sentrux gate --save .

# Compare against baseline
sentrux gate .

# Start MCP server
sentrux --mcp
```

### GUI

```bash
sentrux              # Open GUI
sentrux /path/to/project  # Scan specific directory
```

## Troubleshooting

### No rules file found

Create `.sentrux/rules.toml` in your project root:

```toml
[constraints]
max_cycles = 5
max_cc = 30
```

### High violation count

1. Check if generated files are included (dist/, node_modules/)
2. Add `.sentruxignore` to exclude files
3. Adjust thresholds in rules.toml

### Baseline not found

Baselines are stored in Gitea wiki pages. Ensure:
1. Gitea token has wiki access
2. Wiki is enabled for the repository

## References

- [Sentrux Repository](https://github.com/sentrux/sentrux)
- [Sentrux Documentation](https://github.com/sentrux/sentrux/tree/main/docs)
- [Issue #1080](https://git.terraphim.cloud/terraphim/terraphim-ai/issues/1080) — Integration tracking
- [PR #1081](https://git.terraphim.cloud/terraphim/terraphim-ai/pulls/1081) — Quality gate workflow
