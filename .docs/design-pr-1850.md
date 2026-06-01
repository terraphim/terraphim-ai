# Implementation Plan: terraphim_grep KG Roles, Synonym Layers, and RLM Learning Loop

**Status**: Draft
**Research Doc**: `.docs/research-pr-1850.md`
**Author**: AI Agent
**Date**: 2026-05-31
**Estimated Effort**: 1 day (cleanup + rebase + tests)

## Overview

### Summary

Clean up and land PR #1850, which introduces role-based KG configurations, pre-built domain concept libraries, and an RLM learning feedback loop for `terraphim_grep`. The core feature is sound; the work is primarily removing artefacts, separating concerns, adding missing tests, and ensuring quality gates pass.

### Approach

1. **Clean the PR**: Remove learning artefacts and bundled formatting changes
2. **Rebase**: Apply on current `main` (23a4a953)
3. **Add tests**: Thesaurus generation validation, learning loop integration test
4. **Quality gate**: `cargo test`, `cargo clippy`, `cargo fmt`, `ubs`
5. **Merge**: After approval

### Scope

**In Scope:**
- Role configuration system (`.terraphim/config.toml`)
- Pre-built KG markdown files (3 roles x ~7 concepts each)
- Thesaurus JSON files for Aho-Corasick matching
- KG persistence in `KgCurationRlm` (`persist_concepts`)
- CLI `--kg-path` argument wiring

**Out of Scope:**
- Database-backed KG (future enhancement)
- Real-time sync across agents (future)
- Web UI for curation (separate product)
- Dynamic role discovery (v2)

**Avoid At All Cost** (from 5/25 analysis):
- Committing auto-generated session artefacts (`.terraphim/learnings/`)
- Bundling unrelated formatting changes with feature PRs
- Adding build-time thesaurus generation complexity (keep committed files for now)
- Supporting arbitrary LLM providers beyond OpenRouter in role configs (scope creep)

## Architecture

### Component Diagram

```
+--------------------------------------------------+
|                  terraphim_grep CLI               |
|  +---------------------------------------------+  |
|  |  Args: --role, --thesaurus, --kg-path,      |  |
|  |        --role-config, --answer              |  |
|  +---------------------------------------------+  |
+-----------------------+--------------------------+
                        |
                        v
+--------------------------------------------------+
|              TerraphimGrep (orchestrator)         |
|  +----------------+  +-------------------------+  |
|  |  fff-search    |  |  KG Boost (Aho-Corasick)|  |
|  +----------------+  +-------------------------+  |
|  +----------------+  +-------------------------+  |
|  | Sufficiency    |  |  RLM Fallback           |  |
|  | Judge          |  |  (LLM synthesis)        |  |
|  +----------------+  +-------------------------+  |
+-----------------------+--------------------------+
                        |
                        v
+--------------------------------------------------+
|              KgCurationRlm (learning loop)        |
|  +---------------------------------------------+  |
|  |  extract_and_index() -> persist_concepts()  |  |
|  |  Writes: .terraphim/kg/{role}/learned-*.md  |  |
|  +---------------------------------------------+  |
+-----------------------+--------------------------+
                        |
                        v
+--------------------------------------------------+
|              Knowledge Graph (filesystem)         |
|  .terraphim/kg/{role}/                            |
|    - concept-name.md (hand-curated)               |
|    - learned-slug.md (auto-generated)             |
|  .terraphim/thesaurus-{role}.json (generated)     |
+--------------------------------------------------+
```

### Data Flow

