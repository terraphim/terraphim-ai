# Design & Implementation Plan: Fix Thesaurus Panic (Gitea #1121)

## 1. Summary of Target Behaviour

`build_thesaurus_from_haystack()` must never panic. When the requested role or default role is not found in the config, it must return an error instead of crashing.

## 2. Key Invariants and Acceptance Criteria

| Invariant | Description |
|-----------|-------------|
| No panic | `build_thesaurus_from_haystack` returns `Result` for all inputs |
| Clear error | Error message identifies which role was missing and what roles exist |
| Backward compatible | Function signature unchanged; existing callers unaffected |
| Pattern consistent | Follows `get_search_role()` style at `service/lib.rs:1267-1281` |

## 3. High-Level Design and Boundaries

**Single change**: Replace the unsafe `unwrap_or(&roles[&default_role])` with a safe fallback chain that returns `Result::Err` when no role can be resolved.

**Fallback order** (matches existing codebase convention):
1. Try `search_query.role` (if `Some` and present in `roles`)
2. Try `config.default_role` (if present in `roles`)
3. Try `roles.iter().next()` (first available role)
4. Return error (no roles configured at all)

**Boundaries**:
- Change confined to `build_thesaurus_from_haystack()` only
- No changes to callers, config, or other modules
- No new dependencies

## 4. File/Module-Level Change Plan

| File | Action | Change |
|------|--------|--------|
| `crates/terraphim_middleware/src/thesaurus/mod.rs` | Modify | Replace lines 49-54 with safe role resolution |
| `crates/terraphim_middleware/src/lib.rs` (or `mod.rs`) | Verify | Confirm `Error` type supports config error variant |

## 5. Step-by-Step Implementation Sequence

### Step 1: Replace panic code with safe resolution (3-5 lines)

Replace lines 49-54:
```rust
// BEFORE (panics)
let role_name = search_query.role.clone().unwrap_or_default();
let role: &mut Role = &mut roles
    .get(&role_name)
    .unwrap_or(&roles[&default_role])
    .to_owned();
```

With:
```rust
// AFTER (safe)
let role_name = search_query.role.clone().unwrap_or_default();
let role = roles
    .get(&role_name)
    .or_else(|| roles.get(&default_role))
    .or_else(|| roles.values().next())
    .ok_or_else(|| crate::Error::Middleware(
        format!(
            "No role found: requested='{}', default='{}', available={:?}",
            role_name,
            default_role,
            roles.keys().map(|k| k.original.as_str()).collect::<Vec<_>>()
        )
    ))?
    .to_owned();
```

### Step 2: Verify error type supports this variant

Check that `crate::Error` has a `Middleware(String)` or equivalent variant. If not, use the appropriate existing variant.

### Step 3: Run tests

```bash
cargo test -p terraphim_middleware
cargo test -p terraphim_service
cargo test -p terraphim_agent --features repl-full
```

### Step 4: Manual verification

```bash
# Rebuild
cargo build --release -p terraphim_agent --features repl-full

# Test search still works
terraphim-agent search "knowledge graph" --limit 3
```

## 6. Testing and Verification Strategy

| Criterion | Test Type | Location |
|-----------|-----------|----------|
| No panic with mismatched default_role | Unit test (new) | `crates/terraphim_middleware/src/thesaurus/mod.rs` |
| No panic with empty role_name | Unit test (new) | Same |
| No panic with empty roles map | Unit test (new) | Same |
| Existing search still works | Integration | `cargo test -p terraphim_service` |
| CLI search works end-to-end | Manual | `terraphim-agent search` |

## 7. Risk and Complexity Review

| Risk | Mitigation | Residual |
|------|------------|----------|
| Error variant doesn't exist | Check `Error` enum before coding | None -- trivially fixable |
| Behaviour change: previously panicking, now returns error | This is the intended fix | None |
| Existing callers may not handle the new error path | Caller already maps `Result` with `?` | None |

## 8. Open Questions / Decisions for Human Review

None. The fix is minimal and follows existing codebase patterns.
