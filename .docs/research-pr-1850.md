# Research Document: terraphim_grep KG Roles, Synonym Layers, and RLM Learning Loop

**Status**: Draft
**Author**: AI Agent
**Date**: 2026-05-31
**Reviewers**: TBD

## Executive Summary

PR #1850 introduces role-based knowledge graph (KG) configurations for `terraphim_grep`, enabling domain-specific search enhancement through pre-built concept libraries (DevOps, Rust Engineer, AI Engineer). It adds a feedback loop where the RLM (Recursive Language Model) synthesis step auto-extracts and persists new concepts back into the KG, allowing the knowledge graph to grow with usage. The PR also includes pre-generated thesauri for Aho-Corasick fast matching and bundled `cargo fmt` changes across multiple files.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | This directly advances Terraphim's core value proposition -- intelligent, context-aware code search that improves with use |
| Leverages strengths? | Yes | Builds on existing `terraphim_grep` architecture, `KgCurationRlm`, and Aho-Corasick matching already in the codebase |
| Meets real need? | Yes | Domain-specific search (DevOps vs Rust vs AI) requires different vocabularies; static KGs become stale without learning |

**Proceed**: Yes -- 3/3 YES

## Problem Statement

### Description

`terraphim_grep` currently supports KG-boosted search and RLM synthesis, but:
1. **No domain segmentation**: All searches use the same KG, mixing DevOps, Rust, and AI concepts
2. **Static KGs**: Knowledge graphs are hand-curated and do not evolve with usage
3. **No role-based configuration**: Users cannot switch between domain contexts (e.g., "search as a DevOps engineer" vs "search as a Rust developer")
4. **Missing pre-built concept libraries**: Users must build their own KGs from scratch

### Impact

- Users get irrelevant KG matches when searching across domains
- The system does not improve with usage -- no learning loop
- High barrier to entry: requires manual KG curation before useful search

### Success Criteria

- [ ] Search can be scoped to a role (devops/rust-engineer/ai-engineer)
- [ ] Pre-built KGs provide immediate value for common domains
- [ ] RLM synthesis feeds back into KG automatically (concepts learned from LLM responses)
- [ ] Thesaurus files enable fast Aho-Corasick matching against role-specific synonyms
- [ ] CLI supports `--kg-path` to enable/disable curation

## Current State Analysis

### Existing Implementation

| Component | Location | Purpose |
|-----------|----------|---------|
| `terraphim_grep` | `crates/terraphim_grep/` | Main grep crate with fff-search, KG boost, sufficiency judge, RLM fallback |
| `KgCurationRlm` | `crates/terraphim_grep/src/kg_curation.rs` | Extracts concepts from LLM responses using `ConceptExtractionSignature` |
| `ConceptExtractionSignature` | `crates/terraphim_grep/src/signatures.rs` | Parses LLM output into structured `NewConcept` objects |
| `NewConcept` | `crates/terraphim_grep/src/signatures.rs` | Struct with `name`, `synonyms`, `relationships` |
| `TerraphimGrep` | `crates/terraphim_grep/src/lib.rs` | Main search orchestrator with `.with_llm_client()` and `.with_kg_curation()` |
| `Args` | `crates/terraphim_grep/src/main.rs` | CLI args including `--role`, `--thesaurus`, `--role-config`, `--answer` |

### Data Flow (Current)

```
Query -> fff-search -> KG boost (Aho-Corasick) -> Sufficiency Judge
  -> Sufficient: return ranked results
  -> Insufficient: RLM fallback -> LLM synthesis -> return answer with citations
```

The RLM fallback generates an answer but **discards** the LLM's latent knowledge -- no concepts are extracted or persisted.

### Integration Points

- **LLM client**: `Arc<dyn LlmClient>` wired via `--role-config` JSON or env vars
- **Thesaurus**: JSON file loaded for Aho-Corasick matching (currently manual)
- **KG**: No persistence layer -- concepts exist only in memory during extraction

## Constraints

### Technical Constraints

- **Rust ecosystem**: Must use existing `terraphim_grep` patterns (async, `Arc<dyn>`, `thiserror`)
- **Aho-Corasick**: The `aho-corasick` crate already used for fast substring matching
- **File I/O**: KG persistence is filesystem-based (markdown files) -- no database dependency
- **Feature flags**: `llm` feature gates all LLM-dependent code

