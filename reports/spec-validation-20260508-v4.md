# Spec Validation Report: 2026-05-08 (v4)

**Validator:** Carthos (Domain Architect)
**Date:** 2026-05-08 06:33 CEST
**Prior run:** 2026-05-08 05:33 CEST (v3, branch pr-1343)
**HEAD commit:** `900c343d9` (main, detached HEAD)
**Branch:** main
**Verdict:** FAIL — 4 spec gaps (1 blocker, 1 medium, 1 minor, 1 regression)

---

## Executive Summary

Two commits landed on `main` since the v3 run on `pr-1343`:

| Commit | Summary |
|--------|---------|
| `cde69643b` | fix(test): use unique tempdir in test_tool_index_save_and_load Refs #1340 |
| `900c343d9` | feat(build-runner): agent work [auto-commit] |

Neither addresses any of the three persistent spec gaps identified in v3.

**Critical regression identified:** The `NormalizedTerm` builder pattern fix (`1e9847d20`, Refs
#1266) from `pr-1343` was not merged to main. Eight struct literal initialisations omitting the
four optional fields (`action`, `priority`, `trigger`, `pinned`) remain on main. These cause
`E0063` compilation errors under `--all-features` or when `repl-sessions` feature is active.

Gap count: **4 open** (1 blocker, 1 medium, 1 minor, 1 regression).

---

## What Changed Since v3

The v3 validation was performed on branch `pr-1343` at commit `03d01837f`. We are now on
`main`. The `pr-1343` NormalizedTerm compile fix was not merged.

### New Findings

**REQ-1266-REGRESSION — NormalizedTerm struct literals on main**

Eight initialisations use struct literal syntax omitting the four optional fields introduced by
`design-gitea84-trigger-based-retrieval.md`. These will fail with `E0063` under `--all-features`.

| File | Line | Missing Fields |
|------|------|---------------|
| `crates/terraphim_sessions/src/enrichment/enricher.rs` | 315 | `action`, `priority`, `trigger`, `pinned` |
| `crates/terraphim-session-analyzer/src/kg/builder.rs` | 89 | `action`, `priority`, `trigger`, `pinned` |
| `crates/terraphim-session-analyzer/src/patterns/matcher.rs` | 213 | `action`, `priority`, `trigger`, `pinned` |
| `crates/terraphim-session-analyzer/tests/terraphim_integration_tests.rs` | 25 | (5 occurrences at L25, L74, L229, L239, L367) |

The fix exists on `pr-1343` (commit `1e9847d20`): convert all sites to
`NormalizedTerm::new(id, value).with_url(...)` builder pattern.

---

## Specification-by-Specification Validation

### 1. `design-gitea82-correction-event.md` — FULLY IMPLEMENTED

No changes since v3. All acceptance criteria stable.

Status: **stable**.

---

### 2. `design-gitea84-trigger-based-retrieval.md` — MOSTLY IMPLEMENTED

Primary acceptance criteria implemented. The four optional fields on `NormalizedTerm`
(`action`, `priority`, `trigger`, `pinned`) are confirmed in
`crates/terraphim_types/src/lib.rs:301-327` with builder pattern constructors at `:329-343`.

**Follow-up G-2026-05-07-2 persists:** `Graph list --pinned` CLI sub-command absent.
Confirmed: grep for `GraphSub\|pinned` in `crates/terraphim_agent/src/main.rs` returns 0
matches.

**Regression REQ-1266:** struct literal initialisations downstream of the four new fields
cause `E0063` with `--all-features`. Fix on `pr-1343` not yet merged to `main`.

Status: **partially regressed on main (compile fix absent)**.

---

### 3. `d3-session-auto-capture-plan.md` — FULLY IMPLEMENTED

No changes since v3. All six unit tests confirmed.

Status: **stable**.

---

### 4. `design-single-agent-listener.md` — OPERATIONAL

No changes since v3.

Status: **stable**.

---

### 5. `learning-correction-system-plan.md` — GAP PERSISTS (Phase H)

**Gap G-2026-05-06-1: `guard.rs` module absent**

`crates/terraphim_agent/src/learnings/guard.rs` does not exist.

Current directory contents:
```
capture.rs  compile.rs  export_kg.rs  hook.rs  install.rs
mod.rs  procedure.rs  redaction.rs  replay.rs  suggest.rs
```

Required per spec Phase H:
- `ExecutionTier` enum (Allow / Sandbox / Deny)
- `GuardDecision` type
- `evaluate_command()` function
- Integration with Firecracker sandbox tier

Issue #1274 remains open.

Status: **gap persists**.

---

### 6. `research-single-agent-listener.md` — Research artefact only

No implementation deliverables. Status: **stable**.

---

## Persistent Non-Spec Gap: `meta_coordinator.rs` Orphaned

**Gap G-2026-05-07-1: `meta_coordinator` not declared in `lib.rs`**

`crates/terraphim_orchestrator/src/meta_coordinator.rs` (25 KB) remains absent from `pub mod`
declarations in `crates/terraphim_orchestrator/src/lib.rs`.

Verified at 06:33 CEST: grep for `meta_coordinator` in `lib.rs` returns exit code 1 (no
matches).

PR #1291 (`Fix #1275: wire meta_coordinator module into lib.rs`): **state=open, merged=False**
(confirmed in v3; no merge commit visible in git log since then).

**Severity:** Blocker — dead code; five `#[tokio::test]` functions unreachable; `dispatch_cycle`
integration invariant unverified; `last_cleanup` mutation bug unresolved (issue #1301).

---

## Traceability Matrix

| Req ID | Requirement | Design Ref | Impl Ref | Tests | Evidence | Status |
|-------:|-------------|------------|----------|-------|----------|--------|
| REQ-82-001 | `CorrectionEvent` struct | `design-gitea82 §1.2` | `capture.rs:502` | `test_correction_event_roundtrip` | `capture.rs:502` | PASS |
| REQ-82-002 | `learn_correction()` with redaction | `§1.4` | `mod.rs:41` | `test_learn_correction` | `mod.rs:41` | PASS |
| REQ-82-003 | `LearnSub::Correction` CLI | `§3.1` | `main.rs:3138` | CLI integration test | `main.rs:3138` | PASS |
| REQ-82-004 | Unified `list_all_entries` / `query_all_entries` | `§1.5` | `mod.rs:42-43` | `test_list_all_entries_mixed` | `mod.rs:42` | PASS |
| REQ-84-001 | `trigger::` / `pinned::` directive parsing | `design-gitea84 §2` | `markdown_directives.rs:215` | `parses_trigger_directive` | `markdown_directives.rs:348` | PASS |
| REQ-84-002 | `TriggerIndex` TF-IDF fallback | `§3` | `rolegraph/lib.rs:51` | `two_pass_fallback_to_trigger` | `lib.rs:2196` | PASS |
| REQ-84-003 | `--include-pinned` Search CLI flag | `§7` | `main.rs:718` | AC6 | `main.rs:718` | PASS |
| REQ-84-004 | `Graph list --pinned` CLI command | `§7` | ABSENT | ABSENT | GraphSub enum not in main.rs | FOLLOW-UP |
| REQ-1266-001 | `NormalizedTerm` builder pattern at all init sites | Gitea #1266 | 8 sites in terraphim-session-analyzer / terraphim_sessions | `terraphim_integration_tests.rs` | Struct literal syntax on main; fix on pr-1343 not merged | FAIL (main) |
| REQ-D3-001 | `learn procedure from-session <id>` CLI | `d3-session §design` | `main.rs:3413` | `test_from_session_commands_basic` | `main.rs:3413` | PASS |
| REQ-D3-002 | Trivial command filter | `d3-session §design` | `procedure.rs:412` | `test_from_session_commands_filters_trivial` | `procedure.rs:868` | PASS |
| REQ-D3-003 | Feature-gated `repl-sessions` | `d3-session §dependencies` | `main.rs:3413` | N/A | `#[cfg(feature = "repl-sessions")]` | PASS |
| PH-H-001 | `ExecutionTier` enum | `learning-correction-plan §Phase H` | guard.rs — ABSENT | ABSENT | File does not exist | FAIL |
| PH-H-002 | `evaluate_command()` | Same | Same | ABSENT | Same | FAIL |
| META-001 | `meta_coordinator` in public API | Gitea #1275 | lib.rs — ABSENT | 5 tests unreachable | grep meta_coordinator lib.rs = exit 1 | FAIL |
| SEC-P1-1 | SHA-256 verify before `deserialize_unchecked` | Issue #1313 P1-1 | `medical_artifact.rs:158-164` | `test_artifact_checksum_mismatch_rejected` | `medical_artifact.rs:158` | PASS |
| SEC-ADR-001 | Ollama trust boundary architectural decision | ADR-0001 | `docs/src/adr/0001-ollama-trust-boundary.md` | N/A (doc artefact) | File present, status=Accepted | PASS |

---

## Gap Summary

| Gap ID | Description | Severity | Issue | Status |
|--------|-------------|----------|-------|--------|
| G-2026-05-07-1 | `meta_coordinator` not in `lib.rs` — dead code; PR #1291 open/unmerged | Blocker | #1275 (closed), PR #1291 (open) | OPEN |
| G-2026-05-06-1 | `guard.rs` absent — Phase H Graduated Guard missing | Medium | #1274 (open) | OPEN |
| G-2026-05-07-2 | `Graph list --pinned` CLI sub-command absent | Minor | (no dedicated issue) | FOLLOW-UP |
| REQ-1266-REGR | 8 `NormalizedTerm` struct literal sites on main cause `E0063` | Medium | #1266 (open) | REGRESSION (pr-1343 fix not merged) |

---

## Recommendations (smallest first)

1. **Merge PR #1291** — single line: `pub mod meta_coordinator;` in `lib.rs`. Unblocks dead
   tests and `dispatch_cycle` integration invariant. Highest-priority action.
2. **Merge pr-1343** (or cherry-pick `1e9847d20`) — convert 8 struct literal sites to
   `NormalizedTerm::new(id, value).with_url(...)` builder pattern. Unblocks `--all-features`
   CI. Issue #1266.
3. **Implement `guard.rs` (Phase H)** — `ExecutionTier` enum, `GuardDecision`,
   `evaluate_command()`. Self-contained; no other gaps block it. Issue #1274.
4. **Add `Graph list --pinned` command** — trivial extension per spec §7; add
   `GraphSub::List { pinned: bool }`.

---

## Conclusion

Three spec gaps from v3 persist unchanged. A new regression is confirmed: the
`NormalizedTerm` compile fix from `pr-1343` was not merged to main, leaving 8 struct literal
sites that cause `E0063` under `--all-features`. This degrades the REQ-1266 status from PASS
(on pr-1343) to FAIL (on main).

**Verdict: FAIL — 2 blocker/medium spec gaps; 1 minor follow-up; 1 compile regression on main**

---

<sub>Validated against commit `900c343d9` on main.
Plans directory: 6 specs, unchanged since 2026-05-04.
Gitea API: PR #1291 state confirmed as unmerged via v3 baseline; issue #1274 open.
New since v3: REQ-1266 regression (8 struct literal sites; pr-1343 fix absent from main).</sub>
