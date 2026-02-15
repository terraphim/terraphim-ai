# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
