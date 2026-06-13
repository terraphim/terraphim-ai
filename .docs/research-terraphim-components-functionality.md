# Research Document: Terraphim Components Full Functionality Audit

**Status**: Draft
**Author**: AI Agent
**Date**: 2026-06-13
**Reviewers**: Human

## Executive Summary

Audit of four Terraphim AI components (terraphim-grep, terraphim-agent, terraphim-lsp, terraphim-rlm) to determine what is needed for full functionality. terraphim_grep and terraphim_agent are largely complete with passing tests. terraphim_rlm has a critical KG validation bypass and several Firecracker executor bugs. terraphim_lsp is a placeholder with zero implementation. The plan must address 13 actionable gaps across the four crates plus cross-cutting issue deduplication.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energising? | Yes | These are core Terraphim infrastructure components; broken RLM validation undermines the security model |
| Leverages strengths? | Yes | Deep Rust async, MCP protocol, KG integration -- all Terraphim's core competencies |
| Meets real need? | Yes | RLM KG bypass is a P0 security gap; LSP is needed for IDE integration; session search is a user-facing feature gap |

**Proceed**: Yes -- 3/3 YES

## Problem Statement

### Description
Four core Terraphim components have varying states of completeness:
1. **terraphim_lsp** is a placeholder with zero implementation
2. **terraphim_rlm** has a critical KG validation bypass and Firecracker executor bugs
3. **terraphim_grep** works but lacks CI test coverage for the `code-search` feature
4. **terraphim_agent** is largely complete but missing Session REPL commands and has a feature-gate bug in firecracker module

Additionally, Gitea issue tracking has significant duplication (e.g. 6+ issues for the same Cursor SQLite connector work), which causes agent thrashing.

### Impact
- RLM users execute untrusted code without KG validation -- security gap
- IDE users have no LSP support for KG markdown files
- Agent users cannot import/search sessions from the REPL
- Firecracker VMs leak resources due to missing cleanup
- CI does not test terraphim_grep's primary feature flag

### Success Criteria
1. terraphim_lsp: LSP server with hover, completion, and diagnostics for KG markdown files, compilable from workspace root
2. terraphim_rlm: KG validation executes on the hot path; Firecracker executor bugs fixed; all tests pass
3. terraphim_grep: CI tests `code-search` feature; no regressions
4. terraphim_agent: Session REPL commands functional; firecracker feature guard fixed
5. Gitea: Duplicate issues closed, dependency graph clean

## Current State Analysis

### Existing Implementation

| Component | LOC (approx) | Tests | Compiles | Status |
|-----------|-------------|-------|----------|--------|
| terraphim_grep | 2,600+ | 35 (all pass) | Yes | Production-ready, minor gaps |
| terraphim_agent | 15,000+ | 851 (all pass, 1 ignored) | Yes | Production-ready, session commands missing |
| terraphim_lsp | 6 | 0 | Only from crate dir | Placeholder |
| terraphim_rlm | 5,700+ | 131 (all pass, 1 ignored) | Yes | Functional but security gap |

### Code Locations

| Component | Path | Purpose |
|-----------|------|---------|
| terraphim_grep | `crates/terraphim_grep/` | Hybrid grep with RLM fallback and KG curation |
| terraphim_agent | `crates/terraphim_agent/` | AI Agent CLI with REPL, robot mode, learning capture |
| terraphim_lsp | `crates/terraphim_lsp/` | LSP server for KG markdown (placeholder) |
| terraphim_rlm | `crates/terraphim_rlm/` | Recursive Language Model orchestration |

### Key RLM Validation Gap Detail

