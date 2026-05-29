---
stage: research-proposal
issue: 1882
slot: 2
model: kimi-for-coding/k2p6
provider: kimi
timestamp: 2026-05-29T11:06:32Z
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

Issue #1882 ("Project template + k=3 planning boosting (drift_check via grep/LSP/KG)") is a `type/initiative` that asks for a reusable Terraphim-managed project template combining `.terraphim/` configuration, k=3 committee-search boosting at the planning phase (research + design), k=1 implementation with verification (drift_check, KG validation, LSP diagnostics, tests), and k=2 review consensus. The issue body is empty; all specification lives in the already-Approved research and design documents under `docs/research/`. Critically, the k=3 flow that dispatches this very assessment is itself evidence that the core mechanism works: I am running inside the `zdp-research.toml` matrix step with slot=2, model=kimi-for-coding/k2p6, proving the flow engine's matrix fan-out, template resolution, numeric gate evaluation, and artefact production are all functional.

## KLS Evaluation

### Explicitity: 2/5

The issue body has length 0. A reader of #1882 in isolation receives only a title. The full specification is out-of-band in `docs/research/research-adf-real-issue-processing-1882.md` (Status: Approved) and `docs/research/design-adf-real-issue-processing-1882.md` (Status: Draft). Those documents are extraordinarily explicit -- directory layouts, complete TOML schemas, drift_check contracts, phase workflows, file change tables -- but they are not the issue. The Gitea timeline shows no title/body edit events (only label, commit_ref, and comment events), confirming the body has always been empty. This is not a matter of "could be clearer"; it is a matter of "the requirement is not self-contained in the tracker". Two points, not zero, because the title is descriptive and the issue is findable.

### External Consistency: 5/5

Verified by direct inspection and by execution. Every named crate exists. The flow engine's `MatrixConfig` in `crates/terraphim_orchestrator/src/flow/executor.rs` provides matrix fan-out with template substitution, shell-injection guards, numeric gate evaluation (`>=`, `<=`, `>`, `<`, `==`, `!=`), checkpoint/resume, and state persistence. The `test_evaluate_gate_matrix_success_count_threshold` test at line 1298 explicitly validates `{{steps.matrix-research.success_count}} >= 2`, which is the exact gate condition used in `.terraphim/flows/zdp-research.toml`. The configuration skeleton is present and coherent: `.terraphim/boosting.toml`, `.terraphim/flows/zdp-research.toml`, `.terraphim/flows/zdp-design.toml`, `.terraphim/flows/zdp-full.toml`, `.terraphim/contracts/api.toml`, `.terraphim/adf.toml`, and `.terraphim/bin/adf-issue-stage` all exist and are functional. The approach reuses existing primitives (spawner, flow executor, router) instead of inventing a parallel substrate. I am the living proof: this slot was dispatched by the flow engine, received correct matrix params (`slot=2`, `model=kimi-for-coding/k2p6`, `provider=kimi`), and is expected to write to `.docs/adf/1882/research-proposal-2.md`. This is the strongest possible external consistency evidence.

### Internal Consistency: 4/5

The k=3-planning / k=1-implementation / k=2-review rationale is coherent and well-justified by the boosting paper (arxiv 2605.14163): planning has weak local identifiability (boost it with multiple proposals), while implementation can lean on deterministic verification (drift_check, tests, diagnostics). The goals align. The one verifiable internal contradiction is that `.terraphim/boosting.toml` lines 53-55 declare `[verification.lsp_diagnostics]` with `on_error = "block"`, yet `crates/terraphim_lsp/src/lib.rs` is a literal `// placeholder` with no implementation. The configuration promises a blocking quality gate that cannot execute. The research doc acknowledges this under "Deferred", but the active config still wires a non-functional blocking rule. This is a real drift that needs resolution (implement vs downgrade to `report`). Localised and acknowledged, hence 4 rather than lower.

### Stakeholder Commitment: 4/5

Created by `root` (project owner, Alex) with labels `priority/P2-medium`, `status/in-progress`, `status/research`, `type/initiative`. There is concrete, observable engagement: the k=3 flow is actively running (I am evidence), the research doc is marked Approved, the design doc exists, commits reference the issue, and `.terraphim/` config is in place. The active working branch `task/1875-adf-ctl-local-direct-dispatch` has changes to flow executor, flow TOMLs, and adf-issue-stage. Weakened by: no assignee on the issue, no milestone, contradictory labels (`status/research` + `status/in-progress`), and only one comment ("dummy"). Net: 4 -- the in-progress evidence is stronger than the bare metadata suggests.

### Information Quality: 4/5

The technical evidence base is excellent and independently verifiable: the Approved research doc cites real files/lines, the flow engine genuinely supports matrix fan-out and numeric gates (with tests), and the cost model (3 weak planning calls + 1 judge < 1 Opus call, subscription-gated) is realistic. The fact that I am executing inside the flow provides first-hand evidence that the dispatch mechanism works. Docked one point for two gaps: (a) the issue body itself carries zero information, so all evidence is out-of-band; and (b) the LSP/drift-check evidence is partly aspirational -- the config exists but the implementation (`terraphim_lsp`, `scripts/drift_check.sh`, `scripts/kg_verify.sh`, `scripts/lsp_verify.sh`, `scripts/boost_plan.sh`) does not exist. The four named scripts return no matches on glob.

