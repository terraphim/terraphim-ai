# Routing Scenarios for Testing

This file contains example requests that demonstrate different routing scenarios in the Terraphim LLM Proxy.

## Scenario 1: Simple Question (Default Routing)

**Request**: "What is the capital of France?"

**Expected Route**: Default provider (OpenAI GPT-4o)

**Characteristics**:
- Short, factual question
- No special tools or requirements
- Low token count (~10 tokens)

**Routing Logic**: Phase 1 rule-based routing → Default scenario

---

## Scenario 2: Image Generation (Image Routing)

**Request**: "Generate an image of a sunset over mountains with a lake in the foreground"

**Expected Route**: OpenAI DALL-E 3

**Characteristics**:
- Contains "generate" and "image" keywords
- Visual content request
- Requires image generation capabilities

**Routing Logic**: Phase 1 rule-based routing → Image scenario

---

## Scenario 3: Web Search (Search Routing)

**Request**: "Search for the latest news about artificial intelligence breakthroughs in 2024"

**Expected Route**: Perplexity Sonar Reasoning (with web search)

**Characteristics**:
- Contains "search" and "latest" keywords
- Requires current information
- Benefits from real-time web access

**Routing Logic**: Phase 1 rule-based routing → WebSearch scenario

---

## Scenario 4: Complex Reasoning (Think Routing)

**Request**: "Think step by step to solve this problem: A company has 150 employees. If 20% work in engineering, 30% in sales, and the rest in operations, how many work in operations? Also calculate the ratio of engineering to sales employees."

**Expected Route**: DeepSeek Reasoner

**Characteristics**:
- Contains "think step by step" 
- Multi-step mathematical problem
- Requires analytical reasoning

**Routing Logic**: Phase 1 rule-based routing → Think scenario

---

## Scenario 5: Long Context (Long Context Routing)

**Request**: [50,000+ token document] "Analyze this research paper on quantum computing and summarize the key findings, methodology, and conclusions. Focus on the practical applications and future research directions."

**Expected Route**: Anthropic Claude 3.5 Sonnet (200k context)

**Characteristics**:
- Very long input (>50,000 tokens)
- Requires large context window
- Complex document analysis

**Routing Logic**: Phase 1 rule-based routing → LongContext scenario

---

## Scenario 6: Code Generation (Pattern Matching)

**Request**: "Write a Python function that implements a binary search algorithm with proper error handling and documentation. Include test cases."

**Expected Route**: GitHub Copilot or Claude 3.5 Sonnet

**Characteristics**:
- Contains "code", "function", "implement" keywords
- Programming task
- Requires technical accuracy

**Routing Logic**: Phase 2 pattern matching → code_routing pattern

---

## Scenario 7: Creative Writing (Pattern Matching)

**Request**: "Write a short story about a time traveler who accidentally changes history while trying to save a lost artifact. Include elements of mystery and romance."

**Expected Route**: GPT-4 or Claude 3 Opus

**Characteristics**:
- Contains "write", "story" keywords
- Creative content request
- Narrative generation

**Routing Logic**: Phase 2 pattern matching → creative_routing pattern

---

## Scenario 8: Background Task (Background Routing)

**Request**: "What's 2+2?"

**Expected Route**: Groq Llama 3.1 8B (fast, cheap)

**Characteristics**:
- Very simple question
- Low complexity
- High throughput requirement

**Routing Logic**: Phase 1 rule-based routing → Background scenario

---

## Scenario 9: Tool Usage (Advanced Routing)

**Request**: "Find information about the weather in Tokyo and convert the temperature to Celsius, then create a simple chart showing the 5-day forecast."

**Expected Route**: OpenAI GPT-4o with tool enhancement

**Characteristics**:
- Multiple tool requirements (weather search, conversion, charting)
- Complex multi-step task
- Requires tool coordination

**Routing Logic**: Phase 1 rule-based routing + tool enhancement transformer

---

## Scenario 10: Research and Analysis (Advanced Pattern Matching)

**Request**: "Analyze the impact of remote work on employee productivity and mental health. Consider factors like work-life balance, communication patterns, and technological infrastructure. Provide recommendations for hybrid work policies."

**Expected Route**: Claude 3.5 Sonnet with reasoning enhancement

**Characteristics**:
- Complex analytical task
- Multiple research aspects
- Requires synthesis and recommendations

**Routing Logic**: Phase 2 pattern matching → research_routing pattern + reasoning transformer

---

## Testing Matrix

| Scenario | Route Type | Provider | Model | Latency Target | Cost Optimization |
|----------|------------|----------|-------|----------------|------------------|
| 1 | Default | OpenAI | GPT-4o | <2s | Standard |
| 2 | Image | OpenAI | DALL-E 3 | <10s | Standard |
| 3 | Search | Perplexity | Sonar | <3s | Premium |
| 4 | Think | DeepSeek | Reasoner | <5s | Premium |
| 5 | Long Context | Anthropic | Claude 3.5 | <8s | Optimized |
| 6 | Code | GitHub | Copilot | <3s | Standard |
| 7 | Creative | OpenAI | GPT-4 | <4s | Premium |
| 8 | Background | Groq | Llama 3.1 8B | <1s | High |
| 9 | Tools | OpenAI | GPT-4o | <6s | Standard |
| 10 | Research | Anthropic | Claude 3.5 | <7s | Premium |

## Performance Metrics

### Routing Decision Latency
- Rule-based: <1ms
- Pattern matching: 5-10ms
- Total overhead: <15ms

### Success Rate Targets
- Correct routing: >95%
- Provider availability: >99%
- Response quality: >90% user satisfaction

### Cost Optimization
- Background tasks: 90% cost reduction
- Long context: 40% cost savings
- Specialized routing: 25% overall savings

## Implementation Notes

1. **Token Counting**: Accurate token counting is essential for long context detection
2. **Pattern Matching**: RoleGraph taxonomy must be regularly updated
3. **Provider Health**: Continuous monitoring of provider availability
4. **Caching**: Route decisions and pattern matches should be cached
5. **Monitoring**: Detailed logging for routing decisions and performance

## Test Automation

```bash
# Run all routing scenarios
./scripts/test-routing-scenarios.sh

# Performance benchmarking
cargo bench --bench routing_performance

# Load testing
./scripts/load-test-routing.sh
```