The critical issue is in `crates/terraphim_rlm/src/rlm.rs`: The `execute_code()` and `execute_command()` methods on `TerraphimRlm` do not call `executor.validate()` before execution. The `validator.rs` module provides `KnowledgeGraphValidator` but it exists as dead code on the hot path. Two open PRs (#2614, #2514) are attempting to address this.

### Key LSP Gap Detail

`crates/terraphim_lsp/src/lib.rs` contains only:
```rust
//! LSP hover, completion, and diagnostics for KG markdown files.
// placeholder
```

The crate has its own `Cargo.lock` which conflicts with workspace resolution. No dependencies are declared. No LSP protocol implementation exists.

## Constraints

### Technical Constraints
- All crates must compile under workspace `Cargo.toml` (edition 2024, resolver 2)
- LSP must use a standard LSP library (tower-lsp or lsp-server are conventional choices)
- RLM executor backends: Firecracker (KVM), Docker (bollard), E2B, Local (process)
- Must not introduce mocks -- test against real infrastructure
- Must work on macOS (development) and Linux (production/bigbox)

### Business Constraints
- Must integrate with existing KG infrastructure (terraphim_automata, terraphim_rolegraph)
- Must maintain backward compatibility for all existing APIs
- Session import must support Claude Code, Cursor, and Aider sources

### Non-Functional Requirements

| Requirement | Target | Current |
|-------------|--------|---------|
| RLM command validation latency | < 100ms | N/A (validation skipped) |
| LSP hover response | < 200ms | N/A |
| Session search latency | < 100ms | N/A |

## Vital Few (Essential Constraints)

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| RLM KG validation must execute on hot path | Security: untrusted code can bypass KG safety checks | #3368 open, 2 PRs stalled |
| LSP must compile from workspace root | Otherwise cannot be built in CI or by users | Orphaned Cargo.lock |
| All existing tests must continue to pass | Regression prevention | 1,017 tests currently pass |

## Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Full Firecracker microVM integration (beyond bug fixes) | Requires KVM on Linux; scope creep |
| New LSP features beyond hover/completion/diagnostics | MVP first |
| terraphim_agent TUI rewrite | Existing TUI works; not a bug |
| Cross-crate refactoring | Each crate is independent; scope per-component |

## Dependencies

### Internal Dependencies

| Dependency | Impact | Risk |
|------------|--------|------|
| terraphim_automata | Used by grep, rlm for thesaurus/KG | Low -- stable crate |
| terraphim_service | Used by grep, agent, rlm for LLM | Low -- stable crate |
| terraphim_rolegraph | Used by grep, rlm for role config | Low -- stable crate |
| terraphim_types | Used by all | Low -- types crate |

### External Dependencies

| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| tower-lsp or lsp-server | latest | Low | Could implement LSP from scratch (high effort) |
| rmcp (already in use) | 0.9.0 | Low | Already used by RLM for MCP |

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| RLM validation PRs (#2614, #2514) conflict | Medium | Medium | Review both, merge best approach |
| LSP crate edition 2021 vs workspace 2024 | Medium | Low | Bump edition to 2024 |
| Firecracker tests require KVM (Linux only) | High | Low | Gate behind `firecracker` feature, skip on macOS |
| Duplicate issue closure angers automated agents | Low | Low | Comment with reason before closing |

### Open Questions

1. Which LSP library to use? -- tower-lsp (async, tokio-native) or lsp-server (synchronous, simpler)?
2. Should LSP integrate with terraphim_automata directly or via terraphim_service?
3. Are there existing LSP implementations in the workspace to follow? -- Need to check
4. What is the status of the stalled RLM validation PRs? -- Need owner to decide merge strategy

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| tower-lsp is the right LSP framework | Standard choice for async Rust LSP servers | Would need to switch framework | No -- needs investigation |
| RLM validation PRs are mergeable with minor conflicts | Both touch similar code paths | May need fresh implementation | No -- needs review |
| Session REPL commands follow existing REPL patterns in agent crate | Existing REPL has chat, file, web commands | May need different architecture | No -- needs design |
| Firecracker bugs are in executor implementation, not in API layer | Issues describe specific executor methods | May be architectural | No -- needs code review |

### Multiple Interpretations Considered

| Interpretation | Implications | Why Chosen/Rejected |
|----------------|--------------|---------------------|
| LSP should be a separate binary | Cleaner separation, can run independently | Rejected: adds deployment complexity; MCP is already embedded in RLM |
| LSP should be embedded in terraphim_agent | Simplifies deployment, shares KG loading | Chosen: reduces duplication, follows existing MCP-in-RLM pattern |
| RLM validation should be in executor trait | All executors get validation for free | Chosen: consistent with existing architecture |
| RLM validation should be in rlm.rs call sites | More explicit, easier to audit | Rejected: duplicates logic across code/command paths |

## Research Findings

### Key Insights

1. **terraphim_lsp is the only component with zero implementation** -- it needs a complete build-out from scratch
2. **RLM validation bypass is a known, documented issue** with two stalled PRs -- the fix is understood but not merged
3. **Gitea issue duplication is severe** -- 6+ issues for Cursor SQLite connector alone; this creates agent thrashing and wasted effort
4. **All existing tests pass** -- the foundation is solid; improvements are additive
5. **terraphim_grep is the most complete component** -- only CI wiring and orchestrator enrichment remain

### Relevant Prior Art
- tower-lsp: Standard async LSP framework for Rust, used by rust-analyzer
- RLM's MCP server implementation (`src/mcp_tools.rs`): Shows the pattern for embedding protocol servers in Terraphim crates
- terraphim_agent REPL commands (`src/repl/commands.rs`): Shows the pattern for adding new REPL subcommands

### Technical Spikes Needed

| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| Evaluate tower-lsp vs lsp-server | Determine LSP framework | 2 hours |
| Review stalled RLM validation PRs | Determine merge strategy | 3 hours |
| Investigate Cursor SQLite schema | Understand session import format | 2 hours |
| Profile terraphim_automata loading time | Determine if LSP startup is acceptable | 1 hour |

## Recommendations

### Proceed/No-Proceed
**Proceed** with phased approach: Fix critical security gaps first (RLM validation), then implement missing functionality (LSP, sessions), then polish (CI, deduplication).

### Scope Recommendations
1. **Phase 1 (Critical)**: RLM KG validation hot-path fix
2. **Phase 2 (High)**: terraphim_lsp implementation
3. **Phase 3 (Medium)**: terraphim_agent session REPL commands
4. **Phase 4 (Low)**: CI improvements, Firecracker executor bugs, issue deduplication

### Risk Mitigation Recommendations
- Review and merge or replace stalled RLM validation PRs before writing new code
- Start LSP with a minimal tower-lsp server that returns hardcoded responses, then iterate
- Batch-close duplicate Gitea issues with explanatory comments
- Keep terraphim_grep changes minimal -- it's already working well

## Next Steps

If approved:
1. Spike: Evaluate LSP framework choices
2. Spike: Review RLM validation PRs #2614 and #2514
3. Proceed to Phase 2 (Design) for the implementation plan
4. Begin Gitea issue deduplication

## Appendix

### Reference Materials
- tower-lsp: https://github.com/ebkalderon/tower-lsp
- LSP specification: https://microsoft.github.io/language-server-protocol/
- RLM architecture: `crates/terraphim_rlm/src/lib.rs` (architecture overview in doc comments)

### Build Verification Commands
```bash
# Verify workspace compilation
cargo check --workspace 2>&1

# Verify each crate individually
cargo check -p terraphim_grep 2>&1
cargo check -p terraphim_agent 2>&1
cargo check -p terraphim_rlm 2>&1

# Run all tests
cargo test --workspace 2>&1

# Check for duplicate Gitea issues
gtr list-issues --owner terraphim --repo terraphim-ai --state open | grep -i "cursor\|session\|connector"
```
