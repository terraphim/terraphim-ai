# Design & Implementation Plan: Revert NormalizedTerm.id and Concept.id from String to u64

## 1. Summary of Target Behavior

After this revert, the system will:

1. **`NormalizedTerm.id`**: `u64` instead of `String` (generated via atomic counter)
2. **`Concept.id`**: `u64` instead of `String` (generated via atomic counter)
3. **`Edge.id`**: `u64` instead of `String` (generated via Cantor pairing)
4. **`Node.id`**: `u64` instead of `String`
5. **`Node.connected_with`**: `HashSet<u64>` instead of `HashSet<String>`
6. **`IndexedDocument.nodes`**: `Vec<u64>` instead of `Vec<String>`
7. **`magic_pair(x, y)`**: Returns `u64` (Cantor pairing) instead of `String`
8. **`magic_unpair(z)`**: Takes `u64`, returns `(u64, u64)` instead of `(String, String)`
9. **JSON fixtures**: Contain integer IDs that deserialize correctly
10. **Python bindings**: Align with restored u64 signatures

---

## 2. Key Invariants and Acceptance Criteria

### Invariants

| Invariant | Description |
|-----------|-------------|
| ID uniqueness | `INT_SEQ` atomic counter ensures unique IDs per process |
| magic_pair symmetry | `magic_pair(a, b) == magic_pair(b, a)` |
| magic_unpair correctness | `magic_unpair(magic_pair(a, b)) == (a, b)` |
| No duplicate IDs | Inserting same term twice generates different IDs (new INT_SEQ each time) |
| Serialization round-trip | JSON fixtures deserialize correctly with integer IDs |

### Acceptance Criteria

| # | Criterion | Verification |
|---|-----------|--------------|
| 1 | `cargo build --workspace` succeeds | Build verification |
| 2 | `cargo test --workspace` passes (369+ tests) | Test suite |
| 3 | `cargo clippy --workspace --all-targets -- -D warnings` passes | Lint check |
| 4 | `cargo fmt --all -- --check` passes | Style check |
| 5 | `NormalizedTerm::new(1, ...)` accepts u64 id | Unit test |
| 6 | `Concept::with_id(1, ...)` accepts u64 id | Unit test |
| 7 | `magic_pair(3, 5) == magic_pair(5, 3)` | Property test |
| 8 | `magic_unpair(magic_pair(3, 5)) == (3, 5)` | Property test |
| 9 | JSON fixtures with integer IDs deserialize | Integration test |
| 10 | `terraphim_rolegraph_py` bindings compile | Build verification |

---

## 3. High-Level Design and Boundaries

### Strategy: Restore Original u64 Design

The simplest approach is to revert commit `e0f98ee6` rather than cherry-pick changes. This restores:
- Global `INT_SEQ` atomic counter for sequential IDs
- Cantor pairing for `magic_pair`/`magic_unpair`
- Direct `u64` field types in structs

### Scope Boundaries

**Inside scope (will change)**:
- `crates/terraphim_types/src/lib.rs` - Type definitions
- `crates/terraphim_rolegraph/src/lib.rs` - Graph implementation with magic_pair/magic_unpair
- `crates/terraphim_automata/src/autocomplete.rs` - Autocomplete index
- `crates/terraphim_server/src/api.rs` - API handlers
- `crates/terraphim_rolegraph_py/src/lib.rs` - Python bindings
- JSON fixture files

**Outside scope (unchanged)**:
- UUID dependency (still used elsewhere)
- Serialization format for persisted data (handled separately)
- External API consumers (audit if needed)

### Component Diagram

```
terraphim_types (CORE - types)
    │
    ├── NormalizedTerm { id: u64, ... }
    ├── Concept { id: u64, ... }
    ├── Edge { id: u64, ... }
    ├── Node { id: u64, connected_with: HashSet<u64>, ... }
    └── IndexedDocument { nodes: Vec<u64>, ... }
           │
           ▼
terraphim_rolegraph (uses types)
    │
    ├── RoleGraph { nodes: AHashMap<u64, Node>, edges: AHashMap<u64, Edge>, ... }
    ├── magic_pair(x: u64, y: u64) -> u64  (Cantor pairing)
    └── magic_unpair(z: u64) -> (u64, u64)
           │
           ▼
terraphim_automata (uses types)
    │
    ├── Thesaurus { ... }
    └── AutocompleteIndex { ... }
           │
           ▼
terraphim_server (uses automata + rolegraph)
    │
    └── API handlers
```

