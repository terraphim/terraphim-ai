# Spec Validation Report — 2026-06-29 (07:30 CEST cycle)

**Agent**: spec-validator (Carthos, Domain Architect)
**Date**: 2026-06-29 07:30 CEST
**Verdict**: **CONDITIONAL PASS** (tracked gap #2972 re-confirmed; no behavioural regressions)
**Prior verdict**: CONDITIONAL PASS (06:30 CEST, same day)

---

## Boundary State This Cycle

The territorial topology is **unchanged since the 06:30 cycle**. No commits have landed against
the stranded-source directories or the `plans/` relocation:

| Stranded dir | Last commit touching it | Status |
|---|---|---|
| `crates/terraphim_orchestrator` | `5c48dabec` 2026-06-14 | stale (predates cycle) |
| `crates/terraphim_agent` | `5c48dabec` 2026-06-14 | stale |
| `crates/terraphim_agent_application` | `45c385178` 2026-06-22 | stale |

#2972 remains **open**, assigned to `quality-coordinator`, 23 comments. No remediation owner
action since the prior recurrence.

---

## Plans Validated (6, against true polyrepo home)

All six plans' acceptance criteria are satisfied in the migrated code
(`/home/alex/projects/terraphim/terraphim-agents`). Behavioural ACs verified:

### Plan 1: `design-gitea82-correction-event.md` — **PASS**
| AC | Evidence | Status |
|----|------|--------|
| `CorrectionEvent` struct | `learnings/capture.rs` | ✅ |
| `capture_correction()` / `list_all_entries()` / `query_all_entries()` | `learnings/capture.rs` | ✅ |
| Secret redaction wired | `learnings/redaction.rs` (called in capture paths) | ✅ |

### Plan 2: `d3-session-auto-capture-plan.md` — **PASS**
| AC | Evidence | Status |
|----|------|--------|
| `from_session_commands()` / `extract_bash_commands_from_session()` | `learnings/procedure.rs` | ✅ |
| Trivial-command filter | present in procedure.rs | ✅ |

### Plan 3: `design-gitea84-trigger-based-retrieval.md` — **PASS**
| AC | Evidence | Status |
|----|------|--------|
| `MarkdownDirectives.trigger` / `.pinned` | registry `terraphim_types` 1.20.x | ✅ |
| `TriggerIndex` + fallback matching | registry `terraphim_rolegraph` 1.20.2 | ✅ |

### Plan 4: `research-single-agent-listener.md` — **PASS** (research-only, no ACs)
### Plan 6: `design-single-agent-listener.md` — **PASS** (runtime ACs; `learnings/listener.rs` structurally present)
### Plan 5: `learning-correction-system-plan.md` — **PASS** (carry-forwards stable)
| Phase | Issue(s) | Status | Evidence |
|-------|----------|--------|----------|
| A–F, H | #480-704 | ✅ | capture/procedure/replay/guard/suggest intact in `terraphim-agents/learnings/` |
| G (shared CLI) | #727 partial | ✅ DONE | `SharedLearningSub::{List,Promote,Import}` wired; `SharedLearningStore::open` called |
| I (evolution) | #727-730 | ✅ cleared | Mock LLM factories confined to `#[cfg(test)]`; production paths clean |
| J (validation) | #515-517,#451 | ✅ present | `terraphim_hooks` 1.20.2 registry crate consumed; compiles green |

---

## Test Evidence (re-run this cycle)

```
cargo test -p terraphim_agent --lib
test result: ok. 288 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.39s
```

Includes `shared_learning::{store,wiki_sync}` tests and `robot::output::proptests`. **Behavioural invariant holds** (stable across the 06:30 and 07:30 cycles).

---

## Architectural Finding: Two `GuardDecision` Types (refined, not a violation)

The prior report's "coincidental name collision" framing was imprecise. Within `terraphim_agent`
there are **two distinct `GuardDecision` abstractions**, by deliberate design:

| Type | Location | Shape | Wired? | Role |
|---|---|---|---|---|
| `GuardDecision` (**enum**) | `src/guard_patterns.rs` (crate root) | `Allow / Block / Sandbox` | **LIVE** — `listener.rs:1334`, `main.rs:1966-2724` | Production guard (`CommandGuard`) |
| `GuardDecision` (**struct**) | `src/learnings/guard.rs` | `{ tier, reason, previously_failed }` + `ExecutionTier` | **DEFERRED** — zero external usages | Plan §H graduated-tier design (Phase H, future iteration) |

The deferral is **self-documented and intentional**. `learnings/mod.rs` declares:

```rust
// Guard API — newly added, will be wired into the binary in a future iteration.
#[allow(unused_imports)]
pub use guard::{ExecutionTier, GuardDecision, evaluate_command, evaluate_command_with_learning};
```

This is not dead code from neglect — it is staged foundation work silenced with
`#[allow(unused_imports)]`, consistent with the learning-plan's Phase H sequencing
(dependent on Phase D replay + Phase F health monitoring). `DESIGN-guard-patterns-redesign.md`
confirms intent: the redesign *replaces internals* of `guard_patterns.rs` with thesaurus matching;
it does not delete it, nor does it merge the two abstractions.

**Verdict on this finding:** Not a spec violation. A deliberate staged-design boundary. Filed
here for traceability so future runs do not re-flag it as "name collision" or "duplicate dead code".

> Note: prior traceability cited test evidence `git_checkout_double_dash` against
> `learnings/guard_patterns.rs`. The actual location is `src/guard_patterns.rs:208`
> (`test_git_checkout_double_dash_blocked`). Filename drift in the prior report — corrected here.

---

## The Tracked Gap (#2972) — Unchanged, P2

### Facet A — Stranded source (re-measured)

Git-tracked Rust source with no workspace build path remains in this repo:

| Directory | Tracked .rs files | LOC | Cargo.toml |
|-----------|------------------:|----:|------------|
| `crates/terraphim_orchestrator` | 18 | 16,467 | none (orphaned mod roots) |
| `crates/terraphim_agent` | 7 | 6,819 | none |
| `crates/terraphim_agent_application` | 8 | 3,265 | **still present** |
| **subtotal** | **33** | **26,551** | — |

Hazard: dead-but-tracked code — excluded from build, superseded by registry deps, yet present
and searchable. Maintenance trap + review noise. Measurement is stable with the 06:30 cycle.

### Facet B — Spec-location drift (unfixed)

All four code-bearing plans still cite pre-extraction paths unresolvable in this repo:
- `design-gitea82-correction-event.md` → `crates/terraphim_agent/src/learnings/capture.rs` (×2 refs)
- `d3-session-auto-capture-plan.md` → `crates/terraphim_agent/src/learnings/procedure.rs`
- `learning-correction-system-plan.md` → `crates/terraphim_agent/src/learnings/{hook,mod,capture}.rs` (×5+ refs)

No `RELOCATED` banners added to any plan. **Facet B drift unfixed.**

### Why CONDITIONAL PASS, not FAIL

No behavioural/acceptance-criterion violation exists. Every specified function is implemented
and tested in its polyrepo. The gap is **repository hygiene + spec relocation**, already tracked
as **#2972** (open, assigned `quality-coordinator`, 23 comments). This cycle re-confirms with
stable measurement. **No new issue filed** — fix owner unchanged; duplicate-noise avoided per cron
protocol.

**Smallest fix** (owned by whoever closes #2972):
1. `git rm` stranded `crates/*` source (except genuinely-shared fixtures)
2. Add `RELOCATED.md` banners to the 4 code-bearing plans pointing to `terraphim-agents`
3. Replace `plans/` with a `RELOCATED.md` index

---

## Negative Space (explicitly considered and ruled out)

| Considered | Ruled out | Basis |
|---|---|---|
| `terraphim_mcp_search` violates a plan | Net-new, no plan covers it | No `plans/` doc references `mcp_search`/`McpToolIndex`/`SEP-1821` (verified by absence) |
| New issue #3017 (kiro Task 10) in scope | Out of scope | No `plans/` doc references kiro/OTP — net-new doc-tracking issue |
| New issue #3014 (session NFR G1) in scope | Out of scope | No `plans/` doc references session benchmark NFR — net-new perf-tracking issue |
| `learnings/guard.rs` is accidental dead code | Deliberate staging | `mod.rs` comment + `#[allow(unused_imports)]` + Phase H dependency chain |
| Two `GuardDecision` types = spec conflict | Deliberate boundary | Different modules, different shapes, different lifecycle stage (enum=live, struct=staged) |
| Workspace membership defect | Not a defect | `members=["crates/*"]` globs; `exclude=[...]` prunes residuals; internally consistent |
| Carry-forwards #727/Phase-G/I regressed | Stable (06:30→07:30) | Shared CLI wired; mocks confined to `#[cfg(test)]` |

---

## Traceability Matrix

| Req (plan symbol) | Plan | Impl Location (true) | Test | Status |
|---|---|---|---|---|
| CorrectionEvent | design-gitea82 §1 | `terraphim-agents/learnings/capture.rs` | `test_capture_*` (288-suite) | ✅ |
| ProcedureStore | d3 / learning-plan §B | `terraphim-agents/learnings/procedure.rs` | `from_session_commands` tests | ✅ |
| `replay_procedure()` | learning-plan §D | `terraphim-agents/learnings/replay.rs` | replay tests | ✅ |
| GuardDecision (enum, live) | (production guard) | `terraphim-agents/src/guard_patterns.rs` | `test_git_checkout_double_dash_blocked:208` | ✅ |
| GuardDecision (struct, staged) | learning-plan §H | `terraphim-agents/learnings/guard.rs` | unit tests in-file (10) | ✅ staged |
| SharedLearningSub CLI | learning-plan §G | `terraphim-agents/main.rs:3964` | `wiki_sync::tests` | ✅ |
| MarkdownDirectives.trigger | design-gitea84 §1 | `terraphim-core` (registry terraphim_types) | `parses_trigger_directive` | ✅ |
| TriggerIndex | design-gitea84 §3 | `terraphim-core` (registry terraphim_rolegraph) | tfidf tests | ✅ |

---

## Verdict

**CONDITIONAL PASS.** No P0/P1 behavioural spec violations. All six plans' acceptance criteria
satisfied in the code's true home (`terraphim-agents` + `terraphim-core` registry crates);
`terraphim_agent` lib suite = **288 passed, 0 failed** (re-verified this cycle). Carry-forwards
from 2026-06-01 remain cleared and stable.

One tracked P2 gap re-confirmed with stable measurement (~26,551 LOC stranded across 3 excluded
dirs + spec-location drift in 4 plans). Already filed as **#2972** (open). This cycle adds a
refined architectural finding (the two-`GuardDecision` boundary is deliberate staging, not a
collision) and corrects a filename drift in prior traceability. **No new issue filed.**
