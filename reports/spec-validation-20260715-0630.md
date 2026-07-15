# Spec Validation Report — 2026-07-15 (06:30 CEST cycle)

**Agent**: spec-validator (Carthos, Domain Architect)
**Date**: 2026-07-15 06:30 CEST
**Trigger**: cron schedule (no `@adf:spec-validator` mention)
**Verdict**: **NO CHANGE — territory byte-identical to the 02:31 & 03:30 cycles.**
No Gitea comment posted (would violate the noise-boundary rule). Silent re-survey.

> Disciplined-research discipline: independent re-measurement from first principles,
> no carry-forward of prior numbers. This cycle confirms the 03:30 trigger condition
> has **not fired** and adds one **newly-surveyed** territory (`implementation-plan-2301.md`)
> whose behavioural ACs are satisfied despite stale file-path references.

---

## Boundary Condition (the governing fact)

| Property | Value |
|---|---|
| Last cycle | 2026-07-15 03:30 CEST (`reports/spec-validation-20260715-0330.md`) |
| Time elapsed | 3 hours |
| Commits to repo since last cycle | **0** |
| `origin/main` | `dd4125b39` (== local HEAD == the 03:30 report commit) |
| `HEAD` vs `origin/main` | `0 0` (neither ahead nor behind) |
| Issue #2972 | open; 26 comments; last comment = the 02:31 verdict (02:36:22Z) — unchanged |

**The 03:30 cycle defined a precise trigger condition** for when a future cycle should
post a comment. This cycle tests that trigger directly:

| Trigger condition (from 03:30 §Next-cycle) | Fired? |
|---|---|
| Commit landing touching `crates/terraphim_orchestrator` / `exclude[]` / `AGENTS.md` | ❌ No (0 commits) |
| The `exclude[]` comment or workspace member count changed | ❌ No (identical) |
| Any of the 3 tracked defects remediated (→ PASS/CONDITIONAL transition) | ❌ No (all persist) |
| A genuinely new defect emerged | ❌ No (see Negative Space) |

**Trigger not fired.** Per the 03:30 cycle's own rule: *"Otherwise: silent re-survey."*

---

## Independent Re-measurement (first principles — no carry-forward)

`git ls-files "crates/$d/*.rs" | xargs wc -l`, Cargo.toml presence, workspace members:

| Directory | .rs files | LOC | Cargo.toml | 02:31 / 03:30 cycles | This cycle (06:30) | Delta |
|---|---|---|---|---|---|---|
| `terraphim_orchestrator` | 99 | 62,303 | PRESENT (v1.20.2) | 99 / 62,303 / PRESENT | **99 / 62,303 / PRESENT** | **0 / 0** |
| `terraphim_agent` | 7 | 6,819 | none | 7 / 6,819 / none | **7 / 6,819 / none** | **0 / 0** |
| `terraphim_agent_application` | 8 | 3,265 | PRESENT (v1.0.0) | 8 / 3,265 / PRESENT | **8 / 3,265 / PRESENT** | **0 / 0** |
| **subtotal** | **114** | **72,387** | — | 114 / 72,387 | **114 / 72,387** | **0 / 0** |

`cargo metadata --no-deps`: **18 workspace members** (unchanged).
`Cargo.toml:27`: `"crates/terraphim_orchestrator",` still in `exclude[]` (unchanged).
`Cargo.toml:17-19`: exclude comment still false for `orchestrator` + `agent_application` (2/5, unchanged).

**Byte-identical to both prior cycles. Zero drift.**

---

## Newly-Surveyed Territory: `implementation-plan-2301.md`

The 02:31 and 03:30 cycles scoped validation to `plans/` (Facet B). This cycle extends
the survey to the **root-level** `implementation-plan-2301.md` — an active spec
(Phase 3 ~90% complete, dated 2026-06-08) targeting `crates/terraphim_orchestrator/src/`.
No prior cycle examined it.

### What the plan claims

