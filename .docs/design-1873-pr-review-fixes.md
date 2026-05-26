# Implementation Plan: PR Review Fixes for FffIndexer Migration

**Status**: Draft  
**Research Doc**: `.docs/research-1873-pr-review-fixes.md`  
**Author**: AI Agent (Phase 2 Disciplined Design)  
**Date**: 2026-05-25  
**Estimated Effort**: 3-5 hours  
**Target PR**: #1874

## Overview

### Summary

This plan fixes the structured review findings on PR #1874. It keeps the existing architecture and `IndexMiddleware` trait intact while making the `FffIndexer` implementation honour configured instance state, support explicit source-code file filters, and prove both behaviours with tests.

### Approach

Use a minimal repair approach:

1. Keep `FffIndexer` as the active `ServiceType::Ripgrep` implementation.
2. Preserve cache for fully default stateless indexing only.
3. Bypass cache when `kg_scorer` or frecency state is present, so configured state is not discarded.
4. Introduce a small, testable helper that derives allowed file extensions from `Haystack.extra_parameters`.
5. Set code haystacks to opt into `.rs` explicitly through `.terraphim/role-rust-engineer.json` and related role configs when appropriate.
6. Commit the intended `Cargo.lock` update.

### Scope

**In Scope:**
- Fix cache wrapper bypass of configured `self` state.
- Add configurable extension filtering for FffIndexer.
- Add tests for `.rs` indexing and KG scorer path activation.
- Update `.terraphim` role configs to mark code haystacks explicitly.
- Commit `Cargo.lock` dependency alignment update.
- Update verification notes after fixes.

**Out of Scope:**
- Removing `RipgrepIndexer`.
- Redesigning `IndexMiddleware`.
- Adding new `ServiceType::Fff`.
- Changing `terraphim_service` write-back path unless implementation time reveals a direct compile/runtime issue.
- Implementing ordered search result APIs.

**Avoid At All Cost:**
- Broad default search over all text files for every legacy `Ripgrep` haystack.
- Introducing a cache key that serialises scorer internals.
- Adding test-only hooks to production APIs.
- Rewriting fff-search integration outside the reviewed files.
- Staging unrelated formatting or generated learning files.

## Architecture

### Component Diagram

```text
SearchQuery
  -> search_haystacks()
    -> FffIndexer::default()
      -> index()
        -> if stateless: cached_fff_index()
        -> if stateful: self.index_inner()
          -> FilePicker::collect_files()
          -> allowed_extensions(haystack)
          -> filter files
          -> apply KG scorer/frecency when present
          -> grep_search()
          -> Index<Document>
```

### Data Flow

