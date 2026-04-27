# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Token budget flags wired to `Search` command** in `terraphim_agent` CLI — `--max-tokens`,
  `--budget-mode`, and related flags now propagate to the robot-mode budget engine (Refs #672)

### Documentation
- Module-level `//!` docs added to 8 crates: `haystack_grepapp`, `terraphim-markdown-parser`,
  `terraphim_automata_py`, `terraphim_build_args`, `terraphim_ccusage`, `terraphim_lsp`,
  `terraphim_rolegraph_py`, `terraphim_usage`

---

## [1.15.0] - 2026-04-27

### Added

#### Shared Learning Store and Knowledge Graph Integration
- **`SharedLearningStore`** in `terraphim_agent` implementing `LearningStore` with hybrid graph scoring
- **L0 trust level** and `LearningStore` trait in `terraphim_types` (#846)
- **Learning store wired into orchestrator agent lifecycle** (#848, #849)
- **`LearningStore` on `SharedLearningStore`** with Aho-Corasick graph hybrid scoring (#850)

#### Robot Mode and Forgiving CLI
- **`RobotFormatter`** wired into CLI search; `ForgivingParser` integrated into REPL (#843)
- **Robot mode JSON output** in session commands (#840)
- **Token budget management engine** for robot mode (#707)
- **`BudgetEngine`, `FieldMode`, `OutputFormat`, `RobotConfig`, `RobotResponse`** types

#### Sessions and Real-time Ingestion
- **`SessionConnector::watch()`** for real-time session file ingestion (#839)
- **Auto-import sessions** on first use (#707)
- **Robot mode JSON output** in session commands

#### Codebase Evaluation
- **Manifest types and TOML/YAML loader** in `terraphim_codebase_eval` (#844)
- **`MetricsRunner`** with clippy, test, and tokei delta reporting

#### Firecracker CI Infrastructure
- **Firecracker-accelerated CI workflow** with dedicated Hetzner bigbox runner
- **Shared Rust build cache** via sccache + SeaweedFS S3 on fcbr0
- **VM-cargo-probe job** for end-to-end infrastructure validation
- **Rust-CI VM image** with sccache + SeaweedFS integration

#### Documentation (13 crates)
- Added `//!` module-level documentation to: `terraphim_agent`, `terraphim_service`,
  `terraphim_middleware`, `terraphim_rolegraph`, `terraphim_config`, `terraphim_persistence`,
  `terraphim_mcp_server`
- Added `//!` module-level documentation to: `terraphim_settings`, `haystack_core`,
  `haystack_atlassian`, `haystack_discourse`, `haystack_jmap`, `terraphim_kg_linter`,
  `terraphim_file_search`
- 1625 doc warnings fixed in prior run (2944 → 1319, 55% reduction)

#### Linear Tracker Integration
- **`LinearTracker`** for Linear.app issue lifecycle management

### Fixed
- **`RobotResponse` formatting** in chained command output
- **Exit classifier** trusts `exit_code=0` over pattern matches in orchestrator
- **Config lock release** during I/O to prevent deadlocks
- **`parse_chained_command`** identifies failing subcommand via exit code
- **LLM integration tests** for live environment
- **Desktop frontend** TypeScript errors resolved
- **Kimi and ZAI fallback routes** added to `review_tier` (#328)

### Changed
- **Build system**: `zlob` is now the default linker, with Zig 0.16 + Darwin workaround
- **Test performance**: Agent and server built once; binaries exec'd directly (no re-builds per test)
- **CI routing**: vm-cargo-probe routed through `/api/llm/execute`

### Dependencies
- `teloxide` 0.13 → 0.17
- `jiff` 0.1 → 0.2
- `dialoguer` 0.11 → 0.12
- `bollard` 0.18 → 0.20
- `tower` 0.4 → 0.5
- `config` 0.14 → 0.15
- `indicatif` 0.17 → 0.18

## [1.14.0] - 2026-03-22

### Added

#### Terraphim Symphony Orchestrator
- **New `terraphim_symphony` crate** for automated multi-agent issue dispatch
- **Claude Code runner** with full streaming protocol support
- **Dual-mode orchestration** combining issue-based and time-based scheduling
- **Workspace management** with git operations and context tracking
- **Gitea and Linear tracker integrations** for issue lifecycle management

#### Session File Tracking
- **File access extraction** from agent sessions (`extract_file_accesses`)
- **Files and ByFile subcommands** for session file queries
- **Sessions service methods** for querying by file path

#### Agent Workflows E2E
- **Complete workflow implementation** with 5 workflow templates
- **Real API integration** with Cerebras support
- **Playwright browser tests** for all workflows
- **Quality gate integration** with merge-to-main workflow

#### Orchestrator Improvements
- **Proportional fairness** scheduling algorithm
- **Dual-mode run loop** combining time-based and issue-based dispatch
- **Dependency-aware dispatch** with topological sorting
- **PageRank-aware issue sorting** for prioritization

#### Validation Framework
- **Phase 4/5 disciplined verification and validation** framework
- **Requirements traceability matrix** (REQ -> design -> code -> test)
- **Quality gate reports** for release readiness

### Fixed
- **axum-test 19.x API changes** - Updated test files
- **atty deprecation** - Replaced with `std::io::IsTerminal`
- **Symphony hook timeouts** - Increased for long-running operations
- **Liquid template issues** - Moved to heredoc approach

### Changed
- **TLA+ TypeScript bindings** research and design completed
- **Dependency updates** for axum-test, env_logger, actions/github-script

## [1.8.0] - 2026-02-16

### Added

#### Learning Capture System (Complete Implementation)
- **Core capture logic** (`crates/terraphim_agent/src/learnings/`):
  - `CapturedLearning` struct with full context (command, error output, exit code, timestamp)
  - `capture_failed_command()` with automatic secret redaction via regex
  - Chained command parsing (`cmd1 && cmd2 || cmd3`)
  - Markdown serialization/deserialization for persistent storage
  - `list_learnings()` - list recent learnings with [P]/[G] source indicators
  - `query_learnings()` - search by pattern (substring or exact match)
  - Hybrid storage model: project-specific (`.terraphim/learnings/`) with global fallback (`~/.local/share/terraphim/learnings/`)
  - **15 unit tests** covering all functionality

- **CLI Integration**:
  - `terraphim-agent learn capture <command> --error <msg> [--exit-code N]`
  - `terraphim-agent learn list [--recent N] [--global]`
  - `terraphim-agent learn query <pattern> [--exact] [--global]`
  - `terraphim-agent learn correct <id> --correction <text>` (placeholder)

- **Hook Integration**:
  - `.claude/hooks/learning-capture.sh` - PostToolUse hook for automatic capture
  - **Native Hook Support** (`terraphim-agent learn hook`):
    - Process hook input directly without bash wrapper
    - `terraphim-agent learn hook [--format=claude]` - Reads JSON from stdin
    - `terraphim-agent learn install-hook <claude|codex|opencode>` - One-command setup
    - No external dependencies (no jq required)
    - 156 tests covering hook functionality
  - Automatic capture of failed Bash commands
  - Secret redaction before storage
  - Fail-open design (continues even if capture fails)
  - Environment variable `TERRAPHIM_LEARN_DEBUG` for debug output

- **Documentation**:
  - `docs/src/kg/learnings-system.md` - User guide with architecture, usage, troubleshooting
  - `skills/learning-capture/skill.md` - AI agent skill reference
  - `docs/verification/learning-capture-verification-report.md` - Phase 4 verification
  - `docs/validation/learning-capture-validation-report.md` - Phase 5 validation

#### Guard Thesaurus-Driven Matching
- Replaced 12 static regex patterns with terraphim's Aho-Corasick engine
- JSON thesaurus files for extensible pattern matching:
  - `guard_destructive.json`: 32 entries across 13 concept categories
  - `guard_allowlist.json`: 10 entries across 5 concept categories
- Coverage for 20+ previously unblocked destructive commands:
  - `rmdir`, `chmod`, `chown`, `shred`, `truncate`, `dd`, `mkfs`, `fdisk`
  - `git commit --no-verify`, `git push --no-verify`
  - `git reset --merge`, `git restore --worktree`, `git stash clear`
  - `git branch -D`, `rm -fr`
- CLI flags for custom patterns: `--guard-thesaurus`, `--guard-allowlist`
- **36 tests** for guard patterns

#### Performance Improvements
- **HTTP Client**: Connection pooling with global static clients
  - 30-50% reduction in connection overhead
  - Connection pooling (10 idle per host, 90s timeout)
  - Functions: `get_default_client()`, `get_api_client()`, `get_scraping_client()`
- **Summarization Worker**: 
  - Use `VecDeque` for O(1) removal (was O(n) with Vec)
  - Implemented retry re-queuing with exponential backoff
  - Removed unused `active_workers` counter

#### Documentation
- `crates/terraphim_agent/DESIGN-guard-patterns-redesign.md` - Guard pattern redesign document
- Graph embeddings learnings examples
- Learning via negativa examples for command corrections
- Validation reports for agent, CLI, and tinyclaw crates

### Changed
- OpenRouter: Removed unreachable truncation logic (dead code elimination)
- Hooks: Avoid false positives in secret detection

### Fixed
- Fixed regex patterns in learning capture redaction to handle dashes in API keys
- Workspace: Exclude `terraphim_automata_py` (PyO3 extension - use `maturin develop`)

## [1.7.0] - 2026-02-10

### Added
- Initial learning capture specification
- Guard patterns for command protection
- HTTP client improvements

## [1.6.0] - 2026-01-XX

### Added
- Base implementation of terraphim-agent
- RoleGraph functionality
- Knowledge graph ranking

[1.8.0]: https://github.com/terraphim/terraphim-ai/compare/v1.7.0...v1.8.0
[1.7.0]: https://github.com/terraphim/terraphim-ai/compare/v1.6.0...v1.7.0
