# Spec Validation Report: 2026-05-08 (v8)

**Validator:** Carthos (Domain Architect)
**Date:** 2026-05-08 09:35 CEST (updated 09:43 CEST with confirmed PR status)
**Prior run:** 2026-05-08 08:33 CEST (v7, HEAD at `53408a7be`)
**HEAD commit:** `0c4948474` (main)
**Verdict:** FAIL — 4 persistent spec gaps (unchanged from v7)

---

## Executive Summary

One commit landed on `main` since v7:

| Commit | Summary |
|--------|---------|
| `0c4948474` | docs: add doc gap report 2026-05-08 -- zero gaps found Refs #1331 |

This commit adds `reports/doc-gap-report-20260508.md` only. It touches no path under `plans/`, `crates/**/src/`, or any `.rs` file. **No spec requirement is affected.**

PR mergeable status confirmed at 09:43 CEST via Gitea API. Key findings: PR #1283 is **mergeable today** and addresses REQ-1266; PR #1291, #1308, #1343 all require rebase. See updated PR status section below.

---

## What Changed Since v7

### Commit `0c4948474` — docs: Refs #1331

Scope: `reports/doc-gap-report-20260508.md` only.

Confirms 52/52 crates have module-level `//!` docs and `cargo doc --no-deps --workspace` produces zero content warnings. Declares #1331 ready to close. **No spec requirement is affected.**

### Persistent Gap Status

All four gaps confirmed by direct verification at 09:35 CEST:

| Gap | Verification | Status |
|-----|-------------|--------|
| META-001: `meta_coordinator` absent from `lib.rs` | `grep meta_coordinator crates/terraphim_orchestrator/src/lib.rs` → no output | OPEN |
| PH-H-001/002: `guard.rs` absent from `learnings/` | `ls crates/terraphim_agent/src/learnings/` → no `guard.rs` | OPEN |
| REQ-84-004: `GraphSub` absent from `crates/terraphim_agent/src/main.rs` | grep GraphSub across all crates → no results | OPEN |
| REQ-1266-001: `NormalizedTerm` struct literals on main | 26 occurrences across 14 files in compiled Rust; PR #1343 open, mergeable unknown | REGRESSION |

`NormalizedTerm \{` count: 26 total occurrences across 14 files — unchanged from v7.

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
| REQ-84-004 | `Graph list --pinned` CLI command | `§7` | ABSENT | ABSENT | `GraphSub` not found in any crate | ⚠️ |
| REQ-1266-001 | `NormalizedTerm` builder at all init sites | Gitea #1266 | 26 struct literal sites in compiled Rust | `terraphim_integration_tests.rs` | PR #1343 open, mergeable unknown (API down) | ❌ |
| REQ-D3-001 | `learn procedure from-session <id>` CLI | `d3-session §design` | `main.rs:3413` | `test_from_session_commands_basic` | `main.rs:3413` | ✅ |
| REQ-D3-002 | Trivial command filter | `d3-session §design` | `procedure.rs:412` | `test_from_session_commands_filters_trivial` | `procedure.rs:868` | ✅ |
| REQ-D3-003 | Feature-gated `repl-sessions` | `d3-session §dependencies` | `main.rs:3413` | N/A | `#[cfg(feature = "repl-sessions")]` | ✅ |
| REQ-1340-001 | Unique temp path in `test_tool_index_save_and_load` | Gitea #1340 | `mcp_tool_index.rs` (test fn) | `test_tool_index_save_and_load` | commit `5ac822e77` | ✅ |
| REQ-1331-001 | Module-level `//!` docs on all 52 crates | Gitea #1331 | 52 crates across workspace | N/A (doc artefact) | commits `c2c7151db`–`0c4948474`; `cargo doc` zero warnings | ✅ |
| PH-H-001 | `ExecutionTier` enum | `learning-correction-plan §Phase H` | `guard.rs` — ABSENT | ABSENT | File does not exist in `learnings/` | ❌ |
| PH-H-002 | `evaluate_command()` | Same | Same | ABSENT | Same | ❌ |
| META-001 | `meta_coordinator` in public API | Gitea #1275 | `lib.rs` — ABSENT | 5 tests unreachable | `grep meta_coordinator lib.rs` → no output | ❌ |
| SEC-P1-1 | SHA-256 verify before `deserialize_unchecked` | Issue #1313 P1-1 | `medical_artifact.rs:158-164` | `test_artifact_checksum_mismatch_rejected` | commit `88d2bc6d0` | ✅ |
| SEC-ADR-001 | Ollama trust boundary ADR | ADR-0001 | `docs/src/adr/0001-ollama-trust-boundary.md` | N/A (doc artefact) | commit `6e175eb32`; status=Accepted | ✅ |

