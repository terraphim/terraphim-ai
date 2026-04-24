# Research Document: Next Work Planning (Post #707)

**Status**: Approved
**Date**: 2026-04-23

## Executive Summary

#707 (Token Budget) is merged and closed. Both remotes synced at `497330948`. The backlog has 266 unblocked issues. After analysing the top candidates by impact, feasibility, and dependency chain, three workstreams emerge as highest-value next steps.

## Current State of Main

### Last 10 Commits on main
```
497330948 feat(agent): token budget management engine (PR #836, #707)
071e789c7 fix(orchestrator): harden provider probe circuit breaker (#791)
34891c299 feat(agent): token budget management engine (pre-merge)
2d5d513b1 fix(security): RUSTSEC-2026-0104 audit ignore (#752)
e6605b037 feat(learn): suggestion approval workflow (#833, #85)
e5c3147ef feat(orchestrator): mention-chain coordination (#832, #144)
d668d9f5a feat(adf): product-development + repo-steward (#748/#767/#768)
6871da000 feat(sessions): BM25-ranked session search (#758)
d99ea1be5 feat(sessions): enrichment pipeline (#756)
3eaec7330 chore(docker): bump rust
```

### What Changed in Last 10 Commits (93 files, +10129/-2341)
- Robot mode budget engine (our work)
- Provider probe hardening (circuit breaker + timeout)
- Suggestion approval workflow (shared learning)
- Mention-chain coordination (orchestrator)
- ADF product-development + repo-steward templates
- BM25 session search
- Session enrichment pipeline
- Security audit ignore (RUSTSEC-2026-0104)

## Backlog Analysis

### Top Candidates by Impact and Feasibility

| Issue | Title | PageRank | Priority | Unblocked | Size | Fit |
|-------|-------|----------|----------|-----------|------|-----|
| **#795** | Wire robot mode JSON into session commands | 0.15 | 0 | Yes | ~350 LOC | Directly builds on our #707 work |
| **#697** | Task 1.6 -- Phase 1 tests | 0.15 | 0 | Yes | ~500 LOC | Completes Phase 1 spec |
| **#794** | Persist sessions to disk cache | 0.15 | 0 | Yes | ~300 LOC | Unblocks #793, #787, #788 |
| **#796** | OpenCode/Codex JSONL connectors | 0.15 | 0 | Yes | ~400 LOC | Expands session coverage |
| **#793** | /sessions import --all bulk import | 0.15 | 0 | Yes | ~200 LOC | Depends on #794 (nice-to-have) |
| **#779** | Fix test_api_client_search assertion | 0.15 | 44 | Yes | ~50 LOC | Quick fix, high priority |
| **#766** | Probe architecture for model availability | 0.15 | 35 | Yes | Very Large | Epic with 9 phases |
| **#578** | Wire agent_evolution into ADF orchestrator | 0.0035 | 38 | Yes | Medium | Priority but complex |

### Dependency Chains

```
#794 (session cache) -> #793 (import --all) -> #796 (OpenCode/Codex connectors)
                   \-> #787 (velocity verification)
                   \-> #788 (Tantivy search)

#707 (budget, DONE) -> #795 (wire robot into sessions) -> #697 (Phase 1 tests)

#779 (test fix) -- standalone quick fix
```

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Directly continues our robot mode work |
| Leverages strengths? | Yes | We just built BudgetEngine; #795 uses it |
| Meets real need? | Yes | Robot mode is unusable without CLI wiring |

**Proceed**: Yes (3/3)

## Recommendations

### Recommended Work Order

1. **#779** -- Fix test_api_client_search (quick win, priority=44, ~50 LOC)
2. **#795** -- Wire robot mode into session commands (builds on our #707, ~350 LOC)
3. **#697** -- Phase 1 test suite (completes Phase 1 spec, ~500 LOC)

### Why This Order

- **#779** first: Quick fix that unblocks CI/test-guardian. Priority=44 makes it the highest-priority unblocked issue.
- **#795** second: Direct continuation of #707. We know the robot module intimately. This is where our budget engine gets actually used.
- **#697** third: Closes out Phase 1 with comprehensive tests for everything we've built.

### What to Defer

| Deferred | Reason |
|----------|--------|
| #794 (session cache) | Important but orthogonal to robot mode work |
| #766 (probe architecture) | Very large epic, needs dedicated session |
| #578 (agent_evolution) | Complex, risky, needs more research |
