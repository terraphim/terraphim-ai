---
stage: research-proposal
issue: 1882
slot: 2
model: kimi-for-coding/k2p6
timestamp: 2026-05-28T14:30:00Z
classification: valid
---

## Issue Summary

Issue #1882 proposes a standardised project template for terraphim-ai-managed projects that applies k=3 boosting (parallel proposals + judge-comparison) during the planning phase (research/design), while using k=1 + strong verification (drift_check, LSP, KG, tests) during implementation. The template leverages existing workspace crates (`terraphim_grep`, `terraphim_multi_agent`, `terraphim_orchestrator`, `terraphim_router`, `terraphim_lsp`, `terraphim_kg_linter`, `terraphim_persistence`, `terraphim_validation`) and defines directory layouts, TOML schemas, shell scripts, and CI integration.

## Current State

### Already Implemented (Substantial)

| Component | Status | Location | Notes |
|-----------|--------|----------|-------|
| `boosting.toml` schema | **Exists** | `.terraphim/boosting.toml` | Full per-phase config with planning (k=3), implementation (k=1), review (k=2), and verification stack |
| Drift_check contracts | **Exists** | `.terraphim/contracts/api.toml` | Three rule kinds: `regex.pattern_match`, `kg.concept_required`, `lsp.diagnostic_clean` |
| Planning tier routing | **Exists** | `docs/taxonomy/routing_scenarios/adf/planning_tier.md` | 5 model routes including anthropic/opus, kimi/k2p6, openai/gpt-5.4/5.5, zai-coding-plan |
| Multi-agent pool | **Exists** | `crates/terraphim_multi_agent/src/pool.rs` | PoolConfig with min/max size, load balancing, warming |
| Persistence layer | **Exists** | `crates/terraphim_persistence/src/lib.rs` | DeviceStorage with OpenDAL operators, compression, schema evolution |
| All crates | **Exist** | `crates/` directory | All 9 mentioned crates present in workspace |

### Missing / Not Yet Implemented

| Component | Status | Gap |
|-----------|--------|-----|
| `scripts/drift_check.sh` | Missing | No shell script invoking `terraphim_grep` + KG validation |
| `scripts/lsp_verify.sh` | Missing | No LSP diagnostics runner |
| `scripts/kg_verify.sh` | Missing | No `terraphim-agent validate` wrapper |
| `scripts/boost_plan.sh` | Missing | No k=3 parallel dispatcher via `terraphim_multi_agent` |
| `.terraphim/prompts/` | Missing | No phase-specific prompt templates (research.md, design.md, implement.md, judge-planning.md) |
| `.terraphim/kg/concepts.md` | Missing | No Aho-Corasick thesaurus source for domain concepts |
| `.terraphim/kg/invariants.md` | Missing | No PRE/POST/INV per role |
| `.terraphim/kg/routing.md` | Missing | No task -> tier mapping documentation |
| Project template scaffold | Missing | No `cargo generate` or `git clone` template for new projects |
| `docs/research/<task>/` | Missing | No structured output directories for proposals + synthesis |
| `docs/design/<task>/` | Missing | No design proposal directories |
| `.gitea/workflows/ci.yml` | Missing | No CI pipeline for drift_check + LSP + tests |
| `terraphim_persistence` proposal storage | Not wired | Persistence exists but not used for storing k=3 proposals/verdicts |
| k=3 dispatch in multi_agent | Not wired | Pool exists but no `boost_plan.sh` or equivalent dispatch logic |

## Classification

**Valid** -- The issue describes a coherent architecture that builds on existing capabilities. The core configuration (`boosting.toml`, contracts, planning tier) is already in place, which de-risks the implementation. What remains is:

1. Shell script wrappers around existing crates
2. Project template scaffolding (TOML + directory layout)
3. CI integration
4. Wiring `terraphim_multi_agent` pool to execute k=3 parallel proposals
5. Using `terraphim_persistence` to store proposals for blind-spot mining

The issue is **not stale** (created today), **not a duplicate** (no other issue covers project template + k=3 integration), and **not blocked** (all dependencies exist). It could benefit from **rescoping** into 3-4 smaller deliverable chunks.

## Key Findings

1. **Configuration-complete, integration-incomplete**: `.terraphim/boosting.toml` and `.terraphim/contracts/api.toml` are fully specified and match the issue's design almost exactly. The gap is entirely in executable scripts and template scaffolding.

2. **terraphim_multi_agent pool is ready**: `PoolConfig` supports min/max size, load balancing, and warming. The pool manager uses `terraphim_persistence::DeviceStorage`. However, there is no workflow that dispatches 3 parallel planning agents and aggregates results.

3. **terraphim_persistence is under-utilised**: The crate provides a sophisticated storage abstraction (OpenDAL, compression, schema evolution) but is only used for conversation/memory storage. The issue's 90-day goal explicitly targets using it for proposal/verdict persistence.

4. **No k=3 code paths exist**: A grep across all Rust source for "boost_plan", "drift_check", "parallel_proposal", or "k=3" returns zero results. This is purely a wiring/integration task, not a fundamental algorithmic one.

5. **terraphim_lsp and terraphim_kg_linter are minimal**: Both crates have `lib.rs` only. They may need expansion to support the rule kinds defined in `contracts/api.toml` (`lsp.diagnostic_clean`, `kg.concept_required`).

## Recommendations

1. **Rescope into 3 sub-issues**:
   - **Issue A**: Create shell scripts (`drift_check.sh`, `lsp_verify.sh`, `kg_verify.sh`, `boost_plan.sh`) that wrap existing crates
   - **Issue B**: Create project template directory layout + `cargo generate` template
   - **Issue C**: Wire `terraphim_multi_agent` to execute k=3 planning dispatch and persist results via `terraphim_persistence`

2. **Proceed with Issue A first** -- it has the highest PageRank impact because the scripts unblock manual testing of the entire verification stack.

3. **Defer CI integration** (`.gitea/workflows/ci.yml`) until scripts are proven locally.

4. **Consider terraphim_lsp / terraphim_kg_linter expansion** as a separate research spike before committing to the contract schema.
