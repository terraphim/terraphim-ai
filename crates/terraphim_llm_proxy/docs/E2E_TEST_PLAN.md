# Complete E2E Test Plan - Terraphim LLM Proxy with Claude Code

**Date:** 2025-10-12
**Purpose:** Validate Terraphim LLM Proxy as drop-in replacement for claude-code-router
**Status:** Ready to execute
**Estimated Time:** 3-4 hours

---

## Research Summary

**Claude Code Repository Analysis:**
- ✅ Cloned from https://github.com/anthropics/claude-code
- ❌ **No formal test suite found**
- ✅ Claude Code is NPM package: `@anthropic-ai/claude-code`
- ✅ Configuration via `--settings` flag (JSON file)
- ✅ Supports custom model via `--model` flag

**Conclusion:** Must create functional behavior tests, no reference test suite exists.

---

## Environment Setup

### Option 1: Using Existing Environment Variables (Recommended)

**Available:**
- `ANTHROPIC_API_KEY` - ✅ Already set in environment
- `OPENROUTER_API_KEY` - ✅ Already set in environment

**Configuration:**
```bash
# Use existing ANTHROPIC_API_KEY as proxy API key
export PROXY_API_KEY="$ANTHROPIC_API_KEY"

# Providers will use existing keys
export DEEPSEEK_API_KEY="${DEEPSEEK_API_KEY:-$ANTHROPIC_API_KEY}"
export OPENROUTER_API_KEY="$OPENROUTER_API_KEY"

# Start proxy
./target/release/terraphim-llm-proxy --config config.toml
```

### Option 2: Using 1Password CLI

**For production-like testing:**
```bash
# Use .env.op file with 1Password references
op run --env-file=.env.op -- ./target/release/terraphim-llm-proxy --config config.toml
```

**Created files:**
- `.env.op` - 1Password secret references
- `scripts/start-proxy-with-op.sh` - Startup script with op injection

---

## Complete Test Execution Plan

### PHASE 1: Proxy Setup & Validation (15 minutes)

**Step 1.1: Configure Proxy**
```bash
cd /home/alex/claude_code_agents/terraphim-llm-proxy

# Update config.toml to use existing API keys
cat > config.toml << 'EOF'
[proxy]
host = "127.0.0.1"
port = 3456
api_key = "$ANTHROPIC_API_KEY"  # Will use from environment
timeout_ms = 600000

[router]
default = "test,test-model"

[[providers]]
name = "test"
api_base_url = "http://localhost:8000"  # Placeholder
api_key = "test"
models = ["test-model"]
transformers = []
EOF
```

**Step 1.2: Start Proxy**
```bash
# Use existing ANTHROPIC_API_KEY as proxy key
export PROXY_API_KEY="$ANTHROPIC_API_KEY"

# Start proxy with logging
RUST_LOG=info ./target/release/terraphim-llm-proxy --config config.toml 2>&1 | tee proxy-e2e-test.log &

# Save PID
PROXY_PID=$!

# Wait for startup
sleep 2
```

**Step 1.3: Validate Proxy Running**
```bash
# Test health endpoint
curl http://localhost:3456/health
# Expected: OK

# Test token counting
curl -X POST http://localhost:3456/v1/messages/count_tokens \
  -H "x-api-key: $ANTHROPIC_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"model":"test","messages":[{"role":"user","content":"Hello, world!"}]}'
# Expected: {"input_tokens":9}
```

---

### PHASE 2: Claude Code Integration (30 minutes)

**Step 2.1: Create Claude Code Settings**
```bash
cat > proxy-settings.json << EOF
{
  "api_base_url": "http://127.0.0.1:3456",
  "api_key": "$ANTHROPIC_API_KEY"
}
EOF
```

**Step 2.2: Test Claude Code Connection**
```bash
# Start Claude Code with proxy settings
claude --settings proxy-settings.json --print "Hello, this is a test. Respond with just 'Connection successful.'"

# Expected: Response from proxy
# Check proxy logs for request
```

**Step 2.3: Test Interactive Session**
```bash
# Start interactive Claude Code
claude --settings proxy-settings.json

# In Claude session, test:
# 1. "Hello" - Basic chat
# 2. "What is 2+2?" - Simple question
# 3. "Write a hello world in Rust" - Code generation
```

---

