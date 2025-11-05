# rust-genai Integration in Terraphim AI

Documentation for the rust-genai library integration and future unified LLM architecture.

## Overview

Terraphim AI uses a **fork of rust-genai** for unified multi-provider LLM support:

**Fork:** https://github.com/terraphim/rust-genai (branch: main)

**Currently Used In:**
- `terraphim_multi_agent` crate - Full integration
- `terraphim_service` - Partial (OpenRouter/Ollama custom clients)
- `terraphim_tui` - Uses terraphim_service LLM

## Current Architecture

### Two LLM Implementations

**1. terraphim_service::llm (Current - Used by TUI)**
- Custom `OpenRouterClient`
- Custom `OllamaClient`
- Direct API integration
- Works with current RAG workflow

**2. terraphim_multi_agent::GenAiLlmClient (Advanced)**
- Uses rust-genai library
- Unified interface
- Supports: Ollama, OpenAI, Anthropic, OpenRouter
- Auto-detection from environment variables
- Better error handling

## rust-genai Features

### Multi-Provider Support

**Single Interface:**
```rust
use genai::Client;
use genai::chat::{ChatMessage, ChatRequest, ChatOptions};

let client = Client::default(); // Auto-detects provider from env

let messages = vec![
    ChatMessage::system("You are an expert"),
    ChatMessage::user("Explain X"),
];

let request = ChatRequest::new(messages);
let options = ChatOptions::default()
    .with_max_tokens(2048)
    .with_temperature(0.7);

let response = client.exec_chat("openai/gpt-4", request, Some(&options)).await?;
```

**Providers Supported:**
- **Ollama**: Local models (llama, gemma, etc.)
- **OpenAI**: GPT-3.5, GPT-4, GPT-4o
- **Anthropic**: Claude 3 Opus/Sonnet/Haiku
- **OpenRouter**: Gateway to all models

### Environment Variable Auto-Detection

```bash
# OpenRouter
export OPENROUTER_API_KEY="sk-or-v1-..."

# OpenAI
export OPENAI_API_KEY="sk-..."

# Anthropic
export ANTHROPIC_API_KEY="sk-ant-..."
export ANTHROPIC_API_BASE="https://z.ai/api"  # z.ai proxy support
export ANTHROPIC_AUTH_TOKEN="..."

# Ollama
export OLLAMA_BASE_URL="http://localhost:11434"
```

Client automatically uses correct credentials based on model prefix!

### Unified Message Format

```rust
ChatMessage::system("System prompt")
ChatMessage::user("User message")
ChatMessage::assistant("Assistant response")
```

No need for provider-specific formatting!

### Token Usage Tracking

```rust
let response = client.exec_chat(...).await?;

println!("Input tokens: {}", response.usage.prompt_tokens.unwrap_or(0));
println!("Output tokens: {}", response.usage.completion_tokens.unwrap_or(0));
println!("Total cost: ${}", calculate_cost(&response.usage));
```

### Caching (Built-in)

rust-genai supports response caching:

```rust
let options = ChatOptions::default()
    .with_cache_ttl(3600);  // Cache for 1 hour

// Identical requests within TTL return cached responses
```

**Cache Features:**
- Prompt-based hashing
- TTL (time-to-live) support
- Memory-efficient
- Provider-agnostic

## GenAiLlmClient Implementation

**File:** `crates/terraphim_multi_agent/src/genai_llm_client.rs`

**Key Features:**

```rust
pub struct GenAiLlmClient {
    client: Client,      // rust-genai client
    provider: String,    // "ollama", "openai", "anthropic", "openrouter"
    model: String,       // Model name
}

// Factory methods
GenAiLlmClient::new_ollama(Some("llama3.2:3b"))
GenAiLlmClient::new_openai(Some("gpt-4"))
GenAiLlmClient::new_anthropic(Some("claude-3-sonnet"))
GenAiLlmClient::new_openrouter(Some("openai/gpt-4"))

// From config
GenAiLlmClient::from_config("ollama", Some("gemma3:270m"))

// With custom URL (z.ai proxy)
GenAiLlmClient::from_config_with_url(
    "anthropic",
    Some("claude-3-sonnet"),
    Some("https://z.ai/api")
)

// Auto-proxy detection
GenAiLlmClient::from_config_with_auto_proxy("anthropic", None)
```

**z.ai Proxy Support:**
- Auto-detects `ANTHROPIC_BASE_URL`
- Uses `ANTHROPIC_AUTH_TOKEN` if available
- Seamless proxy integration

## Current TUI Integration

### What's Implemented (This Session)

**TuiService.chat() (service.rs:180-223):**
```rust
// Uses terraphim_service::llm::build_llm_from_role()
if let Some(llm_client) = terraphim_service::llm::build_llm_from_role(&role) {
    let response = llm_client.chat_completion(messages, opts).await?;
    Ok(response)
}
```

