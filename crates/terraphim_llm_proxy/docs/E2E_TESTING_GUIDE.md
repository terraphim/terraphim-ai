# End-to-End Testing Guide - Terraphim LLM Proxy

**Purpose:** Validate complete proxy functionality with real Claude Code client
**Phase:** Week 4 of Phase 1 (MVP)
**Status:** Ready to begin

---

## Overview

This guide provides step-by-step instructions for testing the Terraphim LLM Proxy with a real Claude Code client to validate all routing scenarios and ensure production readiness.

---

## Prerequisites

### 1. Environment Setup

**Required:**
- ✅ Proxy built and ready: `cargo build --release`
- ✅ Test configuration created
- ✅ Provider API keys available

**Providers to Test:**

| Provider | Purpose | API Key Env Var | Free Tier? |
|----------|---------|----------------|------------|
| Ollama | Background/local | - | ✅ Free (local) |
| DeepSeek | Default/think | `DEEPSEEK_API_KEY` | ✅ Credits available |
| OpenRouter | Long context/search/image | `OPENROUTER_API_KEY` | Limited free |

### 2. Test Configuration

**File:** `config.e2e.toml`

```toml
[proxy]
host = "127.0.0.1"
port = 3456
api_key = "sk_test_e2e_proxy_key_for_validation_12345678901234567890"
timeout_ms = 600000

[router]
default = "deepseek,deepseek-chat"
background = "ollama,qwen2.5-coder:latest"
think = "deepseek,deepseek-reasoner"
long_context = "openrouter,google/gemini-2.0-flash-exp"
long_context_threshold = 60000
web_search = "openrouter,perplexity/llama-3.1-sonar-large-128k-online"
image = "openrouter,anthropic/claude-3.5-sonnet"

[security.rate_limiting]
enabled = false  # Disable for testing
concurrent_requests = 100

[security.ssrf_protection]
enabled = false  # Disable for testing

[[providers]]
name = "deepseek"
api_base_url = "https://api.deepseek.com/chat/completions"
api_key = "$DEEPSEEK_API_KEY"
models = ["deepseek-chat", "deepseek-reasoner"]
transformers = ["deepseek"]

[[providers]]
name = "ollama"
api_base_url = "http://localhost:11434/v1/chat/completions"
api_key = "ollama"
models = ["qwen2.5-coder:latest", "llama3.2:3b"]
transformers = ["ollama"]

[[providers]]
name = "openrouter"
api_base_url = "https://openrouter.ai/api/v1/chat/completions"
api_key = "$OPENROUTER_API_KEY"
models = [
    "google/gemini-2.0-flash-exp",
    "anthropic/claude-3.5-sonnet",
    "perplexity/llama-3.1-sonar-large-128k-online"
]
transformers = ["openrouter"]
```

### 3. Environment Variables

```bash
# Create .env.e2e file
cat > .env.e2e << 'EOF'
PROXY_API_KEY=sk_test_e2e_proxy_key_for_validation_12345678901234567890
DEEPSEEK_API_KEY=your_deepseek_api_key_here
OPENROUTER_API_KEY=your_openrouter_api_key_here
RUST_LOG=info
EOF

# Load environment
source .env.e2e
```

---

## Test Scenarios

### Scenario 1: Default Routing (DeepSeek)

**Objective:** Validate default routing to DeepSeek for standard requests

**Request:**
```bash
curl -X POST http://localhost:3456/v1/messages \
  -H "x-api-key: $PROXY_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-3-5-sonnet-20241022",
    "messages": [
      {"role": "user", "content": "Write a hello world function in Rust"}
    ],
    "max_tokens": 1024,
    "stream": false
  }'
```

**Expected:**
- HTTP 200 OK
- Response from DeepSeek (check logs for provider name)
- JSON response with assistant message
- Token usage in response

**Validation Checklist:**
- [ ] Request accepted (200 OK)
- [ ] Routed to deepseek provider (check logs: "provider=deepseek")
- [ ] DeepSeek transformer applied
- [ ] Response contains Rust code
- [ ] Token counts present in response
- [ ] Response time <2 seconds