### PHASE 3: Routing Scenario Tests (60 minutes)

**Test 3.1: Default Routing**

**Update config.toml with real provider:**
```toml
[router]
default = "anthropic,claude-3-5-sonnet-20241022"

[[providers]]
name = "anthropic"
api_base_url = "https://api.anthropic.com/v1/messages"
api_key = "$ANTHROPIC_API_KEY"
models = ["claude-3-5-sonnet-20241022"]
transformers = ["anthropic"]
```

**Test:**
```bash
# Restart proxy with new config
kill $PROXY_PID
PROXY_API_KEY="$ANTHROPIC_API_KEY" RUST_LOG=info \
  ./target/release/terraphim-llm-proxy --config config.toml &

# Test via Claude Code
claude --settings proxy-settings.json --print "Explain what a hash map is"

# Expected: Response from Anthropic
# Validation: Proxy logs show "scenario=Default, provider=anthropic"
```

**Test 3.2: Background Routing (If Haiku Model Available)**

**Claude Code command:**
```bash
# Try to switch model to haiku
claude --settings proxy-settings.json --model haiku --print "Quick task"

# Expected: Routes to background provider if haiku detected
# Validation: Proxy logs show "is_background=true"
```

**Test 3.3: Token Counting Accuracy**

**Multiple test cases:**
```bash
# Test case 1: Simple text
claude --settings proxy-settings.json --print "Hello, world!"
# Check proxy logs for token count, compare with response

# Test case 2: Longer text
claude --settings proxy-settings.json --print "The quick brown fox jumps over the lazy dog. This is a test of token counting accuracy."
# Check token count in logs

# Validation: Within ±2 tokens of expected
```

---

### PHASE 4: Functional Tests (45 minutes)

**Test 4.1: Code Generation**
```bash
claude --settings proxy-settings.json --print "Write a Fibonacci function in Rust with tests"

# Expected: Complete Rust code with function and tests
# Validation: Code is syntactically correct
```

**Test 4.2: Code Explanation**
```bash
# Create a test file
echo 'fn main() { println!("Hello"); }' > test.rs

# Ask Claude to explain it
claude --settings proxy-settings.json --print "Explain what this code does: $(cat test.rs)"

# Expected: Accurate explanation
# Validation: Explanation matches code
```

**Test 4.3: Multi-Turn Conversation**
```bash
# Start interactive session
claude --settings proxy-settings.json

# In session:
# Turn 1: "Let's build a simple HTTP server"
# Turn 2: "Use Axum framework"
# Turn 3: "Add authentication"

# Expected: Context maintained across turns
# Validation: Later responses reference earlier conversation
```

**Test 4.4: File Operations via Claude Code**
```bash
# Interactive session
claude --settings proxy-settings.json

# In session:
# "Read the README.md file and summarize it"

# Expected: Claude reads file through proxy, provides summary
# Validation: Summary matches README content
```

---

### PHASE 5: Error Handling (20 minutes)

**Test 5.1: Invalid API Key**
```bash
# Create settings with wrong key
cat > bad-settings.json << 'EOF'
{
  "api_base_url": "http://127.0.0.1:3456",
  "api_key": "wrong_key"
}
EOF

claude --settings bad-settings.json --print "Test"

# Expected: Authentication error
# Validation: Claude shows clear error message
```

**Test 5.2: Proxy Down**
```bash
# Stop proxy
kill $PROXY_PID

# Try to use Claude
claude --settings proxy-settings.json --print "Test"

# Expected: Connection error
# Validation: Clear "cannot connect" message
```

**Test 5.3: Provider Timeout**
```bash
# Configure very short timeout in config
# Send request
# Expected: Timeout error with clear message
```

---

### PHASE 6: Performance Measurements (30 minutes)

**Test 6.1: Response Time**
```bash
# Measure 10 simple requests
for i in {1..10}; do
    echo "Request $i:"
    time claude --settings proxy-settings.json --print "Say hello"
    echo ""
done

# Calculate median response time
# Target: <3 seconds total (including LLM)
```

**Test 6.2: Token Counting Speed**
```bash
# Measure token counting performance
time for i in {1..100}; do
    curl -s -X POST http://localhost:3456/v1/messages/count_tokens \
      -H "x-api-key: $ANTHROPIC_API_KEY" \
      -H "Content-Type: application/json" \
      -d '{"model":"test","messages":[{"role":"user","content":"Test"}]}' > /dev/null
done

# Target: <50ms per count (100 counts in <5 seconds)
```

