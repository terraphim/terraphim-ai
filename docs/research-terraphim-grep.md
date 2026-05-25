# Research Document: terraphim-grep — Intelligent Hybrid Grep

**Status**: Draft
**Author**: Agent
**Date**: 2026-05-23
**Gitea Issue**: #1743
**Research Phase**: Phase 1 of Disciplined Development

## Executive Summary

terraphim-grep is an intelligent grep tool that uses hybrid search (FFF + ripgrep + KG) for fast deterministic results, falling back to RLM only when needed. The system learns from interactions by writing new concepts back to the knowledge graph, improving future searches. This is the inverse of rlmgrep's "brute force LLM" approach.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Leverages existing Terraphim infrastructure (RoleGraph, HaystackProvider, KgPathScorer) |
| Leverages strengths? | Yes | Terraphim has fast Aho-Corasick search, KG curation, RLM sandboxing already built |
| Meets real need? | Yes | rlmgrep is expensive ($0.01-0.05/query) and slow (15-30s); terraphim-grep targets $0.001/query at 0.1-5s |

**Proceed**: Yes (3/3 YES)

## Problem Statement

### Description
Code search tools either use simple regex (no semantic understanding) or load entire corpuses into LLM context (expensive, slow). Neither approach learns from interactions.

### Impact
- Developers waste time crafting regex patterns
- LLM-based search is prohibitive at scale ($0.01-0.05 per query)
- No learning between queries means repeated work

### Success Criteria
1. 80-90% of queries answered by hybrid search alone (< 0.5s, $0.0001)
2. RLM fallback only when search insufficient (< 5s, $0.005)
3. KG learns new concepts from RLM interactions
4. rlmgrep-compatible CLI interface

## Current State Analysis

### Existing Implementation

| Component | Location | Purpose |
|-----------|----------|---------|
| LLM Bridge (stubbed) | `crates/terraphim_rlm/src/llm_bridge.rs` | HTTP bridge for VM-to-host LLM calls; returns `LlmNotConfigured` without `llm` feature |
| RoleGraph | `crates/terraphim_rolegraph/src/lib.rs` | Knowledge graph with Aho-Corasick O(n) concept detection + TF-IDF fallback |
| HaystackProvider trait | `crates/haystack_core/src/lib.rs` | Uniform async search interface over heterogeneous backends |
| KgPathScorer | `crates/terraphim_file_search/src/kg_scorer.rs` | Scores files by KG concept matches in path; implements `ExternalScorer` for fff-search |
| LLM Client trait | `crates/terraphim_service/src/llm.rs` | `LlmClient` trait with `chat_completion`, `summarize` methods |
| CLI structure | `crates/terraphim_cli/src/main.rs` | Existing clap-based CLI with `Search`, `Find`, `Extract` commands |

### Integration Points

- `terraphim_rlm` uses `terraphim_service::llm::LlmClient` when `llm` feature enabled
- `fff-search` (external crate) provides file discovery with `ExternalScorer` trait
- `KgPathScorer` reads thesaurus and runs Aho-Corasick via `terraphim_automata::find_matches`

### Data Flow (Current)

```
Query → terraphim_cli → CliService → RoleGraph.search() → Ranked results
                                    → HaystackProvider.search() (optional)
```

## Constraints

### Technical Constraints
- **LLM Bridge is stubbed**: `crates/terraphim_rlm/src/llm_bridge.rs:233` returns `LlmNotConfigured` without `llm` feature
- **Edition 2024**: Workspace uses Rust edition 2024
- **No existing hybrid orchestrator**: Need to build `HybridSearcher` from scratch
- **FFF external dependency**: `fff-search` is an external git dependency

