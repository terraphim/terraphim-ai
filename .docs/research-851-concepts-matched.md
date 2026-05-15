# Research Document: #851 Populate concepts_matched and wildcard_fallback

**Status**: Approved
**Date**: 2026-05-15

## Executive Summary

Two fields in `SearchResultsData` (`concepts_matched`, `wildcard_fallback`) are hardcoded to empty/false at two emission sites in `main.rs`. The upstream `terraphim_automata::find_matches` API and `service.get_thesaurus()` provide everything needed to populate them.

## Problem Statement

- `concepts_matched: vec![]` and `wildcard_fallback: false` at lines 2180-2181 and 4236-4237
- Consumers cannot see which KG concepts drove the ranking, nor detect silent query widening

## Current State

| Component | Location | Purpose |
|-----------|----------|---------|
| `SearchResultsData` | `robot/schema.rs:284-295` | Struct with the two fields |
| Emission site 1 | `main.rs:2177-2182` | Direct service search (REPL mode) |
| Emission site 2 | `main.rs:4233-4238` | API client search (remote mode) |
| `find_matches` | `terraphim_automata/src/matcher.rs:18` | Returns `Vec<Matched>` with `.term` |
| `get_thesaurus` | `service.rs:342-346` | Returns `Thesaurus` for a role |

## Constraints

- Do NOT alter `SearchResultsData` struct shape
- Reuse `find_matches` from `terraphim_automata` -- do not duplicate
- Scope <= 500 LOC

## Key Insight

`service.get_thesaurus(&role_name)` is already called elsewhere in `main.rs` (line 2406). The same pattern can extract matched concepts from the query string. For wildcard: if `results.len() == 0` and there was a retry or broadening, set `wildcard_fallback = true`. Currently neither site retries -- so `wildcard_fallback` will be `false` for now, but the plumbing will be in place.

## Recommendations

Proceed. Simple, well-bounded change.
