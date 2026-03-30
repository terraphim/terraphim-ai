# Verification and Validation Report: Issue #538

**Issue**: feat: Add GGUF/llama-cpp backend to terraphim LLM proxy layer
**Repository**: terraphim/terraphim-ai
**Date**: 2026-03-11
**Status**: NOT IMPLEMENTED

---

## Executive Summary

Issue #538 proposes adding local GGUF inference capability via `llama-cpp-rs` for CPU-only LLM inference. The current codebase supports remote (OpenRouter) and local (Ollama) providers, but has no direct GGUF/llama.cpp integration. This feature request remains unimplemented.

| Requirement | Status | Evidence |
|-------------|--------|----------|
| terraphim_llm_local crate exists | NOT MET | No crate found |
| llama-cpp-rs dependency | NOT MET | Not in any Cargo.toml |
| hf-hub dependency | NOT MET | Not in any Cargo.toml |
| CPU-only inference support | NOT MET | No implementation |
| Same trait interface as remote | MET | LlmClient trait exists |

---

## Detailed Verification

### 1. terraphim_llm_local Crate Exists

**Status**: NOT MET

**Evidence**:
```bash
$ ls -la crates/ | grep -E "(llm|local|gguf)"
No matching crates found
```

All existing crates in the workspace:
- terraphim_agent_evolution
- terraphim_agent_messaging
- terraphim_agent_registry
- terraphim_agent_supervisor
- terraphim_atomic_client
- terraphim_automata
- terraphim_config
- terraphim_goal_alignment
- terraphim_kg_agents
- terraphim_kg_orchestration
- terraphim_markdown_parser
- terraphim_mcp_server
- terraphim_middleware
- terraphim_multi_agent
- terraphim_onepassword_cli
- terraphim_persistence
- terraphim_rolegraph
- terraphim_router
- terraphim_server
- terraphim_service
- terraphim_settings
- terraphim_spawner
- terraphim_task_decomposition
- terraphim_tinyclaw
- terraphim_types
- terraphim_tui
- haystack_atlassian
- haystack_core
- haystack_discourse
- haystack_jmap

**No `terraphim_llm_local` crate exists.**

---

### 2. llama-cpp-rs Dependency Present

**Status**: NOT MET

**Evidence**:
```bash
$ grep -r "llama-cpp" crates/*/Cargo.toml
No matches found
```

**Alternative local inference found**: The codebase uses Ollama for local inference via HTTP API (feature = "ollama").

---

### 3. hf-hub Dependency Present

**Status**: NOT MET

**Evidence**:
```bash
$ grep -r "hf-hub" crates/*/Cargo.toml
No matches found
```

No automatic GGUF model download capability exists.

---

### 4. CPU-only Inference Support

**Status**: NOT MET (for GGUF)

**Evidence**:
- No llama.cpp integration found
- No GGUF model loading code found
- No quantization variant selection code found

**Existing local inference**: Ollama integration exists at `crates/terraphim_service/src/llm.rs` (lines 299-564) which provides local inference through Ollama's HTTP API.

---

### 5. Same Trait Interface as Remote

**Status**: MET

**Evidence**:
File: `crates/terraphim_service/src/llm.rs` (lines 31-56)

```rust
#[async_trait::async_trait]
pub trait LlmClient: Send + Sync {
    fn name(&self) -> &'static str;

    async fn summarize(&self, content: &str, opts: SummarizeOptions) -> ServiceResult<String>;

    async fn list_models(&self) -> ServiceResult<Vec<String>> {
        // Default implementation
    }

    async fn chat_completion(
        &self,
        _messages: Vec<serde_json::Value>,
        _opts: ChatOptions,
    ) -> ServiceResult<String> {
        // Default implementation
    }
}
```

This trait is already implemented by:
- `OpenRouterClient` (lines 243-292) - remote API
- `OllamaClient` (lines 301-564) - local via HTTP
- `RouterBridgeLlmClient` (feature = "llm_router") - routing layer

A GGUF implementation would use the same trait.

---

## Current LLM Architecture

