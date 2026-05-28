---
stage: research-synthesis
issue: 1882
timestamp: 2026-05-28T16:36:59Z
classification: valid
---

## Final Classification

**Valid**

All available proposals agree on this classification. The issue describes a coherent, integration-heavy initiative that builds on substantial existing infrastructure. The repository already contains the core configuration (`boosting.toml`, `contracts/api.toml`), planning-tier routing, multi-agent pool primitives, persistence layer, and orchestrator flow engine with matrix fan-out support. What remains is primarily wiring, scaffolding, and productisation rather than fundamental architectural work.

## Synthesis of Findings

### Agreements

1. **Substantial substrate already exists**: `.terraphim/boosting.toml`, `.terraphim/contracts/api.toml`, `docs/taxonomy/routing_scenarios/adf/planning_tier.md`, `crates/terraphim_multi_agent/src/pool.rs`, and `crates/terraphim_persistence/src/lib.rs` are all present and aligned with the issue's goals.

2. **Missing integration layer**: No shell scripts (`drift_check.sh`, `lsp_verify.sh`, `kg_verify.sh`, `boost_plan.sh`), no project template scaffold, no dedicated proposal storage path, and no CI wiring exist yet.

3. **terraphim_lsp is a placeholder**: The crate has only a minimal `lib.rs`, meaning `lsp.diagnostic_clean` contract rules are not yet backed by a real implementation.

4. **Not stale, blocked, or duplicate**: The issue is current, internally coherent, and not duplicated by any other issue.

5. **Should be rescoped into smaller deliverables**: All proposals recommend splitting the work into 3-4 focused sub-issues (scripts, template, wiring, CI).

### Differences

- **Proposal 2** emphasises the `terraphim_multi_agent` pool's readiness and `terraphim_persistence`'s under-utilisation, providing a detailed gap analysis table.
- **Proposal 3** highlights that k=3 planning is *already operational* in this repository via `.terraphim/flows/zdp-research.toml` and the orchestrator's matrix expansion / success-count gating, which de-risks the template effort further.
- **Proposal 3** also raises an important design question: whether `.terraphim/boosting.toml` should remain descriptive configuration or become an actively loaded control surface for the orchestrator.

## Strongest Proposal

**Proposal 3** (openai/gpt-5.4) is the strongest. It correctly identifies that k=3 research flows are *already* working in this repository (`.terraphim/flows/zdp-research.toml`), which fundamentally changes the risk profile from "build new capability" to "productise existing capability". It also surfaces the critical design decision about `boosting.toml`'s role as passive config vs active control surface -- a choice that will affect architecture for all downstream work.

**Proposal 2** (kimi-for-coding/k2p6) is the most actionable for immediate next steps, with its precise gap table and explicit recommendation to start with shell scripts (Issue A) for highest PageRank impact.

## Recommendations

1. **Proceed to design**, treating this as integration/template work rather than greenfield architecture.

2. **Adopt Proposal 2's rescoping** into 3 sub-issues:
   - **Sub-issue A**: Shell script wrappers (`drift_check.sh`, `lsp_verify.sh`, `kg_verify.sh`, `boost_plan.sh`)
   - **Sub-issue B**: Project template directory layout + scaffold
   - **Sub-issue C**: Wire orchestrator to consume `.terraphim/boosting.toml` and persist k=3 planning outputs via `terraphim_persistence`

3. **Address Proposal 3's design question first**: Decide whether `boosting.toml` is loaded at runtime by the orchestrator or remains a human-readable specification. This decision gates Sub-issue C.

4. **Defer CI integration** until scripts are proven locally.

5. **Flag `terraphim_lsp` expansion** as a separate research spike before committing to the full contract schema -- the placeholder crate cannot yet back `lsp.*` verification rules.
