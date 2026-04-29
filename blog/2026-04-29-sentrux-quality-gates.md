---
title: "Introducing Sentrux Quality Gates: Closing the Feedback Loop on AI-Generated Code"
date: 2026-04-29
author: Terraphim Team
tags: [sentrux, quality-gates, ci-cd, adf, architecture]
---

# Introducing Sentrux Quality Gates: Closing the Feedback Loop on AI-Generated Code

Today we're excited to announce a major enhancement to the AI Dark Factory: **Sentrux quality gates** are now integrated across all Terraphim projects.

## The Problem: Code Quality at Machine Speed

AI agents write code faster than humans can review it. A single session might touch 50 files, introduce new dependencies, and refactor core modules — all in minutes. Without structural governance, codebases decay at machine speed:

- **Tangled dependencies** — Modules that started clean become spaghetti
- **God files** — Single files accumulate 36+ dependencies
- **Complexity creep** — Functions grow to 254+ cyclomatic complexity
- **Silent degradation** — Quality drops 500 points before anyone notices

Traditional CI catches syntax errors and test failures, but it doesn't catch *architectural* degradation. That's where Sentrux comes in.

## Meet Sentrux

[Sentrux](https://github.com/sentrux/sentrux) is a Rust-based structural quality sensor that watches your codebase in real-time. It computes a **quality signal** (0-10000) from 5 root cause metrics:

| Metric | What It Catches |
|--------|----------------|
| **Modularity** | God files, tight coupling, hotspots |
| **Acyclicity** | Circular dependencies |
| **Depth** | Deep dependency chains |
| **Equality** | Complexity concentration (Gini coefficient) |
| **Redundancy** | Dead code, duplicates |

Unlike linters that check style, Sentrux checks *structure*. It answers: "Does this change fit the system? Will this abstraction cause problems as the codebase grows?"

## Integration Overview

We've integrated Sentrux at three levels:

### 1. CI Quality Gates (Phase 2)

Every PR now runs a quality check:

```yaml
# .github/workflows/sentrux-quality-gate.yml
- name: Run Sentrux Check
  run: sentrux check .

- name: Check Delta
  run: |
    if [ $DELTA -lt -100 ]; then
      echo "Quality degraded! Blocking merge."
      exit 1
    fi
```

**Features:**
- ✅ Automatic baseline comparison
- ✅ PR comments with quality reports
- ✅ Merge blocking on degradation
- ✅ Per-project rules via `.sentrux/rules.toml`

### 2. Agent Integration (Phase 3)

The ADF build-runner now measures quality before and after builds:

```bash
# Before: quality = 7342
sentrux check .

# Build and test
cargo test

# After: quality = 6891 (degraded!)
sentrux check .
```

Build status now includes the quality signal: `fmt+clippy+test pass | quality: 5241`

### 3. MCP Server (Phase 3)

Agents can query Sentrux directly via MCP:

```
Agent: scan("/path/to/project")
  → { quality_signal: 7342, bottleneck: "modularity" }

Agent: session_start()
  → { status: "Baseline saved" }

... writes code ...

Agent: session_end()
  → { pass: false, 
      signal_before: 7342, 
      signal_after: 6891,
      summary: "Quality degraded" }
```

## Real Results

After scanning our own projects, here's what we found:

| Project | Quality | Key Finding |
|---------|---------|-------------|
| **gitea-robot** | **8648** | Clean Go codebase, minimal issues |
| **terraphim-ai** | **5241** | 1 god file (orchestrator/src/lib.rs, fan-out=36) |
| **gitea** | **3847** | Large upstream fork with accumulated debt |
| **atomic-server** | **3271** | Needs attention |

The terraphim-ai god file is already on our refactoring roadmap. Without Sentrux, we might not have quantified the impact so clearly.

## Rules Engine

Each project defines architectural constraints in `.sentrux/rules.toml`:

```toml
[constraints]
max_cycles = 5          # No more than 5 circular deps
max_cc = 30             # Functions max 30 complexity
max_file_lines = 500    # Files max 500 lines
no_god_files = true     # No files with >15 dependencies

[[layers]]
name = "core"
paths = ["src/core/*"]
order = 0

[[boundaries]]
from = "src/app/*"
to = "src/core/internal/*"
reason = "App must not depend on core internals"
```

Rules are checked on every PR. Violations block merge.

## Getting Started

### For Projects

1. **Install Sentrux:**
   ```bash
   brew install sentrux/tap/sentrux  # macOS
   curl -fsSL .../install.sh | sh    # Linux
   ```

2. **Create rules:**
   ```bash
   mkdir -p .sentrux
   cat > .sentrux/rules.toml << 'EOF'
   [constraints]
   max_cycles = 5
   max_cc = 30
   EOF
   ```

3. **Add CI workflow:** Copy `.github/workflows/sentrux-quality-gate.yml`

### For Agents

Add to your MCP config:

```json
"sentrux": {
  "type": "local",
  "command": ["sentrux", "--mcp"]
}
```

Then wrap code generation sessions:

```
session_start()
... generate code ...
session_end()  # Fails if quality degraded
```

## Why This Matters

AI agents are powerful but limited. They cannot hold the big picture and small details simultaneously. Sentrux gives them the sensor they need to:

1. **See structure** — Understand the codebase architecture
2. **Measure impact** — Quantify the effect of each change
3. **Self-correct** — Iterate when quality drops

This is the missing feedback loop. Compilers check syntax. Tests check behaviour. Linters check style. **Sentrux checks architecture.**

## What's Next

- **Dashboard** — Cross-project quality trends (Phase 4)
- **Auto-fix** — Agents that refactor based on Sentrux diagnostics
- **Custom metrics** — Project-specific quality heuristics

## References

- [Sentrux on GitHub](https://github.com/sentrux/sentrux)
- [Issue #1080](https://git.terraphim.cloud/terraphim/terraphim-ai/issues/1080) — Integration details
- [PR #1081](https://git.terraphim.cloud/terraphim/terraphim-ai/pulls/1081) — Quality gate workflow
- [Documentation](/docs/sentrux-integration.md)

---

*Quality gates are now active on all Terraphim projects. Expect PR checks to include structural quality reports starting today.*
