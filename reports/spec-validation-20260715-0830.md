# Spec Validation Report — 2026-07-15 (08:30 CEST cycle)

**Agent**: spec-validator (Carthos, Domain Architect)
**Date**: 2026-07-15 08:30 CEST
**Trigger**: cron schedule (no `@adf:spec-validator` mention)
**Verdict**: **NO CHANGE — territory byte-identical to the 02:31, 03:30 & 06:30 cycles.**
No Gitea comment posted (would violate the noise-boundary rule). Silent re-survey.

> Disciplined-research discipline: independent re-measurement from first principles,
> no carry-forward of prior numbers. This cycle extends the surveyed set to the
> `docs/plans/` directory (previously unsurveyed as a set) and confirms its two most
> recent / most relevant specs carry **no behavioural AC gap** against in-repo code.

---

## Boundary Condition (the governing fact)

| Property | Value |
|---|---|
| Last cycle | 2026-07-15 06:30 CEST (`reports/spec-validation-20260715-0630.md`) |
| Time elapsed | 2 hours |
| Commits to repo since last cycle | **1** — `b81990890` (the 06:30 report commit itself) |
| `origin/main` | `b81990890` (== local HEAD == the 06:30 report commit) |
| `HEAD` vs `origin/main` | `0 0` (neither ahead nor behind) |
| Files touched by `b81990890` | `reports/spec-validation-20260715-0630.md` **only** (docs/reports — not a trigger path) |
| Issue #2972 | open; 26 comments; last comment = the 02:31 verdict (02:36:22Z) — unchanged |

**The standing trigger condition** (defined 03:30, reconfirmed 06:30) is tested directly:

| Trigger condition | Fired? |
|---|---|
| Commit landing touching `crates/terraphim_orchestrator` / `exclude[]` / `AGENTS.md` | ❌ No (sole commit is docs/reports only) |
| The `exclude[]` comment or workspace member count changed | ❌ No (identical: 4 member entries, 23 exclude entries) |
| Any of the 3 tracked defects remediated (→ PASS/CONDITIONAL transition) | ❌ No (all persist) |
| A genuinely new **behavioural** defect emerged | ❌ No (see Newly-Surveyed Territory below) |

**0/4 triggers fired.** Per the standing rule: *"Otherwise: silent re-survey."*

---

## Independent Re-measurement (first principles — no carry-forward)

`git ls-files "crates/$d/*.rs" | xargs wc -l`, Cargo.toml presence, workspace members:

| Directory | .rs files | LOC | Cargo.toml | 06:30 cycle | This cycle (08:30) | Delta |
|---|---|---|---|---|---|---|
| `terraphim_orchestrator` | 99 | 62,303 | PRESENT (v1.20.2) | 99 / 62,303 / PRESENT | **99 / 62,303 / PRESENT** | **0 / 0** |
| `terraphim_agent` | 7 | 6,819 | none | 7 / 6,819 / none | **7 / 6,819 / none** | **0 / 0** |
| `terraphim_agent_application` | 8 | 3,265 | PRESENT (v1.0.0) | 8 / 3,265 / PRESENT | **8 / 3,265 / PRESENT** | **0 / 0** |

`cargo metadata --no-deps`: **18 workspace packages** (unchanged).
`Cargo.toml:4`: members list unchanged (4 entries: `crates/*`, `terraphim_server`, `terraphim_firecracker`, `terraphim_ai_nodejs`).
`Cargo.toml:27`: `"crates/terraphim_orchestrator",` still in `exclude[]` (unchanged).
`Cargo.toml:17-19`: exclude comment still false for `orchestrator` + `agent_application` (2/5 sampled unchanged).

**Byte-identical to all three prior cycles. Zero drift.**

---

## Newly-Surveyed Territory: `docs/plans/` (29 design/research docs)

Prior cycles scoped validation to `plans/` (Facet B) and the root `implementation-plan-2301.md`.
This cycle extends the survey to `docs/plans/`, which no prior cycle examined as a set.
No `docs/plans/*.md` doc carries a `## Acceptance Criteria` / `### AC` heading frame; the two
most relevant active specs were tested by their **expected target files** instead.

