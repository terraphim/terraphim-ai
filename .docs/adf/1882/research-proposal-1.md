---
stage: research-proposal
issue: 1882
slot: 1
model: opus
provider: claude
timestamp: 2026-05-29T12:21:00+01:00
classification: needs-rescope
kls_scores:
  explicitity: 2
  external_consistency: 5
  internal_consistency: 4
  stakeholder_commitment: 4
  information_quality: 4
  overall_coherence: 4
---

## Issue Summary

Issue #1882 ("Project template + k=3 planning boosting (drift_check via grep/LSP/KG)") is a `type/initiative` that aims to define a reusable Terraphim-managed project template combining a `.terraphim/` configuration layout, k=3 committee-search boosting at the planning phase (research + design), k=1 implementation backed by a deterministic verification stack (drift_check, KG validation, LSP diagnostics, tests), and k=2 review consensus -- built from existing Terraphim crates rather than a new orchestration substrate. Verified directly against the tracker on 2026-05-29 at 12:21 BST: the issue body has length 0; the entire specification lives out-of-band in the already-**Approved** research document (`docs/research/research-adf-real-issue-processing-1882.md`) and a **Draft** design document (`docs/research/design-adf-real-issue-processing-1882.md`). All evidence below was re-verified by direct repository inspection rather than carried forward from prior slots.

## KLS Evaluation

### Explicitity: 2/5

The issue-as-stated is not self-contained. The Gitea API returns a body of length 0; a reader starting from the tracker receives only the title, four labels, and two comments (one a pickup note, one literally "dummy"). The full requirement -- directory layouts, `boosting.toml` schema, drift_check contracts, phase workflows, scope-in/scope-out tables -- exists, but in separate `docs/research/` artefacts, not in the issue.

I considered scoring this 3 on the grounds that the supporting artefacts are extraordinarily explicit and effectively "rescue" the requirement. I rejected that: the Explicitity dimension measures whether the specification *as stated* leaves things unambiguous, and an empty-bodied issue leaves everything implicit. Artefact quality belongs to External Consistency and Information Quality, not here. Two points rather than zero because the title is descriptive and the spec is discoverable for someone who knows where to look.

### External Consistency: 5/5

Verified by direct inspection. Every named crate and config artefact exists: `crates/terraphim_orchestrator/src/flow/` provides the k=3 substrate (matrix fan-out, gate evaluation, state -- `success_count`/`MatrixConfig` references confirmed in `executor.rs`, `config.rs`, `state.rs`); `crates/terraphim_lsp` exists (as a placeholder -- see Internal Consistency); `.terraphim/boosting.toml`, `.terraphim/flows/zdp-research.toml`, `.terraphim/flows/zdp-design.toml`, `.terraphim/flows/zdp-full.toml`, and `.terraphim/flows/adf-useful-work-proof.toml` all exist. The approved research doc correctly identifies the orchestrator flow engine as the natural k=3 substrate, explicitly avoiding a duplicate fan-out primitive in `terraphim_multi_agent`. The approach reuses existing infrastructure rather than inventing a parallel one. Strongest dimension.

### Internal Consistency: 4/5

The k=3-planning / k=1-implementation / k=2-review rationale is coherent: planning has weak local identifiability (boost it with multiple proposals + judge), while implementation can lean on deterministic verification (drift_check, KG validate, tests). The model roster in `boosting.toml` matches this shape (3 planning models + judge, 1 implementation model, 2 review models).