---

### Scenario 2: Background Routing (Ollama Local)

**Objective:** Validate background task routing to local Ollama

**Prerequisites:**
```bash
# Ensure Ollama is running
ollama serve &

# Pull required model
ollama pull qwen2.5-coder:latest
```

**Request:**
```bash
curl -X POST http://localhost:3456/v1/messages \
  -H "x-api-key: $PROXY_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-3-5-haiku-20241022",
    "messages": [
      {"role": "user", "content": "Quick code review task"}
    ],
    "stream": false
  }'
```

**Expected:**
- HTTP 200 OK
- Routed to ollama provider (local, free)
- Fast response (<500ms)

**Validation Checklist:**
- [ ] Request accepted
- [ ] Detected as background (logs: "is_background=true")
- [ ] Routed to ollama provider (logs: "provider=ollama")
- [ ] Response from local model
- [ ] Fast response time
- [ ] No API costs incurred

---

### Scenario 3: Thinking Mode (DeepSeek Reasoner)

**Objective:** Validate thinking mode routing to reasoning model

**Request:**
```bash
curl -X POST http://localhost:3456/v1/messages \
  -H "x-api-key: $PROXY_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-3-5-sonnet-20241022",
    "messages": [
      {"role": "user", "content": "Explain the halting problem"}
    ],
    "thinking": {"enabled": true},
    "stream": false
  }'
```

**Expected:**
- HTTP 200 OK
- Routed to deepseek-reasoner model
- Detailed reasoning in response

**Validation Checklist:**
- [ ] Request accepted
- [ ] Thinking field detected (logs: "has_thinking=true")
- [ ] Routed to think scenario (logs: "scenario=Think")
- [ ] Model: deepseek-reasoner
- [ ] Response shows reasoning depth

---

### Scenario 4: Long Context (Gemini 2.0 Flash)

**Objective:** Validate long context routing for large requests

**Request:**
```bash
# Create a large context request (>60K tokens)
cat > large_context.json << 'EOF'
{
  "model": "claude-3-5-sonnet-20241022",
  "messages": [
    {
      "role": "user",
      "content": "Analyze this codebase: [paste entire large file here - aim for >60K tokens]"
    }
  ],
  "stream": false
}
EOF

curl -X POST http://localhost:3456/v1/messages \
  -H "x-api-key: $PROXY_API_KEY" \
  -H "Content-Type: application/json" \
  -d @large_context.json
```

**Expected:**
- HTTP 200 OK
- Routed to openrouter/gemini-2.0-flash-exp
- Successfully processes large context

**Validation Checklist:**
- [ ] Request accepted
- [ ] Token count >60,000 (logs: "token_count=...")
- [ ] Routed to long_context scenario
- [ ] Model: google/gemini-2.0-flash-exp
- [ ] Response handles full context
- [ ] No token limit errors

---

### Scenario 5: Web Search (Perplexity)

**Objective:** Validate web search tool routing

**Request:**
```bash
curl -X POST http://localhost:3456/v1/messages \
  -H "x-api-key: $PROXY_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-3-5-sonnet-20241022",
    "messages": [
      {"role": "user", "content": "Search for latest Rust news"}
    ],
    "tools": [{
      "name": "web_search",
      "description": "Search the web",
      "input_schema": {
        "type": "object",
        "properties": {
          "query": {"type": "string"}
        }
      }
    }],
    "stream": false
  }'
```

**Expected:**
- HTTP 200 OK
- Routed to perplexity model (search-enabled)
- Tool detected and processed

**Validation Checklist:**
- [ ] Request accepted
- [ ] Web search tool detected (logs: "has_web_search=true")
- [ ] Routed to web_search scenario
- [ ] Model: perplexity/llama-3.1-sonar
- [ ] Tool processed correctly

---

### Scenario 6: Image Analysis (Claude Vision)

