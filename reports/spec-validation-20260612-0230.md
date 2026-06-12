# Spec Validation Report: 2026-06-12 02:30 CEST (Cron)

**Agent**: spec-validator (Carthos, Domain Architect)
**HEAD**: 0904a1a3a3
**Date**: 2026-06-12 02:30 CEST
**Run type**: Cron (no mention context)

## Verdict: CONDITIONAL PASS

1028/1028 tests pass; clippy clean. Six `plans/` plans carry forward unchanged (5 PASS, 1 PARTIAL). **New P1 finding**: issue #2415's KG validation hot-path calls are absent from HEAD — `execute_command()`, `execute_code()`, and `QueryLoop::Command::Run/Code` bypass `executor.validate()` entirely.

---

## Tests

```
Workspace: 1028/1028 PASS (0 failures)
Ignored: 19 live-integration (Ollama/Firecracker)
Clippy: clean (1 pre-existing tokio-tungstenite patch advisory, non-blocking)
```

---

## Plans Validated (6 total)

| Plan | Title | Status | Notes |
|------|-------|--------|-------|
| design-gitea82-correction-event.md | CorrectionEvent for Learning Capture | PASS | Carry-forward |
| d3-session-auto-capture-plan.md | Session-Based Auto-Capture | PASS | Carry-forward |
| design-gitea84-trigger-based-retrieval.md | Trigger-Based KG Retrieval | PASS (registry) | Carry-forward |
| design-single-agent-listener.md | Single Gitea Listener | N/A | Operational plan |
| learning-correction-system-plan.md | Learning and Correction System | PARTIAL | Phases C, F-H deferred |
| research-single-agent-listener.md | Research Document | N/A | Research only |

No plan changes since 2026-06-11 11:30 CEST cron.

---

## New Finding: P1-1 — Issue #2415 hot-path validate() regression

### Finding

`TerraphimRlm::execute_command()`, `TerraphimRlm::execute_code()`, and `QueryLoop::execute_command()` do NOT call `self.executor.validate()` before executing in HEAD `0904a1a3a3`.

### Evidence

**rlm.rs:310–335** (`execute_code`): calls `session_manager.validate_session()` then directly calls `self.executor.execute_code()` — no `executor.validate()` call.

**rlm.rs:366–391** (`execute_command`): calls `session_manager.validate_session()` then directly calls `self.executor.execute_command()` — no `executor.validate()` call.

**query_loop.rs:383–409** (`Command::Run`): calls `executor.execute_command()` directly — no `executor.validate()` call.

**query_loop.rs:411–437** (`Command::Code`): calls `executor.execute_code()` directly — no `executor.validate()` call.

### Diff evidence

`git diff d702be4d98 HEAD -- crates/terraphim_rlm/src/rlm.rs` shows 27-line KG validation gate blocks removed from both `execute_code()` and `execute_command()`.

`git diff d702be4d98 HEAD -- crates/terraphim_rlm/src/query_loop.rs` shows 27-line KG validation gate blocks removed from `Command::Run` and `Command::Code` arms.

### Prior spec-validator PASS

Spec-validator at 2026-06-11 23:29 CEST confirmed ALL 7 ACs of #2415 met at `d702be4d98`. The current HEAD (0904a1a3a3) does not carry those changes in `rlm.rs` and `query_loop.rs`.

### ACs affected

| AC | Requirement | Status |
|----|-------------|--------|
| AC1 | `execute_command()` calls `executor.validate()` before dispatch | FAIL |
| AC2 | `execute_code()` calls `executor.validate()` before dispatch | FAIL |
| AC3 | `QueryLoop::execute_command()` validates `Command::Run` and `Command::Code` | FAIL |
| AC4 | Permissive logs warning and allows execution | FAIL (never reached) |
| AC5 | Normal/Strict block on `is_valid == false` | FAIL (never reached) |
| AC6 | Unit tests for validation-blocks-execution paths | PARTIAL (blocks_unknown tests exist in config.rs) |
| AC7 | `cargo test -p terraphim_rlm` passes | PASS (1028/1028) |

### Executors (not affected)

`LocalExecutor`, `DockerExecutor`, `FirecrackerExecutor` all have `validate()` implemented (wired by #2483). The executor-level implementation is correct; the gap is at the orchestrator layer.

---

## Carry-Forward Findings

### P2-1: ValidationResult.strictness hardcoded Normal

**Files**: `executor/firecracker.rs:525`, `executor/local.rs:186`, `executor/docker.rs:395`
**Finding**: `ValidationResult { strictness: crate::config::KgStrictness::Normal, ... }` hardcoded in all three executors. Should reflect the executor's configured strictness (`self.validator.config.strictness`).
**From**: Security sentinel 2026-06-12 01:22 CEST #2483.

### P2-2: Phase C entity annotation unimplemented

`annotate_with_entities()` / `--semantic` flag not present. Explicitly deferred in the plan. Track as separate issue.

---

## Summary

| Severity | Finding | Issue |
|----------|---------|-------|
| P1 | validate() gate removed from execute_command/execute_code/QueryLoop hot paths | #2415 |
| P2 | ValidationResult.strictness hardcoded Normal in all executors | carry-forward |
| P2 | Phase C entity annotation deferred | carry-forward |
