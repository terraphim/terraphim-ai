# Terraphim Intelligent Routing Examples

This guide demonstrates how the Terraphim LLM Proxy uses intelligent routing to automatically select the best provider and model based on request characteristics.

## Overview

The proxy implements a **three-phase routing strategy**:

1. **Phase 1**: Rule-based routing for common scenarios (images, tools, long context)
2. **Phase 2**: RoleGraph pattern matching for semantic routing  
3. **Phase 3**: Fallback to default provider

## Supported Routing Scenarios

### 1. Image Generation Requests

**When to use**: Requests asking for image creation, visual content, or graphics

**Example Request**:
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

**Expected Route**: OpenAI DALL-E 3 (via image routing configuration)

**Routing Logic**:
- Detects keywords: "generate", "image", "picture", "draw", "create visual"
- Routes to provider configured for image generation
- Uses specialized image generation models

### 2. Web Search and Information Retrieval

**When to use**: Requests requiring current information, real-time data, or web search

**Example Request**:
```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-api-key" \
  -d '{
    "model": "auto",
    "messages": [
      {"role": "user", "content": "Search for the latest news about artificial intelligence"}
    ],
    "tools": [
      {
        "type": "function",
        "function": {
          "name": "web_search",
          "description": "Search the web for current information",
          "parameters": {
            "type": "object",
            "properties": {
              "query": {"type": "string"}
            }
          }
        }
      }
    ]
  }'
```

**Expected Route**: Perplexity Online (sonar-reasoning) or Anthropic Claude with web search

**Routing Logic**:
- Detects web search tools in request
- Identifies search keywords: "search", "latest", "current", "news"
- Routes to providers with real-time web access

### 3. Long Context Processing

**When to use**: Requests with large documents, long conversations, or extensive context

**Example Request**:
```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-api-key" \
  -d '{
    "model": "auto",
    "messages": [
      {"role": "user", "content": "'$(printf 'Analyze this very long research paper. %s' "$(head -c 50000 /dev/urandom | base64)")'"}
    ]
  }'
```

**Expected Route**: Anthropic Claude 3.5 Sonnet (200k context window)

**Routing Logic**:
- Calculates token count (>50,000 tokens triggers long context routing)
- Routes to providers with larger context windows
- Optimizes for cost and performance with long inputs

### 4. Reasoning and Thinking Tasks

**When to use**: Complex problem-solving, mathematical reasoning, step-by-step analysis

**Example Request**:
```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-api-key" \
  -d '{
    "model": "auto",
    "messages": [
      {"role": "user", "content": "Think step by step to solve this complex math problem: A train travels 300 miles in 4 hours. If it increases speed by 25%, how long will it take to travel 450 miles?"}
    ],
    "thinking": {}
  }'
```

**Expected Route**: DeepSeek Reasoner or OpenAI o1-preview

**Routing Logic**:
- Detects "thinking" field in request
- Identifies reasoning keywords: "think", "step by step", "solve", "analyze"
- Routes to specialized reasoning models

### 5. Background and Simple Tasks

**When to use**: Simple questions, quick responses, background tasks

**Example Request**:
```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-api-key" \
  -d '{
    "model": "auto",
    "messages": [
      {"role": "user", "content": "What is the capital of France?"}
    ]
  }'
```

**Expected Route**: Groq Llama 3.1 (fast, cost-effective)

**Routing Logic**:
- Detects simple, factual questions
- Routes to fast, inexpensive models
- Optimizes for high-throughput scenarios

## RoleGraph Pattern Matching

The proxy integrates with Terraphim's RoleGraph system for semantic understanding:

### Available Patterns

1. **think_routing**: Matches requests for reasoning, analysis, step-by-step thinking
   - Keywords: "think", "analyze", "reason", "step by step", "solve"
   - Routes to: DeepSeek Reasoner, OpenAI o1

2. **image_routing**: Matches image generation requests
   - Keywords: "generate", "image", "picture", "draw", "visual"
   - Routes to: OpenAI DALL-E 3, Midjourney

3. **search_routing**: Matches information retrieval requests
   - Keywords: "search", "find", "lookup", "latest", "current"
   - Routes to: Perplexity, Claude with web search

4. **code_routing**: Matches programming and development requests
   - Keywords: "code", "program", "debug", "implement", "function"
   - Routes to: GitHub Copilot, Claude 3.5 Sonnet

5. **creative_routing**: Matches creative writing and content creation
   - Keywords: "write", "create", "story", "poem", "creative"
   - Routes to: GPT-4, Claude 3 Opus

### Pattern Matching Process

1. **Request Analysis**: Extract keywords, context, and intent
2. **Pattern Matching**: Compare against RoleGraph patterns
3. **Confidence Scoring**: Rate pattern matches (0-100%)
4. **Route Selection**: Choose highest confidence match
5. **Fallback**: Use rule-based routing if no strong pattern match

