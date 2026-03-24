---
title: "CRITICAL: rust-genai v0.4.4 → v0.6.0 upgrade with breaking changes"
labels: ["priority/P0", "type/breaking-change", "component/llm", "vendor/rust-genai"]
assignees: []
milestone: ""
---

## Summary

**Echo reports critical drift** in the rust-genai LLM abstraction layer. Current fork is 2 minor versions behind upstream with multiple breaking API changes.

## Current State

- **Version:** v0.4.4-WIP (terraphim fork, branch `merge-upstream-20251103`)
- **Commit:** 0f8839ad
- **Location:** Root `Cargo.toml` [patch.crates-io]
- **Upstream:** v0.6.0-beta (in development), v0.5.3 (latest stable)

## Breaking Changes

### 1. Dependency Conflict (BLOCKING)
- **Change:** `reqwest` upgraded 0.12 → 0.13 in v0.5.0
- **Impact:** Workspace uses reqwest 0.12 - version mismatch causes compilation failure
- **Severity:** CRITICAL

### 2. ChatResponse.content Type Change
- **Change:** `Vec<MessageContent>` → `MessageContent` (v0.5.0)
- **Impact:** All code accessing `.content` field
- **Migration:** Update from `response.content[0]` to `response.content`
- **Affected files:** 
  - `terraphim_service/src/*.rs`
  - `terraphim_multi_agent/src/*.rs`

### 3. StreamEnd.content Type Change
- **Change:** Now `Option<MessageContent>` (v0.5.0)
- **Impact:** Streaming response handlers
- **Migration:** Add Option handling for streaming end content

### 4. ChatRequest Iterator Changes
- **Change:** `append/with_...(vec)` functions now take iterators (v0.5.0)
- **Impact:** Request builder patterns
- **Migration:** Pass iterators instead of Vec directly

### 5. ContentPart Restructuring
- **Change:** `ContentPart::Binary(Binary)` required (v0.5.0)
- **Impact:** Multimodal content handling
- **Migration:** Update binary content construction

### 6. Namespace Strategy
- **Change:** ZAI namespace changes - default models use `zai::` prefix (v0.5.0)
- **Impact:** Model name resolution in config
- **Migration:** Update model names in configs

### 7. Groq Namespace Requirement
- **Change:** Groq requires `groq::_model_name` format (v0.6.0-beta)
- **Impact:** Groq provider configuration
- **Migration:** Update Groq model references

### 8. AuthResolver for Model Listing
- **Change:** `all_model_names()` now requires `AuthResolver` (v0.6.0-beta)
- **Impact:** Model listing functionality
- **Migration:** Pass AuthResolver when listing models

## Affected Crates

- [ ] `terraphim_multi_agent` - Direct genai dependency
- [ ] `terraphim_service` - LLM service layer
- [ ] `terraphim_config` - Model configuration
- [ ] `terraphim_tinyclaw` - Telegram bot LLM integration

## Reproduction

```bash
# Check current version
cargo tree -p genai | head -5

# Attempt to update fork
cargo update -p genai
# Fails due to reqwest version conflict
```

## Proposed Migration Plan

1. **Phase 1: Dependency Update**
   - [ ] Create `feat/genai-v0.6-migration` branch
   - [ ] Update workspace reqwest from 0.12 to 0.13
   - [ ] Verify all crates compile with reqwest 0.13

2. **Phase 2: Fork Update**
   - [ ] Rebase terraphim/rust-genai fork to v0.5.3
   - [ ] Test fork compatibility
   - [ ] Update Cargo.toml patch to new commit

3. **Phase 3: API Migration**
   - [ ] Update `ChatResponse.content` access patterns
   - [ ] Update streaming handlers for `StreamEnd`
   - [ ] Update request builders
   - [ ] Update binary content handling

4. **Phase 4: Configuration Updates**
   - [ ] Add namespace handling for ZAI models
   - [ ] Update Groq model references
   - [ ] Update model listing code

5. **Phase 5: Testing**
   - [ ] Run integration tests
   - [ ] Test LLM providers (OpenAI, Anthropic, Groq)
   - [ ] Test streaming responses
   - [ ] Test multimodal content

## References

- [Upstream CHANGELOG](https://github.com/jeremychone/rust-genai/blob/main/CHANGELOG.md)
- [Migration Guide v0.3→v0.4](https://github.com/jeremychone/rust-genai/blob/main/doc/migration/migration-v_0_3_to_0_4.md)
- [terraphim/rust-genai fork](https://github.com/terraphim/rust-genai)

## Blocked By

- #ISSUE-2 (if reqwest upgrade is separate)

## Blocks

- MCP SDK upgrade (coordinated reqwest version needed)

## Verification

```rust
// Before (v0.4.x):
let content = &response.content[0];

// After (v0.5.x):
let content = &response.content;
```

---

**Echo's Assessment:** This drift affects the core LLM abstraction. Zero-deviation principle violated. Immediate synchronization required.
