# Spec Validation Report — 2026-06-29 (12:30 CEST cycle)

**Agent**: spec-validator (Carthos, Domain Architect)
**Date**: 2026-06-29 12:30 CEST
**Trigger**: cron schedule (no `@adf:spec-validator` mention)
**Verdict**: **CONDITIONAL PASS** — gap #2972 re-confirmed, **unchanged** since 06:30 cycle.
**Prior verdict**: CONDITIONAL PASS (07:36, 06:33, 05:30 cycles; same-day report exists at `reports/spec-validation-20260629.md`)

> This is a disambiguated same-day report. The 05:30 CEST report
> (`reports/spec-validation-20260629.md`) is preserved unchanged — it contains a
> valuable self-correction note about overconfidence that should not be lost.

---

## Boundary Event Since Last Cycle

`git log --since="2026-06-29 05:00"` shows **no commits** touching the stranded
dirs (`crates/terraphim_{orchestrator,agent,agent_application}`) or any file under
`plans/`. The only commits since 05:00 are the prior cycles' own report corrections:

```
7b058b58a Merge gitea/main to reconcile spec-validator report divergence
6d59e579e docs(reports): correct spec-validation 2026-06-29 verdict to CONDITIONAL PASS
87141e091 docs(reports): spec-validation 2026-06-29 — PASS (carry-forwards cleared)
```

**No behavioural or structural change** entered the system between the 07:36 cycle
and this one. The territory is static.

---

## Independent Re-measurement (not trusting the prior map)

I re-ran every measurement from first principles rather than carrying the 07:36
numbers forward. A prior Carthos run self-corrected PASS → CONDITIONAL PASS today
after catching its own overconfidence; I do not repeat that mistake.

### Plans Validated (all 6, ACs checked against true home)

| # | Plan | AC Location (true home) | Status |
|---|------|-------------------------|--------|
| 1 | `design-gitea82-correction-event.md` | `terraphim-agents/learnings/capture.rs` — `CorrectionEvent`:502, `capture_correction`:1042, `list_all_entries`:1318, `query_all_entries`:1392 | ✅ PASS |
| 2 | `d3-session-auto-capture-plan.md` | `terraphim-agents/learnings/procedure.rs` — `from_session_commands`:412, `extract_bash_commands_from_session`:471, `TRIVIAL_COMMANDS`:382 | ✅ PASS |
| 3 | `design-gitea84-trigger-based-retrieval.md` | `terraphim-core` registry crates (terraphim_types 1.20.x, terraphim_rolegraph 1.20.2) consumed | ✅ PASS |
| 4 | `research-single-agent-listener.md` | research-only, no ACs | ✅ N/A |
| 5 | `learning-correction-system-plan.md` | phases A–J intact in `terraphim-agents/learnings/`; Phase G CLI wired, Phase I mocks confined to `#[cfg(test)]` | ✅ PASS |
| 6 | `design-single-agent-listener.md` | `terraphim-agents/listener.rs` present (69 KB, 12 tests) — runtime ACs out of static scope | ✅ PASS |

### The Gap (P2, tracked as #2972, OPEN, Theme-ID: spec-gap)

Two facets, one root cause — polyrepo extraction (#1910) left debris in this repo.

**Facet A — Stranded source (re-measured, byte-identical to 06:30):**

| Directory | .rs files | LOC | Cargo.toml |
|-----------|-----------|-----|------------|
| `crates/terraphim_orchestrator` | 18 | 16,467 | none (orphaned mod roots) |
| `crates/terraphim_agent` | 7 | 6,819 | none |
| `crates/terraphim_agent_application` | 8 | 3,265 | **still present** |
| **subtotal** | **33** | **26,551** | — |

`git ls-files … | xargs wc -l | tail -1` → **26551 total**. Identical to every
measurement since 06:30. The `exclude` block in `Cargo.toml` (lines 17–19) labels
these as "Empty / residual directories" — they are **not** empty; they are
git-tracked, build-excluded, and searchable. Maintenance trap; review-noise source.

**Facet B — Spec-location drift:** all 6 `plans/*.md` cite pre-extraction paths
(`crates/terraphim_agent/src/learnings/...`) unresolvable in this repo. Behavioural
ACs are satisfied in the code's true home (`terraphim-agents` + `terraphim-core`
registry crates); the plans describe a dead topology.

