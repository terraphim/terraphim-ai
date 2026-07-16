# Spec Validation Report — 2026-07-15 (10:30 CEST cycle)

**Agent**: spec-validator (Carthos, Domain Architect)
**Date**: 2026-07-15 10:30 CEST
**Trigger**: cron schedule (no `@adf:spec-validator` mention)
**Verdict**: **NO CHANGE — territory byte-identical to the 02:31 / 03:30 / 06:30 / 08:30 / 09:30 cycles.**
No Gitea comment posted (would violate the noise-boundary rule). Silent re-survey.

> Disciplined-research discipline: independent re-measurement from first principles,
> no carry-forward of prior numbers. This cycle re-measures every invariant directly
> and resolves a measurement-tool anomaly (#2972 absent from the `gtr list-issues`
> window) against the authoritative REST API.

---

## Boundary Condition (the governing fact)

| Property | Value |
|---|---|
| Last cycle | 2026-07-15 09:30 CEST (`reports/spec-validation-20260715-0930.md`) |
| Time elapsed | 1 hour |
| `HEAD` | `6600938d506a7523b53abbc9628cdb99863c603e` |
| `origin/main` | `6600938d5` (== local HEAD) |
| `HEAD` vs `origin/main` | `0 0` (neither ahead nor behind) |
| Commits to `main` since 09:30 | **1** — `6600938d5` (the 09:30 report commit itself) |
| Last code commit touching any defect file | `58ad9745b` (orchestrator, 2026-07-06) — pre-today |
| Issue #2972 | **open**; 26 comments; last comment = 02:36:22Z — unchanged |

**The standing trigger condition** (defined 03:30, reconfirmed every cycle since) is tested directly:

| Trigger condition | Fired? |
|---|---|
| Commit landing on `main` touching `crates/terraphim_orchestrator` / `exclude[]` / `AGENTS.md` | ❌ No (sole commit is docs/reports only) |
| The `exclude[]` comment or workspace member count changed | ❌ No (22 exclude entries of which 12 carry Cargo.toml; 18 workspace packages — identical) |
| Any of the 3 tracked defects remediated (→ PASS/CONDITIONAL transition) | ❌ No (all persist) |
| A genuinely new **behavioural** defect emerged | ❌ No (see Newly-Surveyed / Negative Space) |

**0/4 triggers fired.** Per the standing rule: *"Otherwise: silent re-survey."*

---

## Independent Re-measurement (first principles — no carry-forward)

`git ls-files "crates/$d/*.rs" | xargs wc -l`, `Cargo.toml` presence, workspace members,
`exclude[]` comment accuracy — all re-measured this cycle:

| Invariant | 09:30 cycle | This cycle (10:30) | Δ |
|---|---|---|---|
| `terraphim_orchestrator` .rs files | 99 | **99** | 0 |
| `terraphim_orchestrator` LOC | 62,303 | **62,303** | 0 |
| orchestrator `Cargo.toml` | PRESENT (v1.20.2) | **PRESENT (v1.20.2)** | 0 |
| orchestrator in `exclude[]` | line :27 | **line :27** | 0 |
| `exclude[]` `crates/` entries (with Cargo.toml) | 22 (12) | **22 (12)** | 0 |
| workspace packages (`cargo metadata`) | 18 | **18** | 0 |
| orchestrator `src/lib.rs` | PRESENT (84K) | **PRESENT (84K)** | 0 |
| `members` list | 4 glob entries | **4 glob entries** | 0 |
| `plans/RELOCATED.md` "13 active" claim | wrong (18 real) | **wrong (18 real)** | 0 |

**Byte-identical to all five prior cycles. Zero drift.**

---

## Measurement-Tool Anomaly Resolved (#2972)

This cycle surfaced an apparent discrepancy that the boundary-aware discipline requires me
to resolve rather than assume away:

| Observation | `gtr list-issues --state open` | `gtr list-issues --state closed` | Authoritative REST API |
|---|---|---|---|
| #2972 present? | ❌ absent (20 issues, #3083–3131) | ❌ absent | ✅ present |