---

## 4. File/Module-Level Change Plan

| File/Module | Action | Before | After | Dependencies |
|-------------|--------|--------|-------|--------------|
| `crates/terraphim_types/src/lib.rs` | MODIFY | `id: String`, `INT_SEQ` removed | Restore `id: u64`, `INT_SEQ` atomic counter | No external deps |
| `crates/terraphim_rolegraph/src/lib.rs` | MODIFY | String IDs, String-based magic_pair | Restore u64 IDs, Cantor pairing | terraphim_types |
| `crates/terraphim_rolegraph/src/medical.rs` | MODIFY | String edge IDs | Restore u64 edge IDs | terraphim_types |
| `crates/terraphim_rolegraph/examples/*.rs` | MODIFY | String IDs in examples | Restore u64 IDs | terraphim_types, terraphim_rolegraph |
| `crates/terraphim_automata/src/autocomplete.rs` | MODIFY | String term IDs | Restore u64 term IDs | terraphim_types |
| `crates/terraphim_automata/benches/*.rs` | MODIFY | String IDs in benchmarks | Restore u64 IDs | terraphim_types |
| `crates/terraphim_automata/tests/*.rs` | MODIFY | String ID assertions | Restore u64 assertions | terraphim_types |
| `crates/terraphim_automata/src/*.rs` | REVIEW | Various ID usages | Check and update | terraphim_types |
| `terraphim_server/src/api.rs` | MODIFY | String-based unpairing | Restore u64 unpairing | terraphim_types, terraphim_rolegraph |
| `crates/terraphim_rolegraph_py/src/lib.rs` | MODIFY | String magic_pair signatures | Restore u64 signatures | terraphim_types |
| `terraphim_server/fixtures/*.json` | MODIFY | String IDs in fixtures | Restore integer IDs | Deserialize via serde |
| `test-fixtures/*.json` | MODIFY | String IDs | Restore integer IDs | Deserialize via serde |
| `crates/terraphim_agent/data/guard_*.json` | MODIFY | Already updated to strings | May need keep as-is OR convert back | Deserialize via serde |

### Detailed Type Changes

#### terraphim_types/src/lib.rs

```rust
// RESTORE: Global atomic counter (was removed)
use std::sync::atomic::{AtomicU64, Ordering};
static INT_SEQ: AtomicU64 = AtomicU64::new(1);
fn get_int_id() -> u64 {
    INT_SEQ.fetch_add(1, Ordering::SeqCst)
}

// MODIFY: NormalizedTerm
pub struct NormalizedTerm {
    pub id: u64,           // CHANGED from String
    pub value: NormalizedTermValue,
    pub display_value: Option<String>,
    pub url: Option<String>,
}

impl NormalizedTerm {
    pub fn new(id: u64, value: NormalizedTermValue) -> Self {  // CHANGED signature
        Self { id, value, display_value: None, url: None }
    }
    // Remove new_with_uuid() - no longer needed
}

// MODIFY: Concept
pub struct Concept {
    pub id: u64,           // CHANGED from String
    pub value: NormalizedTermValue,
}

impl Concept {
    pub fn new(value: NormalizedTermValue) -> Self {
        Self { id: get_int_id(), value }  // CHANGED: use INT_SEQ
    }
    pub fn with_id(id: u64, value: NormalizedTermValue) -> Self {  // CHANGED signature
        Self { id, value }
    }
}

// MODIFY: Edge
pub struct Edge {
    pub id: u64,           // CHANGED from String
    pub rank: u64,
    pub doc_hash: AHashMap<String, u64>,
}

impl Edge {
    pub fn new(id: u64, document_id: String) -> Self {  // CHANGED signature
        // ...
    }
}

// MODIFY: Node
pub struct Node {
    pub id: u64,                      // CHANGED from String
    pub rank: u64,
    pub connected_with: HashSet<u64>, // CHANGED from HashSet<String>
    // ...
}

// MODIFY: IndexedDocument
pub struct IndexedDocument {
    pub nodes: Vec<u64>,  // CHANGED from Vec<String>
    // ...
}
```

#### terraphim_rolegraph/src/lib.rs

