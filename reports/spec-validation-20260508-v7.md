# Spec Validation Report: 2026-05-08 (v7)

**Validator:** Carthos (Domain Architect)
**Date:** 2026-05-08 08:33 CEST
**Prior run:** 2026-05-08 07:33 CEST (v6, HEAD at `81e155a56`)
**HEAD commit:** `53408a7be` (main)
**Verdict:** FAIL — 4 persistent spec gaps (unchanged from v6)

---

## Executive Summary

Two commits landed on `main` since v6:

| Commit | Summary |
|--------|---------|
| `5ac822e77` | fix(test): use unique temp path in test_tool_index_save_and_load Refs #1340 |
| `53408a7be` | docs: fix rustdoc warnings in terraphim_persistence and terraphim_rolegraph |

`5ac822e77` replaces a static `/tmp/test-mcp-index.json` path with a per-run unique filename (derived from `subsec_nanos`) in `crates/terraphim_agent/src/mcp_tool_index.rs`. This fixes a parallel test interference issue tracked in #1340. It touches no struct fields, trait implementations, or module declarations relevant to any tracked spec requirement.

`53408a7be` corrects rustdoc intra-doc link warnings in `terraphim_persistence` and `terraphim_rolegraph`. Documentation-only.

**Neither commit addresses any of the four tracked spec gaps.**

PR status:
- **PR #1291** (META-001 fix): state=open, merged=False, **mergeable=False** (conflict)
- **PR #1343** (REQ-1266 fix): state=open, merged=False, **mergeable=False** (conflict)

Both remediation PRs remain blocked by merge conflicts accumulated over the succession of doc commits since v6.

---

## What Changed Since v6

### Commit `5ac822e77` — fix(test): Refs #1340

Scope: `crates/terraphim_agent/src/mcp_tool_index.rs`, test function only.

```diff
-        let index_path = temp_dir.join("test-mcp-index.json");
+        let unique = std::time::SystemTime::now()
+            .duration_since(std::time::UNIX_EPOCH)
+            .unwrap()
+            .subsec_nanos();
+        let index_path = temp_dir.join(format!("test-mcp-index-{unique}.json"));
```

This resolves a path collision when `cargo test --workspace` runs multiple test binaries concurrently. **No spec requirement is affected.**

### Commit `53408a7be` — docs: Refs #1331

Scope: `CHANGELOG.md`, `crates/terraphim_persistence/src/lib.rs`, `crates/terraphim_rolegraph/src/lib.rs`.

Doc comments only — escaped `Arc<DeviceStorage>` angle brackets, qualified intra-doc links. Workspace `cargo doc` now produces zero rustdoc warnings. **No spec requirement is affected.**

### Persistent Gap Status

All four gaps confirmed by direct verification at 08:33 CEST:

| Gap | Verification | Status |
|-----|-------------|--------|
| META-001: `meta_coordinator` absent from `lib.rs` | `grep "meta_coordinator" crates/terraphim_orchestrator/src/lib.rs` → no output | OPEN |
| PH-H-001/002: `guard.rs` absent from `learnings/` | `ls crates/terraphim_agent/src/learnings/` → no `guard.rs` | OPEN |
| REQ-84-004: `Graph list --pinned` absent | `grep "GraphSub" crates/terraphim_agent/src/main.rs` → no output | OPEN |
| REQ-1266-001: `NormalizedTerm` struct literals on main | 26 sites in compiled Rust; PR #1343 open, unmergeable | REGRESSION |