### Business Constraints

- **OpenRouter dependency**: Role configs reference OpenRouter free models (`qwen/qwen3-coder:free`)
- **Free model reliability**: Free tier may have rate limits or availability issues
- **No breaking changes**: Must remain backward-compatible with existing `--thesaurus` usage

### Non-Functional Requirements

| Requirement | Target | Current |
|-------------|--------|---------|
| Search latency | < 500ms for code repos | Baseline fff-search |
| KG persistence | Atomic writes, no corruption | N/A (new) |
| Memory | KG loaded on-demand per role | All concepts loaded globally |

## Vital Few (Essentialism)

### Essential Constraints (Max 3)

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Role-scoped search | Without this, KG mixing produces false positives | User reports irrelevant matches when searching across domains |
| KG learning loop | Static KGs become stale; manual curation does not scale | Pre-built KGs have ~20 concepts each; real usage generates 100s |
| Backward compatibility | Existing users have scripts and workflows using `--thesaurus` | CLI already has established interface |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Database-backed KG | File-based markdown is simpler, version-controllable, and matches existing `.docs/` pattern |
| Real-time KG sync across agents | Out of scope for single-user CLI tool; can be added later via git |
| Automatic thesaurus regeneration | Build-time generation acceptable; runtime regeneration adds complexity |
| Web UI for KG curation | CLI-only tool; web UI is separate product |
| Multi-tenant role isolation | Single-user desktop tool; no tenant concept |

## Dependencies

### Internal Dependencies

| Dependency | Impact | Risk |
|------------|--------|------|
| `terraphim_types::Role` | Role config deserialization | Low -- stable type |
| `terraphim_grep::KgCurationRlm` | Core curation logic extended | Low -- additive changes only |
| `terraphim_grep::TerraphimGrep` | Wiring point for KG curation | Low -- builder pattern supports `.with_*()` |

### External Dependencies

| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| `aho-corasick` | 1.x | Low | No alternative needed |
| `serde_json` | 1.x | Low | Standard |
| `tempfile` (dev) | 3.x | Low | Standard for tests |
| `tokio` | 1.x | Low | Async runtime |

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Learning artefacts committed | High (already in PR) | Medium (repo bloat) | Remove `.terraphim/learnings/` before merge |
| Thesaurus JSON drift from KG markdown | Medium | Medium (stale synonyms) | Document regeneration command; CI check |
| Free model rate limiting | Medium | Low (graceful degradation) | Fall back to search-only mode |
| Bundled `cargo fmt` changes | High (already in PR) | Low (review noise) | Separate fmt changes into dedicated PR |
| Concept name collisions | Low | Medium (overwrites) | Slug-based filenames with collision skip logic |

### Open Questions

1. **Should thesaurus files be committed or generated at build time?**
   - Committed: Ensures reproducible builds, but may drift from KG markdown
   - Generated: Always consistent, but adds build dependency
   - **Recommendation**: Commit for now, add `build.rs` generation later

2. **How are roles discovered?**
   - Currently: Hardcoded in `.terraphim/config.toml`
   - Future: Dynamic discovery from `.terraphim/kg/*/`
   - **Recommendation**: Hardcoded list is fine for v1

3. **Should `.terraphim/` be in repo root or user config dir?**
   - Repo root: Version controlled, shared across team
   - User config (`~/.config/terraphim/`): User-specific customisation
   - **Recommendation**: Repo root for defaults; user dir for overrides (future)

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| Users will run `terraphim-grep` from repo root where `.terraphim/` exists | CLI is repo-scoped | Commands fail if run from subdir | No -- needs documentation |
| LLM responses contain extractable concepts reliably | `ConceptExtractionSignature` prompt engineering | Low-quality concepts persisted | Partial -- has tests |
| Markdown is sufficient KG format forever | Current `.docs/` pattern | May need structured format later | No -- but migration path exists |
| Free OpenRouter models are sufficient for curation | Cost constraint | Paid models may be needed for quality | No -- monitor quality |

### Multiple Interpretations Considered

| Interpretation | Implications | Why Chosen/Rejected |
|----------------|--------------|---------------------|
| **A**: Role = search context only | Simple, but no persistent learning | Rejected -- does not solve staleness |
| **B**: Role = KG namespace + learning loop | Concepts scoped to role, KGs grow | **Chosen** -- addresses both segmentation and learning |
| **C**: Global KG with role tags | Single KG, concepts tagged by role | Rejected -- complex filtering, no clear ownership |

