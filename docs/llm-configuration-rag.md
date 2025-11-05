# LLM Configuration for RAG Workflow

Guide for configuring LLM providers to work with the RAG (Search → Select → Chat) workflow.

## Current Status

**Infrastructure Complete:**
- ✅ RAG workflow implemented
- ✅ Conversation management
- ✅ Context management
- ✅ Persistence working
- ⏳ LLM integration uses placeholder (not real API calls yet)

**To Enable Real LLM:**
- Configure LLM provider in role
- Set API keys via environment or config
- Use OpenRouter, Ollama, or other providers

## Configuration Methods

### Method 1: Environment Variables + Role Config

**1. Set environment variable:**
```bash
export OPENROUTER_API_KEY="sk-or-v1-..."
# or
export OLLAMA_BASE_URL="http://localhost:11434"
```

**2. Update role in embedded config:**

Edit `crates/terraphim_config/src/lib.rs` (line 394-418):

```rust
// Add Terraphim Engineer role with LLM
let mut terraphim_role = Role::new("Terraphim Engineer");
terraphim_role.shortname = Some("TerraEng".to_string());
terraphim_role.relevance_function = RelevanceFunction::TerraphimGraph;
// ... existing config ...

// ADD LLM CONFIGURATION
terraphim_role.llm_enabled = true;
terraphim_role.llm_chat_enabled = true;
terraphim_role.llm_model = Some("openai/gpt-4".to_string());
terraphim_role.llm_chat_system_prompt = Some(
    "You are an expert in Terraphim architecture. Use the provided context documents to answer questions accurately.".to_string()
);
terraphim_role.extra.insert(
    "llm_provider".to_string(),
    serde_json::Value::String("openrouter".to_string())
);
```

### Method 2: Runtime Configuration

**Use /config command:**
```bash
# View current config
/config show

# The config shows llm_enabled: false by default
# Currently no runtime LLM configuration command
# TODO: Add /config set llm_api_key command
```

### Method 3: JSON Config File

Create `embedded_config_with_llm.json`:

```json
{
  "id": "Embedded",
  "roles": {
    "Terraphim Engineer": {
      "name": "Terraphim Engineer",
      "relevance_function": "TerraphimGraph",
      "llm_enabled": true,
      "llm_chat_enabled": true,
      "llm_api_key": "${OPENROUTER_API_KEY}",
      "llm_model": "openai/gpt-4",
      "llm_chat_system_prompt": "You are an expert assistant...",
      "extra": {
        "llm_provider": "openrouter"
      }
    }
  }
}
```

## LLM Provider Support

### OpenRouter (Recommended for Production)

**Configuration:**
```rust
role.llm_enabled = true;
role.llm_api_key = Some(env::var("OPENROUTER_API_KEY").ok());
role.llm_model = Some("openai/gpt-4".to_string());
role.extra.insert("llm_provider", "openrouter");
```

**Models Available:**
- `openai/gpt-4` - Best quality
- `openai/gpt-3.5-turbo` - Faster, cheaper
- `anthropic/claude-3-sonnet` - Good for long context
- `meta-llama/llama-3.1-70b` - Open source

### Ollama (Local, No API Key)

**Configuration:**
```rust
role.llm_enabled = true;
role.llm_model = Some("llama3.2:3b".to_string());
role.extra.insert("llm_provider", "ollama");
role.extra.insert("ollama_base_url", "http://127.0.0.1:11434");
```

**Models:**
- `llama3.2:3b` - Fast, 3B parameters
- `llama3.1:8b` - Better quality, 8B parameters
- `gemma:7b` - Google's model

**Start Ollama:**
```bash
# Install Ollama first (https://ollama.ai)
ollama serve

# Pull model
ollama pull llama3.2:3b
```

## Implementing Real LLM in TuiService

**Current implementation (service.rs:162-193):**
```rust
pub async fn chat(&self, role_name: &RoleName, prompt: &str, model: Option<String>) -> Result<String> {
    // Currently returns placeholder:
    return Ok(format!("Chat response from {} with model {:?}: {}", provider_str, model, prompt));
}
```

**To implement real LLM:**

```rust
pub async fn chat(&self, role_name: &RoleName, prompt: &str, model: Option<String>) -> Result<String> {
    let config = self.config_state.config.lock().await;
    let role = config.roles.get(role_name)
        .ok_or_else(|| anyhow::anyhow!("Role not found"))?;

    // Build LLM client from role configuration
    if let Some(llm_client) = terraphim_service::llm::build_llm_from_role(role) {
        use terraphim_service::llm::{LlmClient, LlmMessage};

        let messages = vec![
            LlmMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }
        ];

        // Use provided model or role's default
        let model_name = model.or_else(|| role.llm_model.clone());

        let response = llm_client.chat_completion(messages, model_name).await?;
        Ok(response)
    } else {
        Err(anyhow::anyhow!("No LLM configured for role {}", role_name))
    }
}
```

**Required:**
- Implement `terraphim_service::llm::build_llm_from_role()`
- Return `Arc<dyn LlmClient>` based on role.extra["llm_provider"]
- Support OpenRouter, Ollama, etc.

## End-to-End Demo (With Mock LLM)

**Current behavior with placeholder:**