**Test 6.3: Concurrent Requests**
```bash
# Send 5 requests in parallel
for i in {1..5}; do
    (claude --settings proxy-settings.json --print "Request $i" &)
done
wait

# Expected: All complete successfully
# Validation: Check proxy logs for 5 successful requests
```

---

## Test Execution Commands

### Quick Test Script

**File:** `scripts/quick-e2e-test.sh`

```bash
#!/bin/bash
set -e

echo "Quick E2E Test for Terraphim LLM Proxy"
echo "======================================"

# Use existing environment
export PROXY_API_KEY="$ANTHROPIC_API_KEY"

# Start proxy in background
echo "Starting proxy..."
RUST_LOG=info ./target/release/terraphim-llm-proxy --config config.toml &
PROXY_PID=$!
sleep 3

# Test 1: Health
echo "Test 1: Health check"
curl -sf http://localhost:3456/health && echo "✅ PASS" || echo "❌ FAIL"

# Test 2: Token counting
echo "Test 2: Token counting"
TOKENS=$(curl -s -X POST http://localhost:3456/v1/messages/count_tokens \
  -H "x-api-key: $ANTHROPIC_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"model":"test","messages":[{"role":"user","content":"Hello"}]}' | jq -r '.input_tokens')
if [ "$TOKENS" -ge 5 ] && [ "$TOKENS" -le 10 ]; then
    echo "✅ PASS (tokens: $TOKENS)"
else
    echo "❌ FAIL (tokens: $TOKENS, expected 5-10)"
fi

# Test 3: Claude Code connection
echo "Test 3: Claude Code via proxy"
if command -v claude &> /dev/null; then
    cat > proxy-settings.json << EOF
{
  "api_base_url": "http://127.0.0.1:3456",
  "api_key": "$ANTHROPIC_API_KEY"
}
EOF

    claude --settings proxy-settings.json --print "Say 'test successful'" && echo "✅ PASS" || echo "❌ FAIL"
else
    echo "⚠️  SKIP: Claude Code not installed"
fi

# Cleanup
kill $PROXY_PID
echo ""
echo "Quick test complete!"
```

---

## Documentation Requirements

### E2E_TEST_RESULTS.md Template

```markdown
# E2E Test Results - Terraphim LLM Proxy

**Date:** YYYY-MM-DD
**Duration:** X hours
**Proxy Version:** 0.1.0
**Claude Code Version:** [version]

## Environment
- Proxy: v0.1.0 on 127.0.0.1:3456
- Providers: Anthropic (primary), Ollama (background)
- Configuration: config.toml

## Test Summary

| Phase | Tests | Passed | Failed | Skipped |
|-------|-------|--------|--------|---------|
| Setup | 3 | - | - | - |
| Claude Integration | 3 | - | - | - |
| Routing Scenarios | 5 | - | - | - |
| Error Handling | 3 | - | - | - |
| Performance | 3 | - | - | - |
| **Total** | **17** | **-** | **-** | **-** |

## Detailed Results

### Phase 1: Setup
[Test results...]

### Phase 2: Claude Code Integration
[Test results...]

### Phase 3: Routing Scenarios
[Test results...]

## Performance Measurements

- Median latency: ___ ms
- Token counting accuracy: ___%
- Concurrent request handling: ___ req/s

## Issues Found

| # | Severity | Issue | Status |
|---|----------|-------|--------|
| 1 | | | |

## Recommendations

**Production Readiness:** ✅/⏳/❌
**Deployment Recommendation:** [Details]
```

---

## Success Criteria

### Minimum (Required for Phase 1 Completion)

- [ ] Proxy starts successfully with existing environment variables
- [ ] Health endpoint responds
- [ ] Token counting works and is accurate (95%+)
- [ ] Claude Code can connect to proxy
- [ ] Basic chat works through proxy
- [ ] At least 1 routing scenario works
- [ ] SSE streaming functional

### Target (100% Phase 1)

- [ ] All 6 routing scenarios validated
- [ ] All error scenarios handled gracefully
- [ ] Performance acceptable (<3s total response time)
- [ ] Zero critical compatibility issues
- [ ] Complete test documentation

### Stretch (Excellence)

