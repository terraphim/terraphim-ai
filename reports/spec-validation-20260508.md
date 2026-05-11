# Spec Validation Report: 2026-05-08

**Validator:** Carthos (Domain Architect)
**Date:** 2026-05-08 02:33 CEST
**Prior run:** 2026-05-07 18:10 CEST (v4)
**HEAD commit:** `9524866154c898ace23ac49545098e86e248d787`
**Branch:** `main`
**Verdict:** FAIL — 2 persistent spec gaps (1 blocker, 1 medium); 1 process discrepancy; 1 format-compatibility observation

---

## Executive Summary

Three commits landed on `main` since v4 (commit `2526414d7`, 18:10 CEST):

| Commit | Summary |
|--------|---------|
| `6e175eb32` | docs(adr): add ADR-0001 Ollama trust boundary decision Refs #1313 #1318 |
| `88d2bc6d0` | fix(security_checklist): verify shard checksums before `deserialize_unchecked` Refs #1313 |
| `952486615` | ci: add `terraphim_automata/medical` feature to workspace test run Refs #1313 |

The SHA-256 checksum mitigation for security finding P1-1 is correctly implemented and now covered by CI. ADR-0001 formalises the Ollama trust boundary decision. Both persistent spec gaps from v4 remain open. Issue #1322 (tracking the P1-1 finding) is still open despite the mitigation being merged — a process discrepancy requiring closure.

---

## Specification-by-Specification Validation

### 1. `design-gitea82-correction-event.md` — FULLY IMPLEMENTED

No changes since v4. All eight unit tests and CLI integration test confirmed stable.

- `CorrectionType` enum: `capture.rs:44`
- `CorrectionEvent` struct: `capture.rs:502`
- `capture_correction()`: `mod.rs:41`
- `LearningEntry` enum: `capture.rs:1225`
- `list_all_entries`, `query_all_entries`: `mod.rs:42-43`
- `LearnSub::Correction` CLI: `main.rs:3138`

Status: **stable**.

---

### 2. `design-gitea84-trigger-based-retrieval.md` — MOSTLY IMPLEMENTED / MINOR GAP

No changes since v4. All primary acceptance criteria implemented.

Follow-up G-2026-05-07-2 persists: `kg list --pinned` CLI sub-command (`KgSub` enum) absent from `main.rs`.

Status: **stable (minor follow-up persists)**.

---

### 3. `d3-session-auto-capture-plan.md` — FULLY IMPLEMENTED

No changes since v4. All six unit tests confirmed, `#[cfg(feature = "repl-sessions")]` in place.

Status: **stable**.

---

### 4. `design-single-agent-listener.md` — OPERATIONAL

Infrastructure unchanged. No code-level regression.

Status: **stable**.

---

### 5. `learning-correction-system-plan.md` — GAP PERSISTS (Phase H)

**Gap G-2026-05-06-1: `guard.rs` module absent**

`crates/terraphim_agent/src/learnings/guard.rs` does not exist. Issue #1274 remains open.

Confirmed via directory listing (02:33 CEST):
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

`crates/terraphim_orchestrator/src/meta_coordinator.rs` (741 lines, five `#[tokio::test]` functions) remains absent from `pub mod` declarations in `crates/terraphim_orchestrator/src/lib.rs`.

Verified at 02:33 CEST: `grep "meta_coordinator" crates/terraphim_orchestrator/src/lib.rs` → 0 matches.

PR #1291 (`Fix #1275: wire meta_coordinator module into lib.rs`) remains `state=open, merged=False`.

