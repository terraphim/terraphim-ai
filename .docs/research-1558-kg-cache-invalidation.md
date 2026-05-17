# Research Document: #1558 Offline CLI KG Cache Invalidation

## 1. Problem Restatement and Scope

### Problem
The offline `terraphim-agent` CLI caches the knowledge graph (KG) thesaurus/automata at startup and never invalidates it. After writing new `.md` concept files to a role's `kg.knowledge_graph_local.path`, the cached thesaurus remains stale — new concepts are invisible to `extract`, `search`, and `suggest` commands until the server runs once.

### IN Scope
- Offline CLI (`terraphim_agent`) thesaurus cache invalidation
- KG file change detection mechanism
- `terraphim-agent extract`, `search`, `suggest` commands for offline use

### OUT of Scope
- Server-side cache management (already working, see `terraphim_service/src/lib.rs:524-577`)
- Tantivy full-text index (ADR rejection documented)
- Session search (separate feature)
- File watching / background processes

---

## 2. User & Business Outcomes

### User-Visible Behavior
**Before fix:** User writes new concept files → runs `terraphim-agent extract` → returns "No matches found" silently
**After fix:** User writes new concept files → runs `terraphim-agent extract` → returns correct matches (or new command explicitly rebuilds cache)

### Business Outcomes
- Offline CLI becomes a first-class citizen for KG authoring workflow
- Reduced confusion for new users following the offline quick-start path
- `kg-rlm-ingest` skill verification protocol works without server workaround

---

## 3. System Elements and Dependencies

| Component | Location | Role | Key Behavior |
|-----------|----------|------|--------------|
| `Thesaurus` | `terraphim_types/src/lib.rs:720` | KG concept index with `source_hash` field | Hash used for cache invalidation (server only) |
| `VALIDATION_KG_THESAURUS` | `terraphim_agent/src/kg_validation.rs:18` | Global OnceLock cache for offline CLI | **No staleness detection** |
| `build_kg_thesaurus_from_dir` | `terraphim_agent/src/learnings/capture.rs:726` | Builds thesaurus from KG `.md` files | **No source_hash computation** |
| `find_kg_dir` | `terraphim_agent/src/learnings/capture.rs:815` | Locates KG directory | Used by offline CLI |
| `compute_kg_source_hash` | `terraphim_automata/src/builder.rs:39` | Computes SHA-256 hash of KG files | **Server uses this; offline CLI does not** |
| `load_thesaurus_from_automata_path` | `terraphim_service/src/lib.rs:147` | Server-side thesaurus loading with staleness check | Reference implementation |
| `Cache::Flush` | `terraphim_agent/src/main.rs:934` | Manual cache flush command | **Exists but no rebuild** |
| `KgSub` enum | `terraphim_agent/src/main.rs:1232` | KG subcommands | **Only has `List`, no `Rebuild`** |

### Data Flow

**Server (working):**
```
KG files → compute_kg_source_hash() → compare with cached source_hash → if stale: rebuild
```

**Offline CLI (broken):**
```
KG files → build_kg_thesaurus_from_dir() → OnceLock (never invalidated)
```

---

## 4. Constraints and Their Implications

| Constraint | Why It Matters | Implication |
|------------|---------------|-------------|
| Must work offline | Core use case | Can't rely on server for cache invalidation |
| Minimal overhead | CLI responsiveness | Staleness check must be fast (single dir stat) |
| No breaking changes | Existing users | `cache flush` must still work |
| Fail-open design | `kg_validation.rs` docs | Cache errors should not break commands |
| KG path per-role | Multi-role support | Must handle per-role KG paths |

---

## 5. Risks, Unknowns, and Assumptions

### ASSUMPTIONS
- `compute_kg_source_hash` is cheap enough to call on every command (single `read_dir` + hash)
- The `source_hash` field in `Thesaurus` is already persisted and can be compared

### UNKNOWNS
- Whether `build_kg_thesaurus_from_dir` is used anywhere else besides validation

### RISKS
| Risk | Severity | Mitigation |
|------|----------|------------|
| Auto-detection adds latency to every extract/search | Medium | Only stat dir mtime, not full hash, on hot path |
| `OnceLock` prevents cache refresh without restart | High | Need alternative caching pattern |
| `source_hash` not stored in offline CLI persistence | High | May need to store hash separately |

---

## 6. Context Complexity vs. Simplicity Opportunities

### Complexity Sources
1. Two different caching patterns: `OnceLock` (offline CLI) vs `RwLock<Roles>` (server)
2. `Thesaurus.source_hash` exists but offline CLI doesn't populate or check it
3. Multiple code paths: `kg_validation.rs`, `learnings/capture.rs`, service `lib.rs`

### Simplification Strategies
1. **Proposal 2 (Auto-detection)** is simpler than Proposal 1 (explicit rebuild):
   - Only requires adding hash check before using cached thesaurus
   - No new CLI subcommand needed
   - Zero ergonomics impact

2. **Reuse existing infrastructure**:
   - `compute_kg_source_hash` already exists and is tested
   - `Thesaurus.source_hash` field already exists

3. **Separate cache from hot path**:
   - Check hash on command entry, not inside `OnceLock`
   - If stale, rebuild synchronously (acceptable for CLI use)

---

## 7. Questions for Human Reviewer

1. **Auto-detection vs explicit rebuild**: Should we auto-detect (rebuild on stale hash) or require explicit `kg rebuild` command? Auto-detection is better UX but changes the caching semantics.

2. **Hash storage**: The offline CLI uses `OnceLock` with no persistence. Should we store the `source_hash` in the persistence layer (`terraphim_persistence`) to survive restarts, or just compute it fresh each time?

3. **Hot path impact**: `extract`/`search`/`suggest` are called frequently. Should the hash check be:
   - (a) On every invocation (accurate but slower)
   - (b) On first call per session (fast but stale until restart)
   - (c) Background thread to pre-warm (complex)

4. **Backward compatibility**: The `cache flush` command exists. Should it also reset the new auto-detection state?

5. **Test strategy**: Should we add integration tests that write KG files and verify extract returns correct results?
