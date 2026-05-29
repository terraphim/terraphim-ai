---
stage: research-proposal
issue: 1882
slot: 3
model: openai/gpt-5.5
provider: openai
timestamp: 2026-05-29 12:08 BST
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

Issue #1882, titled "Project template + k=3 planning boosting (drift_check via grep/LSP/KG)", is an initiative for a reusable Terraphim project template and local ADF planning pipeline: k=3 committee-style research/design proposals, k=1 implementation backed by deterministic verification, and k=2 review consensus. The Gitea issue body is empty, so the actionable specification must be inferred from repository artefacts, especially `docs/research/research-adf-real-issue-processing-1882.md`, `docs/research/design-adf-real-issue-processing-1882.md`, `.terraphim/boosting.toml`, `.terraphim/flows/zdp-research.toml`, and the `.docs/adf/1882/` evidence files.

## KLS Evaluation

**Explicitity: 2/5**

The issue itself is not self-contained: the body returned by Gitea is empty, leaving only the title, labels, and timeline metadata. The supporting repository documents are much clearer and include phase rationale, flow shape, model rosters, artefact contracts, and deferred scope, but those artefacts are out-of-band from the issue. This makes requirements ambiguous for any agent or stakeholder starting from the tracker alone.

**External Consistency: 5/5**

The direction fits the existing architecture very well. The repository already contains `.terraphim/boosting.toml`, `.terraphim/flows/zdp-research.toml`, `.terraphim/flows/zdp-design.toml`, `.terraphim/flows/zdp-full.toml`, `.terraphim/contracts/api.toml`, `.terraphim/adf.toml`, and `.terraphim/bin/adf-issue-stage`. The approved research document correctly identifies the orchestrator flow engine and `MatrixConfig` as the natural substrate for k=3 planning, avoiding a duplicate fan-out mechanism in `terraphim_multi_agent`.

**Internal Consistency: 4/5**

The core logic is coherent: planning work benefits from multiple independently generated proposals, while implementation can be single-path if backed by tests, drift checks, KG validation, and review. The main internal weakness is scope mixing. The issue title points to a reusable project template, but the current artefacts also cover local ADF repair, useful-work proof, flow executor validation, verification scripts, prompt templates, CI wiring, and LSP deferment. These are related, but not one crisp acceptance unit.

**Stakeholder Commitment: 4/5**

The issue was opened by the project owner and carries `priority/P2-medium`, `status/in-progress`, `status/research`, and `type/initiative` labels. There are two comments, approved research, draft design, and local ADF evidence files, which shows active commitment. The score is not 5 because the issue has no assignee or milestone, and the empty body plus mixed `status/research`/`status/in-progress` labels make the current decision state unclear.

**Information Quality: 4/5**

The evidence base in the repository is solid: the approved research document maps existing components, names constraints, states assumptions, and identifies deferred items; `.docs/adf/1882/adf-flow-verification-validation.md` records a successful deterministic proof flow with matrix success count 3 and flow executor test evidence; `.terraphim/boosting.toml` and `zdp-research.toml` show the intended operational configuration. Information quality is reduced by the empty issue body and by partial implementation gaps, especially the placeholder `terraphim_lsp` crate and absent verification scripts such as `scripts/drift_check.sh`, `scripts/kg_verify.sh`, and `scripts/lsp_verify.sh`.

**Overall Coherence: 4/5**

The work hangs together as a credible Terraphim ADF evolution: use existing flow definitions and configuration to make planning more reliable, then verify implementation with deterministic checks. It is coherent as an architectural direction and partially validated as a local flow. It is less coherent as a single issue because the tracker does not state completion criteria and the remaining work spans several separable streams.

## Classification

**needs-rescope**

The initiative is valid, active, and architecturally aligned, but it should be reshaped before further implementation. The issue body is empty, the approved research/design artefacts are already ahead of the tracker, and the remaining work is heterogeneous: template packaging, verification scripts, prompt templates, CI integration, LSP implementation or explicit deferral, and flow synthesis/checkpoint behaviour. This is not stale because current artefacts and labels show active work; it is not a duplicate based on the available evidence; and it is not blocked because the next actions are known. It needs rescope into a state-aware umbrella plus child issues with explicit acceptance criteria.

## Key Findings

- The Gitea issue body for #1882 is empty, so requirements are not unambiguous at the source of truth.
- The k=3 planning substrate is already present in repository configuration, including `.terraphim/boosting.toml` and `.terraphim/flows/zdp-research.toml`.
- Local deterministic flow evidence exists in `.docs/adf/1882/adf-flow-verification-validation.md`, including matrix execution, aggregate substitution, numeric gate evaluation, artefact production, and targeted test evidence.
- `terraphim_lsp` remains a placeholder, while `.terraphim/boosting.toml` still declares LSP diagnostics with `on_error = "block"`; this creates config/implementation drift.
- Verification shell scripts requested by the initiative are not present under `scripts/`, so drift/KG/LSP verification remains partly aspirational.

## Recommendations

Update #1882 to be a state-aware umbrella issue that links the approved research and draft design documents, records completed flow proof work, and lists only remaining outcomes. Split follow-up work into child issues for verification scripts, prompt templates, CI integration, reusable template scaffolding, and either implementing `terraphim_lsp` diagnostics or downgrading LSP verification to report-only until the crate is real. Do not spend more effort re-researching the general direction; proceed to focused design/implementation tasks after the rescope is reflected in Gitea.
