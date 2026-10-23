# Research: Fix terraphim_grep answer parsing by including JSON instructions

**Status**: Draft
**Canonical Path**: `docs/plans/research-terraphim-grep-answer-json-instructions.md`
**Change Slug**: `terraphim-grep-answer-json-instructions`
**Author**: OpenCode
**Date**: 2026-07-13
**Reviewers**: TBD

## Executive Summary

`terraphim-grep --answer` silently drops valid LLM answers for several cheap,
high-throughput OpenRouter models (including `deepseek/deepseek-v4-flash`). The
root cause is a prompt/parser mismatch: the parser expects JSON, but the prompt
never asks the model for JSON. The fix is to include `AnswerSignature`'s
existing JSON instructions in the prompt sent to the LLM.

## Essential Questions Check

| Question | Answer | Evidence |
|---|---|---|
| Energizing? | Yes | Improves reliability of a core user-facing feature and removes a misleading "model doesn't work" signal. |
| Leverages strengths? | Yes | Straightforward prompt-engineering fix in a Rust crate we can build and test. |
| Meets real need? | Yes | Users selecting models from the OpenRouter catalogue expect answers to be returned, not dropped. |

**Proceed**: Yes.

## Problem Statement

### Description

When `terraphim-grep --answer` triggers RLM fallback, the `AnswerSignature`
parser expects the LLM response to be a JSON object with `answer`, `citations`,
and `confidence` fields. However, the prompt sent to the LLM does not include
those requirements. Some models infer JSON format and succeed; others return
plain text or an empty response, which the parser rejects and the JSON output
shows `"answer": null`.

### Impact

- Users see `null` answers for models that actually produced good text.
- Model-selection benchmarks are misleading (e.g. DeepSeek V4 Flash appears
  broken when it is not).
- The feature degrades silently; there is no user-facing error explaining that
  the response could not be parsed.

### Success Criteria

- `terraphim-grep --answer` returns non-null answers for
  `deepseek/deepseek-v4-flash`.
- The same models that already worked continue to work.
- The fix is tested with at least one previously failing model and one
  previously working model.

## Current State Analysis

### Existing Implementation

In `crates/terraphim_grep/src/lib.rs`, `search_with_rlm_fallback` builds a
single user message and calls the LLM:

```rust
let messages = vec![serde_json::json!({
    "role": "user",
    "content": format!(
        "{}\n\n{}\n\nProvide a brief answer based on the context above.",
        prompt,
        if options.include_answer {
            "Synthesise an answer."
        } else {
            "List the relevant findings."
        }
    )
})];
```

The response is then parsed by `AnswerSignature`:

```rust
pub struct AnswerSignature;

impl RlmSignature for AnswerSignature {
    fn instructions(&self) -> String {
        r#"Return a JSON object with:
- "answer": the synthesised answer
- "citations": array of {source, line (optional), excerpt}
- "confidence": a number between 0 and 1"#.to_string()
    }

    fn parse(&self, raw: &str) -> Result<Self::Output, TerraphimGrepError> {
        serde_json::from_str(raw)
            .map_err(|e| TerraphimGrepError::RlmFailed(format!("failed to parse answer: {}", e)))
    }
}
```

The `instructions()` method is defined but never called when building the
answer prompt.

### Code Locations

| Component | Location | Purpose |
|---|---|---|
| Prompt builder | `crates/terraphim_grep/src/lib.rs` | Builds user message sent to LLM |
| Answer parser | `crates/terraphim_grep/src/signatures.rs` | Defines expected JSON format |
| RLM bridge | `crates/terraphim_service/src/llm.rs` (via registry) | Routes to configured LLM client |

### Data Flow

```
User query
  -> hybrid search (fff-search + KG boost)
  -> sufficiency judge decides RLM fallback
  -> RlmContext.build_prompt()
  -> lib.rs builds user message
  -> LlmClient.chat_completion()
  -> OpenRouter (or Ollama)
  -> raw response string
  -> AnswerSignature.parse()
  -> if parse fails, answer = None
```

### Integration Points

- `terraphim_service::llm::LlmClient::chat_completion` is provider-agnostic;
  the fix stays entirely in `terraphim_grep`.
- `kg_curation.rs` already calls `ConceptExtractionSignature::instructions()`
  when building its prompt, so there is an established pattern to copy.

## Constraints

### Technical Constraints

- The fix must not break existing models that currently return JSON.
- `terraphim_grep` uses `serde_json::from_str` strictly; malformed JSON must
  still be rejected.
- The change must be in the `terraphim-clients` repository because the
  `terraphim_grep` crate lives there.

### Business Constraints

- Low risk and fast: this is a bug fix, not a feature.
- Must be back-portable to the released crate (v1.20.3) if a patch release is
  needed.

### Non-Functional Requirements

| Requirement | Target | Current |
|---|---|---|
| Answer non-null rate for DeepSeek V4 Flash | > 90 % | 0 % |
| No regression for Amazon Nova Micro | > 90 % | ~100 % |
| Additional latency from prompt | < 1 % | 0 % (instructions are tiny) |

## Vital Few

### Essential Constraints