Note: `NormalizedTerm \{` count is 26 (up from 18 cited in v6 — the v6 count listed unique files; this count reflects individual struct-literal lines across all `.rs` files). The set of affected files is unchanged; no new sites were introduced by the two v7 commits.

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
| REQ-1266-001 | `NormalizedTerm` builder at all init sites | Gitea #1266 | 26 struct literal sites in compiled Rust | `terraphim_integration_tests.rs` | PR #1343 open, unmergeable (conflict) | ❌ |
| REQ-D3-001 | `learn procedure from-session <id>` CLI | `d3-session §design` | `main.rs:3413` | `test_from_session_commands_basic` | `main.rs:3413` | ✅ |
| REQ-D3-002 | Trivial command filter | `d3-session §design` | `procedure.rs:412` | `test_from_session_commands_filters_trivial` | `procedure.rs:868` | ✅ |
| REQ-D3-003 | Feature-gated `repl-sessions` | `d3-session §dependencies` | `main.rs:3413` | N/A | `#[cfg(feature = "repl-sessions")]` | ✅ |
| REQ-1340-001 | Unique temp path in `test_tool_index_save_and_load` | Gitea #1340 | `mcp_tool_index.rs` (test fn) | `test_tool_index_save_and_load` | commit `5ac822e77` | ✅ |
| PH-H-001 | `ExecutionTier` enum | `learning-correction-plan §Phase H` | `guard.rs` — ABSENT | ABSENT | File does not exist in `learnings/` | ❌ |
| PH-H-002 | `evaluate_command()` | Same | Same | ABSENT | Same | ❌ |
| META-001 | `meta_coordinator` in public API | Gitea #1275 | `lib.rs` — ABSENT | 5 tests unreachable | `grep meta_coordinator lib.rs` → no output | ❌ |
| SEC-P1-1 | SHA-256 verify before `deserialize_unchecked` | Issue #1313 P1-1 | `medical_artifact.rs:158-164` | `test_artifact_checksum_mismatch_rejected` | commit `88d2bc6d0` | ✅ |
| SEC-ADR-001 | Ollama trust boundary ADR | ADR-0001 | `docs/src/adr/0001-ollama-trust-boundary.md` | N/A (doc artefact) | commit `6e175eb32`; status=Accepted | ✅ |
| DOCS-1331-001 | Module-level rustdoc on all crates | Gitea #1331 | 24+ crates | N/A (doc artefact) | commits `c2c7151db`–`53408a7be` — zero warnings | ✅ |

---

## Gaps

| Gap ID | Description | Severity | Issue | Open PR | Status |
|--------|-------------|----------|-------|---------|--------|
| G-META-001 | `meta_coordinator` not declared in `lib.rs` — dead code; 5 unreachable tests | ❌ Blocker | #1275 (closed), #1301 | PR #1291 (open, **unmergeable — needs rebase**) | OPEN |
| G-PH-H-001 | `guard.rs` absent — `ExecutionTier`, `evaluate_command()` per Phase H spec | ❌ Blocker | #1274 (open) | None | OPEN |
| G-REQ-1266 | 26 `NormalizedTerm` struct-literal sites on main → `E0063` under `--all-features` | ❌ Medium | #1266 (open) | PR #1343 (open, **unmergeable — needs rebase**) | REGRESSION |
| G-REQ-84-004 | `Graph list --pinned` CLI sub-command absent from `GraphSub` enum | ⚠️ Minor | (none) | None | FOLLOW-UP |

---

## Merge Conflict Alert

PR #1291 and PR #1343 are both unmergeable. The conflict is mechanical (main has advanced through 10+ doc commits since the PRs were opened) — the implementation on both branches is sound.

Recommended immediate actions:

1. **Rebase PR #1291** onto current `main` — adds one line `pub mod meta_coordinator;` in `lib.rs`. Conflict-free in substance.
2. **Rebase PR #1343** onto current `main` — migrates struct-literal initialisations to builder pattern. No expected semantic conflicts.

---

## Recommendations (smallest first)

1. **Rebase and merge PR #1291** — one line `pub mod meta_coordinator;` in `lib.rs`. Unblocks 5 tests. Refs #1301.
2. **Rebase and merge PR #1343** — convert 26 struct-literal sites to builder pattern. Restores `--all-features` CI green. Refs #1266.
3. **Implement `guard.rs` (Phase H)** — `ExecutionTier` enum, `GuardDecision`, `evaluate_command()`. Self-contained; no other gaps block it. Refs #1274.
4. **Add `GraphSub::List { pinned: bool }`** — trivial extension; one match arm per spec §7 AC7.

---

## Conclusion

No change in gap count or severity since v6. The two new commits close #1340 (test isolation) and finalise #1331 (rustdoc zero-warnings). Neither touches any tracked spec requirement. Both open remediation PRs (#1291, #1343) remain unmergeable due to accumulated rebase debt.

**Verdict: FAIL**

---

<sub>Last spec-validated commit: 53408a7be
Plans directory: 6 specs, unchanged since 2026-05-04.
Open PRs addressing gaps: PR #1291 (META-001, unmergeable), PR #1343 (REQ-1266, unmergeable).
Gaps with no open PR: G-PH-H-001, G-REQ-84-004.
NormalizedTerm struct literal lines: 26 in compiled Rust.</sub>
