# Spec Validation Report — 2026-07-15 (02:31 CEST cycle)

**Agent**: spec-validator (Carthos, Domain Architect)
**Date**: 2026-07-15 02:31 CEST
**Trigger**: cron schedule (no `@adf:spec-validator` mention)
**Verdict**: **FAIL** — pre-existing gap #2972 has **materially worsened** since the
last cycle (2026-06-29). Not recurrence noise; a genuine state transition.

> Disciplined-research discipline applied: independent re-measurement from first
> principles, no carry-forward of prior numbers. The last cycle's own lesson was
> *"a prior Carthos run self-corrected PASS → CONDITIONAL PASS after catching its
> own overconfidence; I do not repeat that mistake."*

---

## Boundary Event Since Last Cycle — MATERIAL CHANGE

The last cycle (2026-06-29 12:30 CEST) recorded no commits touching the stranded
dirs since 05:00. That was true then. **It is no longer true.**

### Timeline (the key causal fact)

| Time (CEST) | Event |
|---|---|
| 2026-06-29 12:30 | Last Carthos cycle measures orchestrator: **18 files / 16,467 LOC / no Cargo.toml**, labels it "residual polyrepo debris" |
| 2026-06-29 **13:06** | Commit `2f276886c` *"fix(orchestrator): restore god-file decomposition files on main"* lands — **36 min after the last cycle** |
| 2026-06-29 13:06 → now | 9 further orchestrator commits (allowlist, auto-merge gate, KG router) including **2 in July 2026** |
| 2026-07-15 02:31 | **This cycle** — first to measure the post-restoration reality |

**Implication**: the last cycle's verdict was correct for its measurement window.
No cycle has measured the post-restoration state until now. This is not a
re-measurement of an unchanged territory; it is the first measurement of a
changed one.

---

## Independent Re-measurement (Facet A — stranded/restored source)

`git ls-files "crates/$d/*.rs" | xargs wc -l`, Cargo.toml presence, last commit:

| Directory | .rs files | LOC | Cargo.toml | Last commit | Prior cycle |
|---|---|---|---|---|---|
| `terraphim_orchestrator` | **99** | **62,303** | **PRESENT (v1.20.2)** | 2026-07 (active) | 18 / 16,467 / none |
| `terraphim_agent` | 7 | 6,819 | none | 2026-06 (E4a cut) | 7 / 6,819 / none |
| `terraphim_agent_application` | 8 | 3,265 | **PRESENT (v1.0.0)** | 2026-06-22 | 8 / 3,265 / present |
| **subtotal** | **114** | **72,387** | — | — | 33 / 26,551 |

**Delta since last cycle: +81 files, +45,836 LOC, orchestrator Cargo.toml reappeared.**

This is the largest single-territory change any Carthos cycle has recorded for
this repo. It is not noise.

---

## The Worsened Defect (the FAIL)

### Defect 1 — Orchestrator is in build-limbo (structural contradiction)

Commit `2f276886c` stated goal, verbatim:
> "After this commit: 'cargo check --workspace' should compile terraphim_orchestrator"

**Falsified by measurement.** `cargo metadata --no-deps` (independent, not
carry-forward) lists **18 workspace members**; `terraphim_orchestrator` is
**absent**. The exclude block at `Cargo.toml:27` reads
`"crates/terraphim_orchestrator",`.

The restoration restored *files* but left the *exclude* in place. The orchestrator
is now: git-tracked (99 files), actively committed-to (2 July commits), versioned
(v1.20.2), **but build-invisible to `cargo check --workspace`**. It can only be
built with an explicit `-p terraphim_orchestrator` after removing the exclude —
which the restoration commit explicitly expected to be unnecessary.

### Defect 2 — Exclude-comment is now factually false (self-contradicting config)

`Cargo.toml:17-19` comment block states the excluded dirs
*"no longer contain a top-level Cargo.toml"*. Reality check on excluded dirs:

| Excluded dir | Cargo.toml present? | Comment accurate? |
|---|---|---|
| `terraphim_orchestrator` | **YES (v1.20.2)** | ❌ FALSE |
| `terraphim_agent_application` | **YES (v1.0.0)** | ❌ FALSE |
| `terraphim_agent` | no | ✅ true |
| `terraphim_settings` | no | ✅ true |
| `terraphim_automata` | no | ✅ true |

Two of the five checked excluded dirs **contradict the comment that justifies
their exclusion**. A maintainer reading this comment will be misled about what
the exclude does and why. This is a documentation-vs-reality drift — the same
class of defect as the plans-path drift already tracked under #2972 Facet B.

### Defect 3 — AGENTS.md deployment rule now conflicts with repo state

`AGENTS.md` (Bigbox Deployment Rules, item 3) states the orchestrator binary is
> "built from `/home/alex/projects/terraphim/terraphim-agents`, NOT from
> `/data/projects/terraphim/terraphim-ai`"

Yet `crates/terraphim_orchestrator/` in *this* repo now contains a full 62k-LOC
v1.20.2 source tree with active commits. The two repos hold duplicate, possibly
divergent copies of the same crate. **Which is authoritative?** The boundary is
blurred — a classic aggregate-root conflict. AGENTS.md says one thing; the repo
says another.

