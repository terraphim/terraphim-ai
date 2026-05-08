# Spec Validation Report: 2026-05-08 (v5)

**Validator:** Carthos (Domain Architect)
**Date:** 2026-05-08 07:16 CEST
**Prior run:** 2026-05-08 06:33 CEST (v4, main at commit `900c343d9`)
**HEAD commit:** `24a5f64a4` (main)
**Verdict:** FAIL — 4 persistent spec gaps (unchanged from v4); working-tree changes: doc-only, n/a

---

## Executive Summary

Seven commits landed on `main` since v4 (`c0fe9e51d`), all documentation or CI:

| Commit | Summary |
|--------|---------|
| `47624fa49` | docs(spec): add spec validation report 2026-05-08 Refs #1297 |
| `952486615` | ci: add terraphim_automata/medical feature to workspace test run Refs #1313 |
| `88d2bc6d0` | fix(security_checklist): verify shard checksums before deserialize_unchecked Refs #1313 |
| `6e175eb32` | docs(adr): add ADR-0001 Ollama trust boundary decision Refs #1313 #1318 |
| `23a6fc821` | style(automata): fix rustfmt line-break in medical_artifact test |
| `c2c7151db` | docs: add module-level rustdoc to five undocumented crates Refs #1331 |
| `e24694fc5` | docs: add module-level rustdoc to 17 undocumented crates Refs #1331 |
| `926439200` | docs: add module-level rustdoc to final two undocumented binary crates |
| `24a5f64a4` | docs: fix rustdoc intra-doc link warnings and update CHANGELOG Refs #1331 |

**None of these commits addresses any of the four v4 gaps.**

Working-tree changes (unstaged): 4 `.rs` files, all rustdoc link-format fixes only — no behavioural or API change. Spec traceability: **n/a**.

SEC-P1-1 status improved: commit `88d2bc6d0` confirms `medical_artifact.rs:158-164` verifies SHA-256 before `deserialize_unchecked`. Tests confirmed in CI expansion at `952486615`. Promoted to PASS.

---

## What Changed Since v4

### Working-Tree Diff (Unstaged, Not Yet Committed)

| File | Change |
|------|--------|
| `crates/terraphim_middleware/src/lib.rs` | Rustdoc link: `[search_haystacks](path)` → `` `search_haystacks` `` |
| `crates/terraphim_tracker/src/gitea.rs` | Two doc-comment link formats: `[fn]` → backtick form |
| `crates/terraphim_tracker/src/linear.rs` | URL in doc comment wrapped in backticks |
| `crates/terraphim_types/src/lib.rs` | Module-level doc: two type references wrapped in backticks |

These are `///` and `//!` documentation changes. No struct fields, no function signatures, no trait implementations altered. **No spec requirement is affected.**

### Persistent Gap Status

All four gaps confirmed by direct verification at 07:16 CEST:

| Gap | Verification | Status |
|-----|-------------|--------|
| META-001: `meta_coordinator` absent from `lib.rs` | `grep meta_coordinator crates/terraphim_orchestrator/src/lib.rs` → exit 1 | OPEN |
| PH-H-001/002: `guard.rs` absent | `ls crates/terraphim_agent/src/learnings/` → no `guard.rs` | OPEN |
| REQ-84-004: `Graph list --pinned` absent | `grep GraphSub crates/terraphim_agent/src/main.rs` → exit 1 | OPEN |
| REQ-1266-001: `NormalizedTerm` struct literals | PR #1343 state=open, merged=False; fix not on main | REGRESSION |

PR #1291 (META-001 fix): state=open, merged=False. Confirmed at 07:16 CEST.

---

## Traceability Matrix