4 Acceptance Criteria + a §6 Verification Report asserting *"AC1, AC2, AC3, AC4 — all
present in code — PASS"*. The plan references three **NEW** files it expected to create:

| Plan-referenced file (expected) | Present in tree? |
|---|---|
| `crates/terraphim_orchestrator/src/pr_review/extractor.rs` | ❌ MISSING |
| `crates/terraphim_orchestrator/src/pr_review/poster.rs` | ❌ MISSING |
| `crates/terraphim_orchestrator/tests/verdict_poster_tests.rs` | ❌ MISSING |
| `crates/terraphim_orchestrator/src/pr_dispatch.rs` | ✅ present (223 LOC) |
| `crates/terraphim_orchestrator/src/reconcile_impl.rs` | ✅ present (1,832 LOC) |
| `crates/terraphim_orchestrator/src/pr_handlers_impl.rs` | ✅ present (907 LOC) |
| `crates/terraphim_orchestrator/src/spawn_impl.rs` | ✅ present (884 LOC) |

### What the code actually contains (the functionality home)

The `pr_review/` sub-module directory does **not exist**. Instead, the functionality
was consolidated into a single flat module `src/pr_review.rs` (560 LOC) with a parallel
test file `tests/pr_review_tests.rs` (184 LOC, 14 tests). All four ACs are behaviourally
satisfied:

| AC | Plan's stale path | Actual implementation home | Behaviour |
|---|---|---|---|
| **AC1** — verdict comment parseable (`Last reviewed commit:` footer) | `pr_review/extractor.rs` | `pr_review.rs::parse_verdict` (L129) + `pr_poller.rs:269` `contains("Last reviewed commit:")` | ✅ PASS |
| **AC2** — status verdict-derived (`min_confidence`) | `pr_review/poster.rs` | `pr_review.rs::AutoMergeCriteria.min_confidence` (L54) + `evaluate()` (L177) | ✅ PASS |
| **AC3** — classifier sees what drain sees (drain-log tolerance) | `pr_review/extractor.rs` drain read | `lib.rs:207` drain-log, `lib.rs:1105` `Lagged` tolerance, `spawn_impl.rs:521` durable drain fallback, `reconcile_impl.rs:1814` file-backed drain test | ✅ PASS |
| **AC4** — verdict-driven merge + remediation | `verdict_poster_tests.rs` | `auto_merge_impl.rs:417` stale-head guard, `pr_review_tests.rs` 14 tests (parse 5/5, 3/5 conditional, reject no-confidence, reject out-of-range, reject malformed footer, multi-round, evaluate approve/reject) | ✅ PASS |

### Finding classification

This is **spec-path drift** — the same defect class as the plans/ Facet B already
tracked under #2972. The plan's §6 verification report claims a file structure
(`pr_review/extractor.rs`, `pr_review/poster.rs`, `verdict_poster_tests.rs`) that the
restoration commit `2f276886c` did not reproduce. The behavioural ACs are nonetheless
satisfied in the consolidated `pr_review.rs` + `pr_review_tests.rs`.

**Severity: ℹ️ Note** (documentation/path drift, not a behavioural or blocker gap).
No regression; no missing functionality. The plan's file map is stale relative to
the consolidated implementation.

> Note: a `rustc --emit=metadata` parse-check of `pr_review.rs` reported unresolved
> `thiserror`. This is an **environment artifact** (invoking rustc outside cargo, no
> `target/debug/deps` resolution), not a code defect. The 02:31 cycle confirmed the
> crate's Cargo metadata is well-formed. Not treated as evidence of a defect.

---

## Re-Confirmed Defects (unchanged from 02:31 — NOT new findings)

The three structural sub-defects opened by the 02:31 cycle persist unchanged. This
cycle re-measures and confirms **no regression, no remediation, no drift**.

1. **Build-limbo** — orchestrator (99 files, 62k LOC, v1.20.2, actively committed)
   excluded from `cargo check --workspace` via `Cargo.toml:27`, contradicting
   restoration commit `2f276886c`'s stated goal. **Unchanged.**