One verified contradiction remains: `.terraphim/boosting.toml` lines 53-55 declare `[verification.lsp_diagnostics]` with `on_error = "block"`, but `crates/terraphim_lsp/src/lib.rs` is, after its doc comment, the single line `// placeholder`. The config wires a *blocking* quality gate over a crate that cannot execute. The research doc lists LSP under deferred work, so the intent is acknowledged -- but the active config still promises an unrunnable gate. Localised and acknowledged, hence 4. There is also mild scope-mixing (reusable external template vs terraphim-ai's own ADF config), which the goals do not fully reconcile.

### Stakeholder Commitment: 4/5

Created by `root` (project owner) with `priority/P2-medium`, `status/in-progress`, `status/research`, `type/initiative`. Observable engagement is concrete: an active working branch (`task/1875-adf-ctl-local-direct-dispatch`) with in-progress changes to the flow executor, flow TOMLs, and `adf-issue-stage`; commits referencing the issue; an Approved research doc; a Draft design doc; and a populated `.docs/adf/1882/` evidence directory. Weakened by: no assignee, no milestone, two mutually-contradictory `status/*` labels (`research` + `in-progress`), and a low-signal "dummy" comment. Net 4 -- in-progress reality is stronger than the bare metadata suggests.

### Information Quality: 4/5

The technical evidence base is excellent and independently verifiable: the Approved research doc cites real files and components; the flow engine genuinely supports matrix fan-out and gate evaluation; the cost model (3 weak planning proposals + 1 judge < 1 Opus call, subscription-gated) is realistic and matches the roster. Docked one point for two gaps confirmed by inspection: (a) the issue body carries zero information, so all evidence is out-of-band; and (b) the verification implementation is partly aspirational -- `scripts/drift_check.sh`, `scripts/kg_verify.sh`, `scripts/lsp_verify.sh`, and `scripts/boost_plan.sh` return **no matches** on glob, and `terraphim_lsp` is a placeholder. Config exists; implementation lags.

### Overall Coherence: 4/5

The initiative hangs together conceptually and is well-grounded in existing, working infrastructure; the theory (committee-search boosting) matches the practice (a functioning DAG/matrix flow engine that produced these very proposals). The coherence gap is operational, not conceptual: #1882 is an empty-bodied umbrella that (1) conflates an approved, partially-built local-ADF pipeline (the #1875 line) with a not-yet-built reusable project template, and (2) carries config that outruns implementation (the LSP block rule). It coheres as a *direction*; it does not cohere as a single executable acceptance unit.

## Classification

**needs-rescope**

The initiative is valid in direction and architecturally aligned, but it must be reshaped before any further effort -- least of all more research -- is spent. Rationale: (1) the issue body is empty, so the requirement is not self-contained and must first be made state-aware in the tracker; (2) research and design are already complete and the research is **Approved**, so treating #1882 as fresh research duplicates finished effort; (3) the remaining work is heterogeneous (verification scripts, prompt templates, `terraphim_lsp` implement-or-defer, CI wiring, reusable scaffolding) and is not one coherent acceptance unit; (4) a concrete config/implementation drift (LSP blocking gate over a placeholder crate) requires an explicit decision.

I considered and rejected the alternatives. **`duplicate`** is about issue-to-issue tracker duplication; #1882 is not a copy of another issue (its entanglement with #1875 is a *relationship*, not duplication). The run-to-run duplication I observe is a property of the *process*, not the issue's tracker identity -- see Key Findings. **`blocked`** is wrong: the path forward is fully known and unobstructed; it is merely unexecuted. **`stale`** is wrong: active branch, commits, comments, and a live flow run show current work. **`valid`** would be wrong because the issue cannot be picked up and completed as a single unit in its current empty, heterogeneous form. Hence `needs-rescope`.

## Key Findings

- **The bottleneck has shifted from analysis to execution.** Unlike prior slots, this run can observe that research (3 converged proposals), design (`.docs/adf/1882/design-proposal-1.md`, which specifies a one-line `boosting.toml:55` fix plus five child issues B-F with acceptance criteria), and the quality evaluation (PASS at 4.5/5) are *all* complete and converged on `needs-rescope` -- yet **none of the recommended actions have been applied**: the body is still empty, the labels still contradict, and `boosting.toml:55` still reads `on_error = "block"`. Instead the flow is being re-run, producing this very proposal. **Further research runs are themselves the duplication prior slots warned against.** The correct next action is to execute the already-designed rescope, not to generate more analysis.
- **The issue body is empty (length 0), verified against the live tracker at 2026-05-29 12:21 BST.** The specification lives entirely in `docs/research/research-adf-real-issue-processing-1882.md` (Status: Approved, dated 2026-05-28).
- **The k=3 planning substrate exists and is real**, not aspirational: matrix fan-out / gate-evaluation / state code in `crates/terraphim_orchestrator/src/flow/{executor,config,state}.rs`, plus `.terraphim/boosting.toml` and the four `.terraphim/flows/*.toml` definitions.
- **Config/implementation drift confirmed**: `.terraphim/boosting.toml:53-55` sets `[verification.lsp_diagnostics] on_error = "block"`, but `crates/terraphim_lsp/src/lib.rs` is a literal `// placeholder`. The blocking LSP gate cannot execute.
- **Verification scripts are absent**: `scripts/drift_check.sh`, `scripts/kg_verify.sh`, `scripts/lsp_verify.sh`, and `scripts/boost_plan.sh` return no matches on glob, despite `boosting.toml` referencing `drift_check` and `kg_validate` in the implementation verification list.
- **#1882 is intertwined with #1875**: the active branch is `task/1875-adf-ctl-local-direct-dispatch`, and the Approved research doc covers #1875 and #1882 jointly -- the two are not cleanly separable as written.

## Recommendations

1. **Stop re-running the research/design flow on #1882.** The convergent finding is in; the design exists. Executing it is the work that remains. Re-generating proposals (including this one) is wasted spend and recreates the loop the prior slots flagged.
2. **Apply the one code change now** (already designed): `.terraphim/boosting.toml:55` `on_error = "block"` -> `on_error = "report"`, with a comment pointing at the `terraphim_lsp` placeholder and the future re-enable point. This eliminates the only live contradiction with a one-line, declarative edit.
3. **Populate the #1882 body** as a state-aware umbrella: one-paragraph direction, links to the Approved research and Draft design docs, a DONE list (flows, `adf-issue-stage`, `boosting.toml`, contracts, k=3 execution proof in `.docs/adf/1882/`), and a PENDING list linking child issues.
4. **Resolve the contradictory labels**: drop `status/research` (research is approved/done), keep `status/in-progress`; add an assignee/milestone reflecting the active branch.
5. **Create child issues with explicit acceptance criteria** per the design's B-F decomposition: `terraphim_lsp` diagnostics (re-enable point for block mode); verification scripts (`drift_check.sh`, `kg_verify.sh`, `lsp_verify.sh`); `.terraphim/prompts/` templates; CI wiring (depends on scripts); reusable external-project scaffolding (the "template for other repos" interpretation, explicitly distinguished from terraphim-ai's own `.terraphim/`).
6. **Do not regenerate the `docs/research/` artefacts** -- the Approved research and Draft design remain authoritative; proceed to disciplined-implementation for the child items.
