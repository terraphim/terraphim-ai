# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

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

- **Firecracker feature gate** `terraphim_agent` firecracker module and `ApiClient` VM methods now compiled only with `cfg(feature = "firecracker")`, preventing link errors in non-Firecracker builds (2026-06-04)
- **Clippy `manual_flatten`** `server_http_error` test flattened to satisfy the `manual_flatten` lint (Fixes #2133, 2026-06-04)
- **RoleGraph serde defaults** `serde(default)` added to `trigger_descriptions` and `pinned_node_ids` fields in `SerializableRoleGraph`, fixing round-trip deserialisation of configs that omit these fields (Refs #2039, 2026-06-04)
- **Agent HTTP error classification** HTTP 4xx responses now classified as `ErrorGeneral` rather than `ErrorNetwork`; integration test `server_http_error_exits_1` added (Refs #1992, 2026-06-04)
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

- **`rust-format` gate** added to `ci-main.yml` with pre-existing clippy error fixes (Refs #1390, 2026-05-16)

- **`QualityScore` type** added to `IndexedDocument` and `Document` in `terraphim_types`, carrying `logic_score`, `structure_score`, and `composite` (NaN-guarded) fields for downstream ranking (Refs #547)
- **Rustdoc on `terraphim_server`** added `//!` crate-level doc, struct/enum/field docs and function doc-comments across `src/lib.rs`, `src/error.rs`, `src/api.rs`, and all `src/workflows/` sub-modules -- `RUSTDOCFLAGS="-W missing-docs" cargo doc --no-deps` now produces zero warnings
- **Rustdoc on core public types** added doc comments to `ServiceError`, `TerraphimService`, middleware `Error`, rolegraph `Error`, `split_paragraphs`, `DocumentType`, `RouteDirective`, `MarkdownDirectives`, `Edge`, `ChatMessage`, and `Priority` across four crates (Refs #547)
- **Orchestrator webhook fix** resolves project from repo for unqualified `@adf:` mentions

### Security

- **World-readable sensitive config files** now emit tracing error/warn at load time via `warn_if_world_readable()` in orchestrator config and all `conf.d` include files (Refs #826)

### Docs

- **Rustdoc coverage: zero-coverage crates resolved** item-level `///` doc comments added to all public items in `haystack_atlassian`, `haystack_core`, `haystack_discourse`, `terraphim_ccusage`, `terraphim_kg_linter`, and `terraphim_negative_contribution`; workspace coverage now 81 % (up from 30 % at previous scan on 2026-06-04, Refs #2136)
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
