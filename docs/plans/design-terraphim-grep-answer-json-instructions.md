# Implementation Plan: Fix terraphim_grep answer parsing by including JSON instructions

**Status**: Draft
**Canonical Path**: `docs/plans/design-terraphim-grep-answer-json-instructions.md`
**Change Slug**: `terraphim-grep-answer-json-instructions`
**Research**: `docs/plans/research-terraphim-grep-answer-json-instructions.md`
**Author**: OpenCode
**Date**: 2026-07-13
**Estimated Effort**: 0.5 day

## Overview

### Summary

Append the existing `AnswerSignature::instructions()` string to the user
prompt in `terraphim_grep::lib.rs` so the LLM knows it must return JSON. This
aligns the prompt with the parser and fixes the silent null-answer bug for
models that emit plain text.

### Approach

Re-use the already-defined `AnswerSignature::instructions()` text and include
it conditionally when `options.include_answer` is true. No parser or API
changes are required.

### Scope

**In Scope:**
- Modify the prompt in `crates/terraphim_grep/src/lib.rs`.
- Add unit tests for prompt content and JSON parsing.
- Run live verification against one previously failing model
  (`deepseek/deepseek-v4-flash`) and one previously working model
  (`amazon/nova-micro-v1`).
- Update the trade-off doc in `terraphim-ai` with new benchmark results.

**Out of Scope:**
- Changing `terraphim_service` or any other crate.
- Adding retry logic or fallback plain-text parsing.
- Refactoring the signature system.

**Avoid At All Cost**:
- Rewriting the parser to accept plain text (would lose structured citations).
- Adding a new configuration flag to toggle JSON instructions.

## Architecture

### Component Diagram

```
+-----------------+        +-------------------+
|  GrepOptions    |        |  AnswerSignature  |
| include_answer  |------->|  .instructions()  |
+-----------------+        +-------------------+
           |                         |
           v                         v
+--------------------------------------------------+
|  search_with_rlm_fallback (lib.rs)               |
|  - build RlmContext prompt                       |
|  - append instructions if include_answer         |
|  - call chat_completion                          |
+--------------------------------------------------+
                           |
                           v
+--------------------------------------------------+
|  AnswerSignature::parse(llm_response)            |
|  - expects JSON object                           |
+--------------------------------------------------+
```

### Data Flow

```
Query + chunks + KG concepts
  -> RlmContext.build_prompt()
  -> lib.rs appends task instruction + JSON format instruction
  -> LlmClient.chat_completion()
  -> provider (OpenRouter/Ollama)
  -> raw response
  -> AnswerSignature::parse()
  -> structured AnswerWithCitations
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|---|---|---|
| Append instructions to the user message | Works with all providers; minimal diff; matches `kg_curation.rs` pattern. | Separate `system` message — valid but adds a second message for a 1-line fix. |
| Only include JSON instructions when `include_answer` is true | The parser is only invoked in that path. | Always include instructions — harmless but unnecessary. |
| Keep `AnswerSignature::instructions()` unchanged | Already correct and matches the parser. | Rewrite instructions — no need. |

## Expected Lifecycle Artefacts

| Artefact | Path | Required? |
|---|---|---|
| Research | `docs/plans/research-terraphim-grep-answer-json-instructions.md` | Yes — produced |
| Design | `docs/plans/design-terraphim-grep-answer-json-instructions.md` | Yes — this doc |
| ADR | not needed | No — trivial local change |
| Verification | `docs/verification/verification-report-terraphim-grep-answer-json-instructions.md` | Yes — after implementation |
| Traceability | `docs/verification/traceability-matrix-terraphim-grep-answer-json-instructions.md` | Optional for this size |

## File Changes

### Modified Files

| File | Changes |
|---|---|
| `crates/terraphim_grep/src/lib.rs` | Append `AnswerSignature::instructions()` to the user message when `options.include_answer` is true. |
| `crates/terraphim_grep/src/lib.rs` | Add unit tests for prompt content and parsing. |
| `docs/openrouter-model-tradeoff-2026-07-13.md` (terraphim-ai) | Update benchmark results and recommendations after re-run. |

### No New or Deleted Files

## API Design

No public API changes.

## Test Strategy

### Unit Tests

| Test | Location | Purpose |
|---|---|---|
| `test_answer_prompt_includes_json_instructions` | `crates/terraphim_grep/src/lib.rs` | Asserts the user message contains the `AnswerSignature` instructions when `include_answer` is true. |
| `test_answer_signature_parses_valid_json` | `crates/terraphim_grep/src/signatures.rs` (existing area) | Asserts a representative JSON response parses into `AnswerWithCitations`. |
| `test_answer_signature_rejects_plain_text` | `crates/terraphim_grep/src/signatures.rs` | Ensures plain text still fails parsing (regression guard). |

### Integration / Live Verification

| Test | Command | Purpose |
|---|---|---|
| DeepSeek V4 Flash | `OPENROUTER_MODEL=deepseek/deepseek-v4-flash terraphim-grep "fn main" --haystack code --paths crates/terraphim_rlm/src --answer --json` | Confirm non-null `answer`. |
| Amazon Nova Micro | `OPENROUTER_MODEL=amazon/nova-micro-v1 terraphim-grep "fn main" --haystack code --paths crates/terraphim_rlm/src --answer --json` | Confirm still non-null. |

### Benchmark Regression

Re-run:

```bash
python3 scripts/benchmark_openrouter_tradeoff/benchmark.py \
  --query "fn main" \
  --paths crates/terraphim_rlm/src