```
                    +------------------+
                    |   build_llm_     |
                    |   from_role()    |
                    +--------+---------+
                             |
           +-----------------+-----------------+
           |                                   |
    +------v------+                     +------v-------+
    |  OpenRouter |                     |    Ollama    |
    |   (remote)  |                     |   (local)    |
    +-------------+                     +--------------+
```

**Proposed addition**:
```
                    +------------------+
                    |   build_llm_     |
                    |   from_role()    |
                    +--------+---------+
                             |
           +-----------------+-----------------+-----------------+
           |                 |                 |
    +------v------+   +------v-------+   +-----v----------+
    |  OpenRouter |   |    Ollama    |   |  GGUF/Local   |
    |   (remote)  |   |   (local)    |   |  (llama.cpp)  |
    +-------------+   +--------------+   +---------------+
```

---

## Traceability Matrix

| Requirement | Design Element | Code Location | Test Coverage | Status |
|-------------|----------------|---------------|---------------|--------|
| Common LLM trait | LlmClient trait | llm.rs:31-56 | Unit tests | MET |
| Local inference | OllamaClient | llm.rs:301-564 | Integration tests | PARTIAL |
| GGUF support | MISSING | - | NONE | NOT MET |
| Model download | MISSING | - | NONE | NOT MET |
| Quantization selection | MISSING | - | NONE | NOT MET |

---

## Defect Register

| ID | Description | Severity | Resolution | Status |
|----|-------------|----------|------------|--------|
| D001 | No terraphim_llm_local crate | High | Create crate with llama-cpp-rs | OPEN |
| D002 | No hf-hub integration | Medium | Add hf-hub dependency for downloads | OPEN |
| D003 | No GGUF quantization selection | Medium | Implement quantization variant selector | OPEN |

---

## Existing Local Inference Alternative

The codebase currently supports local inference via **Ollama**:

**Configuration**:
```json
{
  "extra": {
    "llm_provider": "ollama",
    "ollama_model": "llama3.1",
    "ollama_base_url": "http://127.0.0.1:11434"
  }
}
```

**Limitations vs GGUF proposal**:
- Requires Ollama service running
- No direct GGUF file loading
- No quantization selection at runtime
- Additional dependency (Ollama binary)

---

## Recommendations

### Option 1: Implement GGUF Support (Full Issue Resolution)

**Effort**: 2-3 days

**Steps**:
1. Create `crates/terraphim_llm_local/` crate
2. Add dependencies: `llama-cpp-rs`, `hf-hub`
3. Implement `LlmClient` trait for GGUF models
4. Add model download/caching via hf-hub
5. Implement quantization variant selection
6. Add configuration options to Role.extra
7. Write integration tests

**Benefits**:
- No Ollama dependency required
- Direct GGUF file support
- Full control over quantization
- Single binary deployment

### Option 2: Document Ollama as Alternative (Partial Resolution)

If GGUF implementation is not prioritized, document that Ollama provides local inference capability with GGUF support (Ollama can import and serve GGUF models).

### Option 3: Close as Not Planned

If Ollama integration is sufficient for the use case, close this issue with explanation.

---

## Conclusion

Issue #538 represents a valid feature request that has **not been implemented**. The codebase has the architectural foundation (`LlmClient` trait) to support this feature, but no GGUF/llama.cpp integration exists.

### GO/NO-GO Decision: NO-GO

**Reasoning**:
- Feature request is not implemented
- No code exists to validate
- Requires implementation Phase 3 work

**Next Steps**:
1. If implementing: Create design document for `terraphim_llm_local` crate
2. If not implementing: Close issue with explanation about Ollama alternative
3. If deferring: Add to backlog with priority label

---

## Appendix: Files Referenced

| File | Path | Purpose |
|------|------|---------|
| LLM trait and providers | `crates/terraphim_service/src/llm.rs` | LlmClient trait, OpenRouter, Ollama impls |
| LLM adapter for evolution | `crates/terraphim_agent_evolution/src/llm_adapter.rs` | Simplified adapter trait |
| Role configuration | `crates/terraphim_config/src/lib.rs` | Role.extra configuration |