| Req ID | Requirement | Design Ref | Impl Ref | Tests | Evidence | Status |
|-------:|-------------|------------|----------|-------|----------|--------|
| REQ-82-001 | `CorrectionEvent` struct | `design-gitea82 §1.2` | `capture.rs:502` | `test_correction_event_roundtrip` | `capture.rs:502` | ✅ |
| REQ-82-002 | `learn_correction()` with redaction | `§1.4` | `mod.rs:41` | `test_learn_correction` | `mod.rs:41` | ✅ |
| REQ-82-003 | `LearnSub::Correction` CLI | `§3.1` | `main.rs:3138` | CLI integration test | `main.rs:3138` | ✅ |
| REQ-82-004 | Unified `list_all_entries` / `query_all_entries` | `§1.5` | `mod.rs:42-43` | `test_list_all_entries_mixed` | `mod.rs:42` | ✅ |
| REQ-84-001 | `trigger::` / `pinned::` directive parsing | `design-gitea84 §2` | `markdown_directives.rs:215` | `parses_trigger_directive` | `markdown_directives.rs:348` | ✅ |
| REQ-84-002 | `TriggerIndex` TF-IDF fallback | `§3` | `rolegraph/lib.rs:51` | `two_pass_fallback_to_trigger` | `lib.rs:2196` | ✅ |
| REQ-84-003 | `--include-pinned` Search CLI flag | `§7` | `main.rs:718` | AC6 | `main.rs:718` | ✅ |
| REQ-84-004 | `Graph list --pinned` CLI command | `§7` | ABSENT | ABSENT | `GraphSub` not in `main.rs` | ⚠️ |
| REQ-1266-001 | `NormalizedTerm` builder at all init sites | Gitea #1266 | 8 sites in session-analyzer / sessions | `terraphim_integration_tests.rs` | PR #1343 open; struct literals on main cause `E0063` | ❌ |
| REQ-D3-001 | `learn procedure from-session <id>` CLI | `d3-session §design` | `main.rs:3413` | `test_from_session_commands_basic` | `main.rs:3413` | ✅ |
| REQ-D3-002 | Trivial command filter | `d3-session §design` | `procedure.rs:412` | `test_from_session_commands_filters_trivial` | `procedure.rs:868` | ✅ |
| REQ-D3-003 | Feature-gated `repl-sessions` | `d3-session §dependencies` | `main.rs:3413` | N/A | `#[cfg(feature = "repl-sessions")]` | ✅ |
| PH-H-001 | `ExecutionTier` enum | `learning-correction-plan §Phase H` | `guard.rs` — ABSENT | ABSENT | File does not exist | ❌ |
| PH-H-002 | `evaluate_command()` | Same | Same | ABSENT | Same | ❌ |
| META-001 | `meta_coordinator` in public API | Gitea #1275 | `lib.rs` — ABSENT | 5 tests unreachable | `grep meta_coordinator lib.rs` → exit 1 | ❌ |
| SEC-P1-1 | SHA-256 verify before `deserialize_unchecked` | Issue #1313 P1-1 | `medical_artifact.rs:158-164` | `test_artifact_checksum_mismatch_rejected` | commit `88d2bc6d0`; CI expanded at `952486615` | ✅ |
| SEC-ADR-001 | Ollama trust boundary ADR | ADR-0001 | `docs/src/adr/0001-ollama-trust-boundary.md` | N/A (doc artefact) | commit `6e175eb32`; file present, status=Accepted | ✅ |
| DOCS-1331-001 | Module-level rustdoc on all crates | Gitea #1331 | 24+ crates | N/A (doc artefact) | commits `c2c7151db`–`24a5f64a4` | ✅ |

---

## Gaps

| Gap ID | Description | Severity | Issue | Open PR | Status |
|--------|-------------|----------|-------|---------|--------|
| G-META-001 | `meta_coordinator` not declared in `lib.rs` — 25 KB dead code; 5 unreachable tests | ❌ Blocker | #1275 (closed), #1301 | PR #1291 (open, unmerged) | OPEN |
| G-PH-H-001 | `guard.rs` absent — `ExecutionTier`, `evaluate_command()` per Phase H spec | ❌ Blocker | #1274 (open) | None | OPEN |
| G-REQ-1266 | 8 `NormalizedTerm` struct-literal sites on main → `E0063` under `--all-features` | ❌ Medium | #1266 (open) | PR #1343 (open, unmerged) | REGRESSION |
| G-REQ-84-004 | `Graph list --pinned` CLI sub-command absent from `GraphSub` enum | ⚠️ Minor | (none) | None | FOLLOW-UP |

---

## Recommendations (smallest first)

1. **Merge PR #1291** — one line `pub mod meta_coordinator;` in `lib.rs`. Unblocks 5 tests and `dispatch_cycle` integration invariant. No code review risk; the module already exists.
2. **Merge PR #1343** — convert 8 struct-literal sites to `NormalizedTerm::new(...).with_url(...)` builder pattern. Restores `--all-features` CI green. Refs #1266.
3. **Implement `guard.rs` (Phase H)** — `ExecutionTier` enum, `GuardDecision`, `evaluate_command()`. Self-contained; no other gaps block it. Refs #1274.
4. **Add `GraphSub::List { pinned: bool }`** — trivial extension; one match arm per spec §7 AC7.
5. **Commit working-tree rustdoc fixes** — the 4 unstaged `.rs` changes are doc-only but should be committed to keep the working tree clean.

---

## Conclusion

No change in gap count or severity since v4. Working-tree changes are doc formatting only; they do not touch any requirement boundary. The traceability matrix is stable on passing requirements. Three failing requirements (META-001, PH-H, REQ-1266) each have a clear, bounded fix path.

**Verdict: fail**

---

<sub>Last spec-validated commit: 24a5f64  
Plans directory: 6 specs, unchanged since 2026-05-04.  
Open PRs addressing gaps: PR #1291 (META-001), PR #1343 (REQ-1266).  
Gaps with no open PR: G-PH-H-001, G-REQ-84-004.</sub>
