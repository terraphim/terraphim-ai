# Spec Validation Report — 2026-07-15 (09:30 CEST cycle)

**Agent**: spec-validator (Carthos, Domain Architect)
**Date**: 2026-07-15 09:30 CEST
**Trigger**: cron schedule (no `@adf:spec-validator` mention)
**Verdict**: **NO CHANGE — territory byte-identical to the 02:31 / 03:30 / 06:30 / 08:30 cycles.**
No Gitea comment posted (would violate the noise-boundary rule). Silent re-survey.

> Disciplined-research discipline: independent re-measurement from first principles,
> no carry-forward of prior numbers. This cycle extends the surveyed set to the
> `adr/` directory (the root Architecture Decision Records) and classifies it as
> **decision-record territory, out of spec-validation scope**.

---

## Boundary Condition (the governing fact)

| Property | Value |
|---|---|
| Last cycle | 2026-07-15 08:30 CEST (`reports/spec-validation-20260715-0830.md`) |
| Time elapsed | 1 hour |
| `HEAD` | `7a8d82379009c168cc5bbb994b60d667027bfea4` |
| `origin/main` | `7a8d8237` (== local HEAD) |
| `HEAD` vs `origin/main` | `0 0` (neither ahead nor behind) |
| Commits to `main` since 08:30 | **1** — `7a8d8237` (the 08:30 report commit itself) |
| `c01124ae1` (test-fix) | lives on `task/3118-fix-vm-reuse-double-release` — **NOT on main/HEAD** |
| Issue #2972 | open; 26 comments; last comment = 02:31 verdict (02:36:22Z) — unchanged |

**The standing trigger condition** (defined 03:30, reconfirmed 06:30 / 08:30) is tested directly:

| Trigger condition | Fired? |
|---|---|
| Commit landing on `main` touching `crates/terraphim_orchestrator` / `exclude[]` / `AGENTS.md` | ❌ No (sole commit is docs/reports only) |
| The `exclude[]` comment or workspace member count changed | ❌ No (23 exclude entries, 18 workspace packages — identical) |
| Any of the 3 tracked defects remediated (→ PASS/CONDITIONAL transition) | ❌ No (all persist) |
| A genuinely new **behavioural** defect emerged | ❌ No (see Newly-Surveyed Territory below) |

**0/4 triggers fired.** Per the standing rule: *"Otherwise: silent re-survey."*

---

## Independent Re-measurement (first principles — no carry-forward)

`git ls-files "crates/$d/*.rs" | xargs wc -l`, `Cargo.toml` presence, workspace members:

| Directory | .rs files | LOC | Cargo.toml | 08:30 cycle | This cycle (09:30) | Delta |
|---|---|---|---|---|---|---|
| `terraphim_orchestrator` | 99 | 62,303 | PRESENT (v1.20.2) | 99 / 62,303 / PRESENT | **99 / 62,303 / PRESENT** | **0 / 0** |
| `terraphim_agent` | 7 | 6,819 | none | 7 / 6,819 / none | **7 / 6,819 / none** | **0 / 0** |
| `terraphim_agent_application` | 8 | 3,265 | PRESENT (v1.0.0) | 8 / 3,265 / PRESENT | **8 / 3,265 / PRESENT** | **0 / 0** |

`cargo metadata --no-deps`: **18 workspace packages** (unchanged).
`Cargo.toml:4`: members list unchanged (4 entries: `crates/*`, `terraphim_server`, `terraphim_firecracker`, `terraphim_ai_nodejs`).
`Cargo.toml`: `crates/terraphim_orchestrator` still in `exclude[]` (23 entries, unchanged).
Exclude-comment accuracy: **12 of 22** excluded `crates/*` dirs actually carry a `Cargo.toml` (unchanged).

**Byte-identical to all four prior cycles. Zero drift.**

---

## Newly-Surveyed Territory: `adr/` (7 root Architecture Decision Records)

Prior cycles scoped validation to `plans/` (Facet B), the root `implementation-plan-2301.md`,
and (08:30) `docs/plans/`. This cycle extends the survey to the root `adr/` decision records.
**They are not specs.**

| ADR | Title | Acceptance Criteria? | Code anchors? | Classification |
|---|---|---|---|---|
| ADR-001 | Layered Architecture & Dependency Direction Rules | 0 | 0 | **Decision** — out of scope |
| ADR-002 | Workspace Dependency Governance Policy | 0 | 0 | **Decision** — out of scope |
| ADR-003 | CLI Product-Line Strategy (Agent/CLI/Repl) | 0 | 0 | **Decision** — out of scope |
| ADR-004 | Feature-Gating and Boundary Boundaries | 1 (governance) | 0 | **Decision** — out of scope |
| ADR-005 | Server Composition Root & Runtime Bootstrap Extraction | 0 | 0 | **Decision** — out of scope |
| ADR-006 | Defer Inversion of KG-Router vs Static-Config Precedence | 0 | 5 (context) | **Decision** — code cited as *context*, not as an AC |
| ADR-007 | Decision Tier + pi-rust Routes via Taxonomy Templates | 0 | 3 (context) | **Decision** — code cited as *context*, not as an AC |

**Domain-modelling rationale.** The bounded-context boundary matters here. An ADR records a
*decision and its rationale*; a spec carries *implementable acceptance criteria validated against
code*. These 7 ADRs satisfy the former, not the latter: 6/7 carry zero ACs; ADR-004's single AC is
a governance rule, not a code-verifiable predicate; ADR-006/007 cite code only as the *factual basis*
of an already-made decision. **`adr/` is decision-record territory, not spec-validation territory.
No behavioural gap. [T4] does not fire.**

