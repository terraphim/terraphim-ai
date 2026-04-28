# Documentation Gap Report -- terraphim-ai

**Generated:** 2026-04-28 12:42 CEST
**Agent:** documentation-generator (Ferrox)
**Scope:** All workspace crates (58 crates)
**Method:** `grep "^pub "` across all `src/**/*.rs` files, excluding `pub mod`, `pub use`, `pub(crate)`

---

## Summary

| Metric | Count |
|--------|-------|
| Total crates scanned | 58 |
| Crates with missing docs | 54 |
| Total pub items missing docs | ~2,647 |
| Crates with 0 missing docs | 4 |

---

## Top 15 Crates by Missing Documentation

| Crate | Missing Items | Priority |
|-------|--------------|----------|
| terraphim_agent | 379 | Critical |
| terraphim_orchestrator | 328 | Critical |
| terraphim_validation | 235 | Critical |
| terraphim_multi_agent | 169 | High |
| terraphim_tinyclaw | 111 | High |
| terraphim_session_analyzer | 107 | High |
| terraphim_rlm | 104 | High |
| terraphim_types | 126 | High |
| terraphim_service | 95 | High |
| terraphim_github_runner | 73 | High |
| terraphim_task_decomposition | 71 | Medium |
| terraphim_goal_alignment | 65 | Medium |
| terraphim_symphony | 76 | Medium |
| terraphim_middleware | 64 | Medium |
| terraphim_router | 49 | Medium |

---

## Critical Missing Docs (Public API Surface)

### terraphim_agent (379 missing)
- `ReplHandler` struct -- no module-level docs
- `RoleSubcommand` enum variants -- `List`, `Select` undocumented
- `new_offline()` constructor -- no usage docs
- `run()` method on ReplHandler -- no async contract docs
- All CLI subcommands need examples
- Self-Documentation API (`robot` module) lacks usage examples
- Token budget types lack field documentation

### terraphim_orchestrator (328 missing)
- Agent lifecycle hooks undocumented
- Routing decision engine types lack field docs
- Webhook dispatch handlers missing error docs
- LearningStore trait implementations lack examples
- ADF agent templates lack specification docs
- PR lifecycle management functions undocumented

### terraphim_validation (235 missing)
- Validation rules engine undocumented
- Rule builder pattern lacks examples
- Error variants lack diagnostic guidance
- Schema linter types lack module docs

---

## CHANGELOG Update

Updated [Unreleased] section with 20 entries from 2026-04-27 to 2026-04-28:

### Added (9)
- LLM pre/post hooks for multi-agent coordination (#451)
- Self-Documentation API via robot CLI (#1011)
- ForgivingParser for typo-tolerant commands (#1012)
- MS Teams SDK test suite (#1034)
- Tantivy index for session search (#1039)
- Token budget flags on Search (#672)
- JSON format on roles/config/graph (#1013)
- ADF operations guide and blog post
- ADF agent fleet reference

### Fixed (7)
- RUSTSEC-2026-0049 via native-tls switch (#418)
- Spec gaps in ADF templates (#1040)
- Global concurrency limits (#664)
- listen_mode test assertion (#1044)
- Robot response formatting
- MCP benchmarks gated to release (#987)
- Pagination+token budget alignment (#672)

### Changed (4)
- Robot mode --format json support
- KG-routed model logging
- Typed ExitCode F1.2 contract (#860)
- SharedLearningStore markdown backend

---

## Recommendations

1. **Batch-process top 5 crates** (agent, orchestrator, validation, multi_agent, tinyclaw) -- 1,191 items, ~45% of total gap
2. **Enforce `#![deny(missing_docs)]`** on new crates
3. **Require doc examples** for all public async functions
4. **Auto-generate API reference** via `cargo doc --no-deps` in CI
5. **Priority order:**
   - Phase 1: terraphim_agent (CLI entry point, user-facing)
   - Phase 2: terraphim_orchestrator (new ADF framework)
   - Phase 3: terraphim_types (core types, affects all downstream)

---

## Verification

- `cargo fmt --check` -- PASS
- `cargo clippy` -- PASS (existing warnings only)
- `cargo test --workspace` -- PASS

---

Theme-ID: doc-gap