**Terraphim Engineer Role (config/lib.rs:418-447):**
```rust
// Auto-detects OPENROUTER_API_KEY or falls back to Ollama
if let Ok(api_key) = std::env::var("OPENROUTER_API_KEY") {
    terraphim_role.llm_api_key = Some(api_key);
    terraphim_role.llm_model = Some("openai/gpt-4o-mini");
    terraphim_role.extra.insert("llm_provider", "openrouter");
}
```

**Works NOW with:**
```bash
export OPENROUTER_API_KEY="sk-or-v1-..."
cargo run --features "repl-full,openrouter" -- repl
```

## Future: Unified Architecture

### Migration Path to rust-genai

**Phase 1:** Add GenAI support alongside current implementation
```rust
// terraphim_service/src/llm.rs
#[cfg(feature = "genai")]
pub fn build_genai_llm_from_role(role: &Role) -> Option<GenAiClient> {
    let provider = get_string_extra(&role.extra, "llm_provider")?;
    let model = role.llm_model.clone()?;
    GenAiClient::new(provider, model).ok()
}
```

**Phase 2:** Update TuiService to prefer GenAI
```rust
// Try GenAI first, fall back to current
if let Some(genai_client) = build_genai_llm_from_role(&role) {
    return genai_client.chat(messages).await;
} else if let Some(llm_client) = build_llm_from_role(&role) {
    return llm_client.chat_completion(messages, opts).await;
}
```

**Phase 3:** Deprecate custom clients
- Remove OpenRouterClient
- Remove OllamaClient
- Use only GenAiClient

## Caching Strategy

### Current Caching (Implemented)

**Conversation Cache:**
- Location: `ContextManager.conversations_cache`
- Type: `AHashMap<ConversationId, Arc<Conversation>>`
- Size: Max 100 conversations
- Eviction: LRU by updated_at
- Purpose: Fast conversation access

**Conversation Index Cache:**
- Location: `OpenDALConversationPersistence.index_cache`
- Type: `RwLock<Option<ConversationIndex>>`
- Purpose: Fast conversation listing without disk I/O

**Thesaurus Cache:**
- Location: Persistence layer
- Pre-built on startup
- Cached in memory and disk
- Purpose: Fast autocomplete

**NOT Cached:**
- LLM API responses (intentional - always fresh)
- Search results (regenerated each time)

### With rust-genai Caching

**Add LLM Response Cache:**
```rust
let options = ChatOptions::default()
    .with_cache_ttl(3600)  // 1 hour TTL
    .with_cache_key_prefix("terraphim_");

// Identical prompts return cached responses
```

**Benefits:**
- Faster response times
- Reduced API costs
- Offline capability (cached responses)
- Configurable per-role

**Tradeoffs:**
- Stale responses if source docs updated
- Cache invalidation complexity
- Memory usage

## Recommended Configuration

### For Development (Ollama)

```toml
# Role configuration
llm_provider = "ollama"
llm_model = "llama3.2:3b"
ollama_base_url = "http://localhost:11434"
```

```bash
ollama pull llama3.2:3b
ollama serve
```

### For Production (OpenRouter)

```toml
llm_provider = "openrouter"
llm_model = "openai/gpt-4o-mini"
llm_api_key = "${OPENROUTER_API_KEY}"
```

```bash
export OPENROUTER_API_KEY="sk-or-v1-..."
```

### For Enterprise (Anthropic via z.ai)

```toml
llm_provider = "anthropic"
llm_model = "claude-3-sonnet-20240229"
```

```bash
export ANTHROPIC_BASE_URL="https://z.ai/api"
export ANTHROPIC_AUTH_TOKEN="..."
```

## Testing LLM Integration

### 1. Verify Provider Detection

```bash
LOG_LEVEL=debug cargo run --features "repl-full,openrouter" -- repl

# Look for:
[DEBUG] Building LLM client for role: Terraphim Engineer
[DEBUG] Found llm_provider: openrouter
[INFO] Using LLM provider: openrouter
```

### 2. Test Chat Without Context

```bash
Terraphim Engineer> /chat Hello, test message
💬 Sending message: 'Hello, test message'
🤖 Response:
[LLM response]
```

### 3. Test RAG Workflow

```bash
/search knowledge graph
/context add 1,2,3
/chat Explain the architecture

# Should see:
🤖 Response (with context):
[AI response using Architecture, knowledge-graph, knowledge-graph-system docs]
```

### 4. Verify Token Usage

```bash
# rust-genai logs token usage:
[DEBUG] Tokens: 1234 input, 567 output
```

## See Also

- [RAG Search to Memory Guide](./rag-search-to-memory-guide.md) - User guide
- [LLM Configuration](./llm-configuration-rag.md) - Setup guide
- [End-to-End Demo](./end-to-end-rag-demo.md) - Validation script

## Summary

- ✅ rust-genai fork exists and is integrated
- ✅ GenAiLlmClient working in multi_agent crate
- ✅ Current TUI uses terraphim_service::llm (also working)
- ⏳ Future: Migrate to GenAiClient for unified architecture
- ✅ **RAG workflow works NOW with current implementation**

No blocker for demonstration - everything is ready!