| Constraint | Why It's Vital | Evidence |
|---|---|---|
| Keep prompt format compatible with all existing working models | Avoid regressions for users already relying on `amazon/nova-micro-v1` etc. | Benchmark shows these models work today. |
| Fix must be testable without live OpenRouter calls | Deterministic unit tests cannot depend on external APIs. | Existing codebase has unit tests for signatures. |
| Fix only touches `terraphim_grep` | Minimises blast radius; `terraphim_service` is shared with server/TUI. | The bug is local to prompt construction. |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|---|---|
| Rewriting `AnswerSignature` to accept plain text and JSON | Adds complexity and still leaves citations/confidence unstructured; JSON instructions are simpler. |
| Changing the parser to be lenient | Would mask malformed output and reduce confidence in citations. |
| Adding retry logic for parse failures | Treats symptom, not cause; prompt fix should remove need for retries. |

## Dependencies

### Internal Dependencies

| Dependency | Impact | Risk |
|---|---|---|
| `terraphim_service::llm::LlmClient` | Receives messages; no API change needed. | Low |
| `terraphim_grep::signatures::AnswerSignature` | Instructions already defined; just need to use them. | Low |

### External Dependencies

| Dependency | Version | Risk | Alternative |
|---|---|---|---|
| OpenRouter API | live | Low — verification uses real API but tests do not. | Ollama for local validation. |

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Some models still produce malformed JSON after instruction change | Low | Medium | Add a fallback log line so users can see raw output when parsing fails. |
| Increasing prompt length slightly raises token cost | Very low | Very low | Instructions are < 100 tokens; cost is negligible. |
| `include_answer = false` path now also asks for JSON | Low | Low | Use conditional instructions: JSON only when an answer is expected. |

### Open Questions

1. Should the instructions be a `system` message or appended to the `user`
   message? — Design phase will decide; both satisfy the constraint.
2. Do we want to expose parse failures in the CLI output? — Product decision;
   not blocking the fix.

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|---|---|---|---|
| `AnswerSignature::instructions()` is the correct format string. | It matches `AnswerWithCitations` struct fields. | Parser still fails if mismatched. | Yes — direct API call with these instructions produced valid JSON. |
| All LLM providers accept a `system` role message. | OpenAI/Anthropic/OpenRouter/Ollama support system messages. | Some providers reject system role; mitigation is appending to user message. | Partially — widely supported. |
| The fix does not need to change `terraphim_service`. | The bug is in prompt construction, not in the provider client. | Unnecessary churn in shared crate. | Yes — raw API calls prove the model works. |

### Multiple Interpretations Considered

| Interpretation | Implications | Why Chosen/Rejected |
|---|---|---|
| A: Append instructions to user message | Minimal change, single message, works everywhere. | **Preferred** — simplest, no role concerns. |
| B: Add a separate `system` message | Cleaner separation of instructions from context. | Valid alternative; keep as fallback if providers support it. |
| C: Rewrite parser to accept plain text | Removes need for JSON but loses structured citations/confidence. | Rejected — structured output is a design requirement. |

## Research Findings

### Key Insights

1. The parser already knows the required format; the prompt builder simply
   forgot to ask for it.
2. Direct OpenRouter API calls prove the models are capable; the issue is
   purely prompt/parser alignment.
3. `kg_curation.rs` shows the intended pattern: call `.instructions()` and
   include it in the message.

### Relevant Prior Art

- `crates/terraphim_grep/src/kg_curation.rs:46` uses
  `ConceptExtractionSignature {}.instructions()` in its prompt.

### Technical Spikes Needed

| Spike | Purpose | Estimated Effort |
|---|---|---|
| Verify fix with DeepSeek V4 Flash live | Confirm non-null answer after prompt change. | 15 minutes |
| Verify no regression with Nova Micro | Confirm existing model still works. | 15 minutes |

## Recommendations

### Proceed/No-Proceed

Proceed. The fix is low-risk, high-value, and the root cause is confirmed.

### Scope Recommendations

- In scope: include JSON instructions in the answer prompt; add/update unit
  tests; run live verification with at least two models.
- Out of scope: parser rewrites, retry logic, UI error messages.

### Risk Mitigation Recommendations

- Add a unit test asserting that the prompt contains the JSON instructions.
- Add a unit test asserting that a valid JSON LLM response is parsed into
  `AnswerWithCitations`.
- Run the existing benchmark script after the fix to update the trade-off doc.

## Next Steps

If approved:

1. Create `docs/plans/design-terraphim-grep-answer-json-instructions.md`.
2. Conduct specification interview (optional for this small change).
3. Implement the fix in `terraphim/terraphim-clients`.
4. Run unit tests and live verification.
5. Update `terraphim-ai` trade-off doc and benchmark results.

## Appendix

### Reference Materials

- `terraphim/terraphim-clients#77` — bug report with reproduction.
- `terraphim/terraphim-ai#3098` — documentation tracking issue.
- `terraphim/terraphim-ai#3099` — PR with benchmark data.
- Source inspected: `terraphim_grep` v1.20.3 from crates.io registry and
  `terraphim/terraphim-clients` main branch.

### Code Snippets

Current prompt construction (`crates/terraphim_grep/src/lib.rs`):

```rust
let messages = vec![serde_json::json!({
    "role": "user",
    "content": format!(
        "{}\n\n{}\n\nProvide a brief answer based on the context above.",
        prompt,
        if options.include_answer {
            "Synthesise an answer."
        } else {
            "List the relevant findings."
        }
    )
})];
```

Existing instructions (`crates/terraphim_grep/src/signatures.rs`):

```rust
fn instructions(&self) -> String {
    r#"Return a JSON object with:
- "answer": the synthesised answer
- "citations": array of {source, line (optional), excerpt}
- "confidence": a number between 0 and 1"#.to_string()
}
```
