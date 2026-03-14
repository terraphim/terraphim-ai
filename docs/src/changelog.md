# Changelog

This is a chronological changelog for Terraphim AI.

## 2026

### v1.13.0

Release date: 2026-03-14

Highlights:
- **Symphony Orchestrator** -- Autonomous daemon that polls issue trackers (Gitea, Linear) and dispatches coding agents per issue
- **Claude Code Runner** -- New `ClaudeCodeSession` runner using `claude -p` CLI with NDJSON event stream parsing
- **Codex Runner** -- JSON-RPC app-server runner with handshake, multi-turn, and approval flows
- **Workspace Isolation** -- Per-issue workspace directories with lifecycle hooks (`after_create`, `before_run`, `after_run`, `before_remove`)
- **WORKFLOW.md Configuration** -- Single-file configuration with YAML front matter and Liquid prompt templates
- **HTTP Dashboard** -- Real-time view of running sessions, retry queue, and token usage (feature-gated)
- **Hot Reload** -- WORKFLOW.md file watching with live config updates
- **Exponential Backoff** -- Retry failed sessions with configurable backoff (10s, 20s, 40s... capped at 5 minutes)
- **Stall Detection** -- Automatic session termination after configurable inactivity period
- **PageRank Viewer** -- First production deployment: Symphony built a complete web application from six Gitea issues using Claude Code agents

Reference:
- [Symphony QUICKSTART](../crates/terraphim_symphony/QUICKSTART.md)
- [PageRank Viewer Case Study](./case-studies/symphony-pagerank-viewer.md)

### v1.9.0

Release date: 2026-02-20

Highlights:
- **Dynamic Ontology** - Schema-first knowledge graph construction with coverage governance signals
- **GroundingMetadata** - Canonical URIs for normalized entities
- **CoverageSignal** - Quality governance signals for extraction quality
- **SchemaSignal** - Entity extraction with confidence scores
- **HgncNormalizer** - Gene normalization (EGFR, TP53, KRAS, etc.)
- **Feature Gates** - `ontology` (default), `medical`, `hgnc` for flexible dependencies

Published to crates.io:
```toml
terraphim_types = "1.6.0"
```

Reference:
- https://github.com/terraphim/terraphim-ai/releases/tag/v1.9.0

### v1.2.3

All binaries are available at:
- https://github.com/terraphim/terraphim-ai/releases/tag/v1.2.3

## 2025

### v1.0.0

Release date: 2025-11-16

Highlights:
- First stable release with multi-language packages (Rust, Node.js, Python)
- Enhanced search (knowledge graph + grep.app integration)
- MCP server and Claude Code hooks
- CI/CD infrastructure for multi-platform builds

Reference notes (verbatim source):
- `docs/archive/RELEASE_NOTES_v1.0.0.md`

### v1.0.2

Release date: 2025-11-05

Highlights:
- Comprehensive testing and validation for TUI REPL
- Multi-platform macOS artifacts (universal + aarch64 + x86_64)
- Role switching and search validation

Reference notes (verbatim source):
- `docs/archive/RELEASE_NOTES_v1.0.2.md`
