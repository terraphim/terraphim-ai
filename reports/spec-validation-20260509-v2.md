# Spec Validation Report -- 2026-05-09 (v2)

**Agent**: Carthos (Domain Architect, spec-validator)
**Run**: 2026-05-09 06:33 CEST -- HEAD `fc707c31c`
**Previous run**: 2026-05-09 04:51 CEST (HEAD `64c4c9aa0`) -- four gaps open

---

## Executive Summary

**Overall verdict: FAIL** -- 3 persistent gaps remain. G-PR-1349-001 now **CLOSED** (RetryBound fix merged at `3128b6653`).

| Gap | Previous Status | Current Status |
|-----|-----------------|----------------|
| G-META-001: `meta_coordinator` not wired in orchestrator `lib.rs` | PARTIAL | **PARTIAL** -- unchanged; file exists, module not declared |
| G-PH-H-001/002: `guard.rs` absent from learnings | OPEN | **OPEN** -- unchanged |
| G-REQ-84-004: `kg list --pinned` CLI subcommand | OPEN | **OPEN** -- unchanged |
| G-PR-1349-001: RetryBound invariant not enforced | OPEN | **CLOSED** -- merged at `3128b6653` |

**Progress this cycle**: 1 gap closed (G-PR-1349-001). 3 remain.

---

## Plans/ Directory Assessment

### 1. `design-gitea82-correction-event.md` -- PASS

