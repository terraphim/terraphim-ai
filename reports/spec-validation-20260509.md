# Spec Validation Report -- 2026-05-09

**Agent**: Carthos (Domain Architect, spec-validator)
**Run**: 2026-05-09 04:51 CEST -- PR review: branch `task/docs-generation-20260509` HEAD `64c4c9aa0`
**Previous run**: 2026-05-09 04:33 CEST (cron, same HEAD) -- PR review appended below

---

## Executive Summary

**Overall verdict: FAIL** -- 4 persistent gaps remain; G-SAL-001 now CLOSED (artefacts confirmed present on host).

| Gap | Previous Status | Current Status |
|-----|-----------------|---------------|
| G-META-001: `meta_coordinator` absent from orchestrator `lib.rs` | PARTIAL | PARTIAL -- file exists, not wired (confirmed at `64c4c9aa0`) |
| G-PH-H-001/002: `guard.rs` absent from learnings | OPEN | OPEN -- no change |
| G-REQ-84-004: `kg list --pinned` CLI subcommand | OPEN | OPEN -- `KgSub` enum and `--include-pinned` flag both absent (confirmed) |
| G-D3-001: `from-session` subcommand | CLOSED | CLOSED (feature-gated, by design) |
| G-SAL-001: listener deployment artefacts | OPEN | **CLOSED** -- `~/.config/terraphim/listener-worker.json` and `start-listener.sh` confirmed present (created 2026-04-16) |
| G-LCS-001: `SharedLearningStore` CLI | CLOSED | CLOSED (feature-gated, by design) |
| G-PR-1349-001: RetryBound code not merged | OPEN | OPEN -- no RetryBound commits visible in `git log --all`; fix remains on PR branch |

**Newly confirmed CLOSED**: G-SAL-001 (artefacts existed since 2026-04-16; previous report was stale)
**Remaining open**: G-META-001 (partial), G-PH-H-001/002, G-REQ-84-004, G-PR-1349-001

---

## Plans/ Directory Assessment

### 1. `design-gitea82-correction-event.md` -- PASS

**Spec**: Add `CorrectionType` enum and `CorrectionEvent` struct for typed learning capture.

| Requirement | Location | Status |
|-------------|----------|--------|
| `CorrectionType` enum with 7 variants | `crates/terraphim_agent/src/learnings/capture.rs:44-59` | PASS |
| `CorrectionEvent` struct | `crates/terraphim_agent/src/learnings/capture.rs` | PASS |
| `Display` impl for `CorrectionType` | `capture.rs:61-73` | PASS |
| `FromStr` impl for `CorrectionType` | `capture.rs:75-93` | PASS |

All spec requirements for Gitea #82 (Phase 1.1 and 1.2) are implemented. Hooks phases (1.3-1.4) were out of spec scope.

---

### 2. `design-gitea84-trigger-based-retrieval.md` -- PARTIAL

**Spec**: Add `trigger::` and `pinned::` directives to KG entries; build TF-IDF index; two-pass search; `kg list --pinned` CLI.

| Requirement | Location | Status |
|-------------|----------|--------|
| `trigger` and `pinned` fields in `MarkdownDirectives` | `crates/terraphim_types/src/lib.rs:323-326` | PASS |
| Parse `trigger::` and `pinned::` in markdown parser | `crates/terraphim_automata/src/markdown_directives.rs:106-107,215-230` | PASS |
| `TriggerIndex` struct with TF-IDF in rolegraph | `crates/terraphim_rolegraph/src/lib.rs:74-91` | PASS |
| Two-pass search: Aho-Corasick first, TF-IDF fallback | `crates/terraphim_rolegraph/src/lib.rs` | PASS |
| `--include-pinned` flag in search | `crates/terraphim_agent/src/main.rs` (search subcommand) | PASS |
| **`kg list --pinned` dedicated subcommand** (`KgSub` enum) | main.rs | **ABSENT** |

**Gap G-REQ-84-004** (persistent): The spec requires a `kg list --pinned` command under a `KgSub` enum. The `--include-pinned` flag exists in the Search subcommand, but the dedicated `kg list` with `--pinned` filter is not implemented. The `KgSub` enum does not exist in main.rs.

---

### 3. `d3-session-auto-capture-plan.md` -- PASS

**Spec**: `learn procedure from-session <session-id>` subcommand; requires `repl-sessions` feature.

| Requirement | Location | Status |
|-------------|----------|--------|
| `procedure.rs` compiled unconditionally | `crates/terraphim_agent/src/learnings/mod.rs:31` (`pub(crate) mod procedure`) | PASS |
| `FromSession` variant in `ProcedureSub` | `crates/terraphim_agent/src/main.rs:1179-1186` | PASS |
| Feature-gated `#[cfg(feature = "repl-sessions")]` | `main.rs:1179` | PASS (by design) |
| CLI dispatch handler | `main.rs:3417,3438,3445` | PASS |

Previous gap G-D3-001 is **CLOSED**. The spec explicitly named `repl-sessions` as a prerequisite; the feature gate is intentional.

---

### 4. `design-single-agent-listener.md` -- PASS