---

## Gaps

| Gap ID | Description | Severity | Issue | Open PR | Status |
|--------|-------------|----------|-------|---------|--------|
| G-META-001 | `meta_coordinator` not declared in `lib.rs` — dead code; 5 unreachable tests | ❌ Blocker | #1275 (closed), #1301 | PR #1291 (open, mergeable unknown — API down; was False in v7) | OPEN |
| G-PH-H-001 | `guard.rs` absent — `ExecutionTier`, `evaluate_command()` per Phase H spec | ❌ Blocker | #1274 (open) | None | OPEN |
| G-REQ-1266 | 26 `NormalizedTerm` struct-literal sites on main → `E0063` under `--all-features` | ❌ Medium | #1266 (open) | PR #1343 (open, mergeable unknown — API down; was False in v7) | REGRESSION |
| G-REQ-84-004 | `Graph list --pinned` CLI sub-command absent from `GraphSub` enum | ⚠️ Minor | (none) | None | FOLLOW-UP |

---

## PR Status (Confirmed 09:43 CEST)

| PR | Title | Addresses | Mergeable | Status |
|----|-------|-----------|-----------|--------|
| #1291 | Fix #1275: wire meta_coordinator into lib.rs | META-001 | **False** | open, needs rebase |
| #1308 | Fix #1297: close persistent spec gaps | META-001 + others | **False** | open, needs rebase |
| #1343 | Fix #1266: update NormalizedTerm initializers to builder | REQ-1266-001 | **False** | open, needs rebase |
| #1283 | Fix #1266: NormalizedTerm missing fields (add defaults) | REQ-1266-001 | **True** | open, **mergeable — action candidate** |

**PR #1283** is the highest-leverage immediate action: it is mergeable today and restores
`--all-features` compilation. It adds `action: None`, `priority: None`, `trigger: None`,
`pinned: false` defaults at all struct-literal sites. It is a simpler fix than PR #1343
(builder migration) but achieves the same CI outcome. Neither is blocked by the other.

## Merge Conflict Alert

PR #1291, #1308, and #1343 are all unmergeable. The conflict is accumulation-only (main
has advanced through 12+ doc commits since the PRs were opened). Implementation on all
branches is sound.

Recommended immediate actions:

1. **Merge PR #1283** — mergeable today; restores `--all-features` CI green. Refs #1266.
2. **Rebase PR #1291** onto current `main` — adds one line `pub mod meta_coordinator;` in `lib.rs`. Conflict-free in substance.
3. **Rebase PR #1343** onto current `main` — migrates struct-literal initialisations to builder pattern. No expected semantic conflicts (lower priority given PR #1283 available).

---

## Recommendations (smallest first)

1. **Merge PR #1283** — mergeable today; adds missing fields with defaults to 26 struct-literal sites. Restores `--all-features` CI green. Refs #1266.
2. **Rebase and merge PR #1291** — one line `pub mod meta_coordinator;` in `lib.rs`. Unblocks 5 tests. Refs #1301.
3. **Implement `guard.rs` (Phase H)** — `ExecutionTier` enum, `GuardDecision`, `evaluate_command()`. Self-contained; no other gaps block it. Refs #1274.
4. **Add `GraphSub::List { pinned: bool }`** — trivial extension; one match arm per spec §7 AC7.
5. **Close #1331** — v8 confirms 52/52 crates have `//!` docs and zero `cargo doc` warnings. All evidence in commits `c2c7151db`–`0c4948474`.

---

## Conclusion

No change in gap count or severity since v7. The single new commit (`0c4948474`) closes the documentation completeness audit for #1331 — all 52 crates verified. It touches no spec-relevant paths. PR #1291, #1308, and #1343 remain unmergeable (confirmed). PR #1283 is **mergeable today** and is the highest-leverage immediate action.

**Verdict: FAIL**

---

<sub>Last spec-validated commit: 0c4948474
Plans directory: 6 specs, unchanged since 2026-05-04.
Open PRs addressing gaps: PR #1291 (META-001, unmergeable), PR #1308 (META-001, unmergeable), PR #1343 (REQ-1266, unmergeable), PR #1283 (REQ-1266, **mergeable**).
Gaps with no open PR: G-PH-H-001, G-REQ-84-004.
NormalizedTerm struct literal lines: 26 in 14 files (confirmed via Grep tool with escaped brace).
REQ-1331-001 (rustdoc completeness): RESOLVED — ready to close #1331.</sub>
