# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **Trigger index and success capture** wired in agent and middleware (Refs #1066)
- **Session debouncing** for `SessionConnector::watch()` to eliminate duplicate emissions (Refs #815)
- **LLM pre/post hooks** wired in agent command handlers for multi-agent coordination (Refs #451)
- **Self-Documentation API** exposed via robot CLI subcommand (Refs #1011)
- **ForgivingParser** integrated into CLI command dispatch for typo-tolerant command parsing (Refs #1012)
- **MS Teams SDK test suite** with comprehensive SDK tests (Refs #1034)
- **Tantivy index** for session search with BM25 ranking (Refs #1039)
- **Token budget flags** wired to Search command for robot mode output control (Refs #672)
- **JSON format support** on roles/config/graph commands in robot mode (Refs #1013)
- **ResponseMeta** extended with `query` and `role` fields for richer robot output (Refs #1026)
- **ADF operations guide** and blog post for PR fan-out deployment
- **ADF agent fleet reference** documentation
- **PR security sentinel** agent template for automated security review on PR open (Phase 2c) (Refs #953)
- **PR compliance watchdog** agent template for compliance validation on PR open (Phase 2d) (Refs #955)
- **PR test guardian** agent template for test gate enforcement on PR open (Phase 2e) (Refs #954)
- **Per-project PR dispatch** via `IncludeFragment` for scoped agent spawning (Refs #962)
- **Spawner task-body fix** -- agents now spawned with TOML task body, not runtime task_string (Refs #1020)

### Fixed

- **ThesaurusResponse** aligned with server API contract -- `thesaurus` field is now `Option<HashMap<String,String>>` matching actual server response shape (Fixes #1092)
- **RUSTSEC-2026-0049** eliminated by switching serenity to native-tls (Refs #418)
- **Spec gaps** addressed and resolved across ADF orchestrator templates (Refs #1040)
- **Global concurrency limits** enforced in orchestrator to prevent task/memory exhaustion (Refs #664)
- **listen_mode test assertion** updated to match clap error output (Refs #1044)
- **Robot response formatting** corrected in chaining logic
- **MCP latency benchmarks** gated behind release builds to prevent debug flake (Refs #672, Refs #987)
- **Pagination and token budget** wired into search response with test alignments (Refs #672)
- **Performance benchmark thresholds** raised and redundant attributes removed (Refs #987)
- **Clippy warnings** resolved across workspace crates
- **Duplicate test functions** resolved after origin merge
- **Robot search output contract** regression fixed (Refs #905)
- **Spawner task-body** -- agents now receive proper TOML task body instead of runtime string (Refs #1020)

### Changed

- **Robot mode** now honours --format json for consistent machine-readable output
- **Orchestrator** uses KG-routed model in Quickwit logs and AgentRunRecord
- **CLI exit codes** aligned with F1.2 contract (typed ExitCode, not bare exit(1)) (Refs #860)
- **Learning store** implemented on SharedLearningStore with markdown backend
- **Token budget management** engine active for robot mode output control
- **PR dispatch** refactored to use `IncludeFragment` for per-project scoping (Refs #962)
- **Test ranking knowledge graph fixture** added for agent testing
- **LLM cost tracking** foundation with genai fork integration (Refs #1075)
- **Spec validation** report for 2026-04-29 documenting 3 fixed, 5 remaining gaps
- **Documentation gap report** generated for 2026-04-29 identifying 43 warnings across workspace
- **Sentrux quality gate** CI workflow added for automated compliance checks
- **Orchestrator quota-to-fallback v2** with Graph router respawn on quota exhaustion (Refs #1084)
- **user-prompt-submit hook** wired into Terraphim AI and OpenCode plugins (Refs #674)
- **LLM usage Phases B+C** -- history grouping, spend aggregation, and budget alerts (Refs #1075)
- **Crate-level rustdoc** added to terraphim_service, terraphim_middleware, terraphim_config, terraphim_persistence, terraphim_agent
- **Single-gap rustdoc** fixed in haystack_core, terraphim_cli, terraphim_mcp_server, terraphim_agent_evolution, terraphim_github_runner_server, terraphim_kg_orchestration, terraphim_onepassword_cli, terraphim_router
- **Session connector rustdoc** added to ClineConnector, ClineMessage, ModelInfo, ClaCursorConnector, and SessionMetadata::new in terraphim_sessions
- **Documentation gap audit** 2026-04-30: 700 undocumented public items across 41 crates identified (Theme-ID: doc-gap)

### Fixed

- **`--server` flag** on listen subcommand now routes through custom error handler
- **Orchestrator PR review** P1/P2 findings addressed
- **multi_agent pool shutdown** -- `flush_usage` now called on pool shutdown
- **Usage provider queries** -- JSON wrapping and clap arg clash resolved

## [1.17.0] - 2026-04-27

### Added

#### ADF Orchestrator Framework
- **New `terraphim_orchestrator` crate** (v1.8.0) -- Autonomous Development Flow orchestrator
- **Multi-agent coordination** with mention-chain support for Gitea
- **PR lifecycle management** -- fan-out to build-runner + pr-reviewer on PR open
- **Push webhook handling** with build-runner spawning
- **Compound-RICE scoring** with Themis persona for issue prioritisation
- **Runtime guardian, upstream synchronizer, and meta-learning agent templates**

#### Session Connectors
- **OpenCode and Codex JSONL session connectors** for real-time session ingestion
- **ClineConnector** for VS Code extension sessions
- **SessionConnector::watch()** for live session monitoring
- **BM25-ranked session search** with search-index feature

#### Learning Store & Knowledge Graph
- **SharedLearningStore** with graph hybrid scoring
- **LearningInjector** and `learn inject` CLI command
- **Knowledge graph validation workflows** for pre/post-LLM quality gates
- **NormalizedTerm metadata** and `learn export-kg` command

#### CI Infrastructure
- **Firecracker-accelerated CI** with VM lifecycle management
- **Shared Rust build cache** via sccache + SeaweedFS S3 on fcbr0
- **Bigbox runner support** with rch dispatch for build queue management
- **Zig 0.16.0 compilation** for zlob feature (SIMD glob matching)

#### Robot Mode & Auto-Routing
- **Token budget management engine** for robot mode output
- **Auto-route on search** when --role is unset (thesaurus-scored)
- **RobotFormatter** wired into CLI search with JSON/JSONL/table output
- **ForgivingParser** integrated into REPL for typo-tolerant commands

### Fixed
- **zlob compilation** -- Zig 0.16.0 build with Darwin linker workaround
- **Security advisories** -- RUSTSEC-2024-0421, 2026-0098/0099/0104 resolved
- **REPL role selection** -- reads from service instead of hardcoded "Default"
- **CLI exit codes** -- listen --server exits with ERROR_USAGE (2) not 1
- **Graceful degradation** for roles without KG
- **Flaky tests** -- removed data-dependent assertions, fixed timing issues

### Changed
- **Workspace version** bumped to 1.17.0
- **Default features** -- zlob enabled by default on terraphim_file_search and terraphim_mcp_server
- **CI workflow** -- migrated to bigbox + SeaweedFS + rch dispatch
- **Dependency updates** -- removed dead trust-dns-resolver, ring deps

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
