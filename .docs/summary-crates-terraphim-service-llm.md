# crates/terraphim_service (LLM Integration)

## Purpose
Main service layer including LLM client abstraction with multiple provider support.

## Status: Production-Ready
- ~35,123 LOC total (service covers much more than LLM)

## Key Types

### LlmClient (trait)
```rust
#[async_trait]
pub trait LlmClient: Send + Sync {
    fn name(&self) -> &'static str;
    async fn summarize(&self, content: &str, opts: SummarizeOptions) -> ServiceResult<String>;
    async fn list_models(&self) -> ServiceResult<Vec<String>>;
    async fn chat_completion(&self, messages: Vec<Value>, opts: ChatOptions) -> ServiceResult<String>;
}
```

### Providers Implemented
1. **Ollama** - Local model support via `build_ollama_from_role()`
2. **OpenRouter** - Cloud models via `build_openrouter_from_role()`
3. **LLM Router** (feature-gated) - Intelligent routing between providers
   - `RoutedLlmClient` - Library mode (in-process)
   - `ProxyLlmClient` - Service mode (external HTTP proxy)

### Role-based LLM Configuration
- `role_wants_ai_summarize()` - Check if role needs AI
- `build_llm_from_role()` - Auto-detect provider from role.extra
- Fallback chain: llm_provider -> OpenRouter -> Ollama hints
- Environment variable support for proxy URLs

## Relevance to TinyClaw Rebuild
Maps to PicoClaw's HTTPProvider but with more sophisticated routing. The LlmClient trait is the foundation for the agent loop's LLM calls. Key difference: PicoClaw uses OpenAI-compatible format directly, while Terraphim abstracts via the trait. Both approaches work; the trait allows provider-specific optimizations.

For the tool-calling loop, the chat_completion() method needs to handle tool call responses and iterate -- this is not currently built into the trait (it returns a single string, not structured ToolCall objects). Adaptation needed.