**Spec**: Deploy listener agent in tmux with `~/.config/terraphim/listener-worker.json` and `start-listener.sh`.

| Requirement | Status |
|-------------|--------|
| `~/.config/terraphim/listener-worker.json` | PRESENT -- confirmed (741 B, created 2026-04-16) |
| `~/.config/terraphim/scripts/start-listener.sh` | PRESENT -- confirmed (772 B, created 2026-04-16) |
| Rust listener code (`listener.rs`) | PRESENT -- `crates/terraphim_agent/src/listener.rs` |
| `listen` subcommand in CLI | PRESENT |

**Gap G-SAL-001 CLOSED**: Previous report incorrectly listed artefacts as absent. Filesystem check at HEAD `64c4c9aa0` confirms both deployment artefacts exist at `~/.config/terraphim/`. The gap was a stale observation, not a real gap.

---

### 5. `learning-correction-system-plan.md` -- MOSTLY IMPLEMENTED

| Requirement | Status |
|-------------|--------|
| `CorrectionEvent` and `CorrectionType` | PASS |
| `procedure.rs` not gated by `#[cfg(test)]` | PASS -- now `pub(crate)` |
| `SharedLearningStore` CLI subcommands | PASS -- feature-gated `--features shared-learning` |
| `learn shared list/promote/import/stats` | PASS -- wired at `main.rs:3767` |
| Auto-suggest from KG (TODO comment at line 609 of capture.rs) | OPEN -- not implemented |

Previous gap G-LCS-001 is **CLOSED** (the CLI exists, feature-gated by design). The auto-suggest from KG is a residual TODO but is not a spec requirement from this plan document.

---

## Persistent Gap Detail

### G-META-001: `meta_coordinator` partially resolved

- **File status**: `crates/terraphim_orchestrator/src/meta_coordinator.rs` EXISTS (25 KB, updated 2026-05-06)
- **Wiring status**: NOT declared in `crates/terraphim_orchestrator/src/lib.rs` module list
- **Effect**: `meta_coordinator.rs` is dead code -- it compiles independently but cannot be used by other crates
- **Remediation**: Add `pub mod meta_coordinator;` to orchestrator `lib.rs`

### G-PH-H-001/002: `guard.rs` absent

- `crates/terraphim_agent/src/learnings/guard.rs`: ABSENT
- No `pub mod guard;` in learnings `mod.rs`
- PR #1308 (branch `pr-1308` / head SHA `112bc99`) adds this file but has not been merged to the current working branch
- **Remediation**: Merge PR #1308

### G-REQ-84-004: `kg list --pinned` CLI subcommand

- `--include-pinned` is present in the Search subcommand (partial implementation)
- `KgSub` enum does not exist; `kg list --pinned` command path does not exist
- **Remediation**: Add `KgSub` enum and `Kg` subcommand to CLI dispatch; wire to `get_role_graph_pinned` service call (which exists at `main.rs:2258`)

### G-SAL-001: Listener deployment artefacts

- Rust code: present
- Config and launch script: absent from `~/.config/terraphim/`
- **Remediation**: Create `listener-worker.json` and `start-listener.sh` per spec Section 4

---

## New Gap

### G-PR-1349-001: RetryBound fix not merged to working branch

**Severity**: Medium

- Commit `0adaa209c` (`fix(symphony): enforce RetryBound invariant in on_retry_timer Refs #251`) is on branch `task/251-symphony-retry-bound` / `pr-1349`
- The security checklist for this PR was added to the main branch (`fd484da22`) but the CODE FIX was not merged
- Current `crates/terraphim_symphony/src/config/mod.rs` does NOT contain `max_retry_attempts()` accessor
- Current `crates/terraphim_symphony/src/orchestrator/mod.rs` does NOT contain the `next >= max_attempts` guards
- Effect: The claimed set can grow monotonically under sustained slot pressure (TLA+ RetryBound invariant violated)
- Security audit (audit-pr1349.md) also flagged integer truncation (`as u32` cast) that needs remediation before merge
- **Remediation**: Merge PR #1349 after applying security audit recommendations (use `u32::try_from(v).ok().unwrap_or(10).max(1)` in `max_retry_attempts()` and `.saturating_add(1)` for attempt counter)

---

## Traceability Matrix

