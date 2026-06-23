# Research Document: Issue #2677 — Gitea Issue Deduplication

**Status**: Review
**Author**: Echo (implementation-swarm)
**Date**: 2026-06-23
**Issue**: #2677 (Step 10: Gitea Issue Deduplication)

## Executive Summary

#2677 asks to batch-close ~15 duplicate session/Cursor/connector issues. **The issue's body is stale/fabricated**: it cites canonical issue numbers (#3530, #3396, #3085, #3279) that do not exist in this repo, and lists "duplicate" numbers that are either missing or are unrelated open issues (e.g. #2907 = Security Sentinel, #2909 = Tick-stall). Literal execution would **destructively close real, unrelated issues against non-existent canonicals**. The genuine duplicates DO exist (15 open issues across 4 clusters) but their correct resolution is different: they are **orphaned issues for code extracted to the `terraphim-clients` polyrepo** in #1910 (commit `aa7ba99e8`), not duplicates of a canonical in this repo.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Reduces agent thrashing (#2914 phantom-backlog); Echo's core zero-deviation purpose |
| Leverages strengths? | Yes | Fidelity verification — exactly what the Twin Maintainer does |
| Meets real need? | Yes | AGENTS.md "Issue Hygiene" mandates closing polyrepo-orphaned issues |

**Proceed**: Yes (3/3) — but the work must be re-scoped from the issue's fabricated mapping to verified reality.

## Problem Statement

### Description
~15 open issues in the session/Cursor/connector/learn-procedure topic area are duplicates or orphans. The issue body proposes closing specific numbers against specific canonicals.