```
User Query
  -> CLI parses --role, loads role config
  -> TerraphimGrep loads thesaurus for role
  -> fff-search finds matching chunks
  -> KG boost: Aho-Corasick matches concepts against chunks
  -> Sufficiency judge: Sufficient / NeedsSynthesis / Insufficient
    -> Sufficient: return ranked results with KG highlighting
    -> Insufficient: RLM fallback
      -> Build RlmContext (chunks + KG concepts)
      -> Send to LLM for synthesis
      -> Return answer with citations
      -> KgCurationRlm.extract_and_index() parses LLM response
        -> persist_concepts() writes new concepts to KG directory
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Filesystem-based KG (markdown) | Simple, version-controllable, matches existing `.docs/` pattern | Database (complexity), JSON only (less readable) |
| Role-scoped KGs | Prevents domain mixing; enables role-specific thesauri | Global KG with tags (filtering complexity) |
| Skip existing files in persist | Prevents overwrites; manual curation takes precedence | Always overwrite (loses manual edits) |
| Commit thesaurus JSON files | Reproducible builds; no build-time dependency | Build-time generation (adds complexity) |
| OpenRouter free models in role configs | Zero cost; sufficient for curation | Paid models (cost), local models (speed) |

### Eliminated Options (Essentialism)

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| Runtime thesaurus regeneration | Adds build dependency; complexity not justified for v1 | Maintenance burden, slower builds |
| Global KG with role tags | Complex filtering logic; no clear concept ownership | Performance hit, confusing UX |
| Database persistence (SQLite) | Overkill for CLI tool; filesystem is sufficient | Deployment complexity, schema migrations |
| Multi-tenant role isolation | Single-user desktop tool; no tenant concept | Massive scope increase |
| Automatic concept merging/updating | Complex conflict resolution; skip-on-exist is simpler | Data loss risk, merge algorithm needed |

### Simplicity Check

> "Minimum code that solves the problem. Nothing speculative."

**What if this could be easy?**

The simplest design: users create markdown files in `.terraphim/kg/{role}/`. `terraphim_grep` reads them at startup, builds an Aho-Corasick automaton, and uses it for KG boost. When RLM runs, new concepts are appended as new files. No database, no complex sync, no runtime generation.

**Current design matches this.** The only additions are:
- Role config file (`.terraphim/config.toml`)
- Pre-built concept libraries
- `--kg-path` CLI flag
- `persist_concepts()` method

**Senior Engineer Test**: A senior engineer would recognise this as the minimal viable approach. No over-engineering detected.

**Nothing Speculative Checklist**:
- [x] No features the user didn't request
- [x] No abstractions "in case we need them later"
- [x] No flexibility "just in case"
- [x] No error handling for scenarios that cannot occur
- [x] No premature optimization

## File Changes

### New Files

| File | Purpose |
|------|---------|
| `.terraphim/config.toml` | Role definitions and configuration |
| `.terraphim/kg/ai-engineer/*.md` | Pre-built AI/ML concepts |
| `.terraphim/kg/devops/*.md` | Pre-built DevOps concepts |
| `.terraphim/kg/rust-engineer/*.md` | Pre-built Rust concepts |
| `.terraphim/role-ai-engineer.json` | AI engineer role LLM config |
| `.terraphim/role-devops.json` | DevOps role LLM config |
| `.terraphim/role-rust-engineer.json` | Rust engineer role LLM config |
| `.terraphim/thesaurus-ai-engineer.json` | Aho-Corasick thesaurus for AI role |
| `.terraphim/thesaurus-devops.json` | Aho-Corasick thesaurus for DevOps role |
| `.terraphim/thesaurus-rust-engineer.json` | Aho-Corasick thesaurus for Rust role |

### Modified Files

| File | Changes |
|------|---------|
| `crates/terraphim_grep/src/kg_curation.rs` | Add `kg_path` field, `with_kg_path()`, `persist_concepts()` |
| `crates/terraphim_grep/src/main.rs` | Add `--kg-path` arg, wire `KgCurationRlm` with path |
| `.gitignore` | Add `.terraphim/learnings/` |

### Deleted Files (from PR cleanup)

| File | Reason |
|------|--------|
| `.terraphim/learnings/*.md` | Session artefacts; should not be committed |

## API Design

### Public Types (no changes to existing public API)

The changes are internal to `terraphim_grep` and additive:

```rust
// KgCurationRlm - already pub, extended with kg_path
#[cfg(feature = "llm")]
pub struct KgCurationRlm {
    llm_client: Arc<dyn LlmClient>,
    kg_path: Option<std::path::PathBuf>,  // NEW
}

impl KgCurationRlm {
    pub fn new(llm_client: Arc<dyn LlmClient>) -> Self;  // Existing
    pub fn with_kg_path(mut self, path: std::path::PathBuf) -> Self;  // NEW
    pub async fn extract_and_index(&self, query: &str, chunks: &[Chunk],
    ) -> Result<Vec<NewConcept>>;  // Existing, now calls persist_concepts
}
```

### CLI Arguments

```rust
#[derive(Parser)]
struct Args {
    // ... existing args ...
    
    /// KG directory for persisting learned concepts
    #[arg(long)]
    kg_path: Option<PathBuf>,
}
```

### Error Types (no new errors)

Uses existing error handling:
- `log::warn!()` for I/O failures (non-fatal)
- `Result` from `extract_and_index()` for LLM/signature failures

## Test Strategy

### Unit Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_persist_concepts_writes_markdown_files` | `kg_curation.rs` | Verify concept -> markdown file creation |
| `test_persist_concepts_skips_existing_files` | `kg_curation.rs` | Verify no overwrites |
| `test_persist_concepts_empty_is_noop` | `kg_curation.rs` | Verify empty concept list is no-op |
| `test_persist_concepts_slug_generation` | `kg_curation.rs` | Verify special chars replaced with hyphens |

### Integration Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_kg_learning_loop` | `tests/integration.rs` | End-to-end: query -> RLM -> KG file created |
| `test_role_config_loading` | `tests/integration.rs` | Verify role configs parse correctly |

### Property Tests

Not required for this feature -- behaviour is deterministic file I/O.

## Implementation Steps

### Step 1: Clean PR (Remove Artefacts)
**Files:** `.terraphim/learnings/*`, `.gitignore`
**Description:** Remove learning artefacts; add `.terraphim/learnings/` to `.gitignore`
**Tests:** N/A
**Estimated:** 15 minutes

```bash
git rm .terraphim/learnings/*.md
echo ".terraphim/learnings/" >> .gitignore
```

### Step 2: Extract Formatting Changes
**Files:** `crates/terraphim_orchestrator/src/control_plane/output_parser.rs`, `crates/terraphim_orchestrator/src/lib.rs`, `crates/terraphim_spawner/src/config.rs`
**Description:** Remove `cargo fmt` changes from PR; they will be applied separately
**Tests:** N/A
**Estimated:** 15 minutes

```bash
# Revert fmt-only changes in unrelated crates
git checkout main -- crates/terraphim_orchestrator/src/control_plane/output_parser.rs
git checkout main -- crates/terraphim_orchestrator/src/lib.rs
git checkout main -- crates/terraphim_spawner/src/config.rs
```

### Step 3: Rebase on Current Main
**Files:** All PR files
**Description:** Rebase branch on `main` (23a4a953) to resolve any conflicts
**Tests:** `cargo build --workspace`
**Estimated:** 30 minutes

```bash
git fetch origin
git rebase origin/main
```

### Step 4: Add Thesaurus Generation Documentation
**Files:** `.terraphim/README.md` (new)
**Description:** Document how thesaurus files are generated from KG markdown
**Tests:** N/A
**Estimated:** 30 minutes

```markdown
# Thesaurus Generation

Thesaurus JSON files are generated from KG markdown files:

```bash
cd .terraphim
# Future: cargo run --bin generate-thesaurus -- --role rust-engineer
```

For now, regenerate manually when KG markdown changes.
```

### Step 5: Add Integration Test for Learning Loop
**Files:** `crates/terraphim_grep/tests/integration.rs` (new or existing)
**Description:** Test that RLM synthesis creates KG files
**Tests:** New integration test
**Estimated:** 2 hours

```rust
#[tokio::test]
#[cfg(feature = "llm")]
async fn test_kg_learning_loop() {
    let tmp = tempfile::TempDir::new().unwrap();
    let kg_path = tmp.path().join("kg");
    
    // Setup: create TerraphimGrep with mock LLM
    // Query with --answer to trigger RLM
    // Assert: KG file created in kg_path
}
```

### Step 6: Verify Role Configs Have No Secrets
**Files:** `.terraphim/role-*.json`
**Description:** Ensure no API keys committed in role configs
**Tests:** Manual review + grep
**Estimated:** 15 minutes

```bash
grep -r "api_key\|sk-or\|sk-" .terraphim/role-*.json || echo "No secrets found"
```

### Step 7: Run Quality Gates
**Files:** All
**Description:** Full quality verification
**Tests:** `cargo test`, `cargo clippy`, `cargo fmt --check`, `ubs`
**Estimated:** 30 minutes

```bash
cargo test --workspace
cargo clippy --workspace
cargo fmt --all -- --check
ubs --only=rust crates/terraphim_grep/
```

### Step 8: Update PR and Merge
**Files:** N/A
**Description:** Push cleaned branch, update PR description, merge
**Tests:** N/A
**Estimated:** 15 minutes

## Rollback Plan

If issues discovered:
1. Revert merge commit: `git revert -m 1 <merge-commit>`
2. The `.terraphim/` directory can be safely deleted; no database migration needed
3. CLI remains backward-compatible without `--kg-path`

## Dependencies

### New Dependencies

None. Uses existing crates.

### Dependency Updates

None.

## Performance Considerations

### Expected Performance

| Metric | Target | Measurement |
|--------|--------|-------------|
| KG persistence | < 10ms per concept | Benchmark `persist_concepts` |
| Thesaurus loading | < 50ms at startup | Time `Args` parsing + file read |
| Memory per role | < 1MB for 50 concepts | Profile Aho-Corasick automaton |

### Benchmarks to Add

```rust
#[bench]
fn bench_persist_concepts(b: &mut Bencher) {
    let tmp = tempfile::TempDir::new().unwrap();
    let concepts = vec![/* 10 concepts */];
    b.iter(|| persist_concepts(&concepts, "query", tmp.path()));
}
```

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Thesaurus generation script | Deferred | Future PR |
| Dynamic role discovery | Deferred | Future PR |
| User config directory override | Deferred | Future PR |
| Integration test for learning loop | In Plan | This PR |

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Performance targets agreed
- [ ] Human approval received
