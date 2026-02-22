# Intelligent Routing Demo

This example demonstrates the Terraphim LLM Proxy's intelligent routing capabilities using the RoleGraph pattern matching system.

## Features Demonstrated

### 1. Three-Phase Routing Strategy
- **Phase 1**: Rule-based routing for common scenarios (images, tools, long context)
- **Phase 2**: RoleGraph pattern matching for semantic routing
- **Phase 3**: Fallback to default provider

### 2. Supported Routing Scenarios

#### Image Generation
```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-api-key" \
  -d '{
    "model": "auto",
    "messages": [
      {"role": "user", "content": "Generate an image of a sunset over mountains"}
    ]
  }'
```
**Expected Route**: OpenAI DALL-E 3

#### Web Search
```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-api-key" \
  -d '{
    "model": "auto",
    "messages": [
      {"role": "user", "content": "Search for the latest news about AI"}
    ],
    "tools": [
      {
        "type": "function",
        "function": {
          "name": "web_search",
          "description": "Search the web for current information"
        }
      }
    ]
  }'
```
**Expected Route**: Perplexity Online (sonar-reasoning)

#### Long Context
```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-api-key" \
  -d '{
    "model": "auto",
    "messages": [
      {"role": "user", "content": "'$(printf 'Analyze this very long document. %s' "$(head -c 50000 /dev/urandom | base64)")'"}
    ]
  }'
```
**Expected Route**: Anthropic Claude 3.5 Sonnet (200k context)

#### Reasoning/Thinking
```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-api-key" \
  -d '{
    "model": "auto",
    "messages": [
      {"role": "user", "content": "Think step by step to solve this complex math problem: ..."}
    ]
  }'
```
**Expected Route**: DeepSeek Reasoner (via pattern matching)

#### Background Tasks
```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-api-key" \
  -d '{
    "model": "auto",
    "messages": [
      {"role": "user", "content": "Write a simple haiku about programming"}
    ]
  }'
```
**Expected Route**: Groq (fast, cheap)

### 3. Pattern Matching with RoleGraph

The proxy uses Terraphim's RoleGraph system to semantically match requests against patterns in the taxonomy:

- **think_routing**: Matches requests for reasoning, step-by-step thinking
- **image_routing**: Matches image generation requests
- **search_routing**: Matches web search and information retrieval
- **code_routing**: Matches code generation and programming tasks
- **creative_routing**: Matches creative writing and content creation

### 4. Provider Configuration

Each provider is configured with:
- API endpoint and authentication
- Available models and their capabilities
- Cost and performance characteristics
- Special features (context length, tool use, etc.)

### 5. Response Transformations

The proxy automatically transforms responses between provider formats:
- OpenAI: Standard format, no transformation needed
- Anthropic: System prompt moved to messages
- DeepSeek: Content blocks flattened
- Ollama: Various format adaptations

## Usage

1. Start the proxy:
```bash
cargo run -- --config config.example.toml
```

2. Send requests with `"model": "auto"` to enable intelligent routing

3. Check the logs to see routing decisions:
```bash
RUST_LOG=debug cargo run -- --config config.example.toml
```

## Configuration

Edit `config.example.toml` to:
- Add your API keys
- Configure provider endpoints
- Adjust routing thresholds
- Enable/disable transformers

## Streaming Support

All providers support streaming responses. The proxy handles format differences automatically:
- OpenRouter: Direct HTTP streaming with SSE format conversion
- Other providers: Genai library with format normalization

## Monitoring

The proxy provides detailed logging for:
- Routing decisions and phase used
- Pattern matching results
- Provider response times
- Token usage and costs
- Error handling and fallbacks