**Resolution.** `gtr list-issues` returns a fixed window of the 20 most-recently-numbered
open issues (3083–3131). #2972 (opened 2026-06-29) falls outside that window. A direct
`GET /api/v1/repos/terraphim/terraphim-ai/issues/2972` returns:

```
state:      open
closed_at:  None
comments:   26
title:      docs(plans): relocate stranded specs to polyrepo boundaries after #1910 extraction
```

**Conclusion: no state transition occurred.** The anomaly was a measurement-tool boundary
(pagination window), not a territorial change. #2972 remains open and is the standing
tracking issue for the three defects below. Last comment unchanged at 02:36:22Z.

> **Calibration note for future cycles.** Do not rely on `gtr list-issues` alone to detect
> a #2972 state transition — its default window excludes older open issues. Verify directly
> via the REST API (`GET .../issues/2972`) before concluding "issue absent = closed."

---

## Re-Confirmed Defects (unchanged from 02:31 — NOT new findings)

The three structural sub-defects opened by the 02:31 cycle persist unchanged. This
cycle re-measures and confirms **no regression, no remediation, no drift**.

1. **Build-limbo** — orchestrator (99 files, 62,303 LOC, v1.20.2, actively committed)
   excluded from `cargo check --workspace` via `Cargo.toml:27`, contradicting
   restoration commit `2f276886c`'s stated goal. **Unchanged.**
2. **Self-contradicting exclude comment** — `Cargo.toml:17-19` falsely claims excluded
   dirs "no longer contain a top-level Cargo.toml". Re-measurement: **12 of 22**
   excluded `crates/*` entries actually *do* carry a Cargo.toml (orchestrator,
   agent_application, automata_py, rolegraph_py, build_args, haystack_atlassian,
   haystack_discourse, symphony, rlm, github_runner, github_runner_server, gitea_runner).
   **Unchanged.**
3. **Aggregate-root conflict** — `crates/terraphim_orchestrator/` (62k LOC, 84 KB
   `src/lib.rs`) duplicates the source `AGENTS.md` rule 3 says lives only in
   `~/projects/terraphim/terraphim-agents`. Authoritative boundary still blurred. **Unchanged.**

**Facet B** (plans/ spec-location drift): unchanged; all 6 archived plans still cite
dead monorepo paths, ACs satisfied in polyrepo homes, `plans/RELOCATED.md` still
claims "13 active workspace crates" vs 18 actual.

