# Research Document: Fast/Cheap LLM Build-Runner with Markdown Command Extraction

**Status**: Draft
**Author**: opencode (k2p6)
**Date**: 2026-05-11
**Reviewers**: Alex

## Executive Summary

The current build-runner agent executes a hardcoded bash script with fixed cargo commands (fmt, clippy, build, test). This approach is rigid, doesn't adapt to project changes, and cannot leverage learnings from previous runs. We propose redesigning build-runner to use fast/cheap LLM models (haiku, zai/glm-5-turbo) to dynamically extract and execute build commands from markdown documentation, while leveraging terraphim-agent's learning system to remember successful command sequences per project.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Reduces CI maintenance burden, enables self-adapting builds |
| Leverages strengths? | Yes | Uses existing terraphim_router (cost-aware), terraphim_automata (markdown parsing), terraphim-agent (learning) |
| Meets real need? | Yes | Current build-runner fails on rate limits, has no learning, requires manual updates when build process changes |

**Proceed**: Yes - 3/3 YES

## Problem Statement

### Description
The current build-runner is a deterministic bash script that:
1. Runs fixed commands: `cargo fmt`, `cargo clippy`, `cargo build`, `cargo test`
2. Has no awareness of project-specific build requirements
3. Cannot learn from previous successful builds
4. Fails on rate limits without intelligent retry
5. Does not adapt when build processes change (e.g., adding wasm target, new crate structure)

### Impact
- **Maintenance overhead**: Every build process change requires updating the hardcoded script
- **Fragility**: Rate limits cause build failures that block PR merges
- **Missed optimizations**: Cannot skip unnecessary steps based on changed files
- **No learning**: Same mistakes recur because learnings aren't applied to build commands

### Success Criteria
- [ ] Build commands extracted dynamically from markdown docs (BUILD.md, CONTRIBUTING.md)
- [ ] terraphim-agent learnings queried and applied per project
- [ ] Cost < $0.01 per build (using haiku/glm-5-turbo)
- [ ] Latency < 30s for command extraction
- [ ] Fallback to current deterministic script if LLM fails

## Current State Analysis

### Existing Implementation

**build-runner agent** (`/opt/ai-dark-factory/conf.d/terraphim.toml` lines 796-899):
- Layer: Growth
- CLI: `/bin/bash`
- Model: n/a (deterministic)
- Task: Hardcoded bash script with cargo fmt, clippy, build, test
- Status posting: POST_STATUS helper for Gitea commit status API

**Rate limit handling** (from orchestrator logs):
- Initial run fails with "rate limit" exit code 1
- Retry agent spawned with kimi/k2p5 fallback
- Retry succeeds but doesn't update commit status

### Relevant Components

| Component | Location | Purpose |
|-----------|----------|---------|
| build-runner | `terraphim.toml:796-899` | Hardcoded bash build script |
| terraphim_router | `crates/terraphim_router/src/` | Cost-aware provider routing |
| terraphim_automata | `crates/terraphim_automata/src/markdown_directives.rs` | Markdown directive parsing |
| terraphim-agent | `~/.cargo/bin/terraphim-agent` | Learning capture and query |
| llm_proxy | `crates/terraphim_service/src/llm_proxy.rs` | Multi-provider LLM proxy |
| pr_gate | `crates/terraphim_orchestrator/src/pr_gate.rs` | Latest status resolution (fixed today) |

### Data Flow

```
Push Event → build-runner → rch exec → cargo commands → Status POST
                    ↓
            (rate limit) → Retry with kimi/k2p5
                    ↓
            (status bug) → Old failure status persists
```

### Integration Points

1. **Gitea Commit Status API**: POST_STATUS posts to `$GITEA_URL/api/v1/repos/.../statuses/$SHA`
2. **rch**: Remote compilation dispatch to bigbox + SeaweedFS S3 cache
3. **terraphim-agent learn**: `~/.cargo/bin/terraphim-agent learn query "<keywords>"`
4. **Markdown directives**: terraphim_automata parses `route::`, `action::`, etc. from .md files

## Constraints

### Technical Constraints
- **Must maintain deterministic fallback**: LLM extraction must not break existing builds
- **Must use cheap models only**: Cost target < $0.01 per build
- **Must integrate with existing status posting**: POST_STATUS helper must continue working
- **Must respect rch**: Remote compilation must still dispatch correctly

### Business Constraints
- **No regression in build reliability**: Current 82.83% cache hit rate must be maintained
- **Backward compatibility**: Old PRs without BUILD.md must still build

### Non-Functional Requirements
| Requirement | Target | Current |
|-------------|--------|---------|
| Build latency | < 5 min | ~3 min (with cache) |
| LLM extraction latency | < 30s | N/A |
| Cost per build | < $0.01 | $0.00 (deterministic) |
| False positive rate | < 1% | 0% (deterministic) |

## Vital Few (Essentialism)

### Essential Constraints (Max 3)
1. **Cheap models only**: Must use haiku, zai/glm-5-turbo, or ollama (CostLevel::Cheap)
2. **Deterministic fallback**: If LLM extraction fails, fall back to current hardcoded script
3. **Learning integration**: Must query terraphim-agent learnings for known commands

### Eliminated from Scope
| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Full natural language build descriptions | Too complex for cheap models |
| Multi-step LLM reasoning | Increases latency beyond target |
| Replacing rch with local builds | rch cache is critical for performance |
| Real-time build optimization | Out of scope for Phase 1 |

## Dependencies

