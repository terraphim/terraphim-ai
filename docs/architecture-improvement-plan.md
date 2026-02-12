# Terraphim Architecture Improvement Plan

**Date:** 2026-02-12  
**Related report:** `docs/architecture-review-report.md`

---

## Goal

Improve architectural reuse and reduce dependency surface while preserving existing functionality and release velocity.

---

## Guiding Principles

1. **Default-minimal build first** (opt-in heavy features).
2. **Stable reuse interfaces** (traits + thin public APIs).
3. **Strict dependency direction** (core -> domain -> app -> adapters).
4. **Incremental, reversible steps** (small PRs, clear acceptance criteria).

---

## Phase 0 — Baseline & Governance (1 week)

## Deliverables
- ADR for architecture layering and dependency direction.
- ADR for workspace dependency policy.
- Dependency baseline snapshot (duplicate crates + compile timings for key binaries).

## Actions
- Define allowed shared dependencies that must be `workspace = true`.
- Add CI guard: fail when core shared deps are version-pinned in member crates.
- Establish “minimal profile” commands as quality gate.

## Exit criteria
- Policy documented + enforced in CI.
- Baseline metrics stored in docs for before/after comparison.

---

## Phase 1 — Boundary Extraction for Reuse (2–3 weeks)

## Deliverables
- Extracted runtime bootstrap module/crate for rolegraph + indexing initialization.
- Shared router builder to remove duplicated route registrations.

## Actions
1. Extract server startup indexing logic from `terraphim_server/src/lib.rs` into a reusable initializer component.
2. Create a single route composition function used by both production and test router constructors.
3. Keep `main.rs` and server entrypoints as thin composition roots.

## Exit criteria
- No duplicate route lists in server code.
- Startup/indexing orchestration reusable from tests and other adapters.

---

## Phase 2 — Service Decomposition (3–4 weeks)

## Deliverables
- `terraphim_service` split by capability modules with clear traits:
  - search service
  - KG/thesaurus service
  - document service
  - LLM/chat service
- Reduced size/complexity of monolithic `lib.rs`.

## Actions
- Move long branching workflows (e.g., thesaurus load/build fallback chains) into strategy objects.
- Introduce typed service interfaces for easier integration testing and reuse.
- Keep crate facade stable; evolve internals behind module boundaries.

## Exit criteria
- `lib.rs` acts mostly as facade/wiring.
- Each service capability has isolated tests and explicit dependencies.

---

## Phase 3 — Dependency Surface Reduction (2–3 weeks)

## Deliverables
- All common dependencies unified via workspace policy.
- Feature-gated heavy integrations isolated by default.
- Core-model dependency trim proposal implemented (as feasible).

## Actions
1. Normalize shared deps (`tokio`, `serde`, `serde_json`, `reqwest`, `chrono`, `uuid`, etc.) to workspace definitions.
2. Audit and isolate heavy/optional paths (VM execution, multi-agent, schema generation, desktop-only behaviors).
3. Review `terraphim_types` and `terraphim_config` for feature/profile splits:
   - `types-core` / `types-schema` style split (or equivalent feature split)
   - pure config-model vs runtime bootstrap separation

## Exit criteria
- Minimal server/CLI build avoids linking optional heavy stacks.
- Duplicate dependency count decreased from baseline.

---

## Phase 4 — CLI Product-Line Consolidation (2 weeks)

## Deliverables
- ADR for `terraphim_agent` / `terraphim_cli` / `terraphim_repl` strategy.
- Shared command runtime crate used by all retained binaries.

## Actions
- Decide between:
  - single primary binary with feature modes, or
  - multiple binaries sharing one command execution core.
- Remove duplicate runtime wiring and duplicated command logic.

## Exit criteria
- Clear ownership and purpose per binary.
- Reduced maintenance and compile duplication.

---

## Quality Gates per Phase

For each phase/PR:
- `cargo build --workspace`
- `cargo clippy`
- `cargo test --workspace` (or targeted crates when scoped)
- `ubs <changed-files>`
- dependency diff evidence attached in PR notes

---

## Risk Register

1. **Risk:** Refactoring disrupts existing API behavior.  
   **Mitigation:** preserve public facades; add compatibility tests before moving internals.

2. **Risk:** Feature gating breaks downstream packaging.  
   **Mitigation:** validate minimal + full feature matrices in CI.

3. **Risk:** Large PRs slow reviews.  
   **Mitigation:** enforce phase-based, small, single-purpose PRs.

---

## Success Metrics

- Fewer duplicated crates in `cargo tree -d` for core targets.
- Reduced compile times for default server and CLI builds.
- Reduced LOC in composition roots (`main.rs`, server `lib.rs`) and monolithic services.
- Clearer module ownership documented via ADRs and crate READMEs.

---

## Immediate Next Steps (this week)

1. Approve ADR templates and create ADR-001/002 (layering + dependency policy).  
2. Implement CI guard for workspace dependency consistency.  
3. Open issue set for Phase 1 extraction work (router unification + bootstrap extraction).  
4. Track baseline metrics in `docs/` for objective before/after comparison.
