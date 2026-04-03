# Research Document: Revert NormalizedTerm.id and Concept.id from String to u64

## 1. Problem Restatement and Scope

### Problem in Own Words

The `u64` → `String` migration for `NormalizedTerm.id` and `Concept.id` (commit `e0f98ee6`) was applied WITHOUT the deferred research phase that was explicitly planned. Issue #141 was marked "defer -- needs its own research phase" in `docs/research/2026-03-30-next-pr-selection.md`, but the change was implemented the next day as a "fix" rather than a researched architectural change.

### User-Visible Changes

- **Performance regression**: Benchmark shows ~130x slower ID generation, ~2.7x slower batch insertions
- **Memory overhead**: String IDs require heap allocation per ID; u64 is stack-only
- **Test failures**: Multiple thesaurus loading tests fail due to JSON fixtures having integer IDs
- **Broken Python bindings**: `terraphim_rolegraph_py` still uses u64 signatures for `magic_pair`/`magic_unpair`

### IN Scope

- Reverting `NormalizedTerm.id`, `Concept.id`, `Edge.id`, `Node.id` back to `u64`
- Restoring `INT_SEQ` global atomic counter for ID generation
- Restoring `magic_pair`/`magic_unpair` as u64-based pairing functions
- Updating JSON fixtures to use integer IDs
- Fixing Python bindings to match new signatures
- Ensuring all tests pass

### OUT of Scope

- Changing the UUID dependency (still used elsewhere in codebase)
- Modifying serialization formats for stored data (handled separately)
- Python bindings implementation details (signature updates only)

---

## 2. User & Business Outcomes

### Visible Changes After Revert

1. **Performance**: Autocomplete and search operations will be faster (benchmarks show 2-130x improvement for ID-heavy operations)
2. **Memory**: Reduced heap allocations for ID generation
3. **Stability**: Existing tests pass, JSON fixtures deserialize correctly

### Business Value

- Faster search response times for knowledge graph operations
- Reduced memory pressure on long-running services
- Restored compatibility with existing JSON fixtures and external consumers

---

## 3. System Elements and Dependencies

### Core Type Files

| File | Role | Current State |
|------|------|---------------|
| `crates/terraphim_types/src/lib.rs` | Defines `NormalizedTerm`, `Concept`, `Edge`, `Node`, `IndexedDocument` | All IDs are `String`; `INT_SEQ` removed; `magic_pair` replaced with String version |

### Crates with Direct Type Usage

| Crate | Files | Impact |
|-------|-------|--------|
| `terraphim_rolegraph` | `src/lib.rs`, `src/medical.rs`, `examples/*.rs` | Core graph implementation uses String IDs; `magic_pair`/`magic_unpair` are String-based |
| `terraphim_automata` | `src/autocomplete.rs`, `src/lib.rs`, `benches/*.rs` | Thesaurus with `NormalizedTerm`; benchmarks affected |
| `terraphim_server` | `src/api.rs` | Uses `magic_unpair` with String |
| `terraphim_service` | `src/lib.rs` | LLM client building |
| `terraphim_config` | `src/lib.rs` | Role configuration |
| `terraphim_agent` | `src/*.rs` | CLI and REPL |
| `terraphim_rolegraph_py` | `src/lib.rs` | **OUT OF SYNC** - Python bindings still use u64 for `magic_pair`/`magic_unpair` |

### JSON Fixture Files

| File | Status |
|------|--------|
| `terraphim_server/fixtures/thesaurus_Default.json` | Contains integer IDs (will fail to deserialize with String types) |
| `terraphim_server/fixtures/haystack/*.json` | Contains integer IDs |
| `test-fixtures/term_to_id.json` | Contains integer IDs |
| `crates/terraphim_agent/data/guard_*.json` | Already converted to string IDs |

### Removed Code to Restore

```rust
// OLD: Global atomic counter (removed in e0f98ee6)
use std::sync::atomic::{AtomicU64, Ordering};
static INT_SEQ: AtomicU64 = AtomicU64::new(1);
fn get_int_id() -> u64 {
    INT_SEQ.fetch_add(1, Ordering::SeqCst)
}

// OLD: magic_pair/magic_unpair (u64-based Cantor pairing)
pub fn magic_pair(x: u64, y: u64) -> u64 {
    if x >= y { x * x + x + y } else { y * y + x }
}
pub fn magic_unpair(z: u64) -> (u64, u64) {
    let q = (z as f64).sqrt().floor() as u64;
    let l = z - q * q;
    if l < q { (l, q) } else { (q, l - q) }
}
```

---

## 4. Constraints and Their Implications

### Performance Constraint

**Benchmark evidence shows**:
- UUID `to_string()`: ~215 ns vs atomic u64: ~1.6 ns (130x slower)
- Batch insertion (1K): String ~46µs vs u64 ~17µs (2.7x slower)
- Lookup (10K): String ~9.5µs vs u64 ~7.8µs (22% slower)

**Implication**: For ID generation in hot paths, u64 is significantly faster. The revert would improve autocomplete and knowledge graph operations.