### (a) `docs/plans/design-kg-driven-runner-allowlist.md` → target lives IN this repo → TESTED → PASS

`terraphim_gitea_runner` is **in the workspace** (re-added at `Cargo.toml:53` per #3097), so
the design's ACs are testable against in-repo code. Every measurable AC is satisfied:

| Design AC | Test | Result |
|---|---|---|
| Delete `DeterministicPlanner` from `policy.rs` | `grep DeterministicPlanner policy.rs` | ✅ ABSENT |
| Delete `const ALLOWLIST` + `const RCH_CARGO_SUBCMDS` | `grep` in `policy.rs` | ✅ ABSENT |
| `lib.rs` pub-uses `TaxonomyPlanner` | `lib.rs:39,46` | ✅ `pub mod taxonomy_policy; pub use TaxonomyPlanner;` |
| `TaxonomyPlanner` struct in new file | `taxonomy_policy.rs:97` | ✅ `pub struct TaxonomyPlanner` |
| `default_policy.md` taxonomy file | file presence | ✅ PRESENT |
| `config.rs` has `taxonomy_dir` | `config.rs:62` | ✅ `pub taxonomy_dir: Option<PathBuf>` |

**Verdict: PASS.** No spec gap. Well-implemented design.

### (b) `docs/plans/offline-default-design-2026-03-30.md` → target crate POLYREPO-EXTRACTED → NOT VALIDATABLE HERE

Target crate `terraphim_agent` is in `exclude[]` (`Cargo.toml:22`) as a polyrepo-extracted dir
(7 .rs files, no Cargo.toml in this repo). Expected file `crates/terraphim_agent/src/tui_backend.rs`
is absent — but this is the **Facet B pattern** governed by `plans/RELOCATED.md`: *"should not
attempt to validate these specs against local source; the relevant crates no longer reside in this
repository."* The implementation lives in `terraphim-agents`. **Not a behavioural gap in this repo.**

### Finding classification

`docs/plans/` carries **no new behavioural defect** against in-repo code. One spec (kg-runner-allowlist)
passes cleanly; the remainder either target polyrepo-extracted crates (Facet B) or are historical
superseded designs. **[T4] trigger does not fire.**

---

## Re-Confirmed Defects (unchanged from 02:31 — NOT new findings)

The three structural sub-defects opened by the 02:31 cycle persist unchanged. This
cycle re-measures and confirms **no regression, no remediation, no drift**.

1. **Build-limbo** — orchestrator (99 files, 62k LOC, v1.20.2, actively committed)
   excluded from `cargo check --workspace` via `Cargo.toml:27`, contradicting
   restoration commit `2f276886c`'s stated goal. **Unchanged.**
2. **Self-contradicting exclude comment** — `Cargo.toml:17-19` falsely claims excluded
   dirs "no longer contain a top-level Cargo.toml". Sharper re-measurement: **12 of 13**
   excluded `crates/*` entries actually *do* carry a Cargo.toml (orchestrator,
   agent_application, gitea_runner, symphony, rlm, etc.). The tracked 2/5 sample is a
   conservative subset of this same static structural fact — unchanged.
3. **Aggregate-root conflict** — `crates/terraphim_orchestrator/` (62k LOC, has `src/lib.rs`
   85 KB) duplicates the source `AGENTS.md` rule 3 says lives only in
   `~/projects/terraphim/terraphim-agents`. Authoritative boundary still blurred. **Unchanged.**

**Facet B** (plans/ spec-location drift): unchanged; all 6 archived plans still cite
dead monorepo paths, ACs satisfied in polyrepo homes, `plans/RELOCATED.md` still
claims "13 active workspace crates" vs 18 actual.

**2301 plan** (root): path-drift ℹ️ Note, behavioural ACs satisfied — unchanged from 06:30.

---

## What I Explicitly Did NOT Find (Negative Space)

| Considered | Ruled out | Basis |
|---|---|---|
| New material change since 06:30 | None | 0 code commits; only the 06:30 report commit landed |
| Remediation of any of the 3 defects | None | exclude[], comment, AGENTS.md all unchanged |
| Regression / new stranded dir | None | Same 3 dirs, same LOC, same Cargo.toml state; 26 crates/ dirs total |
| #2972 closed (state transition) | No — open, 26 comments | Direct API: state=open, closed_at=null |
| New `Theme-ID: spec-gap` tracking issue | None | No open issue carries the marker |
| `docs/plans/` behavioural AC gap (kg-runner) | None | All 6 tested ACs satisfied in `terraphim_gitea_runner` |
| `docs/plans/` offline-default gap | None testable here | Target crate polyrepo-extracted (Facet B) |

---

## Decision: No Gitea Comment (noise-boundary rule)

Per the cron-schedule protocol and the documented boundary rule
(`spec-validation-20260629-1230.md` §Meta-Finding, invoked since the 02:31 report):
comment privilege is reserved for **material change or state transition**.

This cycle has **neither**:
- Code LOC delta: **0** (the sole commit is this lineage's own docs/report)
- New stranded dir: **none**
- Regression: **none**
- State transition: **none**
- New behavioural AC violation: **none** (kg-runner-allowlist passes; offline-default is Facet B)

Posting a 27th comment that says "still FAIL, nothing moved, plus docs/plans surveyed clean"
2 hours after the 26th would be the recurrence-noise the boundary rule exists to prevent.

> Per task protocol cron branch: *"If nothing found, exit 0 silently."* This cycle found
> **no new gaps** (the 3 defects are already tracked; `docs/plans/` is a PASS/NA territory).
> Silent re-survey is the correct action; this report exists only as a survey-log entry so the
> next cycle can calibrate its measurement window against a known no-change checkpoint and
> knows the `docs/plans/` territory has now been surveyed (kg-runner PASS; offline-default Facet B).

---

## Traceability Matrix (unchanged + docs/plans rows added)

| Req (plan symbol) | Plan | Impl Location (true home) | Status |
|---|---|---|---|
| CorrectionEvent / capture_correction | design-gitea82 | `terraphim-agents/learnings/capture.rs` | ✅ PASS (polyrepo) |
| from_session_commands / TRIVIAL_COMMANDS | d3-session | `terraphim-agents/learnings/procedure.rs` | ✅ PASS (polyrepo) |
| MarkdownDirectives.trigger / TriggerIndex | design-gitea84 | `terraphim-core` registry (1.20.x) | ✅ PASS (consumed) |
| ListenerConfig / ListenerRuntime | single-agent-listener | `terraphim-agents/listener.rs` | ✅ PASS (polyrepo) |
| 2301 AC1–AC4 (verdict comment posting) | implementation-plan-2301 | `pr_review.rs` + `pr_review_tests.rs` (consolidated) | ✅ PASS (behaviour) / ℹ️ path-drift |
| **TaxonomyPlanner / CommandPolicy / DeterministicPlanner removal** | **design-kg-driven-runner-allowlist** | **`terraphim_gitea_runner/taxonomy_policy.rs` + `policy.rs` (in-repo)** | **✅ PASS** |
| **TuiBackend offline default** | **offline-default-design-2026-03-30** | **`terraphim_agent/tui_backend.rs` (polyrepo `terraphim-agents`)** | **✅ PASS (polyrepo) / N/A here** |
| *Workspace exclude[] comment accuracy* | (implicit invariant) | `Cargo.toml:17-19` | ❌ FAIL (unchanged) |
| *"cargo check --workspace compiles orchestrator"* | commit `2f276886c` goal | `Cargo.toml:27` exclude | ❌ FAIL (unchanged) |
| *Single authoritative orchestrator source* | AGENTS.md §Bigbox rule 3 | dual 62k-LOC copies | ⚠️ AMBIGUOUS (unchanged) |

---

## Next-cycle Trigger Condition (unchanged from 03:30)

A future cycle should post a comment (not exit silent) **only if** one of:
- A commit lands touching `crates/terraphim_orchestrator` / `exclude[]` / `AGENTS.md`, OR
- The `exclude[]` comment or workspace member count changes, OR
- Any of the 3 defects is remediated (→ PASS/CONDITIONAL PASS transition), OR
- A genuinely new **behavioural** defect emerges (a ℹ️ path-drift note does not qualify).

Otherwise: silent re-survey. The 02:31 report remains the standing reference for the
three tracked defects; this report adds the `docs/plans/` territory to the surveyed set.
