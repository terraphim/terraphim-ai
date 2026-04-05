# Research Document: Remaining PR Merge Follow-Up Work

**Status**: Approved
**Author**: Agent
**Date**: 2026-04-05
**Reviewers**: Alex

## Executive Summary

Three work items remain after closing all 13 Gitea PRs: (1) refactoring `shared_learning/store.rs` from direct `sqlx` to `terraphim_persistence`, (2) upgrading serenity to fix RUSTSEC-2026-0049 in the Discord channel adapter, and (3) coordinating the NormalizedTerm ID type (u64 vs String) for the mention.rs automata migration. Research finds item 1 is self-contained, item 2 has a clear but breaking path, and item 3 is blocked on an architectural decision.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Security posture improvement, architecture consistency, unblocking PR #185 value |
| Leverages strengths? | Yes | `terraphim_persistence` already exists and is well-proven by `terraphim_usage` |
| Meets real need? | Yes | CVE fix required; shared_learning is dead code until persistence refactor completes |

**Proceed**: Yes (3/3)

## Problem Statement

### Description

Three separate but related issues remain after the PR merge campaign:

1. **shared_learning uses direct sqlx** instead of the project's `terraphim_persistence` layer (OpenDAL-backed key-value storage). The module is disabled and contributes zero value until refactored.

2. **RUSTSEC-2026-0049 (rustls-webpki CRL bypass)** is still exploitable via `serenity 0.12.5 -> hyper-rustls 0.24 -> rustls 0.21 -> rustls-webpki 0.101.7`. Our Cargo.toml git patches fix the top-level dependency path but serenity's transitive chain is unaffected.

3. **NormalizedTerm ID migration** (u64 -> String) was attempted in PR #185, reverted on gitea main (`9c8dd28f`), and is now a blocking question for the mention.rs automata rewrite.

### Impact