| Req ID | Requirement | Design Ref | Impl Ref | Tests | Evidence | Status |
|-------:|-------------|------------|----------|-------|----------|--------|
| REQ-82 | CorrectionType enum + CorrectionEvent struct | `plans/design-gitea82-correction-event.md` | `crates/terraphim_agent/src/learnings/capture.rs:44-93` | inline unit tests | code present at HEAD | PASS |
| REQ-84-001 | Parse `trigger::` and `pinned::` directives | `plans/design-gitea84-trigger-based-retrieval.md §2` | `crates/terraphim_automata/src/markdown_directives.rs:106-107,215-230` | inline tests | code present at HEAD | PASS |
| REQ-84-002 | TriggerIndex TF-IDF in rolegraph | `plans/design-gitea84-trigger-based-retrieval.md §3` | `crates/terraphim_rolegraph/src/lib.rs:74-91` | inline | code present at HEAD | PASS |
| REQ-84-003 | Two-pass search with fallback | `plans/design-gitea84-trigger-based-retrieval.md §4-6` | `crates/terraphim_rolegraph/src/lib.rs` | inline | code present at HEAD | PASS |
| REQ-84-004 | `KgSub` enum + `kg list --pinned` CLI | `plans/design-gitea84-trigger-based-retrieval.md §7` | `crates/terraphim_agent/src/main.rs` | ABSENT | grep confirms absent | FAIL |
| D3-001 | `learn procedure from-session` subcommand | `plans/d3-session-auto-capture-plan.md` | `main.rs:1179,3417,3438,3445` | gated `repl-sessions` | feature-gated by design | PASS |
| SAL-001 | Listener worker config + launch script | `plans/design-single-agent-listener.md §4` | `~/.config/terraphim/listener-worker.json`, `scripts/start-listener.sh` | manual | artefacts confirmed present 2026-04-16 | PASS |
| LCS-001 | `SharedLearningStore` CLI | `plans/learning-correction-system-plan.md` | `main.rs:3767` | gated `shared-learning` | feature-gated by design | PASS |
| PH-H-001 | `guard.rs` in learnings module | phase-H plan (referenced via G-PH-H) | `crates/terraphim_agent/src/learnings/guard.rs` | ABSENT | filesystem check confirms absent | FAIL |
| META-001 | `pub mod meta_coordinator` in orchestrator `lib.rs` | orchestrator spec | `crates/terraphim_orchestrator/src/lib.rs` | none (dead code) | grep confirms absent from lib.rs | FAIL |
| PR-1349 | RetryBound invariant enforcement in symphony | `reports/audit-pr1349.md`, Gitea #251 | `crates/terraphim_symphony/src/config/mod.rs`, `orchestrator/mod.rs` | in PR branch only | no merge commit in `git log --all` | FAIL |

---

## Structural Observation

The two-cycle pattern continues: code fixes land on PR branches, documentation (security checklists, traceability matrices) is committed to the working branch, but PR merges are not happening. This run confirmed that pattern persists at `64c4c9aa0`:
- G-PH-H-001/002: PR #1308 adds `guard.rs` -- not merged
- G-PR-1349-001: PR #1349 adds RetryBound fix -- not merged; no merge commit in `git log --all`

This run also corrected a stale false-negative: G-SAL-001 was reported OPEN in the 03:33 run, but filesystem inspection shows artefacts created 2026-04-16. The gap was never real; the validator was not checking the filesystem.

**Priority recommendations**:
1. Merge PR #1349 (apply `u32::try_from` and `.saturating_add(1)` remediations from `reports/audit-pr1349.md` first)
2. Merge PR #1308 to close G-PH-H-001/002
3. Add `pub mod meta_coordinator;` to `crates/terraphim_orchestrator/src/lib.rs` (one-line fix, no PR required)
4. Implement `KgSub` enum in `crates/terraphim_agent/src/main.rs` per `plans/design-gitea84-trigger-based-retrieval.md §7`

Items 3 and 4 are self-contained and do not depend on merging open PRs. Items 1 and 2 unblock the traceability chain for phase-H and symphony respectively.

---

## PR Review: task/docs-generation-20260509 (04:51 CEST)

**Verdict: concerns** -- #851 fix fully traced; doc-coverage initiative lacks CI gate and design artefact.

### PR Traceability Matrix

| Req ID | Requirement | Design Ref | Impl Ref | Tests | Status |
|-------:|-------------|------------|----------|-------|--------|
| REQ-851-001 | `wildcard_fallback` in offline Search envelope | Gitea #851 | `Service.rs:351-367` | `phase1_robot_mode_tests.rs::test_wildcard_fallback_serialized_in_robot_output` | PASS |
| REQ-851-002 | `Thesaurus_matched` from Thesaurus in offline Search | Gitea #851 | `Service.rs:369-381`, `schema.rs:391` | `phase1_robot_mode_tests.rs::test_Thesaurus_matched_populated_from_Thesaurus` | PASS |
| REQ-851-003 | wildcard_fallback + Thesaurus_matched in server-proxy mode | Gitea #851 | `main.rs:4026-4142` | none (no offline integration test) | CONCERNS |
| REQ-DOCS-001 | Rustdoc on public items in terraphim_automata | none (no ADR) | `builder.rs`, `lib.rs`, `markdown_directives.rs` | no `cargo test --doc` gate | CONCERNS |
| REQ-DOCS-002 | Rustdoc on public items in terraphim_types | none (no ADR) | `terraphim_types/src/lib.rs` | no `cargo test --doc` gate | CONCERNS |

**Recommendations**:
1. Add `#![warn(missing_docs)]` or a `cargo doc --no-deps` CI step failing on warnings
2. Add `plans/doc-coverage-policy.md` naming target crates and the CI gate
3. Add `#[ignore]` integration test for server-mode wildcard/Thesaurus path
4. Commit unstaged rustdoc additions in terraphim_config, terraphim_Service, terraphim_Service, terraphim_settings

