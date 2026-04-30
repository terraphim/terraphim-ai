# Research Document: Documentation and Website Alignment

**Status**: Draft
**Author**: opencode (automated validation)
**Date**: 2026-04-30
**Reviewers**: alex

## Executive Summary

Full validation of all published documentation (terraphim.ai, terraphim.rs, docs.terraphim.ai) against terraphim-agent v1.17.0 built from latest main reveals 2 critical bugs, 4 high-severity documentation inaccuracies, 4 medium-severity broken links/mismatches, and 4 low-severity issues. The Quickstart guide describes a completely fabricated REPL experience that does not match the actual binary.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|---------|
| Energizing? | Yes | Documentation is the first impression; broken docs = lost users |
| Leverages strengths? | Yes | We have the validation data and both repos to fix |
| Meets real need? | Yes | Users cannot follow the Quickstart; search crashes |

**Proceed**: Yes -- 3/3

## Problem Statement

### Description

Published documentation on terraphim.ai and docs.terraphim.ai does not match the actual behaviour of terraphim-agent v1.17.0. The Quickstart guide documents a REPL that does not exist. The search command panics on the default config. Version numbers are stale across all pages.

### Impact

- New users cannot follow the Quickstart (commands don't work)
- The core `search` command crashes (panic in thesaurus fallback)
- The `evaluate` subcommand is documented but not implemented
- Version numbers (v1.16.0 header, v1.16.31 downloads) lag behind actual v1.17.0

### Success Criteria

1. Every command shown in published docs runs without error on a fresh install
2. Version numbers are consistent and current
3. The Quickstart guide describes the actual REPL experience
4. All internal links resolve (no 404s)
5. No documented subcommands that don't exist

## Current State Analysis

### Existing Implementation

Two repositories:

| Repo | Path | Purpose |
|------|------|---------|
| terraphim-ai | `~/projects/terraphim/terraphim-ai` | Main codebase, docs/src/, docs.terraphim.ai content |
| terraphim.ai | `~/projects/terraphim/terraphim.ai` | Zola website, content/ directory |

### Code Locations -- Website

| Component | Location | Purpose |
|-----------|----------|---------|
| Zola config | `terraphim.ai/config.toml` | Version numbers, nav, metadata |
| Homepage | `terraphim.ai/content/_index.md` | Landing page |
| Quickstart | `terraphim.ai/content/docs/quickstart.md` | Fabricated REPL guide |
| terraphim-agent cap | `terraphim.ai/content/capabilities/terraphim-agent.md` | Documents `evaluate` subcommand |
| Installation | `terraphim.ai/content/docs/installation.md` | Download links with version refs |
| Releases | `terraphim.ai/content/releases.md` | Version info |
| Blog posts (7) | `terraphim.ai/content/posts/*.md` | All verified present |
| Capability pages (10) | `terraphim.ai/content/capabilities/*.md` | All verified present |

### Code Locations -- Main Repo (docs)

| Component | Location | Purpose |
|-----------|----------|---------|
| Command rewriting howto | `docs/src/command-rewriting-howto.md` | Accurate, matches reality |
| Learning capture (Claude) | `docs/src/howto/learning-capture-claude-code.md` | Accurate |
| Learning capture (opencode) | `docs/src/howto/learning-capture-opencode.md` | Accurate |
| MCP integration | `docs/src/howto/mcp-integration-claude-opencode.md` | Accurate |
| TUI docs | `docs/src/tui.md` | Mostly accurate (REPL commands match) |
| Learning compile | `docs/src/learning-compile.md` | Accurate |

### Actual REPL Commands (v1.17.0)

From `terraphim-agent repl --help` and live testing:

```
/search <query> [--role <role>] [--limit <n>] [--semantic] [--concepts]
/config [show|set]
/role [list|select]
/graph
/chat [message]
/summarize <target>
/update <subcommand>  (check, install, rollback, list)
/help [command]
/quit
```

### Actual CLI Subcommands (v1.17.0)

From `terraphim-agent --help`:

```
search, roles, config, graph, chat, extract, replace, validate,
suggest, hook, guard, interactive, repl, setup, check-update,
update, learn, sessions, listen
```

NOT present: `evaluate`, `import`, `export`, `connect`

## Constraint Identification

### Technical Constraints
- Zola static site (terraphim.ai) -- changes require `zola build` and deploy
- mdBook (docs.terraphim.ai) -- changes require mdbook build from docs/src/
- Version numbers in `config.toml` must be updated on each release
- The `evaluate` subcommand is NOT implemented -- must be removed from docs OR implemented
- The thesaurus panic is a CODE bug, not a docs bug -- tracked separately

### Business Constraints
- Website changes deploy via Cloudflare Pages
- Blog posts are marketing material -- tone must be preserved
- Download links must point to correct GitHub release assets

### Integration Constraints
- `terraphim.rs` is a separate deployment (different repo or same?)
- docs.terraphim.ai builds from `docs/src/` in terraphim-ai repo

## Vital Few (Essentialism)

### Essential Constraints (Max 3)

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Quickstart must match actual REPL | First experience for new users | Every command in current Quickstart fails |
| Version numbers must be current | Trust signal, download accuracy | v1.16.0 header vs v1.17.0 binary |
| No documented features that don't exist | Users will try them and lose trust | `evaluate` subcommand documented but missing |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Fix the thesaurus panic (code bug) | Separate issue, code fix not docs fix |
| Rewrite blog posts | Marketing material, not inaccurate in substance |
| terraphim.rs full audit | Different repo, mostly accurate |
| docs.terraphim.ai full audit (200+ pages) | Too large, focus on user-facing pages |
| Implement `evaluate` subcommand | Out of scope for docs fix |
| MedGemma evaluation claims in sub-millisecond post | Marketing, not user-blocking |

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| New release changes REPL commands again | Medium | High | Pin docs to current version, note "as of v1.17.0" |
| Download links break on next release | High | Medium | Use `/latest/` URL pattern instead of hardcoded version |
| Stale version in config.toml repeats | High | Low | Add release checklist item |

### Open Questions

1. Should the `evaluate` subcommand be REMOVED from docs or should it be IMPLEMENTED first? -- Decision needed from Alex
2. Is `terraphim-cli` published on crates.io? If not, should docs reference it? -- Needs verification
3. Should the Quickstart describe REPL mode or CLI subcommands as primary? -- UX decision
4. terraphim.rs -- is it in the same repo or separate? -- Needs clarification

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| terraphim.ai is built with Zola | config.toml present | Wrong build process | Yes |
| docs.terraphim.ai builds from docs/src/ | mdBook structure in repo | Wrong source files | Yes |
| Version should be updated to v1.17.0 | Latest built binary | Premature if not released | Partially |
| The Quickstart REPL section should describe actual /commands | Live testing | Wrong UX if design changes | Yes |

## Research Findings

### Key Insights

1. **The Quickstart is the #1 problem** -- it describes an entirely fictional REPL experience that would frustrate any new user attempting to follow it
2. **Local repo docs (docs/src/howto/*.md) are accurate** -- the command-rewriting howto, learning capture guides, and MCP integration guide all match the actual binary
3. **The website Quickstart was likely generated by an LLM** that hallucinated the REPL interface without verifying against the binary
4. **The `evaluate` subcommand was planned but never implemented** -- it appears in the capability page but has no corresponding code
5. **Version drift is systematic** -- config.toml has `version = "1.16.0"` and `release_version = "1.16.31"` but binary is v1.17.0

### Relevant Prior Art

- The local docs in `docs/src/tui.md` are mostly accurate for the REPL commands
- The `docs/src/howto/mcp-integration-claude-opencode.md` is a well-written, accurate guide that could serve as a template for the Quickstart rewrite

## Recommendations

### Proceed/No-Proceed

**Proceed** -- The fixes are straightforward documentation edits. The code bug (thesaurus panic) should be filed as a separate issue.

### Scope Recommendations

1. **Must fix**: Quickstart rewrite, version bump, remove `evaluate` reference
2. **Should fix**: Fix internal links, standardise download URLs
3. **Nice to have**: Add "as of v1.17.0" version annotations

### Risk Mitigation Recommendations

- Pin version references with "as of vX.Y.Z" footnotes
- Add a CI check that validates documented commands against the binary

## Next Steps

If approved:
1. File a GitHub issue for the thesaurus panic (code bug)
2. Proceed to Phase 2 design plan
3. Execute documentation fixes across both repos
