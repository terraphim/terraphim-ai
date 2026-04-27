# Implementation Plan: Add query and role fields to ResponseMeta

**Status**: Approved
**Research Doc**: `.docs/research-1026-query-role-response-meta.md`
**Author**: opencode (glm-5.1)
**Date**: 2026-04-27
**Issue**: #1026
**Estimated Effort**: 30 minutes

## Overview

### Summary
Add `query: Option<String>` and `role: Option<String>` to `ResponseMeta` struct, populate them at both search command sites, and add unit tests.

### Approach
Follow existing builder pattern. Add fields to struct, add `with_query()`/`with_role()` builder methods, chain them at both `ResponseMeta::new("search")` call sites.

### Scope

**In Scope:**
1. Add `query`/`role` fields to `ResponseMeta`
2. Add `with_query()`/`with_role()` builder methods
3. Populate at both search call sites (direct + server API)
4. Unit tests for new fields

**Out of Scope:**
- Error response query/role (not needed)
- REPL mode changes (human-facing)
- SearchResultsData changes (PR #969)
- listen --server test fix (pre-existing V001)

**Avoid At All Cost:**
- Changing any other ResponseMeta consumers
- Adding fields to SearchResultsData or RobotResponse
- Modifying the robot-mode error envelope

### Simplicity Check

**What if this could be easy?** Two fields, two builders, two call sites. That's it.

**Senior Engineer Test**: This is the simplest possible change. Two Option fields with skip_serializing_if.

## File Changes

### Modified Files
| File | Changes |
|------|---------|
| `crates/terraphim_agent/src/robot/schema.rs` | Add 2 fields to `ResponseMeta`, 2 builder methods, 2 tests |
| `crates/terraphim_agent/src/main.rs` | Chain `.with_query().with_role()` at 2 call sites |

## API Design

### Struct Changes
```rust
pub struct ResponseMeta {
    pub command: String,
    pub elapsed_ms: u64,
    pub timestamp: DateTime<Utc>,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_corrected: Option<AutoCorrection>,
    // ... rest unchanged
}
```

### Builder Methods
```rust
pub fn with_query(mut self, query: impl Into<String>) -> Self {
    self.query = Some(query.into());
    self
}

pub fn with_role(mut self, role: impl Into<String>) -> Self {
    self.role = Some(role.into());
    self
}
```

### Call Site Changes

**Site 1 (line 1873):**
```rust
ResponseMeta::new("search")
    .with_elapsed(start.elapsed().as_millis() as u64)
    .with_query(&query)
    .with_role(role_name.as_str())
```

**Site 2 (line 3825):**
```rust
ResponseMeta::new("search")
    .with_elapsed(start.elapsed().as_millis() as u64)
    .with_query(&query)
    .with_role(role_name.as_str())
```

## Test Strategy

### Unit Tests
| Test | Location | Purpose |
|------|----------|---------|
| `test_meta_with_query_and_role` | `schema.rs` | Builder methods populate fields |
| `test_meta_serialization_omits_none` | `schema.rs` | Fields absent from JSON when None |

### Verification
- `cargo test -p terraphim_agent --test exit_codes` (11 tests, existing)
- `cargo test -p terraphim_agent robot::schema` (new + existing schema tests)
- `cargo fmt && cargo clippy`

## Implementation Steps

### Step 1: Add fields and builders to ResponseMeta
**Files:** `crates/terraphim_agent/src/robot/schema.rs`
**Description:** Add `query` and `role` Option fields between `version` and `auto_corrected`. Add `with_query()` and `with_role()` builder methods. Initialise both to `None` in `new()`.
**Tests:** `test_meta_with_query_and_role`, `test_meta_serialization_omits_none`
**Estimated:** 10 minutes

### Step 2: Populate fields at search call sites
**Files:** `crates/terraphim_agent/src/main.rs`
**Description:** Chain `.with_query(&query).with_role(role_name.as_str())` at both `ResponseMeta::new("search")` call sites.
**Tests:** Run `cargo test -p terraphim_agent`
**Dependencies:** Step 1
**Estimated:** 5 minutes

### Step 3: Quality gates
**Description:** `cargo fmt`, `cargo clippy`, `cargo test -p terraphim_agent`
**Dependencies:** Step 2
**Estimated:** 5 minutes

## Rollback Plan

Revert the two files -- all changes are additive with no breaking modifications.

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Verify query variable name at both sites | Done (confirmed `query` at both) | Research |
| Verify role_name availability at both sites | Done (confirmed `role_name` at both) | Research |

## Approval

- [x] Technical review complete
- [x] Test strategy defined
- [x] Simplicity check passed
- [ ] Human approval received