**Objective:** Validate image content routing

**Request:**
```bash
curl -X POST http://localhost:3456/v1/messages \
  -H "x-api-key: $PROXY_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-3-5-sonnet-20241022",
    "messages": [{
      "role": "user",
      "content": [
        {"type": "text", "text": "What is in this image?"},
        {
          "type": "image",
          "source": {
            "type": "url",
            "url": "https://example.com/image.png"
          }
        }
      ]
    }],
    "stream": false
  }'
```

**Expected:**
- HTTP 200 OK
- Routed to vision model
- Image analyzed

**Validation Checklist:**
- [ ] Request accepted
- [ ] Image detected (logs: "has_images=true")
- [ ] Routed to image scenario
- [ ] Model: anthropic/claude-3.5-sonnet
- [ ] Image processed and described

---

### Scenario 7: SSE Streaming

**Objective:** Validate real-time streaming with SSE

**Request:**
```bash
curl -N -X POST http://localhost:3456/v1/messages \
  -H "x-api-key: $PROXY_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-3-5-sonnet-20241022",
    "messages": [
      {"role": "user", "content": "Count from 1 to 10"}
    ],
    "stream": true,
    "max_tokens": 100
  }'
```

**Expected:**
- SSE event stream
- Real-time chunks
- Proper event format

**Validation Checklist:**
- [ ] Request accepted
- [ ] SSE stream starts immediately
- [ ] message_start event received
- [ ] content_block_delta events stream
- [ ] message_stop event at end
- [ ] Complete response assembled from chunks

---

### Scenario 8: Token Counting Accuracy

**Objective:** Validate token counting matches Claude API

**Request:**
```bash
# Test message
MESSAGE="The quick brown fox jumps over the lazy dog"

# Count via proxy
PROXY_COUNT=$(curl -s -X POST http://localhost:3456/v1/messages/count_tokens \
  -H "x-api-key: $PROXY_API_KEY" \
  -H "Content-Type: application/json" \
  -d "{\"model\":\"test\",\"messages\":[{\"role\":\"user\",\"content\":\"$MESSAGE\"}]}" \
  | jq -r '.input_tokens')

echo "Proxy counted: $PROXY_COUNT tokens"
# Expected: 13-15 tokens (±1 from Claude API)
```

**Reference Counts:**
- "Hello, world!" → 4 tokens
- "The quick brown fox jumps over the lazy dog" → 9 tokens
- Empty string → 0 tokens

**Validation Checklist:**
- [ ] Simple text: within ±1 token of reference
- [ ] System prompt: counted correctly
- [ ] Tools: definition tokens included
- [ ] Images: estimated correctly (85/340/1360)
- [ ] Multiple messages: cumulative correct

---

### Scenario 9: Error Handling

**Objective:** Validate graceful error handling

**Test Cases:**

**A. Missing API Key:**
```bash
curl -X POST http://localhost:3456/v1/messages/count_tokens \
  -H "Content-Type: application/json" \
  -d '{"model":"test","messages":[{"role":"user","content":"Hi"}]}'
# Expected: 401 Unauthorized
```

**B. Invalid Model:**
```bash
curl -X POST http://localhost:3456/v1/messages \
  -H "x-api-key: $PROXY_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "nonexistent-model",
    "messages": [{"role": "user", "content": "Test"}]
  }'
# Expected: Provider error or routing fallback
```

**C. Provider Timeout:**
```bash
# Simulate slow provider by using invalid URL
# (Requires temporary config change)
# Expected: 504 Gateway Timeout
```

**Validation Checklist:**
- [ ] 401 for missing API key
- [ ] 401 for invalid API key
- [ ] Appropriate error for missing provider
- [ ] Timeout handling works
- [ ] Error messages are clear and actionable

---

## Testing Methodology

### Phase 1: Manual Testing (Day 22-23)

