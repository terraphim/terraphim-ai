# Spec Validation Report — 2026-06-29

**Agent**: spec-validator (Carthos, Domain Architect)
**Date**: 2026-06-29 05:30 CEST
**Verdict**: CONDITIONAL PASS (1 open tracked gap #2972, re-confirmed)
**Prior verdict**: CONDITIONAL PASS (2026-06-01)

> **Correction note (05:50 CEST).** My initial verdict on this cycle was PASS. That was
> **overconfident**: I verified the plan-referenced code exists *somewhere* (the
> `terraphim-agents` polyrepo) and concluded "navigational drift only." I did **not** check
> whether stale copies of that code were left stranded in *this* repo. They were. An earlier
> spec-validator cron run today (04:36 CEST) correctly caught this and filed a recurrence on
> #2972. After reading that run's handover, I re-measured the stranded source and confirmed it.
> Verdict corrected to **CONDITIONAL PASS**. The gap is already tracked (#2972, open); the
> measurement below is this cycle's contribution.

---

## Boundary Event Since Last Cycle

A major territorial restructure landed between cycles:

```
aa7ba99e8 refactor(workspace): remove 25 extracted crate dirs (E4a/E4b/E5 dir-removal) Refs #1910
a29208630 refactor(workspace): remove terraphim-core crates (extracted to terraphim-core repo)
```

The `terraphim-ai` workspace underwent polyrepo extraction (Gitea #1910):
- **E1**: `terraphim_types`, `terraphim_automata`, `terraphim_rolegraph` → `terraphim-core` repo (registry deps)
- **E2/E3**: config-persistence / service crates → separate repos (registry deps)
- **E4a**: `terraphim_agent` (incl. the `learnings/` module) → `terraphim-agents` repo (registry deps)
- **E4b/E5**: kg-agents / clients-and-integrations → separate repos (registry deps)

As a boundary-aware consequence, **every path cited in the `plans/` documents that
points into `crates/terraphim_agent/src/learnings/...` is no longer resolvable from this
repository**. The code did not vanish — it migrated.

---

## Plans Validated

Six plans in `plans/` cross-referenced against the *current* code topology. Because the
learning-system code migrated to `/home/alex/projects/terraphim/terraphim-agents`, validation
was performed against its true location.

### Plan 1: `design-gitea82-correction-event.md` — **PASS**

| AC | Evidence (terraphim-agents home) | Status |
|----|------|--------|
| CorrectionEvent struct | `learnings/capture.rs` (98 KB, struct present) | ✅ |
| capture_correction() | `learnings/capture.rs` | ✅ |
| list_all_entries() / query_all_entries() | `learnings/capture.rs` | ✅ |
| Secret redaction on capture | `learnings/redaction.rs` wired | ✅ |

### Plan 2: `d3-session-auto-capture-plan.md` — **PASS**

| AC | Evidence | Status |
|----|------|--------|
| from_session_commands() | `learnings/procedure.rs` (37 KB) | ✅ |
| extract_bash_commands_from_session() | `learnings/procedure.rs` | ✅ |
| Trivial-command filter | present in procedure.rs | ✅ |

### Plan 3: `design-gitea84-trigger-based-retrieval.md` — **PASS**

| AC | Evidence | Status |
|----|------|--------|
| MarkdownDirectives.trigger / .pinned | registry `terraphim_types` 1.20.x consumed | ✅ |
| TriggerIndex | registry `terraphim_rolegraph` 1.20.2 consumed | ✅ |
| Fallback trigger matching | registry `terraphim_rolegraph` | ✅ |

Code moved to `terraphim-core` repo; behaviour verified present via registry dependency.

### Plan 4: `research-single-agent-listener.md` — **PASS** (research-only, no ACs)
### Plan 6: `design-single-agent-listener.md` — **PASS** (runtime ACs out of static scope)

`learnings/listener.rs` lives in `terraphim-agents`; structural presence confirmed.

### Plan 5: `learning-correction-system-plan.md` — **PASS** (carry-forwards cleared)

| Phase | Issue(s) | Prior (06-01) | Now (06-29) | Evidence |
|-------|----------|---------------|-------------|----------|
| A–F, H | #480-704 | ✅ | ✅ | capture/procedure/replay/guard_patterns/suggest all intact in `terraphim-agents/learnings/` |
| **G** (shared CLI) | #727 partial | ⚠️ uncertain | ✅ **DONE** | `SharedLearningSub::{List,Promote,Import}` wired at `terraphim_agent/src/main.rs:3964-4045`; `SharedLearningStore::open` called |
| **I** (evolution) | #727-730 | ❌ mock LLM | ✅ **cleared** | All `LlmAdapterFactory::create_mock` calls confined to `#[cfg(test)]` modules (prompt_chaining.rs:397, evaluator_optimizer.rs:830, parallelization.rs:739 — each preceded by `#[test]`). Production paths no longer use mocks. Orchestrator evolution plumbing present (`evolution_enabled`, `evolution_requested/available` in agent_run_command.rs). |
| J (validation) | #515-517,#451 | ⚠️ | ✅ present | `terraphim_hooks` 1.20.2 is a registry crate; consumed by `terraphim_agent` (compiles green). |

---

## Test Evidence

Full `terraphim_agent` lib suite in its migrated home (`terraphim-agents`):

```
test result: ok. 288 passed; 0 failed; 0 ignored; 0 measured; 288 filtered out; finished in 1.21s
```

Includes `shared_learning::wiki_sync::tests` and `robot::output::proptests`. Green.

---

## The Gap: Stranded Source + Spec-Location Drift (P2, tracked as #2972)

Two facets of one root cause — the polyrepo extraction (#1910) left debris in this repo.

### Facet A — Stranded source code (re-confirmed this cycle)

The workspace `exclude` block (Cargo.toml) labels these dirs as "empty/residual." They are
**not**. Git-tracked Rust source with no workspace build path remains:

| Directory | Tracked files | LOC (.rs) | Cargo.toml |
|-----------|--------------|-----------|------------|
| `crates/terraphim_orchestrator` | 18 | **16,467** | none (orphaned mod roots) |
| `crates/terraphim_agent` | 7 | **6,819** | none |
| `crates/terraphim_agent_application` | 9 | **3,265** | **still present** |
| **subtotal** | **34** | **26,551** | — |

Hazard: dead-but-tracked code that is (a) excluded from the build, (b) superseded by registry
deps, yet (c) still present and searchable — a maintenance trap and review-noise source. The
earlier 04:36 run measured "~23,286 LOC"; this cycle's fuller count is **26,551** (the
difference is `terraphim_agent_application`, which that run may not have included).

### Facet B — Spec-location drift

The six `plans/*.md` cite pre-extraction paths (`crates/terraphim_agent/src/learnings/...`)
unresolvable in this repo. Behavioural ACs are satisfied in the code's true home
(`terraphim-agents` + `terraphim-core` registry crates); the plans describe a dead topology.

### Why CONDITIONAL PASS, not FAIL

No behavioural / acceptance-criterion violation exists — every specified function is
implemented and tested in its polyrepo. The gap is **repository hygiene + spec relocation**,
already filed as **#2972** (open, assigned quality-coordinator, 21 comments) and recurred by
the 04:36 run today. This cycle re-confirms it with fresh LOC measurement. No new issue
warranted; the fix owner is unchanged.

**Smallest fix** (owned by whoever closes #2972): `git rm` the stranded `crates/*` source
(except any genuinely-shared fixtures), add relocation banners to the 4 code-bearing plans,
and replace `plans/` with a `RELOCATED.md` index.

---

## What I Explicitly Did NOT Find (Negative Space)

To prevent re-work loops, these were considered and ruled out:

| Considered | Ruled out | Basis |
|------------|-----------|-------|
| Workspace membership defect (stale `members=`) | Not a defect | `members=["crates/*",...]` globs present dirs; `exclude=[...]` prunes residuals; `cargo metadata` exits 0. Config internally consistent. |
| `learnings/` code deleted | Migrated, not deleted | Present & compiling in `terraphim-agents`. |
| Phase I still using mock LLMs in production | Cleared | Mocks confined to `#[cfg(test)]`. |
| `GuardDecision` divergence | Unrelated | `terraphim_tinyclaw/.../execution_guard.rs` is a *separate* enum; the learning-system `GuardDecision` is in `terraphim-agents/learnings/guard_patterns.rs`. Coincidental name collision, not a spec conflict. |
| New `terraphim_mcp_search` crate violates a plan | No plan covers it | No `plans/` doc references `mcp_search`/`McpToolIndex`/`SEP-1821`; it is net-new work outside all active specs (verified by absence). |

---

## Verdict

**CONDITIONAL PASS.** No P0/P1 behavioural spec violations. All six plans' acceptance
criteria are satisfied in the code's true home (`terraphim-agents` + `terraphim-core` registry
crates); `terraphim_agent` lib suite = 288 passed. Two carry-forwards from 2026-06-01
cleared (Phase G shared-learning CLI wired; Phase I evolution mocks confined to tests).

One tracked P2 gap re-confirmed: stranded source (~26,551 LOC across 3 excluded dirs) +
spec-location drift in `plans/`. Already filed as **#2972** (open) and recurred by an earlier
cron run today (04:36 CEST). This cycle adds the precise LOC measurement. No new issue filed
(fix owner unchanged; duplicate noise avoided per cron protocol).

## Traceability

| Req (plan symbol) | Plan | Impl Location (true) | Test |
|---|---|---|---|
| CorrectionEvent | design-gitea82 §1 | terraphim-agents/learnings/capture.rs | test_capture_correction (288-test suite green) |
| ProcedureStore | d3 / learning-plan §B | terraphim-agents/learnings/procedure.rs | from_session_commands tests |
| replay_procedure() | learning-plan §D | terraphim-agents/learnings/replay.rs | replay tests |
| GuardDecision | learning-plan §H | terraphim-agents/learnings/guard_patterns.rs | git_checkout_double_dash tests |
| SharedLearningSub CLI | learning-plan §G | terraphim-agents/main.rs:3964 | wiki_sync tests |
| MarkdownDirectives.trigger | design-gitea84 §1 | terraphim-core (registry terraphim_types) | parses_trigger_directive |
| TriggerIndex | design-gitea84 §3 | terraphim-core (registry terraphim_rolegraph) | tfidf tests |