With `adr/` now classified, **every spec-adjacent directory in this repository has been surveyed**
(`plans/`, `plans/archive/`, `docs/plans/`, `adr/`, root plans). Remaining `.docs/` and
`docs/design*` entries are working-file summaries and historical design notes (the `.docs/` tree is
explicitly auto-generated summaries per AGENTS.md §Documentation Organization), not active governed
specs.

---

## Re-Confirmed Defects (unchanged from 02:31 — NOT new findings)

The three structural sub-defects opened by the 02:31 cycle persist unchanged. This
cycle re-measures and confirms **no regression, no remediation, no drift**.

1. **Build-limbo** — orchestrator (99 files, 62k LOC, v1.20.2, actively committed)
   excluded from `cargo check --workspace` via `Cargo.toml:27`, contradicting
   restoration commit `2f276886c`'s stated goal. **Unchanged.**
2. **Self-contradicting exclude comment** — `Cargo.toml:17-19` falsely claims excluded
   dirs "no longer contain a top-level Cargo.toml". Sharper re-measurement: **12 of 22**
   excluded `crates/*` entries actually *do* carry a Cargo.toml (orchestrator,
   agent_application, automata_py, rolegraph_py, build_args, haystack_atlassian,
   haystack_discourse, symphony, rlm, github_runner, github_runner_server, gitea_runner).
   The tracked 2/5 sample is a conservative subset of this static structural fact. **Unchanged.**
3. **Aggregate-root conflict** — `crates/terraphim_orchestrator/` (62k LOC, has `src/lib.rs`
   85 KB) duplicates the source `AGENTS.md` rule 3 says lives only in
   `~/projects/terraphim/terraphim-agents`. Authoritative boundary still blurred. **Unchanged.**

**Facet B** (plans/ spec-location drift): unchanged; all 6 archived plans still cite
dead monorepo paths, ACs satisfied in polyrepo homes, `plans/RELOCATED.md` still
claims "13 active workspace crates" vs 18 actual.

**2301 plan** (root): path-drift ℹ️ Note, behavioural ACs satisfied — unchanged from 08:30.
**kg-runner-allowlist** (`docs/plans/`): PASS (surveyed 08:30). **offline-default** (`docs/plans/`):
Facet B / N/A here (surveyed 08:30).

---

## What I Explicitly Did NOT Find (Negative Space)

| Considered | Ruled out | Basis |
|---|---|---|
| New material change since 08:30 | None | 0 code commits on main; only the 08:30 report commit landed |
| Remediation of any of the 3 defects | None | exclude[], comment, AGENTS.md all unchanged |
| Regression / new stranded dir | None | Same 3 dirs, same LOC, same Cargo.toml state; 18 workspace pkgs |
| #2972 closed (state transition) | No — open, 26 comments | Direct API: state=open, closed_at=null |
| New `Theme-ID: spec-gap` tracking issue | None | No open issue carries the marker |
| `adr/` behavioural AC gap | None | 7 ADRs classified as decision-records, out of validation scope |

---

## Decision: No Gitea Comment (noise-boundary rule)

Per the cron-schedule protocol and the documented boundary rule
(`spec-validation-20260629-1230.md` §Meta-Finding, invoked since the 02:31 report):
comment privilege is reserved for **material change or state transition**.

This cycle has **neither**:
- Code LOC delta: **0** (the sole commit on main is this lineage's own docs/report)
- New stranded dir: **none**
- Regression: **none**
- State transition: **none**
- New behavioural AC violation: **none** (`adr/` is decision-record territory, out of scope)

Posting a 27th comment that says "still FAIL, nothing moved, plus adr/ classified out of scope"
1 hour after the 26th would be the recurrence-noise the boundary rule exists to prevent.

> Per task protocol cron branch: *"If nothing found, exit 0 silently."* This cycle found
> **no new gaps** (the 3 defects are already tracked; `adr/` is decision-record territory, not
> a spec gap). Silent re-survey is the correct action; this report exists only as a survey-log
> entry so the next cycle can calibrate its measurement window against a known no-change
> checkpoint and knows the `adr/` territory has now been classified (out of scope).

---

## Traceability Matrix (unchanged + adr/ classification row added)

| Req (plan/decision symbol) | Plan / ADR | Impl Location (true home) | Status |
|---|---|---|---|
| CorrectionEvent / capture_correction | design-gitea82 | `terraphim-agents/learnings/capture.rs` | ✅ PASS (polyrepo) |
| from_session_commands / TRIVIAL_COMMANDS | d3-session | `terraphim-agents/learnings/procedure.rs` | ✅ PASS (polyrepo) |
| MarkdownDirectives.trigger / TriggerIndex | design-gitea84 | `terraphim-core` registry (1.20.x) | ✅ PASS (consumed) |
| ListenerConfig / ListenerRuntime | single-agent-listener | `terraphim-agents/listener.rs` | ✅ PASS (polyrepo) |
| 2301 AC1–AC4 (verdict comment posting) | implementation-plan-2301 | `pr_review.rs` + `pr_review_tests.rs` (consolidated) | ✅ PASS (behaviour) / ℹ️ path-drift |
| TaxonomyPlanner / CommandPolicy / DeterministicPlanner removal | design-kg-driven-runner-allowlist | `terraphim_gitea_runner/taxonomy_policy.rs` + `policy.rs` (in-repo) | ✅ PASS |
| TuiBackend offline default | offline-default-design-2026-03-30 | `terraphim_agent/tui_backend.rs` (polyrepo `terraphim-agents`) | ✅ PASS (polyrepo) / N/A here |
| *ADR-001 … ADR-007* | `adr/` (decision records) | n/a — decisions, not specs | ℹ️ OUT OF SCOPE (classified this cycle) |
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
three tracked defects; this report adds the `adr/` classification (out of scope) and
completes the survey of all spec-adjacent directories in the repository.