2. **Self-contradicting exclude comment** — `Cargo.toml:17-19` falsely claims excluded
   dirs "no longer contain a top-level Cargo.toml" for 2 of 5 checked dirs
   (`orchestrator`, `agent_application`). **Unchanged.**
3. **Aggregate-root conflict** — `crates/terraphim_orchestrator/` (62k LOC) duplicates
   the source `AGENTS.md` rule 3 says lives only in `~/projects/terraphim/terraphim-agents`.
   Authoritative boundary still blurred. **Unchanged.**

**Facet B** (plans/ spec-location drift): unchanged; all 6 archived plans still cite
dead monorepo paths, ACs satisfied in polyrepo homes, `plans/RELOCATED.md` still
claims "13 active workspace crates" vs 18 actual.

---

## What I Explicitly Did NOT Find (Negative Space)

| Considered | Ruled out | Basis |
|---|---|---|
| New material change since 03:30 | None | 0 commits; HEAD == origin/main == dd4125b39 |
| Remediation of any of the 3 defects | None | exclude[], comment, AGENTS.md all unchanged |
| Regression / new stranded dir | None | Same 3 dirs, same LOC, same Cargo.toml state |
| #2972 closed (state transition) | No — open, 26 comments | Direct API: state=open, closed_at=null |
| New `Theme-ID: spec-gap` tracking issue | None | No open issue carries the marker |
| 2301 plan behavioural AC gap | None | All 4 ACs satisfied in consolidated `pr_review.rs` |
| 2301 plan missing functionality | None | parse_verdict, evaluate, drain-log, auto_merge all present |

---

## Decision: No Gitea Comment (noise-boundary rule)

Per the cron-schedule protocol and my own documented boundary rule
(`spec-validation-20260629-1230.md` §Meta-Finding, invoked by the 02:31 report):
comment privilege is reserved for **material change or state transition**.

This cycle has **neither**:
- LOC delta: **0** (vs +45,836 at 02:31)
- New stranded dir: **none**
- Regression: **none**
- State transition: **none** (the transition was the 02:31 finding itself)
- New behavioural AC violation: **none** (2301 ACs satisfied despite path drift)

The 2301 path-drift finding is a **ℹ️ Note**, not a blocker — the same class as the
already-tracked Facet B, and remediation is a one-line doc update to the plan's §6
file map. Posting a 27th comment that says "still FAIL, nothing moved, plus one ℹ️
note" 3 hours after the 26th would be the recurrence-noise the boundary rule exists
to prevent.

> Per task protocol cron branch: *"If nothing found, exit 0 silently."* This cycle
> found **no new gaps** (the 3 defects are already tracked; the 2301 finding is a
> ℹ️ Note, not a gap). Silent re-survey is the correct action; this report exists
> only as a survey-log entry so the next cycle can calibrate its measurement window
> against a known no-change checkpoint and knows the 2301 territory has now been
> surveyed.

---

## Traceability Matrix (unchanged + 2301 row added)

| Req (plan symbol) | Plan | Impl Location (true home) | Status |
|---|---|---|---|
| CorrectionEvent / capture_correction | design-gitea82 | `terraphim-agents/learnings/capture.rs` | ✅ PASS (polyrepo) |
| from_session_commands / TRIVIAL_COMMANDS | d3-session | `terraphim-agents/learnings/procedure.rs` | ✅ PASS (polyrepo) |
| MarkdownDirectives.trigger / TriggerIndex | design-gitea84 | `terraphim-core` registry (1.20.x) | ✅ PASS (consumed) |
| ListenerConfig / ListenerRuntime | single-agent-listener | `terraphim-agents/listener.rs` | ✅ PASS (polyrepo) |
| 2301 AC1–AC4 (verdict comment posting) | implementation-plan-2301 | `pr_review.rs` + `pr_review_tests.rs` (consolidated, not `pr_review/` sub-dirs) | ✅ PASS (behaviour) / ℹ️ path-drift |
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
three tracked defects; this report adds the 2301 territory to the surveyed set.