```

Compare `results-*.json` with the baseline to confirm previously failing
models now return answers.

## Implementation Steps

### Step 1: Include instructions in the prompt

**File:** `crates/terraphim_grep/src/lib.rs`
**Description:** Modify `search_with_rlm_fallback` to append
`AnswerSignature::instructions()` when `options.include_answer` is true.
**Estimated:** 15 minutes

```rust
let instructions = if options.include_answer {
    Some(signatures::AnswerSignature {}.instructions())
} else {
    None
};

let messages = vec![serde_json::json!({
    "role": "user",
    "content": format!(
        "{}\n\n{}\n\n{}\n\nProvide a brief answer based on the context above.",
        prompt,
        if options.include_answer {
            "Synthesise an answer."
        } else {
            "List the relevant findings."
        },
        instructions.unwrap_or_default()
    )
})];
```

Care must be taken to avoid an extra blank line when `instructions` is empty.
A cleaner formulation:

```rust
let task = if options.include_answer {
    "Synthesise an answer."
} else {
    "List the relevant findings."
};
let format_instructions = if options.include_answer {
    signatures::AnswerSignature {}.instructions()
} else {
    String::new()
};
let content = if format_instructions.is_empty() {
    format!("{}\n\n{}\n\nProvide a brief answer based on the context above.", prompt, task)
} else {
    format!(
        "{}\n\n{}\n\n{}\n\nProvide a brief answer based on the context above.",
        prompt, task, format_instructions
    )
};
let messages = vec![serde_json::json!({"role": "user", "content": content})];
```

### Step 2: Add unit tests

**File:** `crates/terraphim_grep/src/lib.rs`
**Description:** Add tests that verify the prompt contains the expected
format instructions and that the parser works on representative output.
**Estimated:** 45 minutes
**Dependencies:** Step 1

### Step 3: Build and run unit tests

```bash
cargo test -p terraphim_grep
```

**Estimated:** 5 minutes

### Step 4: Live verification

Run the two live commands listed in the Test Strategy.
**Estimated:** 30 minutes (includes waiting for API responses)

### Step 5: Re-run benchmark and update trade-off doc

**Files:** `docs/openrouter-model-tradeoff-2026-07-13.md`,
`scripts/benchmark_openrouter_tradeoff/results-*.json`
**Estimated:** 20 minutes

### Step 6: Commit and create PR in terraphim-clients

**Estimated:** 15 minutes

## Rollback Plan

If regressions are detected:

1. Revert the single commit changing `crates/terraphim_grep/src/lib.rs`.
2. Re-run unit tests and the Nova Micro live check to confirm previous behaviour
   is restored.

No feature flags are required.

## Dependencies

### No New Dependencies

### Dependency Updates

None.

## Performance Considerations

### Expected Performance

| Metric | Target | Measurement |
|---|---|---|
| Prompt token increase | < 100 tokens | Count instructions text. |
| RLM latency change | Within normal variance | Benchmark before/after. |

### Benchmarks to Add

None; reuse `scripts/benchmark_openrouter_tradeoff/benchmark.py`.

## Open Items

| Item | Status | Owner |
|---|---|---|
| Confirm whether `system` message alternative is preferred by maintainer | Pending | Reviewer |

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Human approval received