```text
Haystack.extra_parameters
  -> allowed_extensions()
  -> file.relative_path extension filter
  -> grep_search(files)
  -> Document { id, title, url, body, description }
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Cache only stateless indexer calls | Avoids discarding configured KG/frecency state while preserving existing speed path. | Include scorer/frecency in cache key; too complex and fragile. |
| Preserve markdown-only default | Maintains RipgrepIndexer parity for existing docs haystacks. | Search all files by default; too broad. |
| Use `extra_parameters.extensions` for source opt-in | Simple, explicit, testable. | Infer from path name like `crates`; too magical. |
| Add helper for extension filtering | Makes behaviour independently testable. | Inline closure only; harder to test. |
| Update role JSON code haystacks | Required for real `.terraphim` acceptance path. | Rely on default broad search; rejected. |

### Eliminated Options (Essentialism)

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| Trait redesign to pass role/thesaurus | Not needed for review fixes. | Large blast radius across indexers. |
| New service type | Requires config/schema migration. | Delays PR and increases compatibility risk. |
| Full frecency persistence tests | Requires LMDB setup and temp DB lifecycle. | More moving parts than needed for P1 fixes. |
| Ordered `Index` API | Outside current data model. | Bigger semantic change. |

### Simplicity Check

The simplest design is to keep the existing indexer and add two local decisions: whether this call can use the stateless cache, and which extensions this haystack permits. This solves both P1s without changing public traits or adding new service types.

**Senior Engineer Test**: The design is not overcomplicated; it is a contained repair of two incorrect assumptions.

**Nothing Speculative Checklist:**
- [x] No new service type.
- [x] No trait redesign.
- [x] No removal of old indexer.
- [x] No generic plugin/filter framework.
- [x] Tests cover the exact review failures.

## File Changes

### New Files

| File | Purpose |
|------|---------|
| None | No new production files required. |

### Modified Files

| File | Changes |
|------|---------|
| `crates/terraphim_middleware/src/indexer/fff.rs` | Fix cache/state path, add extension filter helper, use helper in `index_inner`. |
| `crates/terraphim_middleware/tests/fff_indexer.rs` | Add tests for `.rs` indexing and KG scorer activation/state preservation. |
| `.terraphim/role-rust-engineer.json` | Add explicit source-code extension filter for `crates` haystack. |
| `.terraphim/role-ai-engineer.json` | Add explicit source-code extension filter for `crates` haystack if intended. |
| `.terraphim/role-devops.json` | Add explicit extension filter only if this role expects code search. |
| `Cargo.lock` | Commit Cargo-produced fff-search alignment. |
| `.docs/verification-1873-traceability-matrix.md` | Update after fixes. |
| `.docs/validation-1873-final-report.md` | Update acceptance evidence after fixes. |

### Deleted Files

| File | Reason |
|------|--------|
| None | No deletion in review-fix pass. |

## API Design

### FffIndexer State Predicate

```rust
impl FffIndexer {
    fn is_stateful(&self) -> bool {
        self.kg_scorer.is_some() || self.frecency.is_some()
    }
}
```

### Index Implementation

```rust
impl IndexMiddleware for FffIndexer {
    async fn index(&self, needle: &str, haystack: &Haystack) -> Result<Index> {
        if self.is_stateful() {
            self.index_inner(needle, haystack).await
        } else {
            cached_fff_index(needle, haystack).await
        }
    }
}
```

### Extension Filter Helper

```rust
impl FffIndexer {
    fn allowed_extensions(haystack: &Haystack) -> Vec<String> {
        let params = haystack.get_extra_parameters();
        if let Some(value) = params.get("extensions") {
            return parse_extensions(value);
        }
        if let Some(value) = params.get("extension") {
            return parse_extensions(value);
        }
        if params.get("type").is_some_and(|v| v == "markdown") {
            return vec!["md".to_string(), "markdown".to_string()];
        }
        vec!["md".to_string()]
    }

    fn file_extension_allowed(relative_path: &str, allowed: &[String]) -> bool {
        Path::new(relative_path)
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| allowed.iter().any(|allowed| allowed == ext))
    }
}
```

### Role JSON Shape

```json
{
  "location": "crates",
  "service": "Ripgrep",
  "read_only": true,
  "extra_parameters": {
    "extensions": "rs,toml,md"
  }
}
```

Prefer a comma-separated string because `extra_parameters` already serialises simple string-like provider parameters.

## Test Strategy

### Unit Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_allowed_extensions_defaults_to_markdown` | `fff.rs` | Preserve legacy default. |
| `test_allowed_extensions_parses_comma_list` | `fff.rs` | Validate `rs,toml,md`. |
| `test_file_extension_allowed` | `fff.rs` | Validate filtering helper. |

### Integration Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_fff_indexes_rs_file_when_extension_configured` | `tests/fff_indexer.rs` | Fails on current `.md` filter, passes after fix. |
| `test_fff_does_not_index_rs_file_by_default` | `tests/fff_indexer.rs` | Preserves markdown-only default. |
| `test_fff_with_kg_scorer_uses_stateful_path` | `tests/fff_indexer.rs` | Proves configured scorer path is not discarded. |

### Acceptance Test