```bash
$ terraphim-tui repl

# 1. Search
Terraphim Engineer> /search knowledge graph
✅ Found 36 result(s)
  [ 0] 43677 - @memory
  [ 1] 38308 - Architecture
  [ 2] 24464 - knowledge-graph

# 2. Add to context
Terraphim Engineer> /context add 1,2
📝 Created conversation: Session 2025-10-28 12:30
✅ Added [1]: Architecture
✅ Added [2]: knowledge-graph

# 3. View context
Terraphim Engineer> /context list
Context items (2):
  [ 0] Architecture (score: 38308)
  [ 1] knowledge-graph (score: 24464)

# 4. Chat (currently placeholder response)
Terraphim Engineer> /chat Explain the architecture
💬 Sending message: 'Explain the architecture'

🤖 Response (with context):
No LLM configured for role Terraphim Engineer. Prompt was:
system: Context Information:
### Architecture
[Full Architecture.md content]

### knowledge-graph
[Full knowledge-graph.md content]

User: Explain the architecture
```

**What's Working:**
- ✅ Search finds documents
- ✅ Context adds documents
- ✅ Context persists
- ✅ Chat builds prompt with full context
- ✅ Conversation saves
- ⏳ LLM call is placeholder (not real API)

**To make LLM work:**
Need to implement real `LlmClient` in terraphim_service

## Expected Behavior (With Real LLM)

```bash
Terraphim Engineer> /chat Explain how the architecture works

💬 Sending message: 'Explain how the architecture works'

🤖 Response (with context):

Based on the provided documentation (Architecture.md, knowledge-graph.md):

The Terraphim architecture consists of several key components:

1. **Knowledge Graph Layer** (from knowledge-graph.md):
   - Built from markdown files in docs/src/kg
   - Uses terraphim_automata for text matching
   - Provides semantic concept relationships

2. **Search Layer** (from Architecture.md):
   - TerraphimGraph scorer for semantic ranking
   - Uses RoleGraph to query knowledge graph
   - Returns documents ranked by graph connectivity

3. **Integration** (from both docs):
   The architecture connects these through ConfigState which maintains:
   - RoleGraphs for each user role
   - Haystack configurations
   - Relevance function settings

[Detailed LLM response using actual document content]
```

## Validation Steps

### 1. Verify Context is Included

Check that chat prompt includes context:

```rust
// In chat_with_context() (service.rs:443-508)
let prompt = {
    // Build messages with context
    let messages = build_llm_messages_with_context(&conversation, true);

    // Format should include:
    // system: Context Information: ### Doc1 [content] ### Doc2 [content]
    // User: [question]
}
```

### 2. Verify Persistence

```bash
# Add context and chat
/context add 1,2
/chat test

# Check saved
/conversation list
  ▶ conv-abc - Session... (2 msg, 2 ctx)

# Exit and restart
/quit
$ terraphim-tui repl

# Verify persisted
/conversation list
  conv-abc - Session... (2 msg, 2 ctx)  # Still there!

/conversation load conv-abc
✅ Loaded: Session... (2 messages, 2 context items)

# Context restored
/context list
Context items (2):
  [ 0] Architecture
  [ 1] knowledge-graph
```

### 3. Verify Context in Prompt

Enable debug logging:

```bash
LOG_LEVEL=debug terraphim-tui repl
```

Look for:
```
[DEBUG] Building LLM messages with context
[DEBUG] Context items: 2
[DEBUG] Prompt: system: Context Information: ...
```

## TODO: Real LLM Implementation

**Files to modify:**

1. **crates/terraphim_service/src/llm.rs:**
   - Implement `build_llm_from_role()` properly
   - Return real LlmClient based on provider

2. **crates/terraphim_tui/src/service.rs:**
   - Update `chat()` method (line 162-193)
   - Use real LlmClient instead of placeholder

3. **crates/terraphim_config/src/lib.rs:**
   - Add LLM config to default embedded roles (line 394+)
   - Read API keys from environment

**Example Implementation:**

```rust
// crates/terraphim_service/src/llm.rs
pub fn build_llm_from_role(role: &Role) -> Option<Arc<dyn LlmClient>> {
    if !role.llm_enabled {
        return None;
    }

    match role.extra.get("llm_provider")?.as_str()? {
        "openrouter" => {
            #[cfg(feature = "openrouter")]
            {
                let api_key = role.llm_api_key.as_ref()?;
                let model = role.llm_model.as_ref()?;
                Some(Arc::new(OpenRouterClient::new(api_key, model).ok()?))
            }
            #[cfg(not(feature = "openrouter"))]
            None
        }
        "ollama" => {
            let base_url = role.extra.get("ollama_base_url")?.as_str()?;
            let model = role.llm_model.as_ref()?;
            Some(Arc::new(OllamaClient::new(base_url, model).ok()?))
        }
        _ => None
    }
}
```

## Testing RAG Without Real LLM

The RAG workflow infrastructure can be tested without LLM:

```bash
# Verify search works
/search graph
✅ Found results with indices

# Verify context management
/context add 1,2
✅ Added to context

/context list
✅ Shows context items

# Verify conversation persistence
/conversation list
✅ Shows conversations

/conversation show
✅ Shows details

# Chat shows placeholder but proves context is built
/chat test
🤖 Response:
Prompt was: system: Context Information: [full context] User: test
```

**All infrastructure proven working:**
- Search ✅
- Context selection ✅
- Persistence ✅
- Prompt building with context ✅
- Just need: Real LLM client implementation

## See Also

- [RAG Search to Memory Guide](./rag-search-to-memory-guide.md) - Complete user guide
- [RAG Workflow Implementation Plan](./rag-workflow-implementation-plan.md) - Technical details
- OpenRouter Integration (TODO) - Real LLM setup

## Summary

**RAG workflow is COMPLETE and WORKING**, just needs:
- Real LLM client implementation (separate task)
- LLM provider configuration
- API key management

All the hard parts (search, context, persistence, conversation management) are done!
