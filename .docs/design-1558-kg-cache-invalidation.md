# Design & Implementation Plan: #1558 Offline CLI KG Cache Invalidation

## 1. Summary of Target Behavior

The offline `terraphim-agent` CLI will automatically detect when KG markdown files have changed and rebuild its cached thesaurus on first use per session. No manual `kg rebuild` command required — staleness is detected transparently via `source_hash` comparison.

**User Flow (After Fix):**
```
User writes new concept files to kg/
User runs terraphim-agent extract "new concept"
→ CLI detects hash mismatch → rebuilds thesaurus → returns correct matches
```

## 2. Key Invariants and Acceptance Criteria

### Invariants
- Thesauri are always rebuilt when source hash changes
- No stale concepts appear in `extract`/`search`/`suggest` results
- Rebuild is transparent to user (silent)

### Acceptance Criteria
| ID | Criterion | Test Type |
|----|----------|-----------|
| AC1 | New `.md` file added to KG path → `extract` finds it within same session | Integration |
| AC2 | Modified `.md` file in KG path → `extract` reflects changes within same session | Integration |
| AC3 | Deleted `.md` file from KG path → `extract` no longer matches it within same session | Integration |
| AC4 | Hash check adds <50ms latency on first `extract` per session | Benchmark |
| AC5 | Session restart forces fresh build (no persistent cache) | Unit |
| AC6 | `cargo test -p terraphim_agent` passes | CI |
| AC7 | `cargo test -p terraphim_sessions` passes (unchanged) | CI |

## 3. High-Level Design and Boundaries

### Problem Analysis
The offline CLI uses `OnceLock` which initializes once and never updates. The server uses `RwLock<Roles>` with hash-checking logic (`terraphim_service/src/lib.rs:524-577`).

### Solution: Replace `OnceLock` with Hash-Checked Lazy Init

**Current (broken):**
```rust
static VALIDATION_KG_THESAURUS: OnceLock<Option<Thesaurus>> = OnceLock::new();
// get_or_init() → returns cached value forever
```

**New (fixed):**
```rust
struct CachedThesaurus {
    thesaurus: Thesaurus,
    source_hash: String,
}

static KG_CACHE: OnceLock<CachedThesaurus> = OnceLock::new();

fn get_thesaurus() -> &'static Thesaurus {
    let cached = KG_CACHE.get_or_init(|| {
        let (thesaurus, hash) = build_and_hash_kg();
        CachedThesaurus { thesaurus, source_hash: hash }
    });

    // Check if stale on EVERY call (per-session freshness)
    if let Ok(Some(new_hash)) = compute_kg_source_hash(&kg_path) {
        if new_hash != cached.source_hash {
            let (thesaurus, hash) = build_and_hash_kg();
            return CachedThesaurus { thesaurus, source_hash: hash };
        }
    }
    &cached.thesaurus
}
```

**Better approach — per-session check, not per-call:**
Cache stores `(thesaurus, source_hash, session_id)` where session_id tracks if we're in same session. On first access per session, check hash. This avoids per-call overhead.

Actually, simplest approach: **compute hash on first access, store it with thesaurus in OnceLock, compare on subsequent accesses within same process**.

### Components
| Component | Responsibility | Change |
|-----------|---------------|--------|
| `kg_validation.rs` | Global thesaurus cache | Replace `OnceLock<Option<Thesaurus>>` with `OnceLock<CachedThesaurus>` + hash check |
| `learnings/capture.rs` | Build thesaurus from KG dir | Add `build_kg_thesaurus_with_hash()` returning (Thesaurus, hash) |
| `terraphim_automata::builder` | Hash computation | Already exists as `compute_kg_source_hash()` |

### Boundaries
- **Inside scope**: `terraphim_agent` crate only
- **Outside scope**: Server (`terraphim_service`), persistence layer, CLI commands

## 4. File/Module-Level Change Plan

| File/Module | Action | Before | After |
|-------------|--------|--------|-------|
| `crates/terraphim_agent/src/kg_validation.rs` | Modify | `OnceLock<Option<Thesaurus>>` | `OnceLock<CachedThesaurus>` with hash check |
| `crates/terraphim_agent/src/learnings/capture.rs` | Modify | `build_kg_thesaurus_from_dir()` | Add `build_kg_thesaurus_with_hash()` returning (Thesaurus, hash) |

### Detail

**`kg_validation.rs` changes:**
1. Add `CachedThesaurus` struct: `{ thesaurus: Thesaurus, source_hash: String, kg_path: PathBuf }`
2. Change `VALIDATION_KG_THESAURUS: OnceLock<Option<Thesaurus>>` → `OnceLock<CachedThesaurus>`
3. Add `get_thesaurus_with_auto_rebuild()` function that:
   - Gets cached value or initializes
   - Computes current `compute_kg_source_hash()`
   - If mismatch → rebuilds and updates cache
   - Returns `&'static Thesaurus`