- Item 1: ~2062 lines of disabled code that should be providing shared learning across agents
- Item 2: Known CVE in production dependency chain
- Item 3: Blocks the Aho-Corasick mention detection rewrite (PR #185's mention.rs)

### Success Criteria

1. `shared_learning` module compiles, builds with workspace, uses `terraphim_persistence` for storage
2. No vulnerable `rustls-webpki 0.101.x` in `cargo tree` output
3. A documented decision on NormalizedTerm ID type with clear rationale

## Current State Analysis

### Item 1: shared_learning Persistence

**Current code location**: `crates/terraphim_agent/src/shared_learning/` (4 files, 2062 lines, disabled)

| Component | Location | Purpose |
|-----------|----------|---------|
| `mod.rs` | `shared_learning/mod.rs` | Module root, re-exports |
| `types.rs` | `shared_learning/types.rs` | `SharedLearning`, `TrustLevel`, `QualityMetrics`, `LearningSource` (575 lines) |
| `store.rs` | `shared_learning/store.rs` | SQLite-backed store with BM25 dedup (952 lines, 11 sqlx queries) |
| `wiki_sync.rs` | `shared_learning/wiki_sync.rs` | Gitea wiki sync via `gitea-robot` CLI (508 lines) |

**Store.rs public API** (what must be preserved):
- `SharedLearningStore::open(config)` -- create/open store
- `store_with_dedup(learning)` -- insert with BM25 deduplication check
- `find_similar(query, limit)` -- BM25 similarity search
- `get(id)` / `list_all()` / `list_by_trust_level(level)` -- queries
- `promote_to_l2(id)` / `promote_to_l3(id)` -- trust escalation
- `record_application(id, agent, effective)` -- usage tracking
- `close()` -- graceful shutdown

**How `terraphim_persistence` works** (model: `terraphim_usage/src/store.rs`):
- Types implement `Persistable` trait (`Serialize + DeserializeOwned`)
- Each record gets a unique key (e.g., `usage/metrics/{agent_name}.json`)
- `DeviceStorage` provides OpenDAL operators (filesystem, memory, sqlite, dashmap backends)
- No SQL -- pure key-value JSON storage
- Backends configurable via `DeviceSettings` profiles

**Key constraint**: BM25 similarity search requires scanning multiple documents. The current sqlx approach runs SQL queries with term frequencies. With key-value persistence, we'd need to load and scan records in-memory. This is feasible for the expected dataset size (hundreds to low thousands of learnings).

### Item 2: Serenity/Discord CVE

**Vulnerable chain**:
```
terraphim_tinyclaw (discord feature)
  -> serenity 0.12.5
    -> hyper-rustls 0.24.2
      -> rustls 0.21.12
        -> rustls-webpki 0.101.7 (VULNERABLE)
```

**Current state**:
- `serenity` 0.12.5 is the latest crates.io release
- serenity's `next` branch has API-breaking changes (no `EventHandler::message()`, `ChannelId::say()` removed, etc.)
- PR #353 tried upgrading to `next` branch but broke 5 compilation points in `discord.rs`
- The `discord` feature in tinyclaw is optional (`default = ["telegram", "discord"]`)

**Affected file**: `crates/terraphim_tinyclaw/src/channels/discord.rs` (~190 lines)

**Serenity `next` API changes observed**:
1. `EventHandler::message()` removed (new trait name/method)
2. `msg.author.bot` -> `msg.author.bot()` (field -> method)
3. `msg.content` type changed (needs `.as_str()`)
4. `ChannelId::say()` removed (new API pattern)
5. `Client::builder().event_handler(handler)` expects `Arc<Handler>` instead of `Handler`

### Item 3: NormalizedTerm ID Type

**Current state on main**:
```rust
// terraphim_types/src/lib.rs:286
pub struct NormalizedTerm {
    pub id: u64,  // integer ID
    ...
}

pub struct Concept {
    pub id: u64,  // integer ID
    ...
}
```

**What PR #185 tried**: Change `id: u64` to `id: String` (UUID-based) across all types.

**What gitea/main did**: 
- `e0f98ee6`: Changed u64 -> String
- `9c8dd28f`: **Reverted** String -> u64 ("revert ID types from String/UUID to u64 integer")

**Impact of u64 ID**:
- `NormalizedTerm::new(id: u64, ...)` -- callers must provide integer IDs
- Used in: `terraphim_file_search`, `terraphim_hooks`, `terraphim_automata`, `terraphim_orchestrator`
- PR #185's `mention.rs` passed `format!("cap-{}", idx)` (String) as first arg, causing type mismatch

**Files using NormalizedTerm**:
- `crates/terraphim_file_search/src/watcher.rs` -- uses `NormalizedTerm { id: counter, ... }` directly
- `crates/terraphim_file_search/benches/kg_scoring.rs` -- same pattern
- `crates/terraphim_hooks/src/replacement.rs` -- `NormalizedTerm::new(1u64, ...)`
- `crates/terraphim_automata/` -- uses NormalizedTerm via Thesaurus

## Constraints

### Technical Constraints
- **No sqlx in shared_learning**: Must use `terraphim_persistence` (OpenDAL)
- **Serenity 0.12.5 is the latest stable**: No fixed version exists on crates.io
- **NormalizedTerm.id is u64**: Reverted from String; any change requires coordinated migration across 4+ crates
- **BM25 dedup**: Must preserve similarity search capability after persistence refactor

### Business Constraints
- Discord adapter is optional (feature-gated) -- can be disabled without breaking core
- shared_learning is new code (no existing users) -- breaking changes are acceptable
- Security CVE should be resolved promptly

### Non-Functional Requirements
| Requirement | Target | Notes |
|-------------|--------|-------|
| Build | `cargo build --workspace` passes | Zero errors |
| Persistence | Use existing OpenDAL backends | No new dependencies |
| CVE | No rustls-webpki < 0.102.x | Verify via `cargo tree` |

## Vital Few (Essentialism)

### Essential Constraints (Max 3)

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Use terraphim_persistence | Architecture consistency; avoids adding sqlx dep | AGENTS.md, all other modules use it |
| Fix RUSTSEC-2026-0049 | Known CVE in dependency chain | cargo audit, deny.toml |
| Keep NormalizedTerm.id as u64 | Reverted decision; changing needs full crate migration | gitea commit 9c8dd28f |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Full UUID migration (u64 -> String) for NormalizedTerm | Gitea already reverted this; not our decision to re-make |
| BM25 scoring optimization | Premature; in-memory scan is fine for expected dataset |
| Wiki sync refactor | Works fine as-is using `gitea-robot` CLI |
| Serenity `next` branch full migration | Too many breaking changes; simpler alternatives exist |

## Dependencies

### Internal Dependencies

| Dependency | Impact | Risk |
|------------|--------|------|
| `terraphim_persistence` | Must implement `Persistable` for `SharedLearning` | Low -- well-documented pattern |
| `terraphim_types` | `SharedLearning` types must serialize cleanly | Low -- already uses serde |
| `terraphim_tinyclaw` | Discord channel adapter | Low -- feature-gated, optional |

### External Dependencies

| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| `serenity` | 0.12.5 | CVE in transitive deps | Disable discord feature; or patch hyper-rustls |
| `sqlx` | N/A | Removing dependency | N/A -- replacing with terraphim_persistence |

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| BM25 perf regression in key-value store | Low | Medium | Profile; optimize later if needed |
| Serenity API breaks during upgrade | High | Low | Feature-gate discord; defer migration |
| NormalizedTerm migration breaks automata | Medium | High | Keep u64; adapt mention.rs to use u64 IDs |

### Open Questions

1. **Should the Discord adapter be disabled until serenity releases 0.13?** -- Needs Alex's input
2. **Is in-memory BM25 scan acceptable for shared_learning?** -- Expected dataset is hundreds of learnings; should be fine
3. **Should mention.rs automata rewrite use u64 IDs with a lookup table instead of String names?** -- Needs design decision

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| Dataset will be < 10K learnings | New feature, limited agents | Need pagination/streaming | No |
| serenity 0.13 will fix the CVE | Upstream pattern | Extended vulnerability window | No |
| u64 IDs are permanent decision | Gitea revert commit | Re-migration effort | Partially |

### Multiple Interpretations Considered

| Interpretation | Implications | Status |
|----------------|--------------|--------|
| **Disable discord feature** | Removes CVE vector; loses Discord integration | Viable short-term |
| **Patch hyper-rustls in serenity** | Fork serenity, patch deps | High maintenance burden |
| **Wait for serenity 0.13** | CVE remains until release | Risky for production |
| **Use serenity `next` branch** | API breaks need fixing | PR #353 showed 5 break points |

## Research Findings

### Key Insights

1. **shared_learning refactor is straightforward**: The `Persistable` trait pattern from `terraphim_usage` maps directly. Each `SharedLearning` record gets key `shared-learning/{id}.json`. BM25 search loads records and scores in-memory.

2. **Serenity CVE is best fixed by disabling the feature**: serenity 0.12.5 is the latest stable; the `next` branch has too many breaking changes. Disabling `discord` from default features removes the vulnerable dependency chain entirely.

3. **NormalizedTerm ID decision is already made**: gitea main reverted String -> u64. The mention.rs automata rewrite should adapt to u64 IDs using a counter or hash, not try to change the type back.

4. **wiki_sync.rs has zero sqlx dependency**: It only uses `std::process::Command` to call `gitea-robot`. No changes needed.

5. **types.rs has zero sqlx dependency**: Pure data types with serde. No changes needed beyond possibly removing the `uuid` dependency (currently unused since IDs are strings generated in store.rs).

### Relevant Prior Art

- `terraphim_usage/src/store.rs`: Proven pattern for Persistable records with OpenDAL
- `terraphim_agent_evolution/src/memory.rs`: Persistable for versioned state objects
- `terraphim_persistence/src/thesaurus.rs`: Persistable for Thesaurus (similar domain)

## Recommendations

### Proceed/No-Proceed

**Proceed** on all three items with the following prioritization:

1. **Item 2 (Serenity CVE)** -- Highest priority, simplest fix (disable feature default)
2. **Item 1 (shared_learning refactor)** -- Medium priority, self-contained
3. **Item 3 (NormalizedTerm)** -- Lowest priority, needs design before code

### Scope Recommendations

- Item 1: Refactor `store.rs` only (952 lines -> ~400 lines). Keep `types.rs` and `wiki_sync.rs` as-is.
- Item 2: Remove `discord` from default features in tinyclaw. Document how to re-enable.
- Item 3: Do not change NormalizedTerm.id type. Create a new issue for the automata mention rewrite that works with u64 IDs.

### Risk Mitigation Recommendations

- Build-verify after each change
- Feature-gate shared_learning behind `shared-learning` feature flag
- Keep `deny.toml` RUSTSEC-2026-0049 suppression until serenity releases a fix

## Next Steps

1. Alex approves/rejects this research
2. If approved, proceed to Phase 2 (Design) with implementation plan
3. Implementation follows disciplined-development Phase 3

## NormalizedTerm ID Decision (2026-04-05)

**Decision**: Keep `NormalizedTerm.id` and `Concept.id` as `u64` (integer IDs). Do NOT change to String/UUID.

**Rationale**:
- The u64 -> String migration was attempted on gitea main (`e0f98ee6`) but deliberately reverted (`9c8dd28f`: "revert ID types from String/UUID to u64 integer")
- The revert was intentional and represents an architectural decision made after evaluating the impact
- Changing to String/UUID would require cascading updates across 5+ crates: `terraphim_file_search`, `terraphim_hooks`, `terraphim_automata`, `terraphim_orchestrator`, `terraphim_types`
- The automata mention rewrite (PR #185's `mention.rs`) should adapt to use u64 IDs via a counter or hash, NOT change the type

**Implications for mention.rs rewrite**:
- Use `NormalizedTerm::with_auto_id()` or `NormalizedTerm::new(counter, ...)` with a simple u64 counter
- Do NOT use `format!("cap-{}", idx)` as the first argument -- that's a String, not u64
- Example fix: `NormalizedTerm::new(idx as u64, key.clone().into())` where `idx` is a `usize` counter

**Follow-up**: File a separate Gitea issue for the automata mention detection rewrite with explicit u64 ID usage.

## Appendix

### Code Snippets

**Persistable pattern (from terraphim_usage)**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetricsRecord {
    pub key: String,
    pub agent_name: String,
    // ... fields
}

impl Persistable for AgentMetricsRecord {
    fn new(key: String) -> Self { Self { key, ..Default::default() } }
    async fn save(&self) -> terraphim_persistence::Result<()> {
        self.save_to_all().await
    }
    fn get_key(&self) -> String {
        format!("usage/metrics/{}.json", self.normalize_key(&self.agent_name))
    }
}
```

**Vulnerable dependency chain**:
```
terraphim_tinyclaw --features discord
  -> serenity 0.12.5
    -> hyper-rustls 0.24.2
      -> rustls 0.21.12
        -> rustls-webpki 0.101.7  <-- CVE
```