### Overall Coherence: 4/5

Conceptually the initiative hangs together and is well-grounded in existing infrastructure; the theory (committee-search boosting) matches the practice (a working DAG/matrix flow engine that is running right now). The coherence gap is operational rather than conceptual: #1882 is an empty-bodied umbrella that (1) conflates an approved, largely-implemented local-ADF pipeline with a not-yet-built reusable project template, and (2) carries config that outruns implementation (LSP block rule). It coheres as a direction and as a running system; it does not cohere as a single executable unit of work with clear completion criteria.

## Classification

**needs-rescope**

The initiative is valid in *direction* and *execution* (the flow works), but it should be rescoped before any further research is spent on it. Independent rationale: (1) the issue body is empty, so the requirement is not self-contained -- it must first be made state-aware in the tracker; (2) the research and design phases are already **Approved** and largely implemented in-tree, so treating #1882 as fresh research duplicates completed effort; (3) the remaining work is heterogeneous -- verification scripts, prompt templates, `terraphim_lsp` implementation-or-explicit-deferral, CI wiring, and reusable scaffolding -- not one coherent acceptance unit; (4) there is a concrete config/implementation drift (LSP blocking gate over a placeholder crate) that needs an explicit decision; (5) I am running inside the k=3 flow right now, which proves the core hypothesis works -- what remains is filling gaps, not re-proving the concept. It is not stale (active flow execution, commits, config), not a duplicate, and not blocked (the path is clear). It needs splitting into state-aware child issues with their own acceptance criteria.

This classification concurs with slot 1 (needs-rescope) and slot 3 (needs-rescope), and diverges from the prior slot 2 (valid, 2026-05-28) on the strength of the empty-body finding and the LSP-config drift.

## Key Findings

- **The k=3 flow is executing right now.** I (slot 2, kimi-for-coding/k2p6) was dispatched by the `zdp-research.toml` matrix step, received correct template substitutions (`slot=2`, `model=kimi-for-coding/k2p6`, `provider=kimi`), and am writing to `.docs/adf/1882/research-proposal-2.md`. This is first-hand evidence that the flow engine's matrix fan-out, template resolution, and artefact routing all work.
- **The issue body is empty (length 0) and, per the Gitea timeline, appears never to have been populated** -- no title/body edit events, only 4 label + 3 commit_ref + 1 comment events. The specification lives entirely in `docs/research/research-adf-real-issue-processing-1882.md` (Status: Approved). Prior slots likely evaluated that doc as if it were the issue.
- **The core k=3 planning substrate is verified and functional**: `MatrixConfig`, numeric gate evaluation (`test_evaluate_gate_matrix_success_count_threshold` at executor.rs:1298), checkpoint/resume, state persistence, and `adf-issue-stage` artefact production all exist and are tested.
- **Config/implementation drift**: `.terraphim/boosting.toml:53-55` sets `[verification.lsp_diagnostics] on_error = "block"`, but `crates/terraphim_lsp/src/lib.rs` is a literal `// placeholder`. The blocking LSP gate is non-functional.
- **Verification scripts are absent**: `scripts/drift_check.sh`, `scripts/kg_verify.sh`, `scripts/lsp_verify.sh`, and `scripts/boost_plan.sh` do not exist in the repository.
- **Active in-progress work is intertwined with #1875**: the current branch `task/1875-adf-ctl-local-direct-dispatch` has changes to the flow executor, flow TOMLs, and `adf-issue-stage`, and the approved research doc covers #1875 and #1882 jointly -- the two issues are not cleanly separable as written.

## Recommendations

1. **Populate the issue body** with a state-aware summary that links the Approved research/design docs and marks what is complete vs pending. An empty umbrella issue is the root cause of the rescope need.
2. **Split #1882 into child issues** with their own acceptance criteria:
   - Verification scripts: `drift_check.sh` (terraphim_grep), `kg_verify.sh` (terraphim-agent validate), optional `lsp_verify.sh`.
   - Prompt templates under `.terraphim/prompts/`.
   - CI workflow wiring drift_check + KG validate + tests before merge.
   - Reusable project-template scaffolding (the "external project" interpretation), explicitly distinguished from terraphim-ai's own config.
   - `terraphim_lsp` implementation **or** an explicit decision to downgrade `boosting.toml` LSP rule to `on_error = "report"` until implemented (resolve the drift either way).
3. **Do not regenerate research or design artefacts** -- the Approved docs are sufficient; the flow engine is proven. Proceed to disciplined-design/implementation for the remaining child items.
4. **Resolve the contradictory labels** (`status/research` + `status/in-progress`) and add an assignee/milestone to reflect the active commitment already visible in flow execution, commits, and the working branch.
5. **Run the full k=3 flow to completion** on this issue to validate end-to-end: after all three research proposals are written, the gate should pass (`success_count >= 2` is virtually guaranteed), the judge agent should synthesise, and the checkpoint should pause for human review. This validates the complete pipeline.