### Internal Dependencies
| Dependency | Impact | Risk |
|------------|--------|------|
| terraphim_router | Cost-aware routing | Low - already proven |
| terraphim_automata | Markdown parsing | Low - battle-tested |
| terraphim-agent | Learning query | Low - simple CLI interface |
| llm_proxy | Multi-provider fallback | Medium - needs testing |

### External Dependencies
| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| opencode CLI | latest | Low | claude CLI |
| rch | latest | Low | local cargo |

## Risks and Unknowns

### Known Risks
| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| LLM hallucinates incorrect build commands | Medium | High | Validate against known patterns, deterministic fallback |
| Markdown parsing fails on non-standard docs | Low | Medium | Graceful degradation to fallback |
| terraphim-agent learnings are stale | Medium | Low | Timestamp filter, explicit validation |
| Cost exceeds $0.01 per build | Low | Medium | Cost tracking, model selection enforcement |

### Open Questions
1. **What markdown files should be parsed?** BUILD.md, CONTRIBUTING.md, .github/workflows/*.yml, or all .md files?
2. **How to validate extracted commands?** Run in dry-run mode first? Compare against known safe patterns?
3. **How to structure learnings?** What format should terraphim-agent capture build commands in?

### Assumptions Explicitly Stated
| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| haiku can reliably extract shell commands from markdown | Anthropic benchmarks | Build failures, increased costs | No - needs testing |
| zai/glm-5-turbo is free and fast enough | Current routing config shows is_free:: true | Rate limits, slow builds | Yes - used in production |
| terraphim-agent learn query returns relevant commands | CLI help shows `learn query` | Missed optimizations, repeated failures | No - needs testing |
| Markdown directives in build docs follow standard patterns | terraphim_automata parses `route::`, `action::` | Parsing failures | Partial - needs extension |

### Multiple Interpretations Considered
| Interpretation | Implications | Why Chosen/Rejected |
|----------------|--------------|---------------------|
| **A**: Full LLM-driven build (extract + execute) | Maximum flexibility, highest risk | Rejected - too risky for CI |
| **B**: LLM extracts commands, deterministic validation | Balanced flexibility and safety | **Chosen** - best risk/reward |
| **C**: Learning-only (no LLM, just query past commands) | Lowest cost, lowest flexibility | Rejected - doesn't handle new projects |

## Research Findings

### Key Insights
1. **Ollama is already configured as CostLevel::Cheap** in bridge.rs - local models available
2. **zai/glm-5-turbo is marked `is_free:: true`** in planning/implementation tiers
3. **haiku is used by upstream-synchronizer** - proven for simple tasks
4. **terraphim_automata already parses markdown directives** - can be extended for build commands
5. **Latest status bug was fixed today** in pr_gate.rs - commit `9eb43d5b7`

### Relevant Prior Art
- **bridge.rs:175**: Ollama mapped to `CostLevel::Cheap` with `Latency::Fast`
- **planning_tier.md**: zai route with `is_free:: true` and `glm-5.1` model
- **implementation_tier.md**: zai route with `is_free:: true` and `glm-5-turbo` model
- **terraphim_automata**: Parses `route::`, `action::`, `trigger::` from markdown

### Technical Spikes Needed
| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| haiku command extraction accuracy | Test if haiku reliably extracts shell commands from BUILD.md | 2 hours |
| terraphim-agent learning format | Verify learn query returns actionable build commands | 1 hour |
| Markdown build directive parsing | Extend terraphim_automata to parse `build::` directives | 4 hours |

## Recommendations

### Proceed/No-Proceed
**Proceed** with **Interpretation B**: LLM extracts commands, deterministic validation executes.

### Scope Recommendations
- Phase 1: Add `build::` markdown directive parser to terraphim_automata
- Phase 2: Create build-runner-llm agent that queries learnings + parses markdown
- Phase 3: Integrate cost-aware routing (haiku/zai) for command extraction
- Phase 4: Add validation layer (dry-run mode for extracted commands)

### Risk Mitigation Recommendations
1. **Always run deterministic fallback** if LLM extraction fails or validation rejects
2. **Cost tracking**: Log every LLM call cost, alert if > $0.01
3. **Command whitelist**: Only allow known-safe patterns (cargo *, make, npm, etc.)
4. **Learning verification**: Cross-reference extracted commands against terraphim-agent learnings

## Next Steps

If approved:
1. Create build-runner-llm agent template in terraphim.toml
2. Extend terraphim_automata with `build::` directive parser
3. Add terraphim-agent learning integration (query + capture)
4. Test with haiku on sample BUILD.md files
5. Deploy alongside existing build-runner with feature flag

## Appendix

### Reference Materials
- `docs/taxonomy/routing_scenarios/adf/implementation_tier.md` - zai free model routing
- `crates/terraphim_service/src/llm/bridge.rs:175` - Ollama CostLevel::Cheap
- `crates/terraphim_automata/src/markdown_directives.rs` - Directive parsing
- `crates/terraphim_router/src/strategy.rs` - CostOptimized routing strategy

### Code Snippets
```rust
// From bridge.rs - Ollama is already cheap
Provider::new("ollama", "Ollama Local", ...)
    .with_cost(CostLevel::Cheap)
    .with_latency(Latency::Fast)
```

```bash
# From build-runner - current hardcoded commands
/home/alex/.local/bin/rch exec -- cargo fmt --all -- --check
/home/alex/.local/bin/rch exec -- cargo clippy --workspace --all-targets -- -D warnings
/home/alex/.local/bin/rch exec -- cargo build --workspace
/home/alex/.local/bin/rch exec -- cargo test --workspace --no-fail-fast
```