Run a role-configured search that should return Rust source:

```bash
cargo run -p terraphim-cli -- search --role rust-engineer async
```

Expected evidence: at least one result with `.rs` in `url` or source path.

## Implementation Steps

### Step 1: Add Failing Tests

**Files:** `crates/terraphim_middleware/tests/fff_indexer.rs`, `crates/terraphim_middleware/src/indexer/fff.rs`  
**Description:** Add tests for default markdown-only behaviour, explicit `.rs` indexing, and stateful scorer path.  
**Expected Before Fix:** `.rs` indexing and stateful scorer tests fail.  
**Estimated:** 45 minutes.

### Step 2: Fix Cache/State Handling

**Files:** `crates/terraphim_middleware/src/indexer/fff.rs`  
**Description:** Add `is_stateful()` and route stateful instances directly to `self.index_inner()`.  
**Tests:** KG scorer/stateful test passes.  
**Dependencies:** Step 1.  
**Estimated:** 30 minutes.

### Step 3: Add Configurable Extension Filtering

**Files:** `crates/terraphim_middleware/src/indexer/fff.rs`  
**Description:** Implement `allowed_extensions()` and `file_extension_allowed()`. Use them in `index_inner()`.  
**Tests:** Unit helper tests, `.rs` integration tests.  
**Dependencies:** Step 1.  
**Estimated:** 1 hour.

### Step 4: Update Role Configs

**Files:** `.terraphim/role-rust-engineer.json`, optional `.terraphim/role-ai-engineer.json`, optional `.terraphim/role-devops.json`  
**Description:** Add explicit `extra_parameters.extensions` to code haystacks. Keep docs haystacks markdown-default.  
**Tests:** `ProjectConfig::load_from_dir()` and CLI smoke search.  
**Dependencies:** Step 3.  
**Estimated:** 30 minutes.

### Step 5: Commit Cargo.lock Alignment

**Files:** `Cargo.lock`  
**Description:** Stage the Cargo-produced lockfile update removing stale registry `fff-search 0.8.2`.  
**Tests:** `cargo check -p terraphim_grep --features code-search`, `cargo check -p terraphim_middleware`.  
**Estimated:** 15 minutes.

### Step 6: Verification and PR Update

**Files:** `.docs/verification-1873-traceability-matrix.md`, `.docs/validation-1873-final-report.md`  
**Description:** Update evidence, run full checks, post re-review response.  
**Tests:** Full command list below.  
**Estimated:** 1 hour.

## Verification Commands

```bash
cargo fmt -- --check
cargo clippy -p terraphim_middleware
cargo test -p terraphim_middleware --test fff_indexer -- --nocapture
cargo test -p terraphim_middleware --lib test_allowed_extensions
cargo check -p terraphim_middleware
cargo check -p terraphim_grep --features code-search
cargo check -p terraphim_service
cargo check -p terraphim-cli
ubs --only=rust crates/terraphim_middleware
```

## Rollback Plan

If review-fix implementation causes issues:

1. Revert extension-filter helper and tests.
2. Keep original FffIndexer migration intact.
3. Revert role JSON `extra_parameters` changes.
4. Leave `Cargo.lock` aligned if dependency graph remains correct.

## Dependencies

### New Dependencies

None.

### Dependency Updates

None beyond committing the existing lockfile alignment.

## Performance Considerations

| Metric | Target | Expected Impact |
|--------|--------|-----------------|
| Default markdown haystack search | No regression | Same or slightly faster due extension helper. |
| Code haystack search | Acceptable for `crates` | More files than markdown; still bounded by fff-search. |
| Stateful KG scorer path | No cache speedup | Acceptable; correctness over cache. |

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Decide exact extension list for AI and DevOps roles | Pending | Implementer/user |
| Decide whether service write-back should switch to `FffIndexer` now | Deferred unless requested | Implementer |

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Performance targets agreed
- [ ] Human approval received
