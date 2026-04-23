# Research Document: Token Budget Management (Gitea #707)

**Status**: Draft
**Author**: AI Agent
**Date**: 2026-04-22
**Issue**: https://git.terraphim.cloud/terraphim/terraphim-ai/issues/707
**Specification**: `docs/specifications/terraphim-agent-session-search-tasks.md` - Phase 1, Task 1.5

## Executive Summary

Task 1.5 requires wiring together existing token budget primitives (FieldMode, RobotConfig, TokenBudget, Pagination) that were built in Task 1.1 into an operational pipeline. The core types and schemas exist but are not connected: field filtering is declared but never applied, truncation is isolated, and token budget tracking is never populated in responses. The work is primarily integration -- creating a `budget.rs` module that orchestrates these pieces into a coherent flow.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Core enabler for AI agent integration -- makes robot mode actually usable |
| Leverages strengths? | Yes | All primitives already built; this is wiring, not invention |
| Meets real need? | Yes | Without budget management, AI systems cannot reliably consume output |

**Proceed**: Yes (3/3)

## Problem Statement

### Description

Robot mode output (Task 1.1) returns full, unbounded content. AI agents consuming this output need predictable, bounded responses: limited tokens, filtered fields, truncated content, and pagination metadata. Without this, a single search query could return megabytes of JSON that exceeds an agent's context window.

### Impact

AI agents (Claude Code, Cursor, Aider) cannot use robot mode for non-trivial queries. Any search returning large documents will flood the consumer's context window.

### Success Criteria

An AI agent can call `search "async error handling" --max-tokens 1000 --fields summary --max-content-length 200` and receive a bounded, predictable response with truncation indicators and pagination metadata.

## Current State Analysis

### Existing Implementation

The `robot/` module (`crates/terraphim_agent/src/robot/`) contains 5 files:

| Component | Location | Purpose | Status |
|-----------|----------|---------|--------|
| `output.rs` | `FieldMode`, `OutputFormat`, `RobotConfig`, `RobotFormatter` | Output formatting config + truncation | Types exist, methods isolated |
| `schema.rs` | `RobotResponse`, `ResponseMeta`, `TokenBudget`, `Pagination`, `SearchResultItem` | Response envelope types | Types exist, never populated with budget data |
| `exit_codes.rs` | `ExitCode` | Exit codes | Complete |
| `docs.rs` | `SelfDocumentation` | Self-documentation API | Complete |
| `mod.rs` | Re-exports | Module wiring | Complete |

### Code Locations -- What Exists vs What's Missing

**Already exists (from Task 1.1):**
- `FieldMode` enum: Full, Summary, Minimal, Custom(Vec<String>) -- `output.rs:49-62`
- `RobotConfig.max_tokens: Option<usize>` -- `output.rs:75`
- `RobotConfig.max_results: Option<usize>` -- `output.rs:77`
- `RobotConfig.max_content_length: Option<usize>` -- `output.rs:79`
- `RobotConfig.fields: FieldMode` -- `output.rs:81`
- `RobotFormatter.truncate_content()` -- `output.rs:139-151`
- `RobotFormatter.estimate_tokens()` (4 chars = 1 token) -- `output.rs:153-155`
- `RobotFormatter.would_exceed_budget()` -- `output.rs:157-162`
- `TokenBudget` struct with `max_tokens`, `estimated_tokens`, `truncated` -- `schema.rs:115-132`
- `Pagination` struct with `total`, `returned`, `offset`, `has_more` -- `schema.rs:99-113`
- `SearchResultItem.preview_truncated: bool` -- `schema.rs:184`
- `ResponseMeta.token_budget: Option<TokenBudget>` -- `schema.rs:58`
- `ResponseMeta.pagination: Option<Pagination>` -- `schema.rs:56`

**Missing (the gap):**
1. No `budget.rs` module -- the orchestration layer doesn't exist
2. `FieldMode` is never applied to filter fields from any data type
3. `RobotFormatter.truncate_content()` is never called as part of a pipeline
4. `TokenBudget` is never constructed or populated in `ResponseMeta`
5. `Pagination` is never constructed or populated in `ResponseMeta`
6. No method to apply budget to a `Vec<SearchResultItem>` and return a budgeted result
7. No way to progressively consume results until token budget is exhausted
8. CLI flags (`--max-tokens`, `--max-content-length`, `--max-results`) not wired anywhere

### Data Flow (Current -- Broken)

```
Search query → Results → RobotFormatter.format() → JSON (unbounded)
                                        ↑
                            truncate_content() exists but never called
                            FieldMode exists but never applied
                            TokenBudget schema exists but never populated
```

