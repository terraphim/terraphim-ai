# KLS Quality Evaluation: Merge-Coordinator Single-Writer Design

**Artefact evaluated**: `docs/plans/design-merge-coordinator-single-writer-2026-08-27.md`
**Issue**: terraphim/terraphim-ai `#3295` (producer for digital-twins `#165`)
**Base**: `23445b9e1862104a1d68d6a155b44ef7533483df`
**Evaluation date**: 2026-08-27
**Framework**: Krogstie–Lindland–Sindre (KLS) six-dimension model
**Evaluator**: quality-gate review (session-scoped, no-commit)

---

## Verdict

**CONDITIONAL PASS / NO-GO UNTIL BLOCKERS RESOLVED.**

The design is approved as a design checkpoint only. Implementation must not start
until the blocking decisions in §4 are recorded on the trackers.

---

## 1. Scores

| Dimension | Score | Rationale |
|---|---|---|
| **Physical quality** | **4** | Complete and self-contained: problem statement, 29 evidence anchors with file:line, seven decisions, eight invariants, five components, ordered TDD steps, full RED test plan, security matrix, rollout/rollback, verification ladder. Persistent, reachable, unambiguous layout. |
| **Empirical quality** | **4** | Every claim is traceable to base-commit evidence (E1–E29); test plan is executable as written (deterministic marker-file harness, stateful fake per project non-mock policy); verification ladder is runnable command-by-command. |
| **Syntactic quality** | **4** | Well-formed Markdown with stable section numbering, consistent cross-references (D/E/INV/T/M/C/B identifiers), consistent notation for file:line anchors and decision tables. |
| **Semantic quality** | **4 (after corrections)** | The original draft contained a correctness contradiction: it demanded same-name/different-project coexistence (T2) while keeping the name-keyed `active_agents` map and listing re-keying as out of scope. The applied corrections (§2) resolve this — canonical re-key is mandatory and in scope — and close the head-SHA bypass hole. After corrections the model is internally and externally consistent. |
| **Pragmatic quality** | **3** | Strong for the intended audience (implementer, reviewer, ops) but unresolved execution dependencies cap it: the terraphim-agents child is now linked as `#136`, but the #2892/PR #3130 merge order is undecided, and the exact-head precondition may skip merges against servers that omit SHAs (an accepted operational trade-off that on-call must know). |
| **Social quality** | **3** | Stakeholder agreement is partially achieved: the design is reviewable and traceable to `#3295`, but two stakeholder decisions (deployment-duality ownership; #2892 absorption vs sequencing) are still open, and monitor owners must be told the `LockHeld` exit-code change (B4) before rollout. |
| **Average** | **3.67** | |

---

## 2. Applied corrections (all incorporated into the design artefact)

1. **Canonical re-key** — D1 now *mandates* re-keying `active_agents` and every
   access site from name-only to canonical `(project_id, agent_name)` via
   `agent_key(def)`, so same-name agents in different projects coexist inside one
   process and across processes; D4's in-process lookup is
   `active_agents.contains_key(&agent_key(def))`; T2 proves both spawn inside one
   orchestrator process with companion cross-process T2b; re-keying removed from
   out-of-scope; summary seams now begin with the re-key.
2. **Reserved-id mapping** — reserved legacy id `__global__` maps to safe disk
   component `global` before path construction; every non-reserved
   project/agent/explicit-key component validated against
   `^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$` at config load.
3. **Mandatory exact heads** — automatic merge requires *both* the evaluated and
   the fresh head SHA; either missing ⇒ skip (fail-closed, no legacy `None`
   bypass); unequal ⇒ skip as head moved (D6.4, INV6, M2b).
4. **Dual-repo delivery** — C2 explicitly spans canonical production orchestrator
   delivery in `terraphim-agents` plus synchronized integration in
   `terraphim-ai`; neither PR alone may claim readiness; B1 requires the child
   issue linked before code and forbids uncommitted copying between repos.
5. **Strict TDD/Clippy** — C1 unit tests are RED-first; verification ladder runs
   `cargo clippy … --all-targets --all-features -- -D warnings`.

---

## 3. Status transition

`PROPOSED` → `CONDITIONALLY APPROVED` — design checkpoint preserved; the status
block in the design artefact records the blocking conditions and points here.

---

## 4. Blocking decisions (must be recorded before implementation)

1. **Resolved — `terraphim-agents#136` created and linked** (B1): the canonical
   production-orchestrator child exists. Its C2 half lands via git only — no
   uncommitted copying (scp/cp) between repos.
2. **Triage terraphim-ai PR #3130 for #2892**: decide up front whether the
   `GiteaOperations` trait work (#2892 / PR #3130) lands first or is absorbed
   into this task and superseded; D7's seam depends on the outcome.
3. **Freeze two-repo delivery order and require both exact heads**: fix the
   terraphim-agents ↔ terraphim-ai PR ordering, review both PRs at their exact
   head SHAs, and enforce that neither PR alone may claim readiness.

---

## 5. Gate decision

**Preserve the design checkpoint; do NOT implement until the blockers in §4 are
recorded on the trackers.** Re-open this evaluation if any correction is reverted
or if either blocker resolution changes the design's seams.
