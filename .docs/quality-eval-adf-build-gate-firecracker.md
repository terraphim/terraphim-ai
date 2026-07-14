# Quality Evaluation: ADF Build Gate Repair + Firecracker Runner Hardening

**Documents**: `.docs/research-adf-build-gate-firecracker.md` (Phase 1),
`.docs/design-adf-build-gate-firecracker.md` (Phase 2)
**Phase Transition**: Phase 1 → 2 (research) and Phase 2 → 3 (design)
**Status**: Research **PASS**; Design **CONDITIONAL PASS**
**Evaluator**: Claude Code session (KLS framework)
**Date**: 2026-07-13

## Executive Summary

Both documents were re-verified line-by-line against the codebase before scoring;
all corrections found were applied during this evaluation (env-assignment-stripping
loophole, atomic plan rejection, reuse of the existing `VmProvider` trait instead of
a proposed duplicate, host-only precedent in `ci-firecracker.yml`). The research
document passes cleanly. The design passes conditionally: two Track 2 unknowns
(source-delivery into the VM; `rust-ci` vm_type registration) are correctly
identified but must be settled by the named spike before steps 6-9 are implemented,
and three decisions remain with the human approver.

## KLS Dimension Scores — Research Document

| Dimension | Score | Justification |
|-----------|-------|---------------|
| Physical | 5/5 | Clean structure, tables, code-location map with `file:line` anchors; follows the Phase 1 template including Essentialism sections |
| Empirical | 4/5 | Precise actor/component naming; the two-runner-population distinction is called out explicitly. Minor: dense for a reader without bigbox context |
| Syntactic | 5/5 | Internally consistent after this pass; open questions marked answered where the appendix resolved them; no dangling placeholders remain |
| Semantic | 5/5 | Every code claim verified against the repo (`policy.rs:104-130`, `taxonomy_policy.rs:176-192`, `vm_executor.rs`, `rlm/Cargo.toml:43`, both workflow files). The one inaccurate claim found (registration-job survival mechanism) was corrected and strengthened the argument |
| Pragmatic | 5/5 | Two-track split lets a decision-maker unwedge immediately without committing to Track 2; spikes, risks, and assumptions are actionable |
| Social | 3/5 | Single-author draft; reviewer (Alex) has not yet signed off — inherent to a pre-approval document, not a defect |

**Average**: 4.5/5 — **Minimum**: 3/5 (Social). **PASS.**

## KLS Dimension Scores — Design Document

| Dimension | Score | Justification |
|-----------|-------|---------------|
| Physical | 5/5 | Template-complete: architecture diagrams, file-change tables, API sketch, test strategy, rollback, approval checklist |
| Empirical | 4/5 | Concrete signatures and env-var names; gherkin acceptance criteria. Minor: assumes reader knows fcctl-web endpoints |
| Syntactic | 4/5 | Consistent after this pass (provider naming, test locations, step numbering all reconciled with the corrected API section) |
| Semantic | 4/5 | Now matches the code: reuses the existing `VmProvider` trait / `with_provider()` hook at `task_worker.rs:221` rather than inventing `VmSessionProvider`. Residual: fcctl-web's create/poll/delete contract is documented only from workflow curl calls, not from its source (private repo) — spike-gated |
| Pragmatic | 4/5 | Steps are sequenced, sized, and independently reviewable; kill-switch rollback is one env var. Deduction: steps 6-9 depend on two unknowns the spike must settle first; sequencing should not start until it does |
| Social | 3/5 | Three explicit human decisions outstanding (deadlock breaker, canary approach, ADR timing); approval checklist unticked |

**Average**: 4.0/5 — **Minimum**: 3/5 (Social). **CONDITIONAL PASS.**

## Essentialism Evaluation (both documents)

| Check | Status | Evidence |
|-------|--------|----------|
| Vital Few Focus (≤5 items) | Pass | Design scope is exactly 5 items; research constraints capped at 3 |
| Eliminated Noise | Pass | Both docs carry explicit out-of-scope + Avoid-At-All-Cost (5/25) lists |
| Effortless Path | Pass | Track 2 reduced to one trait implementation + one selection point + one flag after the duplicate-trait proposal was eliminated |
| 90% Rule | Pass | Track 1 is unavoidable (wedged gate); Track 2 is the user's stated objective with an ADR-sanctioned rationale |

## Decision

**Research: PASS.** May proceed as the basis for design.

**Design: CONDITIONAL PASS.** May proceed to implementation for **Track 1
immediately**; **Track 2 steps 6-9 are blocked** until the conditions below clear.

### Conditions (Track 2)
1. Run the fcctl-web spike (design Step 8's verification, pulled forward): boot a
   VM, confirm auth, settle the **source-delivery contract** (shared mount vs
   in-VM clone vs tarball push) and the **vm_type registration** (`rust-ci` absent
   from `ci-firecracker.yml` options; runner default is `bionic-test`).
2. Resolve `timeout_seconds` vs `timeout_ms` against the live fcctl-web API.

### Human decisions (resolved 2026-07-13, recorded in the design doc)
1. Deadlock breaker: **authorised force-merge** (green local evidence in commit
   message, per-PR authorisation).
2. Track 2 rollout: **canary one runner unit**, then roll to all three or roll back.
3. ADR supersede timing: **after canary evidence**.

### Commendations
- Evidence discipline: every code claim in both documents now carries a verified
  `file:line` anchor.
- The policy-loophole finding (denied `curl`/`python3` executing behind
  `VAR=$(…)` stripping) upgrades the Track 2 rationale from "brittle" to
  "demonstrably wrong boundary".
- Genuine elimination: the design got *smaller* during review (no new trait, no
  new abstraction), which is the intended direction of Phase 2.
