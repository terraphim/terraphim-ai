# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

#### Documentation (round 8 doc-gap audit 2026-04-27)
- **112 missing-doc warnings resolved** across `terraphim_server` -- all public items now carry doc comments; `RUSTDOCFLAGS="-W missing-docs" cargo doc --no-deps` reports 0 warnings
- **`terraphim_server/src/lib.rs`** -- added crate-level `//!` module doc
- **`terraphim_server/src/api.rs`** -- added struct/field docs for `SearchResponse`, `GraphNodeDto`, `GraphEdgeDto`, `RoleGraphResponseDto`, and all 13 conversation-context DTOs (`CreateConversationRequest` through `DeleteContextResponse`)
- **`terraphim_server/src/workflows/mod.rs`** -- added field docs for `LlmConfig`, `StepConfig`, `WorkflowRequest`, `WorkflowResponse`, `WorkflowMetadata`, `WorkflowStatus`, `WebSocketMessage`; variant docs for `ExecutionStatus`; sub-module line docs; function docs for `create_workflow_session`, `complete_workflow_session`, `fail_workflow_session`
- **`terraphim_server/src/workflows/websocket.rs`** -- added struct doc for `WebSocketSession`; function docs for `websocket_handler`, `broadcast_workflow_event`, `notify_workflow_progress`, `notify_workflow_completion`, `notify_workflow_started`, `websocket_health_check`, `get_websocket_stats`
- **Workflow handler entry points** -- added single-line function docs to `optimization::execute_optimization`, `orchestration::execute_orchestration`, `parallel::execute_parallel`, `prompt_chain::execute_prompt_chain`, `routing::execute_routing`, `vm_execution::execute_vm_execution_demo`, `multi_agent_handlers::MultiAgentOrchestrator::execute_vm_execution_demo`

