---
stage: research-proposal
issue: 1882
slot: 3
model: openai/gpt-5.4
timestamp: 2026-05-28T16:34:00+01:00
classification: valid
---

## Issue Summary

Issue #1882 asks for a standard project template for Terraphim-managed projects that uses k=3 parallel planning proposals during research and design, then switches to k=1 implementation backed by verification tools such as drift checks, knowledge-graph validation, LSP diagnostics, and tests. The proposal also expects reusable project scaffolding under `.terraphim/`, executable helper scripts, CI wiring, and a way to persist planning outputs for later analysis.

## Current State

The codebase already contains several important parts of this idea, but mostly as local Terraphim AI infrastructure rather than as a reusable project template. `docs/taxonomy/routing_scenarios/adf/planning_tier.md` already defines the planning tier model roster, including `openai/gpt-5.4`. `.terraphim/boosting.toml` and `.terraphim/contracts/api.toml` already describe per-phase routing and example drift-check contracts. `.terraphim/adf.toml` and `.terraphim/flows/zdp-research.toml` already wire a k=3 research flow for this repository, and `crates/terraphim_orchestrator/src/flow/config.rs`, `flow/executor.rs`, and `flow/state.rs` already implement generic matrix fan-out, per-step agent execution, and aggregated matrix results. `crates/terraphim_multi_agent/src/pool.rs` and `crates/terraphim_persistence/src/lib.rs` provide pool management and storage primitives. What is still missing is the reusable template layer the issue describes: there is no `scripts/boost_plan.sh`, `scripts/drift_check.sh`, `scripts/lsp_verify.sh`, or `scripts/kg_verify.sh`; there is no project scaffold matching the directory layout in the issue; `.terraphim/boosting.toml` is present but not visibly consumed by the flow engine; and `crates/terraphim_lsp/src/lib.rs` is still a placeholder rather than an implementation backing contract-based diagnostics.

## Classification

**valid**

The issue is current, internally coherent, and aligned with code that already exists in the repository. The repository has enough substrate to justify the work: planning-tier routes exist, local ADF flows exist, matrix execution exists, and supporting crates are present. The remaining work is mainly integration and productisation of those pieces into a reusable project template. The issue is not stale, not obviously blocked, and not clearly a duplicate, although it would likely benefit from follow-up implementation issues split by scaffold, verification wrappers, and persistence/integration work.

## Key Findings

- The k=3 planning concept is not just theoretical in this repo: `.terraphim/flows/zdp-research.toml` already defines a three-slot research flow, and the orchestrator flow engine already supports matrix expansion and success-count gating.
- The proposed template is only partially realised today: configuration artefacts exist in `.terraphim/`, but the reusable scaffold, verification shell wrappers, and a concrete LSP-backed drift-check path are still missing.
- `terraphim_lsp` is currently a placeholder crate, which means the issue's desired `lsp.*` verification rules are not yet backed by a meaningful implementation.
- `terraphim_persistence` and the multi-agent pool provide usable primitives, but the current repository state does not show a dedicated path that stores or compares planning proposals as first-class template outputs.

## Recommendations

Proceed to design, but treat the work as an integration/template initiative rather than a greenfield architecture effort. The next phase should separate three concerns clearly: reusable project scaffolding, executable verification wrappers, and orchestration/persistence wiring for k=3 planning outputs. Before implementation starts, the design should also decide whether `.terraphim/boosting.toml` remains descriptive configuration or becomes an actively loaded control surface for the orchestrator, because that choice affects where the real source of truth lives.
