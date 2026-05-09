# Spec Validation Report: 2026-05-09

**Validator:** Carthos (Domain Architect)
**Date:** 2026-05-09 09:23 CEST (updated from 08:33 run)
**Prior run:** 2026-05-08 02:33 CEST
**HEAD commit:** `da80ef44ae2c84f2ce3f170da34c3e3dcbeaef58`
**Branch:** `main`
**Verdict:** FAIL — 2 persistent spec gaps (1 blocker, 1 medium); 1 minor follow-up; 1 spec-gap RESOLVED (#251 RetryBound)

---

## Update: Issue #251 — RetryBound Invariant — RESOLVED

**Commit:** `da80ef44a` — `fix(symphony): enforce RetryBound invariant on slot exhaustion and poll failure Refs #251`

### TLA+ Invariant Mapping

The `RetryBound` safety invariant from `specs/symphony/SymphonyOrchestrator.tla`:

```
RetryBound == \A i \in DOMAIN retryCount: retryCount[i] <= MaxRetries
```

| TLA+ Action | Condition | Rust Implementation | Location |
|------------|-----------|---------------------|----------|
| `RetryFire` | `retryCount[i] < MaxRetries` | `next_attempt <= max_retries` → `schedule_retry()` | `orchestrator/mod.rs:631–638` |
| `RetryGiveUp` | `retryCount[i] >= MaxRetries` | `next_attempt > max_retries` → `claimed.remove()` | `orchestrator/mod.rs:624–630` |

Both violation paths guarded identically:
- **Slot exhaustion path** (`available_slots() == 0`): lines 622–639
- **Poll failure path** (`tracker.fetch_candidate_issues()` error): lines 591–608

### Configuration

`ServiceConfig::max_retry_attempts()` added at `config/mod.rs:184–190`:
- Default: 10 (matches TLA+ `MaxRetries` parameter default)
- Configurable via `agent.max_retry_attempts` in WORKFLOW.md front matter

### Regression Tests (Refs #1389)

All 144 tests pass (`cargo test --manifest-path crates/terraphim_symphony/Cargo.toml`):

| Test | Scenario | Invariant Verified |
|------|----------|-------------------|
| `slot_exhaustion_at_max_retries_releases_claim` | attempt=3, max=3 → claim released | RetryGiveUp path |
| `poll_failure_at_max_retries_releases_claim` | attempt=3, max=3, tracker error → claim released | Poll-failure GiveUp |
| `slot_exhaustion_below_max_retries_reschedules` | attempt=2, max=10 → reschedule at attempt=3 | RetryFire path |

### Verdict for #251: **PASS**

The fix correctly enforces `RetryBound`. No retry attempt counter can exceed `max_retry_attempts`; the claim is released on exhaustion in both failure modes. The TLA+ invariant holds in the Rust implementation.

---

## Executive Summary

Eight commits landed on `main` since the 2026-05-08 report (HEAD `9524866154c`):

| Commit | Summary |
|--------|---------|
| `bb1e31235` | docs(symphony): field-level docs to snapshot structs; fix redundant link warning |
| `fd484da22` | docs(reports): add security_checklist for PR #1349 RetryBound fix |
| `8747f2552` | docs(reports): add spec-validation and compliance reports 2026-05-08 |
| `7609a320a` | docs(reports): full traceability matrix for PR #1349 |
| `a67f129b1` | fix(tests): replace cargo-run subprocess with assert_cmd timeout |
| `003088893` | fix(tests): use Terraphim Engineer role in test_full_feature_matrix |
| `2b2996ce0` | docs: fix private intra-doc link warning in terraphim_server |
| `32b7333c8` | feat(build-runner): agent work [auto-commit] |

None of the eight commits address spec gaps. All are documentation, test fixes, or build automation. The two persistent spec gaps (blocker: `meta_coordinator` orphaned; medium: `guard.rs` absent) remain unchanged. One process improvement: issue #1322 is now closed, resolving PROCESS-1322 from the prior run.

---

## Specification-by-Specification Validation

### 1. `design-gitea82-correction-event.md` — FULLY IMPLEMENTED

No changes since 2026-05-08 report. All eight unit tests and CLI integration test remain stable.

- `CorrectionType` enum: `capture.rs:44`
- `CorrectionEvent` struct: `capture.rs:502`
- `capture_correction()`: `mod.rs:41`
- `LearningEntry` enum: `capture.rs:1225`
- `list_all_entries`, `query_all_entries`: `mod.rs:42-43`
- `LearnSub::Correction` CLI: `main.rs:3138`

Status: **stable**.

---

### 2. `design-gitea84-trigger-based-retrieval.md` — MOSTLY IMPLEMENTED / MINOR GAP

Primary acceptance criteria remain implemented. All trigger parsing, TF-IDF index, and two-pass search are in place.

**Gap G-2026-05-07-2 persists (clarified):**

The spec (§7) specifies:

```
Kg(KgSub),

enum KgSub {
    List { pinned: bool }
}
```

What exists in `main.rs:730-739` is:

```rust
Graph {
    role: Option<String>,
    top_k: usize,
    /// Show only pinned entries
    pinned: bool,
}
```

The pinned-filter capability exists but via the `Graph` command, not a dedicated `Kg(KgSub)` subcommand structure. The user-facing command is `terraphim-agent graph --pinned` rather than the spec-required `terraphim-agent kg list --pinned`. This is a functional but non-conformant implementation of REQ-84-004.

Status: **stable (minor follow-up persists, command surface diverges from spec)**.

---

### 3. `d3-session-auto-capture-plan.md` — FULLY IMPLEMENTED

No changes since prior report. All six unit tests confirmed, `#[cfg(feature = "repl-sessions")]` in place.

Status: **stable**.

---

### 4. `design-single-agent-listener.md` — OPERATIONAL

Infrastructure unchanged. No code-level regression.

Status: **stable**.

---

### 5. `learning-correction-system-plan.md` — GAP PERSISTS (Phase H)

**Gap G-2026-05-06-1: `guard.rs` module absent**

`crates/terraphim_agent/src/learnings/guard.rs` does not exist. Issue #1274 remains open.

Confirmed directory listing (08:33 CEST):
```
capture.rs  compile.rs  export_kg.rs  hook.rs  install.rs
mod.rs  procedure.rs  redaction.rs  replay.rs  suggest.rs
```

No `guard.rs`.

Required per spec Phase H:
- `ExecutionTier` enum (Allow / Sandbox / Deny)
- `GuardDecision` type
- `evaluate_command()` function
- Integration with Firecracker sandbox tier

**Severity:** Medium — no automated command safety evaluation before procedure replay.

Phases A–G confirmed implemented per prior runs. Status: **gap persists**.

---

### 6. `research-single-agent-listener.md` — RESEARCH COMPLETE

Phase 1 artefact only; no implementation deliverables. Status: **stable**.

---

## Persistent Non-Spec Gap: `meta_coordinator.rs` Orphaned

**Gap G-2026-05-07-1: `meta_coordinator` not declared in `lib.rs`**

`crates/terraphim_orchestrator/src/meta_coordinator.rs` (25KB, modified 2026-05-06) remains absent from `pub mod` declarations in `crates/terraphim_orchestrator/src/lib.rs`.

Verified at 08:33 CEST: `grep "meta_coordinator" crates/terraphim_orchestrator/src/lib.rs` → 0 matches.

PR #1291 (`Fix #1275: wire meta_coordinator module into lib.rs`) remains `state=open, merged=False`.

**Severity:** Blocker — all 741 lines of dead code; five `#[tokio::test]` functions unreachable; `dispatch_cycle` integration invariant unverified; `last_cleanup` mutation bug unresolved (tracked in issue #1301).

---

## Process Improvements Since Prior Run

### PROCESS-1322 RESOLVED: Issue #1322 Closed

Issue #1322 (`fix(security): replace deserialize_unchecked in automata sharded_extractor`) is now `state=closed`.

The closure correctly reflects the architectural decision documented in commit `88d2bc6d0`: daachorse 1.0.x exposes no checked variant; SHA-256 verification before `deserialize_unchecked` is the accepted mitigation, implemented in `medical_artifact.rs:158-164` and covered by CI since the 2026-05-08 report.

PROCESS-1322 is **resolved**.

---

## Traceability Matrix

| Req ID | Requirement | Design Ref | Impl Ref | Tests | Evidence | Status |
|-------:|-------------|------------|----------|-------|----------|--------|
| REQ-82-001 | `CorrectionEvent` struct | `design-gitea82 §1.2` | `capture.rs:502` | `test_correction_event_roundtrip` | `capture.rs:502` | PASS |
| REQ-82-002 | `capture_correction()` with redaction | `§1.4` | `mod.rs:41` | `test_capture_correction` | `mod.rs:41` | PASS |
| REQ-82-003 | `LearnSub::Correction` CLI | `§3.1` | `main.rs:3138` | CLI integration test | `main.rs:3138` | PASS |
| REQ-82-004 | Unified `list_all_entries` / `query_all_entries` | `§1.5` | `mod.rs:42-43` | `test_list_all_entries_mixed` | `mod.rs:42` | PASS |
| REQ-84-001 | `trigger::` / `pinned::` directive parsing | `design-gitea84 §2` | `markdown_directives.rs:215` | `parses_trigger_directive` | `markdown_directives.rs:348` | PASS |
| REQ-84-002 | `TriggerIndex` TF-IDF fallback | `§3` | `rolegraph/lib.rs:51` | `two_pass_fallback_to_trigger` | `lib.rs:2196` | PASS |
| REQ-84-003 | `--include-pinned` search CLI flag | `§7` | `main.rs:718` | AC6 | `main.rs:718` | PASS |
| REQ-84-004 | `kg list --pinned` CLI command | `§7` | `main.rs:730` (via `Graph --pinned`, not `KgSub::List`) | Partial | Command surface diverges from spec | PARTIAL |
| REQ-D3-001 | `learn procedure from-session <id>` CLI | `d3-session §design` | `main.rs:3413` | `test_from_session_commands_basic` | `main.rs:3413` | PASS |
| REQ-D3-002 | Trivial command filter | `d3-session §design` | `procedure.rs:412` | `test_from_session_commands_filters_trivial` | `procedure.rs:868` | PASS |
| REQ-D3-003 | Feature-gated `repl-sessions` | `d3-session §dependencies` | `main.rs:3413` | N/A | `#[cfg(feature = "repl-sessions")]` | PASS |
| PH-H-001 | `ExecutionTier` enum | `learning-correction-plan §Phase H` | `guard.rs` — ABSENT | ABSENT | File does not exist | FAIL |
| PH-H-002 | `evaluate_command()` | Same | Same | ABSENT | Same | FAIL |
| META-001 | `meta_coordinator` in public API | Gitea #1275 | `lib.rs` — ABSENT | 5 tests unreachable | `grep meta_coordinator lib.rs` → 0 hits | FAIL |
| SEC-P1-1 | SHA-256 verify before `deserialize_unchecked` | Issue #1313 P1-1 | `medical_artifact.rs:158-164` | `test_artifact_checksum_mismatch_rejected` | `medical_artifact.rs:158` | PASS |
| SEC-P1-1-CI | `medical` feature in CI workspace test | Issue #1313 P1-1 | `.github/workflows/ci-main.yml:231` | CI build | `ci-main.yml:231` | PASS |
| SEC-ADR-001 | Ollama trust boundary architectural decision | ADR-0001 | `docs/src/adr/0001-ollama-trust-boundary.md` | N/A (doc artefact) | File present, status=Accepted | PASS |
| TLA-251-001 | `RetryBound`: retry counter ≤ MaxRetries always | TLA+ `SymphonyOrchestrator.tla` (external) | `orchestrator/mod.rs:592,624` | `slot_exhaustion_at_max_retries_releases_claim`, `poll_failure_at_max_retries_releases_claim`, `slot_exhaustion_below_max_retries_reschedules` | `da80ef44a`; 144/144 tests pass | PASS |
| TLA-251-002 | `RetryGiveUp`: claim released when exhausted | Same | `orchestrator/mod.rs:598,630` | Same | Same | PASS |
| TLA-251-003 | `max_retry_attempts` configurable per WORKFLOW | Issue #251 fix spec | `config/mod.rs:184–190` | `max_retry_attempts_default`, `max_retry_attempts_custom` | `config/mod.rs:533–544` | PASS |

---

## Gap Summary

| Gap ID | Description | Severity | Issue | Status |
|--------|-------------|----------|-------|--------|
| G-2026-05-07-1 | `meta_coordinator` not in `lib.rs` — dead code; PR #1291 open/unmerged | Blocker | #1275 (closed), PR #1291 (open) | OPEN |
| G-2026-05-06-1 | `guard.rs` absent — Phase H Graduated Guard missing | Medium | #1274 (open) | OPEN |
| G-2026-05-07-2 | `kg list --pinned` CLI: spec requires `KgSub::List`; actual is `Graph --pinned` | Minor | (no issue) | FOLLOW-UP |
| PROCESS-001 | Issue #1275 closed without PR #1291 merged | Process | #1275, PR #1291 | OPEN |
| PROCESS-1322 | Issue #1322 open despite P1-1 mitigation merged | Process | #1322 | **RESOLVED** |

---

## Recommendations (smallest first)

1. **Merge PR #1291** — single line: `pub mod meta_coordinator;` in `lib.rs`. Unblocks dead tests and `dispatch_cycle` integration invariant. Highest-priority action.
2. **Implement `guard.rs` (Phase H)** — `ExecutionTier` enum, `GuardDecision`, `evaluate_command()`. Self-contained; no other gaps block it. Issue #1274 open.
3. **Add `kg list --pinned` command** — extend `main.rs` with `Kg(KgSub)` variant and `KgSub::List { pinned }` per spec §7. Trivial; the underlying `pinned_node_ids` infrastructure is already in place.

---

## Conclusion

No regression in existing spec coverage since 2026-05-08. Eight commits landed, all documentation, test, and build — no spec implementation. One process improvement: issue #1322 is now closed, correctly reflecting the SHA-256 mitigation strategy chosen. Two spec gaps persist: one blocker (`meta_coordinator` orphaned, PR #1291 unmerged) and one medium (`guard.rs` Phase H absent). REQ-84-004 remains partial — the pinned filter is implemented via `Graph --pinned` rather than the spec-required `KgSub::List` subcommand structure.

**Verdict: FAIL — 2 open spec gaps (1 blocker, 1 medium) + 1 minor follow-up**

---

<sub>Validated against commit `bb1e312351e8429a36b5de21d87509d380243bd0` on branch `main`.
Plans directory: 6 specs, unchanged since 2026-05-04.
Gitea API confirmed: PR #1291 `merged=False`, issue #1274 `state=open`, issue #1322 `state=closed`.</sub>