## Research Findings

### Key Insights

1. **The PR bundles formatting changes**: ~30 of 34 files are `cargo fmt` changes in unrelated crates (`terraphim_orchestrator`, `terraphim_spawner`). These should be separated.

2. **Learning artefacts committed**: `.terraphim/learnings/` contains session output files (`learning-*.md`) that appear to be auto-generated during agent runs. These should be gitignored, not committed.

3. **Thesaurus files are large but auto-generated**: `thesaurus-rust-engineer.json` is 465 lines; generation logic should be documented or scripted.

4. **KG markdown format is consistent**: All `.terraphim/kg/**/*.md` files follow the same schema:
   ```markdown
   # Concept Name
   
   Description paragraph.
   
   synonyms:: term1, term2, term3
   related:: concept1, concept2
   context:: domain
   cost:: low|medium|high
   ```

5. **No tests for thesaurus generation**: The thesaurus JSON files have no corresponding test or generation script in the PR.

6. **Role configs duplicate model settings**: Each `role-*.json` specifies `llm_provider`, `llm_model`, `api_key` -- potential security risk if keys committed.

### Relevant Prior Art

- **Terraphim KG system**: Existing `.docs/` pattern uses markdown with frontmatter-like syntax
- **Aho-Corasick in terraphim_grep**: Already used for fast concept matching; thesaurus extends this
- **OpenRouter integration**: Recently added in PR #1909 (smoke tests with free models)

### Technical Spikes Needed

| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| Thesaurus generation script | Document how `thesaurus-*.json` is generated from KG markdown | 2 hours |
| Role config validation | Ensure `role-*.json` files are valid and keys are redacted | 1 hour |
| Learning loop integration test | End-to-end test: query -> RLM -> KG file created | 4 hours |

## Recommendations

### Proceed/No-Proceed

**Proceed with conditions**:
1. Remove `.terraphim/learnings/` from PR (add to `.gitignore`)
2. Separate `cargo fmt` changes into dedicated PR
3. Add thesaurus generation script or documentation
4. Verify no API keys in committed `role-*.json` files

### Scope Recommendations

- **Keep**: Role configs, KG markdown, thesaurus files, KG persistence, CLI wiring
- **Remove**: Learning artefacts, bundled formatting changes
- **Add**: Thesaurus generation script, integration test for learning loop

### Risk Mitigation Recommendations

1. **Pre-merge checklist**:
   - [ ] No secrets in role configs
   - [ ] `.terraphim/learnings/` removed and gitignored
   - [ ] `cargo fmt` changes extracted
   - [ ] Thesaurus generation documented

2. **Post-merge**:
   - Add CI check for thesaurus freshness (regenerate and diff)
   - Monitor OpenRouter free model availability
   - Add `.terraphim/` to `.gitignore` template for new repos

## Next Steps

If approved:
1. Clean PR #1850 (remove artefacts, extract fmt changes)
2. Rebase on current `main` (23a4a953)
3. Add missing tests (thesaurus generation, learning loop e2e)
4. Structured review using `disciplined-design` skill
5. Merge after quality gate passes

## Appendix

### Reference Materials
- PR #1850 diff: https://git.terraphim.cloud/terraphim/terraphim-ai/pulls/1850
- Existing terraphim_grep code: `crates/terraphim_grep/src/`
- PR #1909 (OpenRouter smoke tests): Recently merged, provides free model baseline

### Code Snippets

**KG Persistence (new in `kg_curation.rs`)**:
```rust
fn persist_concepts(
    &self,
    concepts: &[NewConcept],
    source_query: &str,
    kg_path: &std::path::Path,
) {
    // Creates markdown files: learned-{slug}.md
    // Skips existing files
    // Logs warnings on I/O errors
}
```

**Role Config (`.terraphim/config.toml`)**:
```toml
[roles.devops]
name = "DevOps"
shortname = "devops"
thesaurus = ".terraphim/thesaurus-devops.json"
kg_path = ".terraphim/kg/devops"
llm_provider = "openrouter"
llm_model = "qwen/qwen3-coder:free"
kg_curation_enabled = true
```