### Data Flow (Target)

```
Search query → Results → BudgetEngine.apply() → Filtered + Truncated + Paginated Results
                                    ↓
                          1. Filter fields by FieldMode
                          2. Truncate content by max_content_length
                          3. Limit results by max_results OR max_tokens
                          4. Track token usage
                          5. Populate Pagination + TokenBudget in ResponseMeta
```

### Integration Points

- **REPL handler** (`repl/handler.rs`): Currently does NOT handle robot commands at all (Task 1.4 partial)
- **`ReplCommand::Robot`** in `repl/commands.rs`: Has `RobotSubcommand` variants (Capabilities, Schemas, Examples, ExitCodes) but no budget-related subcommand
- **`repl/commands.rs` search parsing**: Already has `--limit` flag but not `--max-tokens`, `--max-content-length`, or `--fields`
- **Module re-exports**: `lib.rs` re-exports `FieldMode`, `RobotConfig`, `RobotFormatter` etc.

## Constraints

### Technical Constraints

1. **No new dependencies**: Issue says "optional tiktoken integration" but we should NOT add tiktoken. The 4-chars-per-token heuristic is sufficient and already implemented.
2. **Feature gates**: Robot module is always available (no feature gate), but search results depend on search features.
3. **Backward compatibility**: `RobotConfig::default()` must remain unchanged (enabled: false, max_results: Some(10)).
4. **No REPL handler dependency**: Since Task 1.4 (REPL integration) isn't complete, budget must work as a standalone library module.

### Non-Functional Requirements

| Requirement | Target | Current |
|-------------|--------|---------|
| Token estimation overhead | < 1ms per result | N/A (not used) |
| Budget application | < 5ms for 100 results | N/A |

## Vital Few (Essentialism)

### Essential Constraints (Max 3)

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Budget pipeline must be standalone | Task 1.4 (REPL integration) is not complete; budget must work without REPL | handler.rs has no robot handling |
| No new dependencies | Project constraint from AGENTS.md | Prior epics avoided new deps |
| Must populate existing schemas | TokenBudget and Pagination already exist in schema.rs; must use them, not create new types | schema.rs:115-132, 99-113 |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| tiktoken integration | YAGNI; 4-char heuristic is sufficient and already implemented |
| CLI flag wiring to REPL commands | Task 1.4 scope; this task focuses on the budget engine |
| Budget for non-search data types | Search results are the primary consumer; generalise later |
| Streaming budget (JSONL) | Out of scope; focus on single-response budget first |

## Dependencies

### Internal Dependencies

| Dependency | Impact | Risk |
|------------|--------|------|
| `robot/output.rs` (RobotConfig, RobotFormatter, FieldMode) | Types we extend | Low -- stable, no changes needed |
| `robot/schema.rs` (TokenBudget, Pagination, SearchResultItem, ResponseMeta) | Types we populate | Low -- just construction, no API changes |
| `robot/mod.rs` | Must add `budget` module export | Trivial |

### External Dependencies

None. This is pure Rust with `serde` and `serde_json` (already in workspace).

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Field filtering requires serde_json::Value manipulation | Medium | Medium | Use serde_json to serialize, filter keys, re-serialize. Acceptable for robot mode. |
| Token estimation inaccuracy | Low | Low | 4-chars-per-token is standard heuristic; document it clearly |
| Budget pipeline API design wrong for future consumers | Medium | Medium | Design as trait-based or closure-based for flexibility |

### Open Questions

1. Should budget flags be on ALL commands or only search? **Assumption: Start with search, extensible to others.**
2. Should `--fields` flag apply to `SearchResultItem` specifically or generically via serde? **Assumption: Start with SearchResultItem-specific, generic later.**
3. Should budget application consume the `Vec<SearchResultItem>` or take a reference? **Assumption: Take reference, return new filtered vec.**

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| Budget pipeline is a library function, not a REPL feature | Task 1.4 is incomplete | Must add REPL dispatch later | Yes |
| 4-chars-per-token heuristic is sufficient | Already implemented, industry standard | Slight undercount for code-heavy content | Yes |
| Field filtering via serde_json::Value is acceptable | Robot mode is JSON-first | Slight performance overhead | No |
| Budget applies to search results only (for now) | Search is the primary consumer | Other data types need adaptation | Yes |

### Multiple Interpretations Considered