### Business Constraints
- **12-day implementation roadmap** (from issue #1743)
- **Must maintain rlmgrep compatibility** for CLI interface

### Non-Functional Requirements
| Requirement | Target | Current |
|-------------|--------|---------|
| Cost per query | < $0.001 avg | N/A (doesn't exist) |
| Latency (search-only) | < 0.5s | N/A |
| Latency (with RLM) | < 5s | N/A |
| Learning | KG updates after RLM | N/A |

## Vital Few (Essentialism)

### Essential Constraints (Max 3)
| Constraint | Why Vital | Evidence |
|------------|-----------|----------|
| LLM bridge must work | RLM fallback requires real LLM calls | `llm_bridge.rs:233` currently returns error |
| Hybrid search must be fast | Core value prop is cost/latency | Need parallel search across haystacks |
| KG must update after RLM | Learning is the key differentiator | RLM extracts → RoleGraph.add_concept() |

### Eliminated from Scope
| Item | Why Eliminated |
|------|----------------|
| Firecracker VM integration | Not needed for CLI tool; Docker backend sufficient |
| MCP server integration | Future phase |
| Multi-modal ingestion (PDF/images) | Port from rlmgrep later |
| Streaming output | Not in initial scope |

## Dependencies

### Internal Dependencies
| Dependency | Impact | Risk |
|------------|--------|------|
| terraphim_service::llm::LlmClient | RLM fallback needs chat_completion | Low - trait exists |
| terraphim_rolegraph::RoleGraph | KG reads/writes | Low - well-tested |
| terraphim_file_search::KgPathScorer | Path-based scoring | Low - implements ExternalScorer |
| fff_search | File discovery | Medium - external crate |

### External Dependencies
| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| fff-search | git branch | Medium - external | ripgrep directly |
| genai (LLM) | git fork | Medium | OpenRouter direct, Ollama |

## Risks and Unknowns

### Known Risks
| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| LLM bridge integration complexity | Medium | High | Phase 1 focuses on this; reuse terraphim_service LlmClient |
| External fff-search crate instability | Low | Medium | Implement fallback to ripgrep directly |
| KG automaton rebuild cost | Low | Low | Background task, not blocking |

### Open Questions
1. **Sufficiency Judge threshold tuning** - How to determine "sufficient" vs "needs RLM"? Need empirical tuning.
2. **Context window management** - How to limit retrieved chunks to avoid LLM context overflow?
3. **KG curation rate limiting** - How often to update automata? (Every interaction vs batched)

### Assumptions Explicitly Stated
| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| terraphim_service::llm::LlmClient.chat_completion is sufficient | It's the existing LLM interface | RLM needs different interface | No - may need wrapper |
| RoleGraph.add_concept() is safe for concurrent access | Uses Arc<Mutex<RoleGraph>> in existing code | Data corruption | Partially - existing patterns suggest it's safe |
| FFF search can be called in parallel with KG search | Both are async | Performance not gained | Yes - tokio::join! works |

## Research Findings

### Key Insights
1. **LLM bridge is the critical path** - Without it, RLM fallback cannot work. Must enable `llm` feature and configure real client.
2. **Hybrid search already has components** - KgPathScorer, HaystackProvider, RoleGraph all exist; need orchestration.
3. **CLI structure exists** - terraphim_cli can be extended with new `Grep` command.
4. **Structured signatures don't exist** - Need new `RlmSignature` trait for typed outputs.

### Relevant Prior Art
- **rlmgrep**: Loads ALL files into LLM context; $0.01-0.05/query; no learning
- **ripgrep**: Fast regex search; no semantic understanding
- **GitHub Copilot Chat**: LLM-based but not searchable; no learning

### Technical Spikes Needed
| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| LLM bridge with terraphim_service | Verify chat_completion works for RLM queries | 4 hours |
| Hybrid search parallelization | Verify tokio::join! scales across 3 haystacks | 2 hours |
| Sufficiency judge heuristics | Test coverage/diversity thresholds | 2 hours |

## Recommendations

### Proceed/No-Proceed
**Proceed** - All critical components exist; only orchestration and integration needed.

### Scope Recommendations
1. Create new crate `terraphim_grep` for the hybrid search + RLM logic
2. Extend existing `terraphim_cli` with `grep` subcommand
3. Phase 1 (LLM bridge) is prerequisite for all other phases

### Risk Mitigation Recommendations
1. **Start with LLM bridge verification** - Don't build hybrid search on broken LLM
2. **Use existing LlmClient from terraphim_service** - Don't reinvent HTTP client
3. **Test sufficiency judge with real queries** - Heuristics may need tuning

## Next Steps

If approved:
1. **Phase 2**: Create implementation plan with file-level changes
2. **Step 1**: Add `grep` subcommand to terraphim_cli
3. **Step 2**: Create `HybridSearcher` orchestrating existing components
4. **Step 3**: Implement `SufficiencyJudge`
5. **Step 4**: Wire RLM via terraphim_service::llm::LlmClient
6. **Step 5**: Add `RlmSignature` trait and implementations
7. **Step 6**: Implement KG curation loop

## Appendix

### Code Location Map

```
terraphim-grep/
├── CLI Extension
│   └── crates/terraphim_cli/src/main.rs          # Add Grep subcommand
│
├── Core Logic (new crate: terraphim_grep)
│   └── crates/terraphim_grep/src/
│       ├── lib.rs                               # Module root
│       ├── hybrid_searcher.rs                   # Parallel search orchestration
│       ├── sufficiency_judge.rs                 # Heuristic + LLM judge
│       ├── rlm_context.rs                      # Context building for RLM
│       ├── signatures.rs                       # RlmSignature trait
│       ├── kg_curation.rs                      # RLM → KG feedback loop
│       └── error.rs                            # Error types
│
├── Integration Points
│   ├── crates/terraphim_rlm/src/llm_bridge.rs  # LLM bridge (needs llm feature)
│   ├── crates/terraphim_rolegraph/src/lib.rs   # RoleGraph (reads/writes)
│   ├── crates/terraphim_file_search/src/kg_scorer.rs  # KgPathScorer
│   └── crates/terraphim_service/src/llm.rs     # LlmClient trait
│
└── Research Reference
    └── docs/research-terraphim-intelligent-grep.md  # Original research
```

### Reference Materials
- `crates/terraphim_rlm/src/llm_bridge.rs:199-261` - LLM query method
- `crates/terraphim_rolegraph/src/lib.rs:1-200` - RoleGraph architecture
- `crates/haystack_core/src/lib.rs:8-13` - HaystackProvider trait
- `crates/terraphim_file_search/src/kg_scorer.rs:45-70` - KgPathScorer scoring
- `crates/terraphim_service/src/llm.rs:32-67` - LlmClient trait
