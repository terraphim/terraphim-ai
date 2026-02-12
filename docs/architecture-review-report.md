# Terraphim Architecture Review Report

**Date:** 2026-02-12  
**Reviewer mode:** Architecture-focused (Rust best practices, reuse, dependency minimization)  
**Scope:** Workspace architecture and core runtime path (`terraphim_server` → `terraphim_service` → middleware/persistence/config/types)

---

## 1) Executive Summary

Terraphim has a strong modular intent (many crates, feature flags, clear domain naming), but the current architecture is **overly coupled at runtime** and **inconsistent in dependency governance**. The main improvement opportunity is to move from “many crates” to “cohesive bounded modules with strict dependency direction”.

### Overall assessment
- **Strengths:** clear domain decomposition, good feature-flag presence, local-first support, reusable core data model.
- **Primary risks:** duplicated orchestration logic, dependency drift across crates, oversized “god modules”, and optional features not fully isolated.
- **Priority recommendation:** introduce explicit architecture boundaries and workspace-wide dependency policy enforcement.

---

## 2) What was reviewed

### Key files inspected
- `Cargo.toml` (workspace)
- `terraphim_server/Cargo.toml`, `terraphim_server/src/lib.rs`, `terraphim_server/src/main.rs`, `terraphim_server/src/api.rs`
- `crates/terraphim_service/Cargo.toml`, `crates/terraphim_service/src/lib.rs`
- `crates/terraphim_middleware/Cargo.toml`, `crates/terraphim_middleware/src/lib.rs`
- `crates/terraphim_persistence/Cargo.toml`, `crates/terraphim_persistence/src/lib.rs`
- `crates/terraphim_config/Cargo.toml`, `crates/terraphim_config/src/lib.rs`
- `crates/terraphim_types/Cargo.toml`, `crates/terraphim_types/src/lib.rs`
- `crates/terraphim_agent/Cargo.toml`, `crates/terraphim_cli/Cargo.toml`, `crates/terraphim_repl/Cargo.toml`
- Existing: `DEPENDENCY_MINIMIZATION_REPORT.md`

---

## 3) Architecture Findings

## 3.1 Positive findings

1. **Domain-oriented crate topology exists already**  
   Types/config/persistence/middleware/service/server are conceptually separated.

2. **Feature flags exist in key crates**  
   Example: server DB/LLM/schema/embedded-assets flags in `terraphim_server/Cargo.toml`.

3. **Central persistence abstraction is reusable**  
   `Persistable` in `crates/terraphim_persistence/src/lib.rs` is a good cross-cutting extension point.

4. **Core types are shared broadly**  
   `terraphim_types` acts as a canonical model crate used throughout the stack.

---

## 3.2 Critical architecture issues

1. **Server startup/orchestration logic is too heavy**  
   `terraphim_server/src/lib.rs` includes rolegraph building, document ingest/indexing, recursive file loading, persistence writes, and route registration in one place. This combines composition-root + runtime workflow + indexing concerns.

2. **Large service module with mixed responsibilities**  
   `crates/terraphim_service/src/lib.rs` contains search orchestration, KG preprocessing, LLM integration, queueing hooks, and document workflows with extensive branching. Reuse is reduced because concerns are not isolated.

3. **Duplicate route registration for prod and tests**  
   `terraphim_server/src/lib.rs` repeats large route blocks in both `axum_server` and `build_router_for_tests`, increasing drift risk.

4. **Config crate does too much**  
   `crates/terraphim_config/src/lib.rs` includes config schema/models, default role composition, persistence behavior, rolegraph bootstrap concerns. This violates single responsibility and blocks reuse in lightweight consumers.

5. **Optional functionality still pulls heavy dependencies**  
   `terraphim_server` directly depends on `terraphim_multi_agent`; VM execution behavior in `api.rs` strongly couples chat path to agent runtime concerns.

---

## 3.3 Dependency governance issues

1. **Workspace dependency policy is partial**  
   Root `Cargo.toml` defines workspace dependencies, but many crates still pin direct versions for shared crates (`tokio`, `serde`, `chrono`, `uuid`, `reqwest`, etc.), producing avoidable drift.

2. **Version skew evidence in resolved graph**  
   Duplicate families are visible in `cargo tree -d --workspace -e no-dev` output (examples include multiple versions of `rustyline`, `tokio-tungstenite`, `schemars`, `thiserror`, `reqwest`, `rustls`).

3. **Core types crate is heavier than needed**  
   `terraphim_types/Cargo.toml` includes dependencies that can be split by feature/profile (e.g., schema generation dependencies in non-schema consumers).

4. **Multiple CLI binaries with overlapping dependency footprints**  
   `terraphim_agent`, `terraphim_cli`, and `terraphim_repl` overlap in mission and dependency graph; this reduces reuse and increases maintenance and compile surface.

---

## 3.4 Reuse barriers

- Reusable components exist, but most are not packaged as thin, stable APIs.
- Bootstrap/indexing logic is embedded in executable crates instead of reusable runtime crates.
- Feature slices are mixed at module level rather than isolated behind trait-based interfaces.

---

## 4) Rust Architecture Best-Practice Gaps

- **Boundary control:** dependency direction should be strictly inward (app crates depend on domain crates, not vice versa).
- **Small crates, small APIs:** some crates are broad and mutable, reducing confidence and composability.
- **Feature purity:** optional capabilities should avoid linking heavy optional dependencies unless enabled.
- **Composition root discipline:** wiring and runtime orchestration should be thin and declarative.

---

## 5) Recommendation Summary (high impact)

1. **Define target layered architecture**
   - `core-model` (types-only)
   - `core-config` (pure config model + validation)
   - `infra-persistence` (opendal and profiles)
   - `domain-search` / `domain-kg` (rolegraph/thesaurus workflows)
   - `app-services` (orchestration)
   - `adapters` (axum CLI Tauri)

2. **Unify dependency policy**
   - Use workspace dependencies for all shared crates.
   - Add CI check that blocks direct version pinning for approved shared dependencies.

3. **Refactor large modules by capability**
   - Split `terraphim_service/src/lib.rs` into domain-oriented modules with trait interfaces.
   - Move server startup indexing/bootstrap into reusable runtime initializer crate/module.

4. **Consolidate CLI strategy**
   - Keep one primary terminal app plus thin mode/features, or keep separate binaries but with shared command runtime crate.

5. **Strengthen feature isolation**
   - Gate VM/multi-agent, schema, and other heavy integrations at crate/module boundaries with clear default-minimal profile.

---

## 6) Target Outcomes

If implemented, expected outcomes are:
- Lower incremental compile times and smaller default dependency closure.
- Better component reuse across server/CLI/desktop.
- Fewer architecture regressions (less route and flow duplication).
- Clearer contribution path for new maintainers.

---

## 7) Suggested ADRs to create next

1. ADR: Layered architecture and dependency direction rules.
2. ADR: Workspace dependency governance policy.
3. ADR: CLI product-line strategy (`agent`/`cli`/`repl`).
4. ADR: Feature-gating and optional integration boundaries.
5. ADR: Server composition root and runtime bootstrap extraction.
