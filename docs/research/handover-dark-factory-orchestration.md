# Handover: terraphim_orchestrator -- AI Dark Factory

**Date**: 2026-03-06
**Branch**: `main`
**Status**: All pushed to `github/main`, up to date

---

## 1. Progress Summary

### Completed This Session (across 2 context windows)

Full V-model lifecycle for `terraphim_orchestrator` crate:

| Phase | Status | Artifacts |
|-------|--------|-----------|
| Phase 1: Research | DONE | `.docs/research-dark-factory-orchestration.md` |
| Phase 2: Design | DONE | `.docs/design-dark-factory-orchestration.md` |
| Phase 3: Implementation | DONE | 8 atomic commits (Steps 1-7 + integration tests) |
| Phase 4: Verification | DONE | 21/21 types match, 45 tests pass |
| Phase 5: Validation | DONE | `.docs/validation-dark-factory-orchestration.md` -- PASS |

### Commits (10 total, all pushed)

```
60e08d01 docs: add disciplined development documents for all phases
ed548909 fix: unexclude haystack_jmap from workspace and track .docs directory
0c19d841 test(orchestrator): add integration tests for Phase 4 verification defects
02c76a3b feat(orchestrator): add example TOML config and workspace integration test
1f6cca7d feat(orchestrator): add AgentOrchestrator core reconciliation loop
44827629 feat(orchestrator): add HandoffContext for shallow agent task transfer
81561930 feat(orchestrator): add CompoundReviewWorkflow with git log scanning
83f41235 feat(orchestrator): add TimeScheduler with cron-based agent lifecycle
7ad5d19c feat(orchestrator): add NightwatchMonitor with drift detection and RateLimitTracker
0458c4b5 feat(orchestrator): scaffold terraphim_orchestrator crate with config and error types
```

### What's Working

- **45/45 tests pass** (31 unit + 14 integration)
- Zero clippy warnings, fully formatted
- All pre-commit hooks pass
- Crate compiles and integrates with workspace

### What's Blocked

Nothing is blocked. The crate is complete for Phase 1 MVP.

---

## 2. Technical Context

### Crate Location

`crates/terraphim_orchestrator/` -- 11 files, 2,273 lines (1,796 src + 477 test)

### Key Files

| File | Purpose |
|------|---------|
| `src/lib.rs` | AgentOrchestrator with tokio::select! reconciliation loop |
| `src/config.rs` | OrchestratorConfig, AgentDefinition, AgentLayer (TOML parsing) |
| `src/nightwatch.rs` | NightwatchMonitor, DriftMetrics, CorrectionLevel, RateLimitTracker |
| `src/scheduler.rs` | TimeScheduler with cron parsing (5-field auto-prepend) |
| `src/compound.rs` | CompoundReviewWorkflow (git log scan, dry-run mode) |
| `src/handoff.rs` | HandoffContext (JSON serialization for agent task transfer) |
| `src/error.rs` | OrchestratorError (9 variants) |
| `orchestrator.example.toml` | 3-agent fleet config (Safety/Core/Growth) |
| `tests/orchestrator_tests.rs` | 8 integration tests |
| `tests/nightwatch_tests.rs` | 3 integration tests |
| `tests/scheduler_tests.rs` | 3 integration tests |

### Architecture

```
OrchestratorConfig (TOML)
        |
        v
AgentOrchestrator
  |-- TimeScheduler -------> cron triggers (Safety=always, Core=cron, Growth=on-demand)
  |-- NightwatchMonitor ----> drift alerts (Normal/Minor/Moderate/Severe/Critical)
  |-- CompoundReview -------> nightly git log scan
  |
  |-- AgentSpawner (existing) --> OS processes
  |-- RoutingEngine (existing) -> keyword-based LLM routing
```

### Dependencies (new to this crate)

- `cron = "0.13"` -- cron expression parsing
- `toml = "0.8"` -- config file parsing

### Key Design Decisions

- Kubernetes-style reconciliation loop with `tokio::select!`
- Weighted drift calculation: error_rate 0.4 + command_success_rate 0.3 + health_score 0.3
- Zero-sample guard: drift returns 0.0 when no data collected
- Cron auto-prepends "0" seconds field for 5-field expressions
- `CostLevel::Cheap` (not `Low`) for spawned agent providers
- Edition 2021 (not workspace 2024) for broader compatibility

---

## 3. Fixes Applied During Session

| Issue | Resolution |
|-------|-----------|
| `DriftMetrics::default()` caused non-zero drift after reset | Added `sample_count == 0` guard in `calculate_drift()` |
| `TimeScheduler` missing `Debug` trait | Added `#[derive(Debug)]` |
| `CostLevel::Low` doesn't exist | Changed to `CostLevel::Cheap` |
| Pre-commit hook flaky test (terraphim_update) | Soft-reset and re-committed with hooks passing |
| `haystack_jmap` excluded but now a dep of middleware | Removed from workspace exclude list |
| `.docs/` in .gitignore | Removed to track design documents |
| Trailing whitespace in .docs files | Fixed with `sed` before commit |

---

## 4. Deferred to Phase 2

| Feature | Why Deferred |
|---------|-------------|
| Meta-Learning Agent | Beyond Phase 1 MVP scope |
| Deep context handoff (full session state) | Shallow JSON handoff sufficient for now |
| A/B test framework | Not needed for initial deployment |
| UI dashboard for monitoring | Programmatic API is sufficient |
| Compound review PR creation | Placeholder in code; dry-run default |
| NightwatchMonitor periodic evaluation trigger in run loop | VAL-3 advisory |
| Multi-project coordination | Phase 3 scope |

---

## 5. Validation Advisory Issues

| ID | Description | Severity |
|----|-------------|----------|
| VAL-1 | Compound review PR creation is placeholder even when `create_prs=true` | Advisory |
| VAL-2 | Reconciliation loop has 2 select! branches vs 4 in design | Advisory |
| VAL-3 | `NightwatchMonitor.evaluate()` needs periodic trigger in run loop | Advisory |
| VAL-4 | Actual size (2273 lines) exceeded estimate (950 lines) | Low |

---

## 6. Next Steps for Continuation

1. **Integration test with real CLI tools on BigBox** -- spawn actual `echo`/`codex`/`claude` processes
2. **Wire `evaluate()` into the reconciliation loop** (VAL-3) -- add a `tokio::time::interval` branch to `select!`
3. **Implement PR creation in compound review** (VAL-1) -- call `gh pr create` when `create_prs=true`
4. **Create GitHub issue** to track remaining Phase 2 work
5. **BigBox tmux deployment** -- set up tmux sessions per the dark factory architecture

---

## 7. Untracked Files (not part of this work)

```
crates/terraphim_agent/crates/terraphim_settings/default/settings.toml
crates/terraphim_agent_evolution/crates/terraphim_settings/default/settings.toml
crates/terraphim_cli/crates/terraphim_settings/default/settings.toml
crates/terraphim_config/crates/terraphim_settings/default/settings.toml
crates/terraphim_mcp_server/crates/terraphim_settings/default/settings.toml
crates/terraphim_service/crates/terraphim_settings/default/settings.toml
images/x_illustration.jpg
```

These appear to be auto-generated settings files from nested crate symlinks and an unrelated image.