**Spec**: Add `CorrectionType` enum and `CorrectionEvent` struct for typed learning capture (Gitea #82).

| Requirement | Location | Status |
|-------------|----------|--------|
| `CorrectionType` enum with 7 variants | `crates/terraphim_agent/src/learnings/capture.rs:44-59` | PASS |
| `CorrectionEvent` struct (9 fields) | `capture.rs:501-521` | PASS |
| `Display` + `FromStr` for `CorrectionType` | `capture.rs:61-93` | PASS |
| `capture_correction()` function | `capture.rs:1022-1066` | PASS |
| `list_all_entries()` | `capture.rs:1298-1369` | PASS |
| `query_all_entries()` | `capture.rs:1372-1404` | PASS |
| `LearnSub::Correction` CLI variant | `main.rs:~2000` | PASS |
| Handler for `learn correction` subcommand | `main.rs:3141-3167` | PASS |

All 8 spec requirements for Gitea #82 (Phase 1.1 and 1.2) implemented. Hooks phases (1.3–1.4) are out of spec scope.

---

### 2. `design-gitea84-trigger-based-retrieval.md` -- PARTIAL

**Spec**: Add `trigger::` and `pinned::` directives; build TF-IDF index; two-pass search; `kg list --pinned` CLI.

| Requirement | Location | Status |
|-------------|----------|--------|
| `trigger` and `pinned` fields in `MarkdownDirectives` | `crates/terraphim_types/src/lib.rs:517,519` | PASS |
| Parse `trigger::` and `pinned::` directives | `crates/terraphim_automata/src/markdown_directives.rs:235-250` | PASS |
| `TriggerIndex` struct + `new` / `build` / `query` methods | `crates/terraphim_rolegraph/src/lib.rs:77-210` | PASS |
| `find_matching_node_ids_with_fallback()` | `rolegraph/src/lib.rs:477-500` | PASS |
| `load_trigger_index()` | `rolegraph/src/lib.rs:504-514` | PASS |
| `query_graph_with_trigger_fallback()` | `rolegraph/src/lib.rs:744-842` | PASS |
| `--include-pinned` flag in Search subcommand | `crates/terraphim_agent/src/main.rs` (search) | PASS |
| **`KgSub` enum + `kg list --pinned` CLI** | `main.rs` | **ABSENT** |

**Gap G-REQ-84-004** (persistent): The spec §7 requires a `kg list --pinned` dedicated subcommand under a `KgSub` enum. Only the `--include-pinned` flag in the Search path is implemented. `KgSub` does not exist; the `kg list` command path does not exist.

---

### 3. `d3-session-auto-capture-plan.md` -- PASS

**Spec**: `learn procedure from-session <session-id>` subcommand; feature-gated `repl-sessions`.

| Requirement | Location | Status |
|-------------|----------|--------|
| `FromSession` variant in `ProcedureSub` | `main.rs:1179-1186` | PASS |
| `#[cfg(feature = "repl-sessions")]` gate | `main.rs:1179` | PASS (by design) |
| CLI dispatch handler for `from-session` | `main.rs:3417,3438,3445` | PASS |

The function signature divergence (two-step architecture vs. spec's monolithic `from_session()`) noted in the earlier plans report is a documentation gap only — end-to-end behaviour is correct.

---

### 4. `design-single-agent-listener.md` -- PASS

**Spec**: Deploy listener agent in tmux with config and launch script.

| Requirement | Location | Status |
|-------------|----------|--------|
| `~/.config/terraphim/listener-worker.json` | `~/.config/terraphim/` (741 B, created 2026-04-16) | PASS |
| `~/.config/terraphim/scripts/start-listener.sh` | `~/.config/terraphim/scripts/` (772 B, created 2026-04-16) | PASS |
| `listener.rs` Rust code | `crates/terraphim_agent/src/listener.rs` | PASS |
| `listen` subcommand in CLI | `crates/terraphim_agent/src/main.rs` | PASS |

---

### 5. `learning-correction-system-plan.md` -- MOSTLY IMPLEMENTED

| Requirement | Status |
|-------------|--------|
| `CorrectionEvent` and `CorrectionType` | PASS |
| `procedure.rs` compiled unconditionally | PASS |
| `SharedLearningStore` CLI (feature-gated) | PASS |
| `learn shared list/promote/import/stats` | PASS |
| Auto-suggest from KG (TODO at `capture.rs:609`) | NOT IMPLEMENTED (not a hard spec requirement) |

---

### 6. `research-single-agent-listener.md` -- N/A

Research document. No code changes specified.

---

## Persistent Gap Detail

### G-META-001: `meta_coordinator` not declared in orchestrator `lib.rs`

- **File status**: `crates/terraphim_orchestrator/src/meta_coordinator.rs` EXISTS (25 KB, updated 2026-05-06)
- **Wiring**: NOT declared in `crates/terraphim_orchestrator/src/lib.rs`
- **Effect**: Module is dead code — compiles in isolation but cannot be used by other crates
- **Remediation**: Add `pub mod meta_coordinator;` to orchestrator `lib.rs` (one-line fix)
- **Effort**: Trivial — no PR required

### G-PH-H-001/002: `guard.rs` absent from learnings

- `crates/terraphim_agent/src/learnings/guard.rs`: ABSENT
- No `pub mod guard;` in learnings `mod.rs`
- PR #1308 (branch `pr-1308`, head `112bc99`) adds this file but has not been merged
- **Remediation**: Merge PR #1308

### G-REQ-84-004: `kg list --pinned` CLI subcommand

- `--include-pinned` exists in Search subcommand (partial implementation)
- `KgSub` enum does not exist; `kg list --pinned` command path does not exist
- Service call target (`get_role_graph_pinned`) exists at `main.rs:2258`
- **Remediation**: Add `KgSub` enum and `Kg` subcommand to CLI dispatch per spec §7; wire to existing `get_role_graph_pinned`
- **Effort**: Medium — ~1 hour of CLI scaffolding

---

## Newly Closed Gap

### G-PR-1349-001: RetryBound invariant CLOSED

**Commit**: `3128b6653 fix(symphony): enforce RetryBound invariant on slot exhaustion and poll failure`

**Verification**:
- `ServiceConfig::max_retry_attempts()` at `crates/terraphim_symphony/src/config/mod.rs:189` -- PRESENT
- Slot-exhaustion guard at `orchestrator/mod.rs:622-630` -- PRESENT
- Poll-failure guard at `orchestrator/mod.rs:590-608` -- PRESENT

**Residual concern** (follow-up, not blocker): The cast `.unwrap_or(10) as u32` at `config/mod.rs:191` truncates silently for values > `u32::MAX`. Security audit `reports/audit-pr1349.md` recommended `u32::try_from(v).ok().unwrap_or(10).max(1)`. The truncation is unreachable in practice (no realistic config sets 4.2B retries) but the safe cast would close the audit finding cleanly.

---

## Traceability Matrix

| Req ID | Requirement | Design Ref | Impl Ref | Tests | Status |
|-------:|-------------|------------|----------|-------|--------|
| REQ-82 | CorrectionType + CorrectionEvent | `plans/design-gitea82-correction-event.md` | `capture.rs:44-93` | inline unit tests | PASS |
| REQ-84-001 | Parse `trigger::` and `pinned::` directives | `plans/design-gitea84-trigger-based-retrieval.md §2` | `markdown_directives.rs:235-250` | inline tests | PASS |
| REQ-84-002 | TriggerIndex TF-IDF | `plans/design-gitea84-trigger-based-retrieval.md §3` | `rolegraph/lib.rs:77-210` | inline | PASS |
| REQ-84-003 | Two-pass search with fallback | `plans/design-gitea84-trigger-based-retrieval.md §4-6` | `rolegraph/lib.rs:477-842` | inline | PASS |
| REQ-84-004 | `KgSub` enum + `kg list --pinned` CLI | `plans/design-gitea84-trigger-based-retrieval.md §7` | ABSENT | none | FAIL |
| D3-001 | `learn procedure from-session` subcommand | `plans/d3-session-auto-capture-plan.md` | `main.rs:1179,3417-3445` | gated `repl-sessions` | PASS |
| SAL-001 | Listener worker config + launch script | `plans/design-single-agent-listener.md §4` | `~/.config/terraphim/` | manual confirm | PASS |
| LCS-001 | SharedLearningStore CLI | `plans/learning-correction-system-plan.md` | `main.rs:3767` | gated `shared-learning` | PASS |
| PH-H-001 | `guard.rs` in learnings module | phase-H plan | `learnings/guard.rs` | ABSENT | FAIL |
| META-001 | `pub mod meta_coordinator` in orchestrator `lib.rs` | orchestrator spec | `lib.rs` (absent) | dead code | FAIL |
| PR-1349 | RetryBound invariant in symphony | `reports/audit-pr1349.md`, Gitea #251 | `config/mod.rs:189`, `orchestrator/mod.rs:590-630` | unit tests in commit | **PASS** |

---

## Unrelated Unstaged Change

`crates/terraphim_symphony/src/tracker/gitea.rs` has one unstaged edit: rustdoc link fix (`[Issue](super::Issue)` → `[Issue]`). Not related to any spec. Should be committed or stashed.

---

## Priority Recommendations

1. **Merge PR #1308** — closes G-PH-H-001/002 (guard.rs missing)
2. **One-line fix**: add `pub mod meta_coordinator;` to `crates/terraphim_orchestrator/src/lib.rs` — closes G-META-001 (no PR needed)
3. **CLI scaffolding**: implement `KgSub` enum in `crates/terraphim_agent/src/main.rs` per `plans/design-gitea84-trigger-based-retrieval.md §7` — closes G-REQ-84-004
4. **Follow-up**: apply `u32::try_from` cast in `config/mod.rs:191` to satisfy `audit-pr1349.md` integer-safety recommendation
5. **Commit unstaged rustdoc change** in `tracker/gitea.rs`

Items 2, 4, and 5 are self-contained and do not require branch management.

---

## Verdict

**FAIL** — 3 spec gaps remain open (G-META-001, G-PH-H-001/002, G-REQ-84-004). Architectural intent is implemented correctly in all active specs except the `kg list --pinned` CLI surface. The RetryBound gap (G-PR-1349-001) is now closed.