- [ ] Performance meets targets (<100ms proxy overhead)
- [ ] All edge cases handled
- [ ] Zero issues found
- [ ] Ready for immediate production deployment

---

## Execution Instructions

### Quick Start

**Terminal 1: Start Proxy with Existing Credentials**
```bash
cd /home/alex/claude_code_agents/terraphim-llm-proxy

# Use existing ANTHROPIC_API_KEY
export PROXY_API_KEY="$ANTHROPIC_API_KEY"

# Start proxy
RUST_LOG=info ./target/release/terraphim-llm-proxy --config config.toml 2>&1 | tee logs/e2e-test-$(date +%Y%m%d-%H%M%S).log
```

**Terminal 2: Run Tests**
```bash
# Quick automated tests
./scripts/test-all-scenarios.sh

# Test with Claude Code
cat > proxy-settings.json << EOF
{
  "api_base_url": "http://127.0.0.1:3456",
  "api_key": "$ANTHROPIC_API_KEY"
}
EOF

# Test basic connection
claude --settings proxy-settings.json --print "Connection test"

# Interactive testing
claude --settings proxy-settings.json
```

**Terminal 3: Monitor Logs**
```bash
# Watch routing decisions
tail -f logs/e2e-test-*.log | grep "Routing decision"

# Watch for errors
tail -f logs/e2e-test-*.log | grep -i error
```

---

## Alternative: Test with 1Password CLI

**If using 1Password for secrets:**

```bash
# Start proxy with 1Password secret injection
./scripts/start-proxy-with-op.sh config.toml

# In another terminal
export PROXY_API_KEY=$(op read "op://Shared/OpenRouterClaudeCode/api-key")

# Create Claude settings
cat > proxy-settings.json << EOF
{
  "api_base_url": "http://127.0.0.1:3456",
  "api_key": "$(op read "op://Shared/OpenRouterClaudeCode/api-key")"
}
EOF

# Run Claude Code
claude --settings proxy-settings.json
```

---

## Test Matrix

### All Scenarios to Test

| # | Scenario | Method | Expected Provider | Test Status |
|---|----------|--------|-------------------|-------------|
| 1 | Health check | curl | - | ⏳ |
| 2 | Token counting | curl | - | ⏳ |
| 3 | Authentication | curl | - | ⏳ |
| 4 | Basic chat | Claude Code | Default | ⏳ |
| 5 | Code generation | Claude Code | Default | ⏳ |
| 6 | File operations | Claude Code | Default | ⏳ |
| 7 | Streaming | Claude Code | Default | ⏳ |
| 8 | Error handling | curl (bad key) | - | ⏳ |
| 9 | Multi-turn | Claude Code | Default | ⏳ |

---

## Logging and Monitoring

### Key Log Messages to Watch For

**Successful Startup:**
```
INFO Starting Terraphim LLM Proxy v0.1.0
INFO Configuration validated successfully
INFO ✓ Terraphim LLM Proxy is running on http://127.0.0.1:3456
INFO Ready to accept connections
```

**Successful Request:**
```
INFO Request analyzed: token_count=150, is_background=false...
INFO Routing decision: provider=anthropic, model=claude-3-5-sonnet-20241022, scenario=Default
INFO Request completed successfully: input_tokens=150, output_tokens=45
```

**Expected Errors (for negative tests):**
```
WARN Invalid API key attempt
WARN Request warning error=InvalidApiKey
```

---

## Deliverables After Testing

1. **E2E_TEST_RESULTS.md** - Complete test execution results
2. **TEST_EXECUTION_LOG.md** - Updated with all test outcomes
3. **Performance report** - Latency and throughput measurements
4. **Issues list** - Any bugs found (if applicable)
5. **Updated PROGRESS.md** - Reflect completion status
6. **Production readiness decision** - Go/no-go recommendation

---

## Risk Mitigation

**If Claude Code doesn't support --settings flag properly:**
- Test with environment variables instead
- Document workaround in findings

**If providers don't have API keys:**
- Test with Ollama only (free, local)
- Document which scenarios tested
- Mark provider-specific tests as skipped

**If critical issues found:**
- Document with severity
- Fix critical bugs immediately
- Retest after fixes
- Update timeline if needed

---

**Status:** Ready to execute | All prerequisites met | Environment configured with existing credentials