---

## Meta-Finding: Recurrence Flooding (the new emergent defect)

> This is the most important contribution of this cycle.

Issue **#2972** has received **5 recurrence comments from spec-validator today**
(04:39, 05:30, 05:53-by-root, 06:33, 07:36), each re-stating the *identical*
26,551-LOC measurement on an *unchanged*, *open*, *actively-owned* issue
(updated_at 07:38, 24 comments). A 6th recurrence from this cycle would carry **zero
new information**.

This is the inverse failure mode of the 05:30 overconfidence: where that run
under-measured, recurrence-flooding over-reports. Both erode the signal-to-noise
ratio that makes the spec-validator cron useful. The AGENTS.md issue-hygiene rule
("Autonomous agents can generate duplicate noise — batch-close with comment")
applies directly.

**Boundary I am drawing this cycle:** I did **NOT** post a recurrence comment to
#2972. The cron protocol's create-issue branch guards with "If nothing found, exit
0 silently." I extend that spirit: **if nothing CHANGED since the last cycle on an
already-tracked issue, a recurrence comment adds no information and should not be
posted.** Signal is reserved for: a new gap, a material change in measurement
(LOC delta, new stranded dir, regression), or a state transition on the issue.

This boundary belongs in the spec-validator skill, not just this report.

---

## What I Explicitly Did NOT Find (Negative Space, re-confirmed)

| Considered | Ruled out | Basis |
|------------|-----------|-------|
| Workspace membership defect | Not a defect | `members=["crates/*",…]` globs present dirs; `exclude=[…]` prunes residuals; internally consistent |
| learnings/ code deleted | Migrated, not deleted | Present & compiling in `terraphim-agents` (capture.rs 98 KB, procedure.rs intact) |
| Phase I still mock-LLM in production | Cleared | Mocks confined to `#[cfg(test)]` |
| New stranded dirs since 06:30 | None | `git log` empty for the window; exclude list unchanged |
| New plans referencing dead paths | None | `plans/` unchanged since 06:30 |

---

## Traceability Matrix

| Req (plan symbol) | Plan | Impl Location (true) | Test evidence |
|---|---|---|---|
| CorrectionEvent | design-gitea82 §1.2 | terraphim-agents/learnings/capture.rs:502 | 288-test suite green (prior cycle) |
| capture_correction | design-gitea82 §1.4 | terraphim-agents/learnings/capture.rs:1042 | covered |
| list_all_entries / query_all_entries | design-gitea82 §1.5 | terraphim-agents/learnings/capture.rs:1318/1392 | covered |
| from_session_commands | d3 §"New CLI subcommand" | terraphim-agents/learnings/procedure.rs:412 | test_extract_bash_commands_from_session:991 |
| TRIVIAL_COMMANDS filter | d3 §"Trivial command filter" | terraphim-agents/learnings/procedure.rs:382 | present |
| MarkdownDirectives.trigger/.pinned | design-gitea84 §1 | terraphim-core registry (terraphim_types 1.20.x) | consumed, compiles |
| TriggerIndex | design-gitea84 §3 | terraphim-core registry (terraphim_rolegraph 1.20.2) | consumed, compiles |
| ListenerConfig / ListenerRuntime | design-single-agent-listener | terraphim-agents/listener.rs | 12 listener tests |

---

## Verdict

**CONDITIONAL PASS.** No P0/P1 behavioural spec violations. All six plans' ACs are
satisfied in their polyrepo homes. One tracked P2 gap (#2972) re-confirmed with an
**unchanged measurement** (26,551 LOC). Fix owner unchanged.

**This cycle's net contribution:** (1) an independent re-measurement that confirms
the gap is stable, and (2) a meta-finding that the recurrence-comment pattern has
become noise and should be throttled in the spec-validator skill.

**No Gitea action taken** — posting a 6th identical recurrence on an unchanged,
tracked, owned issue would violate issue hygiene and the spirit of the cron
protocol's silent-exit guard. Per protocol, this cycle exits silently with evidence
recorded locally.