## Configuration Examples

### Basic Routing Configuration

```toml
[router]
default = "openai,gpt-4o"
background = "groq,llama-3.1-8b"
think = "deepseek,deepseek-reasoner"
long_context = "anthropic,claude-3-5-sonnet"
long_context_threshold = 50000
web_search = "perplexity,sonar-reasoning"
image = "openai,dall-e-3"

[[providers]]
name = "openai"
api_base_url = "https://api.openai.com/v1"
api_key = "your-openai-key"
models = ["gpt-4o", "gpt-4o-mini", "dall-e-3"]

[[providers]]
name = "anthropic"
api_base_url = "https://api.anthropic.com"
api_key = "your-anthropic-key"
models = ["claude-3-5-sonnet", "claude-3-opus"]

[[providers]]
name = "deepseek"
api_base_url = "https://api.deepseek.com"
api_key = "your-deepseek-key"
models = ["deepseek-reasoner", "deepseek-chat"]

[[providers]]
name = "groq"
api_base_url = "https://api.groq.com/openai/v1"
api_key = "your-groq-key"
models = ["llama-3.1-8b", "llama-3.1-70b"]
```

### Advanced Configuration with Transformers

```toml
[[providers]]
name = "openai"
api_base_url = "https://api.openai.com/v1"
api_key = "your-openai-key"
models = ["gpt-4o", "gpt-4o-mini"]
transformers = ["enhancetool", "maxtoken", "sampling"]

[[providers]]
name = "anthropic"
api_base_url = "https://api.anthropic.com"
api_key = "your-anthropic-key"
models = ["claude-3-5-sonnet"]
transformers = ["cleancache", "reasoning"]

[[providers]]
name = "deepseek"
api_base_url = "https://api.deepseek.com"
api_key = "your-deepseek-key"
models = ["deepseek-reasoner"]
transformers = ["reasoning", "tooluse"]
```

## Monitoring and Debugging

### Enable Debug Logging

```bash
RUST_LOG=debug cargo run -- --config config.example.toml
```

### Sample Routing Logs

```
DEBUG request_analyzer: Analyzing request with model: auto
DEBUG request_analyzer: Token count: 12750
DEBUG request_analyzer: Long context detected (threshold: 50000)
DEBUG router: Phase 1 rule-based routing: LongContext
DEBUG router: Selected provider: anthropic, model: claude-3-5-sonnet
INFO router: Routing decision: LongContext -> anthropic/claude-3-5-sonnet
```

### Response Headers

```http
X-Router-Provider: anthropic
X-Router-Model: claude-3-5-sonnet
X-Router-Scenario: LongContext
X-Router-Phase: rule-based
X-Router-Token-Count: 12750
```

## Performance Considerations

### Routing Latency

- **Rule-based routing**: <1ms
- **Pattern matching**: 5-10ms
- **Total routing overhead**: <15ms

### Cost Optimization

- Background tasks: 90% cost reduction
- Long context: Optimized token pricing
- Specialized models: Better performance/cost ratio

### Caching

- Route decisions cached for 5 minutes
- Pattern matches cached for similar requests
- Provider performance metrics updated continuously

## Testing Routing

### Unit Tests

```bash
cargo test router::tests::test_route_long_context_scenario
cargo test router::tests::test_route_web_search_scenario
cargo test router::tests::test_pattern_matching_routing
```

### Integration Tests

```bash
cargo test --test integration_test
./scripts/test-routing-scenarios.sh
```

### Load Testing

```bash
cargo bench --bench performance_benchmark
./scripts/load-test-routing.sh
```

## Troubleshooting

### Common Issues

1. **Routing to Default Provider**
   - Check request format and keywords
   - Verify pattern matching configuration
   - Review RoleGraph taxonomy

2. **High Latency**
   - Enable debug logging
   - Check pattern matching performance
   - Consider caching strategies

3. **Incorrect Provider Selection**
   - Verify provider configuration
   - Check model availability
   - Review routing rules

### Debug Commands

```bash
# Test specific routing scenario
curl -X POST http://localhost:8080/debug/route \
  -H "Content-Type: application/json" \
  -d '{"message": "Generate an image of a cat"}'

# View routing statistics
curl http://localhost:8080/debug/stats

# Check provider health
curl http://localhost:8080/debug/health
```

## Best Practices

1. **Configuration**: Use specific models for each scenario
2. **Monitoring**: Track routing decisions and performance
3. **Testing**: Validate routing logic with diverse requests
4. **Optimization**: Adjust thresholds based on usage patterns
5. **Security**: Validate provider credentials and endpoints

## Future Enhancements

- **Machine Learning**: Adaptive routing based on performance
- **A/B Testing**: Compare routing strategies
- **Custom Patterns**: User-defined routing rules
- **Multi-Region**: Geographic routing optimization
- **Cost Budgets**: Per-user routing constraints