| Interpretation | Implications | Why Chosen/Rejected |
|----------------|--------------|---------------------|
| Budget as trait method on RobotFormatter | Tight coupling to formatter | Rejected -- formatter should only format |
| Budget as standalone BudgetEngine struct | Clean separation, testable | Chosen -- single responsibility |
| Budget as free functions in budget.rs | Simplest but less stateful | Alternative if engine is too complex |

## Research Findings

### Key Insights

1. **The gap is wiring, not invention.** All primitive types exist. The work is connecting them into a pipeline.
2. **FieldMode already has 4 variants implemented** (Full, Summary, Minimal, Custom). The mapping from FieldMode to which fields to include/exclude is what's missing.
3. **Token budget is a progressive constraint.** Results must be consumed one-by-one, checking cumulative token count against budget. This is not a simple "take first N results."
4. **The existing `SearchResultItem` already has `preview_truncated: bool`** -- this is the right place for truncation indicators, not a generic `_truncated` wrapper.
5. **`ResponseMeta` already has optional `token_budget` and `pagination` fields** -- we just need to populate them.

### Field Mapping Analysis

For `SearchResultItem`, the field mapping by FieldMode should be:

| Field | Full | Summary | Minimal | Custom |
|-------|------|---------|---------|--------|
| rank | Yes | Yes | Yes | User choice |
| id | Yes | Yes | Yes | User choice |
| title | Yes | Yes | Yes | User choice |
| url | Yes | Yes | - | User choice |
| score | Yes | Yes | Yes | User choice |
| preview | Yes | - | - | User choice |
| source | Yes | - | - | User choice |
| date | Yes | - | - | User choice |
| preview_truncated | Yes | - | - | User choice |

### Token Budget Algorithm

```
fn apply_budget(results, config):
    filtered = filter_fields(results, config.fields)
    truncated = truncate_content(filtered, config.max_content_length)

    if config.max_tokens:
        output = []
        token_count = 0
        for result in truncated:
            serialized = serialize(result)
            tokens = estimate_tokens(serialized)
            if token_count + tokens > max_tokens:
                break
            output.push(result)
            token_count += tokens
    elif config.max_results:
        output = truncated[..max_results]
    else:
        output = truncated

    pagination = Pagination::new(total, output.len(), 0)
    token_budget = TokenBudget::new(max_tokens).with_estimate(token_count)

    return (output, pagination, token_budget)
```

### Relevant Prior Art

- **OpenAI API**: Uses `max_tokens` parameter, returns `usage.prompt_tokens`, `usage.completion_tokens`, `usage.total_tokens`
- **Anthropic API**: Uses `max_tokens`, returns `usage.input_tokens`, `usage.output_tokens`
- Our approach is consistent: budget is pre-emptive (truncate before sending) rather than post-hoc (measure after).

## Recommendations

### Proceed/No-Proceed

**Proceed.** All prerequisites exist. The work is ~300-400 lines of integration code.

### Scope Recommendations

1. Create `budget.rs` with `BudgetEngine` struct
2. Implement field filtering for `SearchResultItem` via serde_json::Value
3. Implement progressive token-budget limiting
4. Wire `BudgetEngine` output to `Pagination` and `TokenBudget` schemas
5. Comprehensive unit tests

### Risk Mitigation Recommendations

- Keep BudgetEngine decoupled from REPL handler (Task 1.4 integration is separate)
- Use `serde_json::Value` for field filtering (simple, correct, acceptable overhead for robot mode)

## Next Steps

If approved:
1. Create design document with exact API signatures
2. Implement `budget.rs`
3. Write unit tests
4. Verify with `cargo test --workspace`

## Appendix

### Code Snippets -- Existing Methods to Integrate

```rust
// output.rs:139-151 -- Truncation (already exists)
pub fn truncate_content(&self, content: &str) -> (String, bool) {
    if let Some(max_len) = self.config.max_content_length {
        if content.len() > max_len {
            let truncated = if let Some(pos) = content[..max_len].rfind(char::is_whitespace) {
                &content[..pos]
            } else {
                &content[..max_len]
            };
            return (format!("{}...", truncated), true);
        }
    }
    (content.to_string(), false)
}

// output.rs:153-155 -- Token estimation (already exists)
pub fn estimate_tokens(&self, text: &str) -> usize {
    text.len() / 4
}

// schema.rs:99-113 -- Pagination (already exists, needs populating)
pub struct Pagination {
    pub total: usize,
    pub returned: usize,
    pub offset: usize,
    pub has_more: bool,
}

// schema.rs:115-132 -- TokenBudget (already exists, needs populating)
pub struct TokenBudget {
    pub max_tokens: usize,
    pub estimated_tokens: usize,
    pub truncated: bool,
}
```