**Severity:** Blocker — all 741 lines of dead code; five `#[tokio::test]` functions unreachable; `dispatch_cycle` integration invariant unverified; `last_cleanup` mutation bug unresolved (tracked in issue #1301).

---

## New Observations Since v4

### OBS-SEC-P1-1: Checksum Mitigation Merged — Issue #1322 Still Open

Commit `88d2bc6d0` correctly implements SHA-256 integrity checking as the mitigation strategy for security finding P1-1:

**Implementation chain verified:**
1. `sharded_extractor.rs`: `Sha256::digest(bytes).into()` computed per shard at save time; stored in `ArtifactHeader.shard_checksums`.
2. `medical_artifact.rs:158`: `Sha256::digest(shard_slice).into()` recomputed at load time; byte-compared to stored digest.
3. `medical_artifact.rs:160`: Hard error on mismatch — `"Shard {} checksum mismatch: artifact may be corrupt or tampered with"`.
4. `sharded_extractor.rs:219`: `deserialize_unchecked` called only on bytes that survived the integrity check.
5. Tamper-detection test (`test_artifact_checksum_mismatch_rejected`): flips one byte in a shard after saving; confirms `load_umls_artifact` returns error containing `"checksum mismatch"`.
6. CI: `ci-main.yml` now includes `terraphim_automata/medical` feature flag; checksum tests run in every build.

The commit message documents the architectural decision: daachorse 1.0.x exposes only `deserialize_unchecked` with no checked variant; checksum verification is the correct mitigation strategy.

**However:** Issue #1322 (`fix(security): replace deserialize_unchecked in automata sharded_extractor`) remains `state=open`. The issue title implies replacement; the merged fix chose verification-before-use instead. Issue #1322 should be updated to document the chosen approach and closed, or retitled to track any remaining concerns.

**Classification:** Process discrepancy. The code is correct; the issue tracker is stale.

---

### OBS-COMPAT: ArtifactHeader Binary Format Incompatibility

`ArtifactHeader` gained a new field `shard_checksums: Vec<[u8; 32]>`. Bincode serialises struct fields positionally without field names. Existing `.bin.zst` UMLS artifact files serialised before commit `88d2bc6d0` will fail to deserialise with a bincode `EOF` or type-mismatch error when loaded by `load_umls_artifact`.

**Impact:** Any cached UMLS artifacts on disk must be regenerated by running `save_to_artifact()`. This is expected for a security-motivated format change; no corrective code action is required. However, operator documentation or a migration note would prevent confusion during deployment.

**Classification:** Observation (expected breaking change; not a spec gap).

---

### OBS-P0: Infrastructure Security Findings Still Open

Security findings P0-1 (Redis `0.0.0.0:6380` unauthenticated) and P0-2 (Ollama `*:11434` unauthenticated) from issue #1313 remain open at the infrastructure level. ADR-0001 formally establishes the architectural intent for P0-2 (bind to `127.0.0.1:11434`); the operational fix (`OLLAMA_HOST=127.0.0.1` in systemd or environment) is a deployment concern outside the `terraphim-ai` codebase.

Neither P0 is a code gap in `terraphim-ai`; both are tracked in issue #1313.

**Classification:** Observation (infrastructure, not spec gap).

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
| REQ-84-004 | `kg list --pinned` CLI command | `§7` | ABSENT | ABSENT | `KgSub` enum not in `main.rs` | FOLLOW-UP |
| REQ-D3-001 | `learn procedure from-session <id>` CLI | `d3-session §design` | `main.rs:3413` | `test_from_session_commands_basic` | `main.rs:3413` | PASS |
| REQ-D3-002 | Trivial command filter | `d3-session §design` | `procedure.rs:412` | `test_from_session_commands_filters_trivial` | `procedure.rs:868` | PASS |
| REQ-D3-003 | Feature-gated `repl-sessions` | `d3-session §dependencies` | `main.rs:3413` | N/A | `#[cfg(feature = "repl-sessions")]` | PASS |
| PH-H-001 | `ExecutionTier` enum | `learning-correction-plan §Phase H` | `guard.rs` — ABSENT | ABSENT | File does not exist | FAIL |
| PH-H-002 | `evaluate_command()` | Same | Same | ABSENT | Same | FAIL |
| META-001 | `meta_coordinator` in public API | Gitea #1275 | `lib.rs` — ABSENT | 5 tests unreachable | `grep meta_coordinator lib.rs` → 0 hits | FAIL |
| SEC-P1-1 | SHA-256 verify before `deserialize_unchecked` | Issue #1313 P1-1 | `medical_artifact.rs:158-164` | `test_artifact_checksum_mismatch_rejected` | `medical_artifact.rs:158` | PASS |
| SEC-P1-1-CI | `medical` feature in CI workspace test | Issue #1313 P1-1 | `.github/workflows/ci-main.yml:231` | CI build | `ci-main.yml:231` | PASS |
| SEC-ADR-001 | Ollama trust boundary architectural decision | ADR-0001 | `docs/src/adr/0001-ollama-trust-boundary.md` | N/A (doc artefact) | File present, status=Accepted | PASS |

---

## Gap Summary

| Gap ID | Description | Severity | Issue | Status |
|--------|-------------|----------|-------|--------|
| G-2026-05-07-1 | `meta_coordinator` not in `lib.rs` — dead code; PR #1291 open/unmerged | Blocker | #1275 (closed), PR #1291 (open) | OPEN |
| G-2026-05-06-1 | `guard.rs` absent — Phase H Graduated Guard missing | Medium | #1274 (open) | OPEN |
| G-2026-05-07-2 | `kg list --pinned` CLI sub-command absent | Minor follow-up | (no issue) | FOLLOW-UP |
| PROCESS-001 | Issue #1275 closed without PR #1291 merged | Process | #1275, PR #1291 | OPEN |
| PROCESS-1322 | Issue #1322 open despite P1-1 mitigation merged | Process | #1322 (open) | ACTION NEEDED |

---

## Recommendations (smallest first)

1. **Merge PR #1291** — single line: `pub mod meta_coordinator;` in `lib.rs`. Unblocks dead tests and `dispatch_cycle` integration invariant. Highest-priority action.
2. **Close or update issue #1322** — document that daachorse 1.0.x has no checked variant; SHA-256 verification (commit `88d2bc6d0`) is the accepted mitigation; close with reference to the merged fix.
3. **Implement `guard.rs` (Phase H)** — `ExecutionTier` enum, `GuardDecision`, `evaluate_command()`. Self-contained; no other gaps block it. Issue #1274 open.
4. **Add UMLS artifact migration note** — warn operators that existing `.bin.zst` artifacts must be regenerated after upgrading past commit `88d2bc6d0`.
5. **Add `kg list --pinned` command** — trivial extension per spec §7; add `KgSub::List { pinned: bool }`.

---

## Conclusion

No regression in existing spec coverage. The SHA-256 checksum mitigation for P1-1 is correctly implemented, tested, and now covered by CI. ADR-0001 formally establishes the Ollama trust boundary. Two spec gaps persist: one blocker (`meta_coordinator` orphaned) and one medium (`guard.rs` absent). One process discrepancy: issue #1322 should be closed to reflect the chosen mitigation approach.

**Verdict: FAIL — 2 open spec gaps (1 blocker, 1 medium) + 1 process discrepancy**

---

<sub>Validated against commit `9524866154c898ace23ac49545098e86e248d787` on branch `main`.
Plans directory: 6 specs, unchanged since 2026-05-04.
Gitea API confirmed: PR #1291 `merged=False`, issue #1274 `state=open`, issue #1322 `state=open`.</sub>