4. Update `validate_command_against_kg()` to call `get_thesaurus_with_auto_rebuild()`

**`learnings/capture.rs` changes:**
1. Add `build_kg_thesaurus_with_hash(kg_dir: &Path) -> Option<(Thesaurus, String)>`:
   - Builds thesaurus
   - Computes hash via `compute_kg_source_hash()`
   - Returns both

**`main.rs` changes (if needed):**
- May need to expose `kg_path` via `find_kg_dir()` for use in `kg_validation.rs`

## 5. Step-by-Step Implementation Sequence

### Step 1: Add `build_kg_thesaurus_with_hash()` helper
**Purpose**: Return both thesaurus and hash for caching
**File**: `learnings/capture.rs`
**Deployable**: Yes (additive, no behavior change yet)

### Step 2: Define `CachedThesaurus` struct
**Purpose**: Store thesaurus + hash + kg_path together
**File**: `kg_validation.rs`
**Deployable**: Yes (new struct, no behavior change)

### Step 3: Implement `get_thesaurus_with_auto_rebuild()`
**Purpose**: Core logic — check hash, rebuild if stale
**File**: `kg_validation.rs`
**Deployable**: Yes (replaces `OnceLock` initialization)

### Step 4: Update `validate_command_against_kg()` to use new getter
**Purpose**: Wire up the auto-rebuild logic
**File**: `kg_validation.rs`
**Deployable**: Yes (transparent behavior change)

### Step 5: Write integration tests
**Purpose**: Verify AC1-AC3
**File**: New test in `kg_validation.rs` or integration test suite
**Deployable**: Yes (tests only)

### Step 6: Run full test suite
**Purpose**: Verify no regressions
**Deployable**: Yes (CI gate)

## 6. Testing & Verification Strategy

| Acceptance Criteria | Test Type | Test Location |
|---------------------|-----------|--------------|
| AC1: New file detected | Integration | Write KG file → call `validate_command_against_kg()` → assert match |
| AC2: Modified file detected | Integration | Modify KG file → rebuild → assert new content |
| AC3: Deleted file detected | Integration | Delete KG file → rebuild → assert no match |
| AC4: Latency < 50ms | Benchmark | Time `get_thesaurus_with_auto_rebuild()` on warm cache |
| AC5: Fresh on restart | Unit | `OnceLock` naturally reinitializes on new process |
| AC6: Tests pass | CI | `cargo test -p terraphim_agent` |
| AC7: No regressions | CI | `cargo test -p terraphim_sessions` |

### Integration Test Pattern
```rust
#[test]
fn test_new_kg_file_detected() {
    let temp_dir = TempDir::new().unwrap();
    let kg_dir = temp_dir.path();

    // Build initial thesaurus
    let file1 = kg_dir.join("concept1.md");
    fs::write(&file1, "synonyms:: test1\n").unwrap();

    let (thesaurus, hash) = build_kg_thesaurus_with_hash(kg_dir).unwrap();
    assert!(thesaurus.get(&"test1".into()).is_some());

    // Add new file
    let file2 = kg_dir.join("concept2.md");
    fs::write(&file2, "synonyms:: test2\n").unwrap();

    // Rebuild with same path
    let new_thesaurus = rebuild_if_stale(kg_dir, &hash);
    assert!(new_thesaurus.get(&"test2".into()).is_some());
}
```

## 7. Risk & Complexity Review

| Risk | Mitigation | Residual Risk |
|------|------------|---------------|
| `OnceLock` race on concurrent access | `OnceLock` guarantees single init; hash check after init is read-mostly | Low |
| Hash computation slow on large KG | `compute_kg_source_hash` is O(n) dir scan; acceptable for CLI | Low |
| Breaking `kg_validation` API | `validate_command_against_kg()` signature unchanged | None |
| `find_kg_dir()` returns different path than server | Both use same `docs/src/kg` convention | Low |

### Complexity Assessment
- **Lines changed**: ~100-150
- **New functions**: 2 (`build_kg_thesaurus_with_hash`, `get_thesaurus_with_auto_rebuild`)
- **Test additions**: 3-5 integration tests
- **Breaking changes**: None

## 8. Open Questions / Decisions for Human Review

All questions answered by reviewer:
1. ✅ Auto-detection chosen (vs explicit rebuild)
2. ✅ No persistence — compute fresh each session
3. ✅ Per-session check (first access per process)
4. ✅ No backward compatibility needed
5. ✅ Integration tests required

**Remaining decision**: Should `kg_validation.rs` also update the `terraphim_sessions` cache if it exists? (No — separate concern, can be future work)