### Memory Constraint

- `u64`: 8 bytes, stack-allocated, no heap allocation
- `String` (16 chars): 24 bytes stack + variable heap

**Implication**: For large knowledge graphs (thousands of nodes/edges), String IDs increase memory pressure significantly.

### Backward Compatibility Constraint

The JSON fixtures contain integer IDs. If we revert, we need to either:
1. Update all JSON fixtures to use integer IDs, OR
2. Add serde deserialization that accepts both strings and integers

**Implication**: Option 1 is cleaner but touches fixtures. Option 2 adds complexity but maintains flexibility.

### Python Bindings Constraint

`terraphim_rolegraph_py` still has u64 signatures for `magic_pair`/`magic_unpair`:
```rust
fn magic_pair(x: u64, y: u64) -> u64    // Python binding expects u64
fn magic_unpair(z: u64) -> (u64, u64)     // But Rust impl uses String!
```

**Implication**: The Python bindings are already out of sync. Reverting to u64 would actually ALIGN them with the Python expectations.

### Documentation Constraint

`docs/src/kg/knowledge-graph-system.md` still documents the u64-based design:
- `nodes: AHashMap<u64, Node>`
- `edges: AHashMap<u64, Edge>`
- `Node.id: u64`
- `Edge.id: u64`

**Implication**: The documentation is correct for the OLD design. Reverting would restore consistency between code and docs.

---

## 5. Risks, Unknowns, and Assumptions

### ASSUMPTIONS (marked for verification)

| Assumption | Why | Verification |
|------------|-----|--------------|
| Single-knowledge-graph use case | The original `INT_SEQ` atomic counter worked within one KG | Verify multi-KG scenarios don't need cross-KG ID uniqueness |
| No external API consumers depend on String IDs | UUIDs were intended for distributed systems | Audit external API consumers |
| Tests don't intentionally test String ID format | Some tests may have string-specific assertions | Review test assertions before revert |

### RISKS

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| Another crate depends on String IDs externally | HIGH | LOW | Audit external API consumers before change |
| JSON fixtures updated for String break if reverted | HIGH | MEDIUM | Update fixtures as part of revert, or use flexible deserialization |
| Benchmarks that were updated for String IDs become invalid | MEDIUM | LOW | Update benchmarks to measure u64 performance instead |
| Python bindings remain out of sync | MEDIUM | HIGH | Update Python bindings as part of this change |

### UNKNOWNS

| Unknown | Why Unknown | How to Resolve |
|---------|-------------|----------------|
| Original motivation for #141 (UUID for "uniqueness per KG") | Not documented | Interview code author or check Gitea issue details |
| Whether distributed KG scenarios actually need cross-KG unique IDs | Design assumption | Research if multiple KGs ever merge or share IDs |
| Impact on any in-flight PRs that depend on String IDs | Not tracked | Check open PRs before implementation |

---

## 6. Context Complexity vs. Simplicity Opportunities

### Complexity Sources

1. **Dual signatures for magic_pair/magic_unpair**: Python bindings expect u64, Rust uses String
2. **JSON fixtures with integer IDs**: Need either conversion or flexible deserialization
3. **UUID dependency still present**: Used elsewhere in codebase; cannot simply remove
4. **Multiple hash map key types changing**: nodes, edges, connected_with, documents

### Simplification Strategies

1. **Restore original u64-based design**: The simplest path is to revert commit `e0f98ee6` entirely, restoring the documented design
2. **Keep flexible deserialization**: Add `#[serde(deserialize_with = "deserialize_id")]`` that accepts both integer and string to ease migration
3. **Update Python bindings alongside**: Treat as part of the same change, not separate
4. **Focus on one crate at a time**: Start with `terraphim_types`, then propagate changes

---

## 7. Questions for Human Reviewer

1. **What was the original problem that issue #141 was trying to solve?** The commit message mentions "UUID generation for Concept uniqueness per KG" but we don't have the issue details. Can you share what specific problem motivated this change?

2. **Are there external API consumers that depend on String IDs?** If other services or APIs expect UUID-format IDs, reverting could break them.

3. **Should JSON fixtures use integer IDs or a flexible format?** We can either update fixtures to integers, or support both integers and strings in deserialization.

4. **Is there a timeline constraint?** The revert should be done carefully; is there pressure to ship this quickly or can it be done properly with testing?

5. **Should we update `docs/src/kg/knowledge-graph-system.md` as part of this?** The docs currently describe the u64 design (which was correct before the change).

6. **Are there any planned features that depend on String/UUID IDs?** Some future feature might need UUID uniqueness; we should know before removing it.

7. **Should the Python bindings be updated as part of this same PR?** They are already out of sync; fixing them together would be cleaner.

8. **Is it acceptable to update all JSON fixtures (integer IDs are fine after revert)?** This is the cleanest approach but touches many fixture files.

9. **Do we need backward compatibility for serialized data?** If there's persisted data with String IDs, we might need migration logic.

10. **Are there any performance SLOs that String IDs were meant to address?** If there were latency goals that String IDs were supposed to help with, reverting might impact them.