```rust
// RESTORE: Cantor pairing (was replaced with String version)
/// Magic pair - Cantor pairing function for edge IDs
pub fn magic_pair(x: u64, y: u64) -> u64 {
    if x >= y { x * x + x + y } else { y * y + x }
}

/// Magic unpair - inverse of Cantor pairing
pub fn magic_unpair(z: u64) -> (u64, u64) {
    let q = (z as f64).sqrt().floor() as u64;
    let l = z - q * q;
    if l < q { (l, q) } else { (q, l - q) }
}
```

---

## 5. Step-by-Step Implementation Sequence

### Step 1: Update `terraphim_types` crate (CORE)
**Purpose**: Restore fundamental type definitions
**Deployable state**: Yes - types only, no dependents yet
**Risk**: Low - isolated change

1. Restore `INT_SEQ` atomic counter and `get_int_id()` function
2. Change `NormalizedTerm.id` from `String` to `u64`
3. **REMOVE** `NormalizedTerm::new_with_uuid()` - delete entirely
4. Change `NormalizedTerm::new(id: impl Into<String>)` to `new(id: u64)`
5. Change `Concept.id` from `String` to `u64`
6. Change `Concept::with_id()` signature to accept `u64`
7. Change `Concept::new()` to use `get_int_id()` instead of UUID
8. Change `Edge.id` from `String` to `u64`
9. Change `Edge::new(id: impl Into<String>)` to `new(id: u64)`
10. Change `Node.id` from `String` to `u64`
11. Change `Node.connected_with` from `HashSet<String>` to `HashSet<u64>`
12. Change `IndexedDocument.nodes` from `Vec<String>` to `Vec<u64>`
13. Update tests in the same file
14. Verify: `cargo test -p terraphim_types`

### Step 2: Update `terraphim_rolegraph` crate
**Purpose**: Restore graph implementation with u64 IDs
**Deployable state**: After Step 1
**Risk**: Medium - many internal changes

1. Restore `magic_pair(x: u64, y: u64) -> u64` (Cantor pairing)
2. Restore `magic_unpair(z: u64) -> (u64, u64)`
3. Update `RoleGraph.nodes` from `AHashMap<String, Node>` to `AHashMap<u64, Node>`
4. Update `RoleGraph.edges` from `AHashMap<String, Edge>` to `AHashMap<u64, Edge>`
5. Update all methods that construct or query nodes/edges
6. Update `init_or_update_node()` signature
7. Update `init_or_update_edge()` calls to use u64
8. Update `medical.rs` if it has edge handling
9. Update all examples in `examples/`
10. Verify: `cargo test -p terraphim_rolegraph`

### Step 3: Update `terraphim_automata` crate
**Purpose**: Restore autocomplete with u64 IDs
**Deployable state**: After Step 1
**Risk**: Medium - thesaurus and autocomplete changes

1. Update `AutocompleteIndex` to use `u64` keys
2. Update `NormalizedTerm` construction in benchmarks
3. Update `NormalizedTerm` construction in tests
4. Verify: `cargo test -p terraphim_automata`

### Step 4: Update `terraphim_server` crate
**Purpose**: Fix API handlers that use magic_unpair
**Deployable state**: After Steps 1-2
**Risk**: Medium - API signature changes

1. Update `api.rs` that calls `magic_unpair` to expect u64 input
2. Update any JSON fixture loading that expects string IDs
3. Verify: `cargo test -p terraphim_server`

### Step 5: Update `terraphim_rolegraph_py` Python bindings
**Purpose**: Align Python bindings with restored signatures
**Deployable state**: After Steps 1-2
**Risk**: Medium - FFI changes

1. Update `magic_pair` wrapper to accept/return u64
2. Update `magic_unpair` wrapper to accept/return u64
3. Verify: `cargo build -p terraphim_rolegraph_py`

### Step 6: Update JSON fixture files
**Purpose**: Ensure fixtures deserialize correctly
**Deployable state**: After Steps 1-5
**Risk**: Medium - many files

1. Convert `thesaurus_Default.json` integer IDs (already integers, verify serde accepts)
2. Convert `haystack/*.json` integer IDs (already integers, verify serde accepts)
3. Convert `test-fixtures/term_to_id*.json` integer IDs (already integers)
4. Convert `crates/terraphim_agent/data/guard_*.json` string IDs to integers OR keep flexible deserializer
5. Verify: `cargo test` on fixture-related tests

### Step 7: Update Documentation
**Purpose**: Restore consistency between code and docs
**Deployable state**: After all code changes
**Risk**: Low - documentation only