**Steps:**
1. Start Ollama: `ollama serve`
2. Start proxy: `RUST_LOG=info ./target/release/terraphim-llm-proxy --config config.e2e.toml`
3. Run each test scenario using curl
4. Monitor logs for routing decisions
5. Validate responses
6. Document any issues

### Phase 2: Claude Code Integration (Day 24)

**Steps:**
1. Configure Claude Code:
   ```json
   {
     "api_base_url": "http://127.0.0.1:3456",
     "api_key": "sk_test_e2e_proxy_key_for_validation_12345678901234567890"
   }
   ```
2. Test basic chat interaction
3. Test code generation request
4. Test different routing scenarios
5. Validate streaming responses
6. Check token counting accuracy

### Phase 3: Automated Testing (Day 25)

**Create test script:** `scripts/e2e-test.sh`

```bash
#!/bin/bash
set -e

echo "Starting E2E tests..."

# Test 1: Health check
echo "Test 1: Health check"
curl -f http://localhost:3456/health || exit 1

# Test 2: Token counting
echo "Test 2: Token counting"
TOKENS=$(curl -s -X POST http://localhost:3456/v1/messages/count_tokens \
  -H "x-api-key: $PROXY_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"model":"test","messages":[{"role":"user","content":"Hello, world!"}]}' \
  | jq -r '.input_tokens')

if [ "$TOKENS" -lt 3 ] || [ "$TOKENS" -gt 6 ]; then
  echo "FAIL: Token count $TOKENS not in expected range 3-6"
  exit 1
fi
echo "PASS: Token count $TOKENS"

# Test 3: Default routing
echo "Test 3: Default routing"
# ... more tests

echo "All E2E tests passed!"
```

---

## Expected Results

### Success Criteria

| Scenario | Success | Evidence |
|----------|---------|----------|
| Default routing | ✅ | Response from DeepSeek |
| Background routing | ✅ | Response from Ollama (free) |
| Think routing | ✅ | Response from DeepSeek Reasoner |
| Long context routing | ✅ | Response from Gemini 2.0 Flash |
| Web search routing | ✅ | Response from Perplexity |
| Image routing | ✅ | Response from Claude Vision |
| SSE streaming | ✅ | Real-time event stream |
| Token counting | ✅ | Within ±1 token of reference |
| Error handling | ✅ | Appropriate HTTP status codes |

### Performance Targets

| Metric | Target | Measurement Method |
|--------|--------|-------------------|
| Median latency | <100ms | Time from request to first byte |
| P95 latency | <500ms | 95th percentile response time |
| Throughput | >100 req/s | Concurrent requests handled |
| Token accuracy | 95%+ | Comparison with Claude API |
| Error rate | <0.1% | Percentage of failed requests |

---

## Logging and Monitoring

### Key Log Messages to Watch

**Routing Decision:**
```
INFO Routing decision: provider=deepseek, model=deepseek-chat, scenario=Default
```

**Request Analysis:**
```
INFO Request analyzed: token_count=150, is_background=false, has_thinking=false, has_web_search=false, has_images=false
```

**Transformer Application:**
```
DEBUG Applied transformer chain: transformers=1
```

**LLM Response:**
```
INFO Request completed successfully: input_tokens=150, output_tokens=45
```

### Debugging Commands

**Follow logs in real-time:**
```bash
RUST_LOG=debug ./target/release/terraphim-llm-proxy --config config.e2e.toml 2>&1 | tee e2e-test.log
```

**Filter for routing decisions:**
```bash
grep "Routing decision" e2e-test.log
```

**Check token counts:**
```bash
grep "Token count" e2e-test.log
```

---

## Issue Tracking Template

### Issue Report Format

```markdown
## Issue: [Brief Description]

**Scenario:** [Which test scenario]
**Date:** YYYY-MM-DD
**Severity:** Critical | High | Medium | Low

**Description:**
[What went wrong]

**Expected Behavior:**
[What should have happened]

**Actual Behavior:**
[What actually happened]

**Logs:**
```
[Relevant log excerpts]
```

**Configuration:**
```toml
[Relevant config section]
```

**Steps to Reproduce:**
1. [Step by step]

**Workaround:**
[If any]

**Fix Required:**
[What needs to be fixed]
```