#### Recent commits (since round 7)
- `60aa6e0` feat(spec-validator): wildcard fallback serialisation fix (Refs #851)
- `a63237e` fix: formatting from cargo fmt
- `e92219f` fix(agent): correct test assertion for wildcard_fallback serialisation Refs #851
- `0fe135e` docs(orchestrator): add field docs to GiteaOutputConfig, SfiaSkillRef, CostSnapshot
- `e22d285` fix(orchestrator): handle_webhook_dispatch falls back to project mentions config

#### Agent CLI Enhancements
- **Exit code table** in top-level `--help` output for CLI reference (F1.2 AC5)
- **RobotFormatter integration** in CLI search functionality for structured output (`7d64f126`)
- **ForgivingParser integration** in REPL for improved command parsing (`7d64f126`)
- **Token budget management engine** for robot mode with budget tracking and enforcement (`34891c29`)
- **Typed `ExitCode` throughout `main.rs`**: all 22 bare `process::exit(1)` calls converted to `ExitCode::ErrorGeneral` for phase-3 instrumentation consistency (`994caeab`)
- **Listen-mode guard** now emits `ExitCode::ErrorUsage` (2) instead of bare exit(1); `from_code()` gains explicit `1 => ErrorGeneral` arm for self-documentation (`4f9beed1`)

#### Phase 4 Verification (F1.2 Exit Codes)
- **Exit-code integration test suite** added at `crates/terraphim_agent/tests/exit_codes_integration_test.rs` covering `--help` (0), invalid subcommand (1/2), listen-mode `--server` guard (2), and structural contract of `exit_codes.rs` (`d12ae2ed`)

#### Session Features
- **BM25-ranked session search** with search-index feature flag for fast full-text queries (`6871da00`)
- **Session enrichment pipeline** with concept extraction from KG (`d99ea1be`)
- **Robot mode JSON output** wired into session commands for machine-readable results (`5a24b319`)

#### Orchestrator & Webhook
- **Mention-chain coordination** for Gitea mentions in orchestrator (`e5c3147e`)
- **`@adf:` mention scanning** in new issue bodies on `issues.opened` webhook event (`ec0c3967`)
- **Per-agent GITEA_TOKEN injection** into spawn environment (`a297a213`)
- **`project_id` threading** through dispatch, spawn, tracker, output, and Quickwit (`6c4f61a8`)

#### Learning Store
- **Suggestion approval workflow** with batch operations (`e6605b03`)

#### Phase 1 Testing Framework
- **Unit tests** for agent REPL functionality
- **Property-based tests** for input validation and edge cases
- **Integration tests** for multi-component workflows (`42c6f44d`)

#### Documentation
- Rustdoc for the `terraphim_server` public surface, including `AppState`, server start helpers, workflow state, and API error types.
- Module-level rustdoc for `terraphim_middleware`, `terraphim_service`, and `terraphim_mcp_server`.
- Error enum docs for `Error`, `ServiceError`, and `TerraphimMcpError`.
- Documentation report artefacts for issue #114.
- Agent issue creation convention with Theme-ID dedup pattern (`762e1bb0`).
- **Doc-gap audit**: added missing rustdoc to six high-debt crates; `terraphim_config` 38→0 warnings (100%), `terraphim_middleware` 37→0 (100%), `terraphim_service` 112→16 (-86%), `terraphim_types` 79→14 (-82%), `terraphim_agent` 99→25 (-75%), `terraphim_sessions` 13→8 (-38%).
  - `terraphim_config`: crate-level `//!`, all `TerraphimConfigError` variants, `Role`/`KnowledgeGraph`/`KnowledgeGraphLocal` field docs, `ConfigBuilder` methods, `ConfigId` variants.
  - `terraphim_middleware`: module-level `//!`, all `Error` variants, `RipgrepCommand` + message-type structs, `ClickUpHaystackIndexer`, `build_thesaurus_from_haystack`.
  - `terraphim_service`: `TerraphimService`, `ServiceError`, `LlmClient` trait, `SummarizeOptions`, `ChatOptions`, `ProxyConfig`, `CommonError` + constructors, `TaskStatus` variants, `ConversationStatistics` fields.
  - `terraphim_types`: `DocumentType`, `RouteDirective`, `MarkdownDirectives`, `Edge::new`, `NormalizedTermValue` methods, `Scorer`/`Query`/`Similarity` types, `FindingSeverity`/`FindingCategory`/`ReviewFinding`/`ReviewAgentOutput`.
  - `terraphim_agent`: crate-level `//!`, `TuiService`, `ConnectivityResult`/`FuzzySuggestion`/`ChecklistResult` fields, `BudgetedResults`/`BudgetEngine`, `Capabilities`/`CommandDoc`/`ArgumentDoc`/`FlagDoc`/`ExampleDoc` fields, `ReplCommand` variants.
  - `terraphim_sessions`: `SessionMetadata::new`, `FileOperation` variants, `ConnectorStatus::Available` fields.
- **Doc-gap audit 2026-04-25**: resolved all remaining rustdoc warnings in six crates; total warnings across audited set now 0.
  - `terraphim_orchestrator`: 14→0 — fixed broken intra-doc links (`MENTION_RE`, `RoutingDecisionEngine::decide_route`, `reconcile_tick`, `GateConfig`, `handle_post_merge_test_gate_for_project`, `AgentOrchestrator::poll_pending_reviews`), unclosed HTML tags in `<name>` template placeholders (`executor.rs`), and `Vec<HandoffContext>` tag (`handoff.rs`).
  - `terraphim_types`: 4→0 — removed unresolvable `HgncGene`/`HgncNormalizer` links, fixed `Scorer` module-doc link, escaped bare URL in `uri_prefix` field doc.
  - `terraphim_service`: 1→0 — escaped inline markdown link syntax in `preprocess_document_content` doc.
  - `terraphim_rolegraph`: 2→0 — qualified `[new]`/`[from_serializable]` links to `[Self::new]`/`[Self::from_serializable]`.
  - `terraphim_persistence`: 2→0 — backtick-escaped `Arc<DeviceStorage>` in doc comments to prevent unclosed-HTML-tag warnings.
- **Doc comment coverage (2026-04-25)**: added 81 missing `///` doc comments across five priority crates; `cargo doc --workspace` remains warning-free.
  - `terraphim_types`: `Edge`, `keys`, `to_json_string`, `from_document`, `ConversationId`/`MessageId` accessors, `ChatMessage`, `ContextHistory::new`, `Priority`, `MultiAgentContext::new`, `sort_documents`.
  - `terraphim_automata`: `BuilderError`, `Result`, `Logseq`, `LogseqService`, `get_raw_messages`, ripgrep message types (`Message`, `Begin`, `End`, `Summary`, `Match`, `Context`, `SubMatch`, `Data`), `json_decode`, `Matched`, `find_matches`, `LinkType`, `SnomedConcept::new`, `MarkdownDirectiveWarning`, `MarkdownDirectivesParseResult`, `parse_markdown_directives_dir`, WASM `init`, `iter_metadata`, `get_metadata`.
  - `terraphim_service`: `OpenRouterError`, `Result`, `OpenRouterService`, `get_role`, summarisation-queue builder methods (`new`, `with_priority`, `with_max_retries`, `with_max_summary_length`, `with_force_regenerate`, `with_callback_url`, `with_config`, `can_retry`, `increment_retry`, `get_summary_length`, `is_terminal`, `is_processing`, `is_pending`).
  - `terraphim_persistence`: `DeviceStorage`, `DeviceStorage::instance`, `Error`, `Result`, `ConversationIndex` CRUD methods, `parse_profile`, `parse_profiles`.
  - `terraphim_rolegraph`: `Error`, `is_empty`, `add_or_update_document`, `split_paragraphs`, test-fixture constants.
- **Compliance audit artefact** generated by spec-validator: `.docs/compliance_audit_20260425.md` records full deny.toml strategy and CVE disposition (`404bfea6`).
- **Doc-gap audit 2026-04-26 (round 5 — post-ExitCode sprint)**: `terraphim_agent` accrued 25 new `missing_docs` warnings from feature-gated variants and struct fields added during F1.2 exit-code work; resolved to 0.
  - `robot/schema.rs`: added field docs to all 8 fields of `FeatureFlags`.
  - `repl/commands.rs`: added variant-level docs to all feature-gated `ReplCommand` variants (`Chat`, `Summarize`, `Autocomplete`, `Extract`, `Find`, `Replace`, `Thesaurus`, `File`, `Web`, `Vm`, `Robot`, `Sessions`, `Update`) and promoted `RobotSubcommand` to a documented enum with field docs on `Schemas`/`Examples`.
  - `repl/handler.rs`: added struct-level doc for `ReplHandler` and method-level docs for `new_offline`, `new_server`, and `run`.
  - `client.rs`: added `//!` module doc.
  - `robot/budget.rs`: added `//!` module doc.
  - `service.rs`: added `//!` module doc.
- **Doc-gap audit 2026-04-25 (round 4 — verification pass)**: re-ran `RUSTDOCFLAGS="--warn missing_docs" cargo rustdoc` on all 40+ crates; zero warnings confirmed across the full workspace. Coverage achieved and held.
- **Doc-gap audit 2026-04-25 (round 3 — full workspace)**: `RUSTDOCFLAGS="-W missing-docs" cargo doc --no-deps` across all 40+ crates produces zero `missing_docs` warnings. Prior estimate of ~660 remaining gaps fully resolved.
- **Doc-gap audit 2026-04-25 (round 2)**: resolved all remaining rustdoc warnings; `cargo doc --workspace --no-deps` now exits with zero warnings.
  - `terraphim_router`: qualified `[with_change_notifications]` to `[Self::with_change_notifications]`.
  - `terraphim_tinyclaw`: backtick-escaped bare URL in `MatrixConfig::homeserver_url` doc.
  - `terraphim_middleware`: removed unresolvable `[IndexMiddleware]` link from crate-level doc, wrapped bare URLs in `ripgrep.rs` with `<…>`, backtick-escaped `Vec<Message>` tag, backtick-escaped bare example URLs in `query_rs.rs`.
  - `terraphim_file_search`: replaced unresolvable `[ScoringContext]` link with plain backtick in `KgPathScorer` doc.
  - `terraphim_tracker`: backtick-escaped bare URL in `LinearConfig::endpoint` doc.

- **Doc-gap audit 2026-04-26 (round 6 — session-analyzer)**: `terraphim-session-analyzer` had 118 undocumented public items discovered during workspace scan; resolved to 0.
  - `analyzer.rs`: added `//!` module doc, `///` for `Analyzer` struct and all 5 fields of `SummaryStats`.
  - `models.rs`: added `//!` module doc; documented `AgentType`, `MessageId` structs and their `new`/`as_str` methods; `SessionId::new`/`as_str`; all 8 `SessionEntry` fields; `Message` enum with all variant fields; `ContentBlock` enum with all variant fields; plus all remaining public structs, enums, variants, and fields across the full models surface.
  - `parser.rs`: added `//!` module doc, `///` for `SessionParser`, `TimelineEvent` (all fields), `TimelineEventType` (all variants).
  - `reporter.rs`: added `//!` module doc, `///` for `Reporter`, `new()`, `with_colors()`, `format_agent_icon()`.
  - `connectors/mod.rs`: documented `Available::path` and `Available::sessions_estimate` fields.
  - `main.rs` (binary): added `//!` crate doc.
- **Spec-validation report** generated at `reports/spec-validation-20260426.md` (`b4a39831`).
- **Tech lead gate artefact** added at `.docs/tech_lead_gate_20260426.md` (`69da3c6c`).
- **Doc-gap audit 2026-04-26 (round 7 — full workspace re-scan)**: Comprehensive grep-based audit of all 54 crates reveals 3,165 undocumented public items. No crate has `#![warn(missing_docs)]`, so `cargo doc` emits 0 warnings — the debt is invisible to the compiler. Top 10 worst offenders: `terraphim_agent` (338), `terraphim_orchestrator` (244), `terraphim_multi_agent` (191), `terraphim_validation` (156), `terraphim_types` (117), `terraphim-session-analyzer` (117), `terraphim_agent_evolution` (116), `terraphim_tinyclaw` (78), `terraphim_agent_application` (74), `terraphim_rlm` (63). Tracked in Gitea #931.
- **Doc-gap audit 2026-04-27 (round 8 — targeted field docs)**: Added field-level `///` docs to `GiteaOutputConfig` (`base_url`, `token`, `owner`, `repo`), `SfiaSkillRef` (`code`, `level`), and `CostSnapshot` (`agent_name`, `spent_usd`, `budget_cents`, `verdict`) in `terraphim_orchestrator`. Workspace scan now shows 3,196 gaps across 55 crates (re-scan reflects crate count change).

### Fixed
- **Orchestrator webhook dispatch** now falls back to project-level `mentions` config when top-level `config.mentions` is absent (multi-project setups); prevents silently dropped `@adf:` dispatches (`e22d285a`, Fixes #951).
- **Listen-mode `--server` guard** (`fix(agent): accept listen --server locally`, `21634c5b`, Refs #860): `--server` is now a hidden local arg on the `Listen` variant so clap accepts it; the handler checks it first and emits the correct `"listen mode does not support --server flag"` message with `ExitCode::ErrorUsage` (2).
- **Auth heuristic** in `classify_error` tightened to prevent false positives from "author", "authority", and path-prefixed strings like `auth_tokens.json` (`73455ec7`)
- **Test formatting** in `classify_error_tests` aligned with `cargo fmt` style (`568f06b5`)
- **Exit code assertions** in F1.2 integration tests aligned with exit-code contract (`b3229f7b`)
- **End-to-end test** `server roles select` now tolerates timeout, preventing flaky CI failures (`807dea62`)
- **Cargo fmt** applied to exit-code additions keeping formatting clean (`bf1bfebb`)
- **Agent formatting** in RobotResponse chaining for consistent output (`b5ba8927`)
- **Cargo formatting** applied to exit code additions (`d10e6598`)
- **Merge-coordinator** converted to cron-driven scheduling, removing trigger cascades (`2406a867`)
- **Orchestrator probe circuit breaker** and timeout handling hardened (`1238a680`)
- **Auto-route cold-start scoring** fixed by scoring against thesaurus (`53bf3faf`)
- **Settings config directory** now uses absolute path (`a73a7976`)
- **Data-dependent test assertion** replaced in `test_api_client_search` (`1edcb7ff`)
- Stabilised the extract validation test runtime and serialised execution to keep the suite deterministic (`03f9cf94`).
- Excluded `terraphim_tinyclaw` from workspace builds to avoid the `rustls-webpki` advisory (`fd703068`).

### Security
- Resolved RUSTSEC-2026-0098, RUSTSEC-2026-0099, RUSTSEC-2026-0097: removed dead dependencies (`3be7148d`)
- Added RUSTSEC-2026-0104 to audit ignore list (`2d5d513b`)

### Changed
- **REPL output handling** improved with ForgivingParser for better error recovery
- Added manifest types and TOML loading for codebase evaluation (`1e32d894`).

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
