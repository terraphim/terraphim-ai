# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **`RUNNER_POLL_TIMEOUT` env var** `terraphim_gitea_runner` now reads `RUNNER_POLL_TIMEOUT` (default `2h`) to control how long a runner waits for a job before timing out; prevents silent hangs during long builds (PR #2978, 2026-06-25)
- **Spawner post-exit fallback** `terraphim_spawner` detects early process exits and falls back gracefully via `post_exit_fallback` + `ExitDetected` event, reducing silent failures during agent restarts (2026-06-23)
- **Weather report markdown output** `terraphim_weather_report` CLI now accepts `--format markdown` to emit a human-readable tier table alongside the existing JSON and table formats (2026-06-23)
- **ADF taxonomy additions** `kimi-k2.7-coding` added to implementation tier; `GLM-5.2` and `MiniMax-M3` added to ADF routing tiers; pi-rust routes added for kimi-for-coding models (2026-06-22/23)
- **KG-driven dynamic command allowlist** `terraphim_gitea_runner` derives its shell command allowlist at runtime from the tier-taxonomy KG, eliminating the static hardcoded set (PR #2804, 2026-06-22)
- **Rustdoc gaps resolved** doc comments added to all public items and struct/enum fields in `terraphim_rlm` (`RlmError` variants and their named fields, `LocalExecutor`, `DockerExecutor`, `SessionStats`, MCP response types), `terraphim_lsp` (module-level `//!` headers for `kg_analysis` and `server` modules), `terraphim_merge_coordinator` (`ExitCode` variants, `PrEvaluation` fields, `PrSummary` fields, `MergeOutcome` variant fields, `MergeCoordinatorError` variant fields), and `terraphim_workspace` (`WorkspaceError` variant fields) -- all four crates now build with `--warn missing-docs` at zero warnings (2026-06-18)
- **Thesaurus matching in robot mode** `thesaurus_matched` field added to `SearchResultsData`; populated from Thesaurus entries that match query terms in both offline and server search paths (Refs #851, 2026-05-31)
- **Context rot detection** `RotStatus` enum with `Fresh`/`Warning`/`Critical` states; `Conversation::with_token_budget()` and `Conversation::check_rot()` methods for monitoring token budget utilization in `terraphim_types` (Refs #1443, 2026-05-31)
- **Rustdoc CI gate** `RUSTDOCFLAGS=-D warnings` enforcement added to PR validation pipeline; fixed broken intra-doc links in `terraphim_persistence`, `terraphim_rolegraph`, and `terraphim_orchestrator` (Refs #1362, 2026-05-31)
- **pr-reviewer agent** scaffolding and initial agent work (2026-06-01)
- **ADF direct-dispatch remediation** project-aware routing, synthetic event context, and shell hardening in `adf-ctl --local trigger --direct` (Refs #1890, PR#1885, 2026-05-30)
- **adf-ctl Unix socket dispatch** `adf-ctl trigger --local --direct` via UDS (`/tmp/adf-ctl.sock`, 0600 permissions) for zero-latency local agent dispatch without SSH (PR#1876, 2026-05-28)
- **terraphim_grep hybrid searcher** complete implementation: parallel KG + grep execution via `tokio::spawn`, CLI with thesaurus discovery, `Serialize` on `GrepResult`/`GrepStats`/`SufficiencyState` (Refs #1743, PR#1825, 2026-05-24)
- **terraphim_merge_coordinator** minimal skeleton proving crate structure (Refs #1805, PR#1823, 2026-05-23)
- **Config-error circuit-breaker** `ExitClass::ConfigError` quarantines agents after 3 consecutive config failures; `AgentDefinition.enabled` field; memory watchdog systemd units; `bigbox-sync.sh` (Refs #1817, PR#1822, 2026-05-23)
- **Rustdoc gaps resolved** doc comments added to all public items in `terraphim_types` (`LlmUsage`, `LlmResult`, `ModelPricing`, `ReviewFinding`, `ReviewAgentOutput`, `FindingSeverity`, `FindingCategory`, `DocumentType`, `MarkdownDirectives`, `Scorer`, `Query`, `Similarity`, `ScoreError`, and score sub-modules) -- 93 warnings eliminated (2026-05-30)
- **terraphim_rlm CLI binary** with 6 commands (code, bash, query, context, snapshot, status) for stateless RLM execution (Refs #RLM-CLI, 2026-05-18)
- **MCP server RLM integration** via process spawning — 6 new tools exposed without linking terraphim_rlm (avoids static init hang) (Refs #RLM-CLI, 2026-05-18)
- **Session search capability** enabled in `terraphim_agent` binary (2026-05-17)
- **Evolution integration** `terraphim_agent_evolution` wired into ADF orchestrator (Refs #1487, 2026-05-15)
- **Ranking regression gate** Kendall-tau snapshot tests added to prevent ranking regressions (Refs #1454, 2026-05-16)
- **Phase 1 robot mode tests** complete test suite for robot CLI output format (Refs #1473, 2026-05-16)
- **cargo-nextest** per-test slow-timeout support added to CI pipeline (Refs #1475, 2026-05-16)
- **Rustdoc gaps resolved** doc comments added to public items in `terraphim_automata`, `terraphim_rolegraph`, `terraphim_middleware`, and `terraphim_sessions` (2026-05-18)

### Fixed

- **Unsafe `set_var` SAFETY comment** added explicit `SAFETY` comment to `set_var` call in `terraphim_tinyclaw` test, documenting the single-threaded invariant (Refs #2492, PR #2971, 2026-06-25)
- **Merge-coordinator const asserts** runtime `assert!()` calls in `merge_coordinator/gitea.rs` converted to compile-time `const {}` blocks, eliminating clippy `assertions_on_constants` warnings (2026-06-25)
- **Weather report clippy fix** nested `if` collapsed to satisfy `clippy::collapsible_if` (2026-06-25)
- **Validation vacuous assert** `assert!(true)` removed and nested `if` collapsed in `terraphim_validation` (2026-06-25)
- **Spawner source files restored** `terraphim_spawner` re-included in workspace after missing source files were restored (2026-06-23)
- **Taxonomy stale model IDs** `openai-codex/opencode` routes replaced with `openai/gpt-5.5`; `MiniMax-3` corrected to `MiniMax-M3` (2026-06-22/23)
- **`list_open_prs` limit** raised from 50 to 300 in `merge_coordinator` to avoid silently missing PRs in large repositories (Refs #2850, PR #2854, 2026-06-21)
- **`rust-version` field** declared as `1.85.0` in workspace and all published crates, aligning with the clippy MSRV configuration (Refs #2770, PR #2774, 2026-06-22)
- **`.sessions/` gitignore** existing tracked `.sessions/` files removed from the git index after the `gitignore` entry was added (Refs #2893, PR #2896, 2026-06-22)
- **Redis security exposure** Docker Compose Redis service now binds to `127.0.0.1:6379` instead of `0.0.0.0:6379` to prevent unintended public exposure of the cache (Refs #1313, 2026-05-31)
- **Nested `cargo run` in exit-code tests** replaced with `cargo_bin!` macro to avoid file-lock deadlock under concurrent `cargo test` (2026-06-01)
- **ADF KG-router fallback respawn loop** closed after quota exit — agents no longer re-routed to `anthropic/sonnet` indefinitely when per-agent config or quota-fallback chose another provider (Refs #1793, PR#1794, 2026-05-22)
- **terraphim_service genai dependency** switched from GitHub fork to crates.io release (PR#1844, 2026-05-24)
- **`publish = false`** removed from `terraphim_service`; publishing constraints hardened workspace-wide (PR#1843, 2026-05-24)
- **`ProxyConfig` and `LlmConfig` api_key** redacted from `Debug` output (Refs #1667, 2026-05-18)
- **`llm_api_key` and `atomic_server_secret`** redacted from `Debug` output (Refs #1661, 2026-05-18)
- **Credential fields** in `Debug` output hardened across config structs (Refs #1667, 2026-05-18)
- **`watch()` init errors** now propagated via oneshot channel instead of being swallowed (Refs #814, 2026-05-17)
- **UTF-8 char-boundary panic** in KG snippet extraction replaced with char-safe helper (Refs #1557, 2026-05-17)
- **`TrackerConfig` api_key** redacted from `Debug` output (Refs #1300, 2026-05-16)
- **`GiteaOutputConfig` token and secret** redacted from `Debug` output (Refs #1300, 2026-05-16)
- **`unsafe ptr::read`** replaced with `arc_memory_only()` in multi_agent examples (Refs #1497, 2026-05-16)
- **Deprecated `tempfile::into_path()`** replaced with `keep()` across workspace (2026-05-16)
- **RLM executor surface** hardened; agent search envelope and OpenCode hook fixed (Refs #870, 2026-05-16)

### CI

- **Native CI build gate repaired** non-existent `terraphim_gitea_runner` test step dropped from `ci-native.yml`; clippy errors blocking `native-ci/build` resolved (PR #2973, 2026-06-25)
- **Bigbox deployment rules** documented in `docs/`: git pull/push only; no cp/scp between machines (2026-06-23)
- **`rust-format` gate** added to `ci-main.yml` with pre-existing clippy error fixes (Refs #1390, 2026-05-16)

- **`QualityScore` type** added to `IndexedDocument` and `Document` in `terraphim_types`, carrying `logic_score`, `structure_score`, and `composite` (NaN-guarded) fields for downstream ranking (Refs #547)
- **Rustdoc on `terraphim_server`** added `//!` crate-level doc, struct/enum/field docs and function doc-comments across `src/lib.rs`, `src/error.rs`, `src/api.rs`, and all `src/workflows/` sub-modules -- `RUSTDOCFLAGS="-W missing-docs" cargo doc --no-deps` now produces zero warnings
- **Rustdoc on core public types** added doc comments to `ServiceError`, `TerraphimService`, middleware `Error`, rolegraph `Error`, `split_paragraphs`, `DocumentType`, `RouteDirective`, `MarkdownDirectives`, `Edge`, `ChatMessage`, and `Priority` across four crates (Refs #547)
- **Orchestrator webhook fix** resolves project from repo for unqualified `@adf:` mentions

### Security

- **World-readable sensitive config files** now emit tracing error/warn at load time via `warn_if_world_readable()` in orchestrator config and all `conf.d` include files (Refs #826)

### Docs

- **`BUILD.md`** build instructions and troubleshooting guide
- **`CONTRIBUTING.md`** contribution guidelines and code of conduct
- **Architecture Decision Records (ADRs)** in `docs/architecture/`
- **API documentation** auto-generated and published

## [0.2.3] - 2024-03-15

### Added

- **Docker Compose** setup for local development
- **Redis caching** support
- **Nginx reverse proxy** configuration
- **Health check endpoints**

## [0.2.2] - 2024-02-28

### Added

- **Knowledge Graph** routing and search
- **Multi-agent coordination** via `terraphim_multi_agent`
- **Session management** with persistence

## [0.2.1] - 2024-02-14

### Added

- **Thesaurus** support for term expansion
- **Document indexing** with quality scores
- **Search API** with ranking

## [0.2.0] - 2024-01-31

### Added

- **Initial release** of Terraphim AI platform
- **Agent spawning** via `terraphim_spawner`
- **LLM routing** with provider fallback
- **Web interface** for search and chat