### Impact
- Agents re-attempt phantom issues (#2914 — "~36 ready issues reference code extracted from this repo")
- Open-issue count (257) is inflated with non-actionable items, distorting `gtr ready` PageRank triage

### Success Criteria
1. Genuine duplicate clusters consolidated to one canonical each
2. Polyrepo-orphaned issues closed with an accurate comment pointing to terraphim-clients
3. **Zero unrelated issues closed** (the #2677 body would have closed several)
4. Open-issue count decreases by ~10-12

## Current State Analysis (verified live 2026-06-23)

### Fabrication in #2677's body

| #2677 claims canonical | Reality | #2677 claims duplicate | Reality |
|---|---|---|---|
| #3530 (Cursor) | **MISSING** | #2515 | open, real Cursor dup |
| | | #2769 | closed "version.workspace" — UNRELATED |
| | | #2907 | open "Security Sentinel" — UNRELATED |
| | | #2909 | open "ADF Tick-stall" — UNRELATED |
| | | #3012, #3351 | **MISSING** |
| #3396 (Sessions REPL) | **MISSING** | #3282, #2977 | **MISSING** |
| #3085 (Sessions expand) | **MISSING** | #2969 | **MISSING** |
| #3279 (Learn from-session) | **MISSING** | #3008 | **MISSING** |

**Conclusion:** the canonical numbers come from a **different repo's numbering** (likely terraphim-agents) or were invented. They cannot be used.

### Why the real duplicates exist — the #1910 polyrepo split

Git log (`aa7ba99e8 refactor(workspace): remove 25 extracted crate dirs (E4a/E4b/E5 dir-removal) Refs #1910`) deleted `crates/terraphim_sessions/`, `crates/terraphim_tui/`, and most of `crates/terraphim_agent/` from THIS repo. They now live in `/data/projects/terraphim/terraphim-clients/crates/terraphim_sessions/`.

Verified:
- `crates/terraphim_sessions/` — **MISSING** from terraphim-ai; EXISTS in terraphim-clients
- `crates/terraphim_tui/` — **MISSING** from terraphim-ai
- `crates/terraphim_agent/src/learnings/` — **MISSING** (only client.rs/repl/commands remain — a thin shim)
- `SessionConnector` trait, `ProcedureStore`, `capture_failed_command`, `compile_corrections_from_dir` — **not in this repo**; in terraphim-clients
- polyrepo connector dir has aider/cline/codex/native/opencode.rs — **no cursor.rs** (Cursor genuinely unimplemented there)

### Real duplicate clusters (all OPEN, all reference extracted crates)

**Cluster A — Cursor SQLite connector (Task 2.5)** — 3 issues, identical scope
- #1983 (2026-06-02, oldest) — **canonical**
- #2403, #2515 — duplicates

**Cluster B — Sessions expand (Task 2.6.4)** — 2 issues
- #2134 (2026-06-15) — **canonical** (older)
- #2222 — duplicate

**Cluster C — Learn procedure from-session (D3 plan)** — 4 issues
- #2084 (2026-06-02, oldest) — **canonical**
- #2162, #2350, #2776 — duplicates

**Cluster D — Session REPL commands `/sessions import,search,list`** — 2 issues
- #2352 (2026-06-16) — **canonical**
- #2435 — duplicate

**Orphaned (polyrepo-located, single-issue, no in-repo canonical):**
- #1986 (redact secrets in connectors), #1988 (benchmarks), #2032/#2033/#2034 (Phase 3 CLI envelope/tests/self-doc), #2140 (2.6.1 sessions import), #2158 (CLI tests), #2356 (KG auto-suggest capture.rs:609)

## Constraints

- **Destructive operation**: closing issues is reversible but noisy; must be precise
- **No canonical in-repo**: the work moved to terraphim-clients; there is no terraphim-ai issue to "consolidate into"
- **Comment accuracy**: each close comment must state the verifiable reason (polyrepo relocation) with the commit/issue evidence, not the fabricated #2677 wording

## Risks and Unknowns

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Close an issue whose scope differs subtly | Med | High | Read each dup body fully before close; this doc lists verified-identical clusters |
| terraphim-clients already has these tracked | Med | Low | Comment invites reopening in this repo if scope is in-repo; points to polyrepo path |
| A canonical (e.g. #2134) might itself be better closed | Low | Low | Keep oldest open per cluster as canonical; minimal-blast-radius choice |

### Assumptions explicitly stated

| Assumption | Basis | Risk if wrong | Verified? |
|---|---|---|---|
| Code moved to terraphim-clients permanently | commit aa7ba99e8 + dir listing | If re-imported, issues become live again | Yes (git + fs) |
| Oldest issue = canonical | standard dedup convention | Could lose a richer newer body | Yes (read bodies) |
| #2677 body is fabricated, not a different-repo reference | canonical #s > max repo issue# (~2920) | If it's cross-repo, my rationale still correct (these belong there) | Yes |

## Recommendations

### Proceed/No-Proceed: PROCEED (re-scoped)

### Proposed action (minimal blast radius)

**Step 1 — Cluster duplicates (close higher-numbered dups, keep oldest open):**
- Close #2403, #2515 → keep #1983 (Cursor)
- Close #2222 → keep #2134 (expand)
- Close #2162, #2350, #2776 → keep #2084 (learn from-session)
- Close #2435 → keep #2352 (Session REPL)

**Step 2 — Polyrepo-orphaned singles (close with relocation comment):**
- #1986, #1988, #2032, #2033, #2034, #2140, #2158, #2356

Each close comment (uniform wording):
> Closing as polyrepo-relocated. This issue targets `crates/terraphim_sessions/` / `crates/terraphim_agent/src/learnings/`, which were extracted to the **terraphim-clients** polyrepo in #1910 (commit `aa7ba99e8`, "remove 25 extracted crate dirs"). The code is not present in terraphim-ai (verified 2026-06-23: directory missing, `SessionConnector`/`ProcedureStore`/`capture_failed_command` not found in-repo). Please re-file in terraphim-clients if still relevant. Refs #2677, #2914 (polyrepo-phantom backlog).

**NOT closed** (per #2677 "keep open, separate concern"): #2776 is a dup of #2084 (close it); #2141 is docs-only with no extracted-crate ref — **leave open**.

### Out of scope
- Implementing the Cursor connector / learn-from-session features (belongs in terraphim-clients)
- The spec doc `terraphim-agent-session-search-tasks.md` (tracks polyrepo work; separate docs issue #2743)
- Re-triaging the ~20 other "session"-keyword issues that are in-repo (#2900 tinyclaw session tests, etc.)

## Next Steps

1. Human/verifier approval of this re-scoped plan
2. Execute cluster + orphan closes via Gitea API with uniform comment
3. Verify open-issue count decreased (~10-12)
4. Post summary comment on #2677 documenting the fabrication finding + actual action taken