1. Update `docs/src/kg/knowledge-graph-system.md`:
   - `nodes: AHashMap<String, Node>` → `nodes: AHashMap<u64, Node>`
   - `edges: AHashMap<String, Edge>` → `edges: AHashMap<u64, Edge>`
   - `Node.id: u64` (already correct in docs)
   - `Edge.id: u64` (already correct in docs)
   - `connected_with: HashSet<u64>` (already correct in docs)
2. Verify: Build docs if applicable

### Step 8: Remove Benchmark File
**Purpose**: Clean up investigation artifacts
**Deployable state**: After docs update
**Risk**: Low - file removal

1. Remove `crates/terraphim_types/benches/id_performance.rs`
2. Remove `[[bench]]` section from `crates/terraphim_types/Cargo.toml`
3. Remove `criterion` dev-dependency from `crates/terraphim_types/Cargo.toml`

### Step 9: Workspace verification
**Purpose**: Ensure all crates work together
**Deployable state**: After all steps
**Risk**: Low - final verification

1. `cargo build --workspace`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo fmt --all -- --check`

---

## 6. Testing & Verification Strategy

| Acceptance Criteria | Test Type | Test Location |
|--------------------|-----------|--------------|
| `NormalizedTerm::new(1, ...)` works with u64 | Unit | `terraphim_types/src/lib.rs` tests |
| `Concept::with_id(1, ...)` works with u64 | Unit | `terraphim_types/src/lib.rs` tests |
| `magic_pair(a, b) == magic_pair(b, a)` | Property | `terraphim_rolegraph/src/lib.rs` tests |
| `magic_unpair(magic_pair(a, b)) == (a, b)` | Property | `terraphim_rolegraph/src/lib.rs` tests |
| JSON with integer IDs deserializes | Integration | `terraphim_server` tests |
| `test_load_thesaurus_from_json` passes | Integration | `terraphim_automata` tests |
| Full workspace builds | Build | CI |
| All workspace tests pass | E2E | CI |

### Property-Based Tests for magic_pair/magic_unpair

```rust
#[test]
fn test_magic_pair_symmetry() {
    for a in 0..1000u64 {
        for b in 0..1000u64 {
            assert_eq!(magic_pair(a, b), magic_pair(b, a));
        }
    }
}

#[test]
fn test_magic_pair_unpair_inverse() {
    for a in 0..1000u64 {
        for b in 0..1000u64 {
            let z = magic_pair(a, b);
            let (unpaired_a, unpaired_b) = magic_unpair(z);
            assert_eq!((a, b), (unpaired_a, unpaired_b));
        }
    }
}
```

---

## 7. Risk & Complexity Review

| Risk | Mitigation | Residual Risk |
|------|------------|---------------|
| Another crate depends on String IDs externally | Audit external API consumers before change | LOW |
| JSON fixtures updated for String break if reverted | Convert fixtures as part of this change | LOW |
| Python bindings remain out of sync | Update as part of this same PR | NONE (fixed in Step 5) |
| Test assertions hardcoded to String format | Review test assertions before revert | MEDIUM |
| Benchmarks measure wrong thing after revert | Update benchmarks to measure u64 performance | LOW |
| INT_SEQ collision under heavy parallelism | AtomicU64 is lock-free, high contention threshold | VERY LOW |

---

## 8. Open Questions / Decisions for Human Review

| # | Question | Options |
|---|----------|---------|
| 1 | **Flexible deserializer (accept both int AND string)?** | **CLEAN BREAK** - No flexible deserializer, update all persisted data |
| 2 | **`NormalizedTerm::new_with_uuid()`?** | **REMOVE** - Delete the function entirely |
| 3 | **Update `docs/src/kg/knowledge-graph-system.md`?** | **YES** - Update as part of this PR to restore consistency |
| 4 | **JSON fixtures: pure integers or hybrid?** | **PURE INTEGERS** - Clean format, no hybrid |
| 5 | **Benchmark `id_performance.rs`?** | **REMOVE** - Created for investigation, no longer needed after revert |

---

## Implementation Notes

### Why Cantor Pairing?

The original `magic_pair` used Cantor pairing (Szudzik's variant):
- `a >= b ? a*a + a + b : b*b + a`
- Produces unique u64 for any pair (a, b)
- bijective: `unpair(pair(a, b)) == (a, b)`
- More efficient than String concatenation + parsing
