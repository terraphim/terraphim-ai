# Research Document: PR #652 Agent Workflows E2E Implementation

**Status**: Review
**Author**: Claude Code (Terraphim AI)
**Date**: 2026-03-10
**PR**: #652 - feat/agent-workflows-e2e

## Executive Summary

PR #652 implements end-to-end agent workflows using terraphim-llm-proxy with Cerebras LLM integration. It replaces parallelization mock data with real LLM output and adds Playwright browser tests for all 5 workflow patterns. However, there are merge conflicts with main in two files that need resolution.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | E2E testing aligns with quality goals |
| Leverages strengths? | Yes | Uses existing Playwright + genai infrastructure |
| Meets real need? | Yes | Replaces mocks with real LLM calls for accurate testing |

**Proceed**: Yes - All 3 YES

## Problem Statement

### Description
The agent workflow examples currently use mock data, which doesn't accurately test the full integration between the frontend, backend, and LLM providers. This PR replaces mocks with real LLM calls via terraphim-llm-proxy.

### Impact
- Tests now validate actual LLM integration
- Catches issues in LLM client configuration
- Ensures workflow patterns work end-to-end

### Success Criteria
- All 5 workflow patterns have passing E2E tests
- Tests use real LLM output (no mocks)
- Integration verified with Cerebras via terraphim-llm-proxy

## Current State Analysis

### Files Changed (8 total)
| Component | Location | Purpose |
|-----------|----------|---------|
| Agent Logic | `crates/terraphim_multi_agent/src/agent.rs` | Adds `get_extra_str` helper, refactors LLM config extraction |
| LLM Client | `crates/terraphim_multi_agent/src/genai_llm_client.rs` | Refactors to use `ServiceTargetResolver` instead of env vars |
| Workflow Handlers | `terraphim_server/src/workflows/multi_agent_handlers.rs` | Adds `get_role_extra_str/f64` helpers (CONFLICT with main) |
| Parallelization Demo | `examples/agent-workflows/3-parallelization/app.js` | Adds `parseLlmResponse` method (CONFLICT with main) |
| Browser Tests | `examples/agent-workflows/browser-automation-tests.js` | Adds setup hooks, increases timeout, adds button selectors |
| Settings Manager | `examples/agent-workflows/shared/settings-manager.js` | Minor fixes |
| Config | `terraphim_server/default/ollama_llama_config.json` | Updates role configurations |
| Gitignore | `examples/agent-workflows/.gitignore` | Adds test artifacts |

### Merge Conflicts

#### Conflict 1: `terraphim_server/src/workflows/multi_agent_handlers.rs`
- **Our version (main)**: Has proper doc comment formatting with blank lines (fixes clippy `doc_lazy_continuation` warnings)
- **Their version (PR #652)**: Missing blank lines in doc comments
- **Resolution**: Keep our doc comment formatting, keep their helper functions

#### Conflict 2: `examples/agent-workflows/3-parallelization/app.js`
- **Our version (main)**: Original `extractPerspectiveAnalysis` method
- **Their version (PR #652)**: Refactored with new `parseLlmResponse` method and fallback handling
- **Resolution**: Use their implementation (adds real LLM parsing)

### Key Technical Changes

#### 1. genai_llm_client.rs - ServiceTargetResolver Pattern
```rust
// New approach: Use ServiceTargetResolver to override endpoint
Client::builder()
    .with_service_target_resolver_fn(
        move |mut st: genai::ServiceTarget| -> genai::resolver::Result<genai::ServiceTarget> {
            st.endpoint = Endpoint::from_owned(url_for_resolver.clone());
            Ok(st)
        },
    )
    .build()
```
**Why**: genai hardcodes adapter default endpoints (e.g., localhost:11434 for Ollama). The resolver allows overriding at request time.

#### 2. agent.rs - Helper Function Pattern
```rust
// New helper to handle both flat and nested extra paths
fn get_extra_str<'a>(...) -> Option<&'a str>
```
**Why**: `#[serde(flatten)]` on `Role.extra` means config JSON with `"extra": {"key": "val"}` results in `extra["extra"]["key"]` rather than `extra["key"]`.

#### 3. Browser Tests - Real LLM Integration
- Timeout increased from 60s to 300s (real LLM calls take time)
- Added setup hooks for each workflow (fill required fields)
- Added button selectors for each workflow

## Constraints

### Technical Constraints
- **LLM Provider**: Requires Cerebras via terraphim-llm-proxy
- **Timeouts**: Real LLM calls need longer timeouts (300s vs 60s)
- **No Mocks**: All tests use real LLM output (per CLAUDE.md guidelines)

### External Dependencies
| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| Cerebras API | Latest | Medium | Fallback to Ollama |
| terraphim-llm-proxy | Latest | Low | Local Ollama |
| Playwright | ^1.40 | Low | None |

## Risks and Unknowns

### Known Risks
| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Cerebras API unavailable | Medium | High | Fallback to Ollama |
| Tests flaky due to LLM latency | High | Medium | Increase timeouts, retry logic |
| Doc comment formatting causes clippy warnings | High | Low | Fix before merge |

### Open Questions
1. Does the ServiceTargetResolver pattern work with all LLM providers? (Cerebras, OpenAI, Anthropic, Ollama)
2. Should we add a `#[cfg(test)]` mock mode for faster CI?

## Research Findings

### Key Insights
1. **ServiceTargetResolver is the right pattern** for genai endpoint override - cleaner than env vars
2. **Helper functions are duplicated** between agent.rs and multi_agent_handlers.rs - potential for consolidation
3. **E2E tests are comprehensive** - cover all 5 workflow patterns with real LLM calls
4. **No mocks used** - aligns with CLAUDE.md testing guidelines

### Vital Few

#### Essential Constraints (Max 3)
| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Must not break existing LLM providers | Risk of regression with ServiceTargetResolver | Changes affect all providers |
| Must fix clippy warnings | CI will fail | doc_lazy_continuation warnings |
| Must handle Cerebras proxy | PR's main purpose | terraphim-llm-proxy integration |

#### Eliminated from Scope
| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Consolidating helper functions | Can be done in follow-up PR |
| Adding mock mode for CI | Not required for this PR |

## Recommendations

### Proceed/No-Proceed
**Proceed with conditions**:
1. Fix merge conflicts (use our doc formatting, their logic)
2. Verify clippy passes after merge
3. Run E2E tests locally before merging

### Scope Recommendations
- Keep the PR focused on E2E implementation
- Do not expand scope to consolidate helpers
- Fix only the conflicts, no additional refactoring

### Risk Mitigation
1. **Before merge**: Run `cargo clippy --workspace` to ensure no warnings
2. **After merge**: Verify E2E tests pass with `npm test` in examples/agent-workflows
3. **CI**: Ensure CI has access to terraphim-llm-proxy or use mock mode for CI

## Next Steps

1. Create implementation plan for conflict resolution
2. Execute merge conflict resolution
3. Verify clippy passes
4. Run E2E tests locally
5. Merge if all checks pass

## Appendix

### Test Commands
```bash
# Rust checks
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --lib

# E2E tests (requires terraphim-llm-proxy running)
cd examples/agent-workflows
npm install
npm test
```

### References
- PR #652: https://github.com/terraphim/terraphim-ai/pull/652
- genai ServiceTargetResolver docs: https://docs.rs/genai/latest/genai/
- Playwright docs: https://playwright.dev/
