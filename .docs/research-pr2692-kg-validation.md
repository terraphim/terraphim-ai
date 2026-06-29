# Research Document: PR #2692 — Wire KG Validation into QueryLoop Hot Paths

**Status**: Draft
**Author**: AI Agent
**Date**: 2026-06-29

## Executive Summary

PR #2692 aimed to wire KG validation into QueryLoop hot paths but was created before the merge sprint. The merge sprint landed multiple PRs (#2902, #2913, #3012, #3033, #3034) that independently implemented most of the PR's goals. The PR is largely superseded. Remaining extractable value: `from_config()` on `KnowledgeGraphValidator`, `#[derive(Debug)]` on `ExecuteResult`, and associated tests.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energising? | Yes | Closes a stale PR and extracts remaining value |
| Leverages strengths? | Yes | Understanding of already-merged codebase |
| Meets real need? | Yes | Issue #2671 still closed; PR adds missing `from_config()` deduplication |

**Proceed**: Yes (3/3 YES)

## Problem Statement

### Description
PR #2692 (47 files, 9566 additions) attempts to wire KG validation but has 39 noise files (handoff artifacts, CI fixes, review dumps) and 8 core RLM files. The core work was independently implemented on main through the merge sprint (19 RLM files changed, 1773 lines added). The PR now has merge conflicts and cannot be merged as-is.

### Impact
- Wastes review time on noise
- PR blocks issue #2671 which should be closed
- `from_config()` deduplication method is missing from main

### Success Criteria
1. Extract unique remaining value from PR #2692
2. Apply it cleanly to main
3. Verify build passes
4. Close PR #2692 as superseded

## Current State Analysis

### What PR #2692 Changed (merge-base to PR branch)
| File | Lines | What |
|------|-------|------|
| `query_loop.rs` | +214 | Validator field, validation in hot paths, tests |
| `rlm.rs` | +32/-? | Arc<Validator>, from_config calls |
| `validator.rs` | +134 | from_config() with thesaurus loading + 5 tests |
| `executor/mod.rs` | +20 | with_validator() wiring in select_executor |
| `executor/docker.rs` | +27 | validator field, with_validator(), validate impl |
| `executor/firecracker.rs` | +32 | Same pattern |
| `executor/local.rs` | +48 | Same pattern + 2 tests |
| `config.rs` | +21 | kg_thesaurus_path field, blocks_unknown Normal fix |

### What's Already in Main (merge-base to current main)
| Feature | Where | How |
|---------|-------|-----|
| Executor validate() with KG | executor/{local,docker,firecracker}.rs | `Option<Arc<KnowledgeGraphValidator>>` field |
| with_validator() | executor/{local,docker}.rs | `pub fn with_validator(mut self, v: Option<Arc<...>>)` |
| validate_command() in QueryLoop | query_loop.rs | `self.executor.validate(input).await` with retry logic |
| validation_retries cell | query_loop.rs | `Cell<u32>` for retry tracking |
| blocks_unknown Normal fix | config.rs | Already returns true for Normal mode |
| Arc<KnowledgeGraphValidator> in rlm | rlm.rs | Already present |

### What's NOT Yet in Main (extractable from PR)
| Feature | Value |
|---------|-------|
| `from_config()` on KnowledgeGraphValidator | Eliminates duplicate build_validator in rlm.rs and executor/mod.rs |
| `#[derive(Debug)]` on ExecuteResult | Minor but useful for debugging |
| 5 from_config tests in validator.rs | Needed test coverage |
| `strictness()` accessor | Exposes validator strictness |

## Constraints

### Technical Constraints
- Must not break existing validation architecture (executor-based validate_command)
- Must compile with `cargo check --workspace`
- Must pass `cargo clippy --workspace`
- Must pass `cargo fmt --all -- --check`

## Vital Few

| Constraint | Why Vital | Evidence |
|------------|-----------|----------|
| Only add from_config, don't restructure | Existing architecture works; PR's approach is incompatible | Main already has 1773 lines of validated RLM changes |
| Don't pull noise files | 39 files are review artifacts, not code | File list shows .review_tmp/, .handoff/, .beads/, .sessions/ |

## Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| `from_config()` signature mismatch with main | Medium | Low | Check existing validator API before implementing |
| Tests fail due to different test framework | Low | Medium | Run tests before commit |

## Recommendations

**Proceed** with minimal extraction: add `from_config()` (+ tests) and `#[derive(Debug)]` to main, close PR #2692.
