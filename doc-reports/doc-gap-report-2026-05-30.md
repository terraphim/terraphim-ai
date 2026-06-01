# Documentation Gap Report — 2026-05-30

**Run by**: documentation-generator (Ferrox)
**Date**: 2026-05-30
**Command**: `RUSTDOCFLAGS="-W missing-docs" cargo doc --no-deps`

## Summary

| Crate | Before | After | Status |
|-------|--------|-------|--------|
| terraphim_types | 93 | 0 | FIXED |
| terraphim_service | 0 | 0 | PASS |
| terraphim_middleware | 0 | 0 | PASS |
| terraphim_rolegraph | 0 | 0 | PASS |
| terraphim_automata | 0 | 0 | PASS |
| terraphim_config | 0 | 0 | PASS |
| terraphim_persistence | 0 | 0 | PASS |
| terraphim_mcp_server | 0 | 0 | PASS |

## Changes Made

### terraphim_types (93 → 0 warnings)

**`crates/terraphim_types/src/llm_usage.rs`** — 19 warnings fixed:
- `LlmUsage` struct: added field docs for `input_tokens`, `output_tokens`, `model`, `provider`, `cost_usd`, `latency_ms`
- `LlmUsage::total_tokens()`, `LlmUsage::with_cost()` — method docs added
- `LlmResult` struct: field docs for `content`, `usage`; method docs for `new()`, `with_usage()`
- `ModelPricing` struct: field docs for `model_pattern`, `input_cost_per_million_tokens`, `output_cost_per_million_tokens`; method doc for `calculate_cost()`

**`crates/terraphim_types/src/review.rs`** — 22 warnings fixed:
- `FindingSeverity` variants: `Info`, `Low`, `Medium`, `High`, `Critical` — doc comments added
- `FindingCategory` variants: `Security`, `Architecture`, `Performance`, `Quality`, `Domain`, `DesignQuality` — doc comments added
- `ReviewFinding` fields: `file`, `line`, `severity`, `category`, `finding`, `suggestion`, `confidence` — doc comments added
- `ReviewAgentOutput` fields: `agent`, `findings`, `summary`, `pass` — doc comments added

**`crates/terraphim_types/src/score/mod.rs`** — 27 warnings fixed:
- Added `//!` module-level doc for the `score` module and sub-modules (`bm25`, `bm25_additional`, `common`, `names`)
- `sort_documents()` function — doc comment added
- `Scorer` struct — struct doc; `new()`, `with_similarity()`, `with_scorer()`, `score()` — method docs added
- `ScoreError::Scoring` variant — doc comment added
- `Query` struct and fields (`name`, `name_scorer`, `similarity`, `size`) — docs added; methods `new()`, `is_empty()`, `name_scorer()`, `similarity()` — docs added
- `Similarity` enum variants: `None`, `Levenshtein`, `Jaro`, `JaroWinkler` — docs added; `similarity()` method — doc added

**`crates/terraphim_types/src/lib.rs`** — 25 warnings fixed:
- `pub mod score;` — inline doc added
- `NormalizedTermValue::new()`, `as_str()` — docs added
- `DocumentType` variants: `KgEntry`, `Document`, `ConfigDocument` — docs added
- `RouteDirective` fields: `provider`, `model` — docs added
- `MarkdownDirectives` fields: `doc_type`, `synonyms`, `priority`, `trigger`, `pinned` — docs added
- `Edge::new()` — doc added
- `Thesaurus::keys()` — doc added
- `IndexedDocument::to_json_string()`, `from_document()` — docs added
- `ConversationId::new()`, `from_string()`, `as_str()` — docs added
- `MessageId::new()`, `from_string()`, `as_str()` — docs added
- `ContextHistory::new()` — doc added
- `MultiAgentContext::new()` — doc added

## CHANGELOG Updates

Added 6 entries for PRs merged 2026-05-20 to 2026-05-30:

**Added:**
- PR#1885 (2026-05-30): ADF direct-dispatch remediation
- PR#1876 (2026-05-28): adf-ctl Unix socket dispatch
- PR#1825 (2026-05-24): terraphim_grep hybrid searcher
- PR#1823 (2026-05-23): terraphim_merge_coordinator skeleton
- PR#1822 (2026-05-23): Config-error circuit-breaker

**Fixed:**
- PR#1844 (2026-05-24): terraphim_service genai dep moved to crates.io
- PR#1843 (2026-05-24): publish=false removed from terraphim_service
- PR#1794 (2026-05-22): ADF KG-router fallback respawn loop closed

## Commit

`6ef389ad` — `docs(types): eliminate 93 missing-doc warnings in terraphim_types`

Refs #1866