---

## Test Results Documentation

### Results Template

**File:** `E2E_TEST_RESULTS.md`

```markdown
# E2E Test Results - Terraphim LLM Proxy

**Date:** YYYY-MM-DD
**Tester:** [Name]
**Proxy Version:** Phase 1, Week 4
**Configuration:** config.e2e.toml

## Test Summary

| Scenario | Status | Latency | Notes |
|----------|--------|---------|-------|
| Default routing | ✅/❌ | XXXms | [Notes] |
| Background routing | ✅/❌ | XXXms | [Notes] |
| Think routing | ✅/❌ | XXXms | [Notes] |
| Long context | ✅/❌ | XXXms | [Notes] |
| Web search | ✅/❌ | XXXms | [Notes] |
| Image analysis | ✅/❌ | XXXms | [Notes] |
| SSE streaming | ✅/❌ | XXXms | [Notes] |
| Token counting | ✅/❌ | - | Accuracy: XX% |
| Error handling | ✅/❌ | - | [Notes] |

**Overall:** X/9 scenarios passing

## Issues Found

[List any issues using issue report template]

## Recommendations

[Any recommendations for improvements]
```

---

## Next Steps After Testing

### If All Tests Pass ✅

1. Document results in E2E_TEST_RESULTS.md
2. Update PROGRESS.md to 95% completion
3. Proceed with performance benchmarks
4. Begin production deployment guide

### If Tests Fail ❌

1. Document all failures with issue reports
2. Prioritize issues by severity
3. Fix critical issues immediately
4. Retest after fixes
5. Update timeline if needed

---

## Automated Test Script

**File:** `scripts/run-e2e-tests.sh`

```bash
#!/bin/bash
# Automated E2E test suite for Terraphim LLM Proxy

set -e

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

PROXY_URL="http://localhost:3456"
RESULTS_FILE="e2e-test-results.json"

echo "=== Terraphim LLM Proxy E2E Tests ==="
echo "Proxy URL: $PROXY_URL"
echo "Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo ""

# Initialize results
echo '{"tests": [], "summary": {}}' > $RESULTS_FILE

# Test 1: Health check
echo -n "Test 1: Health check... "
if curl -sf $PROXY_URL/health > /dev/null; then
  echo -e "${GREEN}PASS${NC}"
else
  echo -e "${RED}FAIL${NC}"
  exit 1
fi

# Test 2: Token counting
echo -n "Test 2: Token counting... "
TOKENS=$(curl -s -X POST $PROXY_URL/v1/messages/count_tokens \
  -H "x-api-key: $PROXY_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"model":"test","messages":[{"role":"user","content":"Hello!"}]}' \
  | jq -r '.input_tokens')

if [ "$TOKENS" -ge 5 ] && [ "$TOKENS" -le 10 ]; then
  echo -e "${GREEN}PASS${NC} (tokens: $TOKENS)"
else
  echo -e "${RED}FAIL${NC} (tokens: $TOKENS, expected 5-10)"
  exit 1
fi

# Add more tests...

echo ""
echo "=== Test Summary ==="
echo "All tests passed!"
```

---

## Success Metrics

### Completion Criteria

- [ ] All 9 test scenarios pass
- [ ] Token counting within 95% accuracy
- [ ] All routing scenarios validated
- [ ] SSE streaming works end-to-end
- [ ] Error handling covers all cases
- [ ] Performance targets met or documented
- [ ] Issues documented with fix plans
- [ ] Results documented in E2E_TEST_RESULTS.md

### Sign-off Requirements

- [ ] Technical lead approval
- [ ] Test results reviewed
- [ ] All critical issues resolved
- [ ] Documentation complete
- [ ] Ready for performance testing

---

**Status:** Ready to begin E2E testing | All prerequisites met | Testing guide complete