**2301 plan** (root): path-drift ℹ️ Note, behavioural ACs satisfied — unchanged from 09:30.
**kg-runner-allowlist / offline-default** (`docs/plans/`): PASS / N/A (surveyed 08:30).
**adr/** (7 decision records): classified out-of-scope (surveyed 09:30). docs/plans/
topology unchanged this cycle.

---

## What I Explicitly Did NOT Find (Negative Space)

| Considered | Ruled out | Basis |
|---|---|---|
| New material change since 09:30 | None | 0 code commits on main; only the 09:30 report commit landed |
| Remediation of any of the 3 defects | None | exclude[], comment, AGENTS.md all unchanged |
| Regression / new stranded dir | None | Same 3 dirs, same LOC, same Cargo.toml state; 18 workspace pkgs |
| #2972 closed (state transition) | No — **open**, 26 comments | Authoritative REST API: `state=open`, `closed_at=null` |
| New `Theme-ID: spec-gap` tracking issue | None | Only #3048 (fleet-health alert) matched incidentally — not a spec-gap tracker |
| New plans in `plans/` | None | `plans/` still `{RELOCATED.md, archive/}` |
| New ADRs | None | Still ADR-001…ADR-007 (7, classified out of scope 09:30) |
| `gtr list-issues` missing #2972 | CLI pagination artifact | Returns 20 most-recent (3083–3131); #2972 outside window; REST API confirms open |
| task/3118 double-release fix on main | No (correct) | Lives on its branch only |

---

## Decision: No Gitea Comment (noise-boundary rule)

Per the cron-schedule protocol and the documented boundary rule
(`spec-validation-20260629-1230.md` §Meta-Finding, invoked since the 02:31 report):
comment privilege is reserved for **material change or state transition**.

This cycle has **neither**:
- Code LOC delta: **0** (the sole commit on main is this lineage's own docs/report)
- New stranded dir: **none**
- Regression: **none**
- State transition: **none** (#2972 open via REST API; the `gtr list-issues` absence is a tool-window artifact, now documented for future cycles)
- New behavioural AC violation: **none**

Posting a 27th comment that says "still FAIL, nothing moved" 1 hour after the 26th would be
the recurrence-noise the boundary rule exists to prevent.

> Per task protocol cron branch: *"If nothing found, exit 0 silently."* This cycle found
> **no new gaps** (the 3 defects are already tracked in #2972; no new behavioural defect
> emerged). Silent re-survey is the correct action; this report exists only as a
> survey-log entry so the next cycle can calibrate its measurement window against a known
> no-change checkpoint, and so the `gtr list-issues` pagination caveat is recorded for
> future state-transition detection.

---

## Traceability Matrix (unchanged)

| Req (plan/decision symbol) | Plan / ADR | Impl Location (true home) | Status |
|---|---|---|---|
| CorrectionEvent / capture_correction | design-gitea82 | `terraphim-agents/learnings/capture.rs` | ✅ PASS (polyrepo) |
| from_session_commands / TRIVIAL_COMMANDS | d3-session | `terraphim-agents/learnings/procedure.rs` | ✅ PASS (polyrepo) |
| MarkdownDirectives.trigger / TriggerIndex | design-gitea84 | `terraphim-core` registry (1.20.x) | ✅ PASS (consumed) |
| ListenerConfig / ListenerRuntime | single-agent-listener | `terraphim-agents/listener.rs` | ✅ PASS (polyrepo) |
| 2301 AC1–AC4 (verdict comment posting) | implementation-plan-2301 | `pr_review.rs` + `pr_review_tests.rs` (consolidated) | ✅ PASS (behaviour) / ℹ️ path-drift |
| TaxonomyPlanner / CommandPolicy / DeterministicPlanner removal | design-kg-driven-runner-allowlist | `terraphim_gitea_runner/taxonomy_policy.rs` + `policy.rs` (in-repo) | ✅ PASS |
| TuiBackend offline default | offline-default-design-2026-03-30 | `terraphim_agent/tui_backend.rs` (polyrepo `terraphim-agents`) | ✅ PASS (polyrepo) / N/A here |
| *ADR-001 … ADR-007* | `adr/` (decision records) | n/a — decisions, not specs | ℹ️ OUT OF SCOPE |
| *Workspace exclude[] comment accuracy* | (implicit invariant) | `Cargo.toml:17-19` | ❌ FAIL (unchanged) |
| *"cargo check --workspace compiles orchestrator"* | commit `2f276886c` goal | `Cargo.toml:27` exclude | ❌ FAIL (unchanged) |
| *Single authoritative orchestrator source* | AGENTS.md §Bigbox rule 3 | dual 62k-LOC copies | ⚠️ AMBIGUOUS (unchanged) |

---

## Next-cycle Trigger Condition (unchanged from 03:30)

A future cycle should post a comment (not exit silent) **only if** one of:
- A commit lands on **main** touching `crates/terraphim_orchestrator` / `exclude[]` / `AGENTS.md`, OR
- The `exclude[]` comment or workspace member count changes, OR
- Any of the 3 defects is remediated (→ PASS/CONDITIONAL PASS transition), OR
- A genuinely new **behavioural** defect emerges (a ℹ️ path-drift / out-of-scope note does not qualify).

Otherwise: silent re-survey. The 02:31 report remains the standing reference for the
three tracked defects. **State-transition detection must use the REST API for #2972**
(`GET .../issues/2972`), not `gtr list-issues` (whose default window excludes older issues).