---

## Facet B — Spec-location drift (unchanged, re-confirmed)

All 6 `plans/*.md` (now under `plans/archive/`) cite pre-extraction monorepo
paths (`crates/terraphim_agent/src/learnings/...`, `crates/terraphim_automata/...`)
unresolvable in this repo. Behavioural ACs are satisfied in the code's polyrepo
home. `plans/RELOCATED.md` documents this correctly. **No change since last
cycle.** (Note: `plans/RELOCATED.md` line 28-32 claims scope is "13 active
workspace crates" but `cargo metadata` shows **18** — a minor doc staleness.)

---

## What I Explicitly Did NOT Find (Negative Space)

| Considered | Ruled out | Basis |
|---|---|---|
| Orchestrator code deleted/regressed | Migrated + restored, not deleted | 99 files, 62k LOC present, compiling tree |
| Behavioural AC violation in the 6 plans | None | ACs satisfied in polyrepo homes (re-confirmed) |
| New stranded dirs | None beyond the 3 | Exclude block otherwise consistent |
| `terraphim_agent` (7-file) changed | No | Byte-identical to last cycle (6,819 LOC) |
| This is a recurrence-comment nuisance | **No** — this is the threshold case | +45,836 LOC delta + Cargo.toml reappearance = material change, per my own boundary rule |

---

## Traceability Matrix

| Req (plan symbol) | Plan | Impl Location (true home) | Status |
|---|---|---|---|
| CorrectionEvent / capture_correction | design-gitea82 | `terraphim-agents/learnings/capture.rs` | ✅ PASS (polyrepo) |
| from_session_commands / TRIVIAL_COMMANDS | d3-session | `terraphim-agents/learnings/procedure.rs` | ✅ PASS (polyrepo) |
| MarkdownDirectives.trigger / TriggerIndex | design-gitea84 | `terraphim-core` registry (types 1.20.x, rolegraph 1.20.2) | ✅ PASS (consumed) |
| ListenerConfig / ListenerRuntime | single-agent-listener | `terraphim-agents/listener.rs` | ✅ PASS (polyrepo) |
| *Workspace exclude[] comment accuracy* | (implicit invariant) | `Cargo.toml:17-19` | ❌ **FAIL** — 2/5 checked dirs contradict comment |
| *"cargo check --workspace compiles orchestrator"* | commit `2f276886c` stated goal | `Cargo.toml:27` exclude | ❌ **FAIL** — orchestrator absent from workspace |
| *Single authoritative orchestrator source* | AGENTS.md §Bigbox rule 3 | `crates/terraphim_orchestrator/` (62k LOC) vs `~/projects/terraphim-agents` | ⚠️ **AMBIGUOUS** — dual sources, possible divergence |

---

## Verdict

**FAIL.** The pre-existing P2 gap (#2972) has **materially worsened** since the
last cycle. The orchestrator restoration (`2f276886c`, Jun 29 13:06) landed 36
minutes after the last measurement and was never re-measured until now. Three
new sub-defects emerged from that single commit:

1. **Build-limbo** — orchestrator is committed & active but excluded from
   `cargo check --workspace`, directly contradicting the restoration commit's
   stated goal.
2. **Self-contradicting config** — the exclude[] comment block is now factually
   false for 2 of 5 checked directories.
3. **Aggregate-root conflict** — AGENTS.md rule 3 says orchestrator lives only
   in `terraphim-agents`; this repo now holds a 62k-LOC duplicate.

**Why this cycle posts a comment (not silent exit):** my own boundary rule
(records `spec-validation-20260629-1230.md` §Meta-Finding) reserves comment
privilege for *"a material change in measurement (LOC delta, new stranded dir,
regression), or a state transition."* +45,836 LOC, +81 files, Cargo.toml
reappearance, and 3 new sub-defects is unambiguously material. This is not the
6th identical recurrence on an unchanged issue — it is the first recurrence
that carries genuinely new information since the gap was opened (2026-06-25).

**No P0/P1 behavioural spec violations** in the 6 archived plans; their ACs
remain satisfied in polyrepo homes. The FAIL is structural/configuration, not
behavioural.

---

## Recommended Remediation (for the implementation owner)

Smallest-fix-first, per requirements-traceability discipline:

1. **Decide the boundary**: Is `crates/terraphim_orchestrator` authoritative
   here, or in `terraphim-agents`? This is a domain decision, not a code change.
2. **If here**: remove `"crates/terraphim_orchestrator"` from `exclude[]`
   (line 27) so the restoration commit's stated goal actually holds; update
   `AGENTS.md` rule 3.
3. **If in terraphim-agents**: `git rm` the 99 files; restore the exclude
   comment's truth; keep rule 3 as-is.
4. **Either way**: fix the exclude[] comment block (`Cargo.toml:17-19`) — it is
   false for `orchestrator` and `agent_application` regardless of decision 1.
5. **Update `plans/RELOCATED.md`** line 28-32: claims "13 active workspace
   crates" but metadata shows 18.
