# Spec Validation Report: 2026-05-08 (v6)

**Validator:** Carthos (Domain Architect)
**Date:** 2026-05-08 07:33 CEST
**Prior run:** 2026-05-08 07:16 CEST (v5, HEAD at `24a5f64a4`)
**HEAD commit:** `81e155a56` (main)
**Verdict:** FAIL — 4 persistent spec gaps (unchanged from v5)

---

## Executive Summary

One commit landed on `main` since v5:

| Commit | Summary |
|--------|---------|
| `81e155a56` | docs: fix remaining rustdoc warnings across 11 crates Refs #1331 |

This is a documentation-only commit. No behavioural, API, or module-structure change. **None of the four v5 gaps are addressed.**

PR status update since v5:
- **PR #1291** (META-001 fix): state=open, merged=False, **mergeable=False** (conflict with main)
- **PR #1343** (REQ-1266 fix): state=open, merged=False, **mergeable=False** (conflict with main)

Both remediation PRs have accumulated merge conflicts as main has advanced through doc commits. They require a rebase before they can land.

---

## What Changed Since v5

### New Commit

`81e155a56` — docs: fix remaining rustdoc warnings across 11 crates Refs #1331

Doc-only. No struct fields, function signatures, trait implementations, or module declarations altered. **No spec requirement is affected.**

### NormalizedTerm Struct Literal Sites — Updated Count

`rg "NormalizedTerm \{" crates/` reveals the sites remain present on `main`. Struct literal syntax exists in:
- `crates/terraphim-session-analyzer/tests/terraphim_integration_tests.rs` (5 sites)
- `crates/terraphim-session-analyzer/src/kg/builder.rs` (1 site)
- `crates/terraphim-session-analyzer/src/patterns/matcher.rs` (1 site)
- `crates/terraphim_automata/src/autocomplete.rs` (1 site)
- `crates/terraphim_automata/tests/autocomplete_tests.rs` (2 sites)
- `crates/terraphim_automata/benches/autocomplete_bench.rs` (1 site)
- `crates/terraphim_agent/src/commands/markdown_parser.rs` (4 sites)
- `crates/terraphim_agent/src/commands/registry.rs` (3 sites)

Note: `crates/terraphim_automata/README.md` contains struct literal examples in documentation — these are not compiled Rust and do not affect `E0063`.

PR #1343 (fixing these to builder pattern) remains open and unmerged, and is now unmergeable without a rebase.

### Persistent Gap Status

All four gaps confirmed by direct verification at 07:33 CEST:

| Gap | Verification | Status |
|-----|-------------|--------|
| META-001: `meta_coordinator` absent from `lib.rs` | `grep "meta_coordinator" crates/terraphim_orchestrator/src/lib.rs` → ABSENT | OPEN |
| PH-H-001/002: `guard.rs` absent from `learnings/` | `ls crates/terraphim_agent/src/learnings/` → no `guard.rs` | OPEN |
| REQ-84-004: `Graph list --pinned` absent | `grep "GraphSub" crates/terraphim_agent/src/main.rs` → ABSENT | OPEN |
| REQ-1266-001: `NormalizedTerm` struct literals on main | 18+ sites in compiled Rust; PR #1343 open, unmergeable | REGRESSION |

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
| REQ-1266-001 | `NormalizedTerm` builder at all init sites | Gitea #1266 | 18+ struct literal sites in compiled Rust | `terraphim_integration_tests.rs` | PR #1343 open, unmergeable (conflict) | ❌ |
| REQ-D3-001 | `learn procedure from-session <id>` CLI | `d3-session §design` | `main.rs:3413` | `test_from_session_commands_basic` | `main.rs:3413` | ✅ |
| REQ-D3-002 | Trivial command filter | `d3-session §design` | `procedure.rs:412` | `test_from_session_commands_filters_trivial` | `procedure.rs:868` | ✅ |
| REQ-D3-003 | Feature-gated `repl-sessions` | `d3-session §dependencies` | `main.rs:3413` | N/A | `#[cfg(feature = "repl-sessions")]` | ✅ |
| PH-H-001 | `ExecutionTier` enum | `learning-correction-plan §Phase H` | `guard.rs` — ABSENT | ABSENT | File does not exist in `learnings/` | ❌ |
| PH-H-002 | `evaluate_command()` | Same | Same | ABSENT | Same | ❌ |
| META-001 | `meta_coordinator` in public API | Gitea #1275 | `lib.rs` — ABSENT | 5 tests unreachable | `grep meta_coordinator lib.rs` → exit 1 | ❌ |
| SEC-P1-1 | SHA-256 verify before `deserialize_unchecked` | Issue #1313 P1-1 | `medical_artifact.rs:158-164` | `test_artifact_checksum_mismatch_rejected` | commit `88d2bc6d0`; CI expanded at `952486615` | ✅ |
| SEC-ADR-001 | Ollama trust boundary ADR | ADR-0001 | `docs/src/adr/0001-ollama-trust-boundary.md` | N/A (doc artefact) | commit `6e175eb32`; file present, status=Accepted | ✅ |
| DOCS-1331-001 | Module-level rustdoc on all crates | Gitea #1331 | 24+ crates | N/A (doc artefact) | commits `c2c7151db`–`81e155a56` | ✅ |

---

## Gaps

| Gap ID | Description | Severity | Issue | Open PR | Status |
|--------|-------------|----------|-------|---------|--------|
| G-META-001 | `meta_coordinator` not declared in `lib.rs` — dead code; 5 unreachable tests | ❌ Blocker | #1275 (closed), #1301 | PR #1291 (open, **unmergeable — needs rebase**) | OPEN |
| G-PH-H-001 | `guard.rs` absent — `ExecutionTier`, `evaluate_command()` per Phase H spec | ❌ Blocker | #1274 (open) | None | OPEN |
| G-REQ-1266 | 18+ `NormalizedTerm` struct-literal sites on main → `E0063` under `--all-features` | ❌ Medium | #1266 (open) | PR #1343 (open, **unmergeable — needs rebase**) | REGRESSION |
| G-REQ-84-004 | `Graph list --pinned` CLI sub-command absent from `GraphSub` enum | ⚠️ Minor | (none) | None | FOLLOW-UP |

---

## Merge Conflict Alert

Both active remediation PRs have become unmergeable as main advanced through 9+ doc commits since the PRs were opened. The implementation on those branches is sound — the blocker is a mechanical rebase, not a design problem.

Recommended immediate action:
1. **Rebase PR #1291** onto current main — `pub mod meta_coordinator;` addition is conflict-free in substance; the doc-commit rebasing should be mechanical.
2. **Rebase PR #1343** onto current main — the builder-pattern migration is isolated to specific struct initialisation sites; no expected semantic conflicts.

---

## Recommendations (smallest first)

1. **Rebase and merge PR #1291** — one line `pub mod meta_coordinator;` in `lib.rs`. Unblocks 5 tests. Rebase needed before merge.
2. **Rebase and merge PR #1343** — convert 18+ struct-literal sites to `NormalizedTerm::new(...).with_url(...)` builder pattern. Restores `--all-features` CI green. Rebase needed before merge.
3. **Implement `guard.rs` (Phase H)** — `ExecutionTier` enum, `GuardDecision`, `evaluate_command()`. Self-contained; no other gaps block it. Refs #1274.
4. **Add `GraphSub::List { pinned: bool }`** — trivial extension; one match arm per spec §7 AC7.

---

## Conclusion

No change in gap count or severity since v5. The sole new commit (`81e155a56`) is documentation-only. Both open remediation PRs (PR #1291, PR #1343) are now in conflict with main and require rebase before they can be merged. Three failing requirements (META-001, PH-H, REQ-1266) each have a clear, bounded fix path; two have PRs ready once rebased.

**Verdict: FAIL**

---

<sub>Last spec-validated commit: 81e155a56
Plans directory: 6 specs, unchanged since 2026-05-04.
Open PRs addressing gaps: PR #1291 (META-001, unmergeable), PR #1343 (REQ-1266, unmergeable).
Gaps with no open PR: G-PH-H-001, G-REQ-84-004.
NormalizedTerm struct literal sites: 18 in compiled Rust (excluding README examples).</sub>
