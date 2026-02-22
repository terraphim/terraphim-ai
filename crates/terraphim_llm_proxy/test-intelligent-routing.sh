#!/bin/bash
# Intelligent Routing Demonstration Script
# Tests 10 queries matching different taxonomy patterns

echo "==================================================================="
echo "Terraphim LLM Proxy - Intelligent Routing Demonstration"
echo "==================================================================="
echo ""
echo "Building and starting llm_proxy..."
echo ""

# Build the proxy
cargo build --release

# Load API keys from 1Password (providers)
export OPENROUTER_API_KEY=$(op read "op://TerraphimPlatform/TruthForge.api-keys/openrouter-api-key")
export ANTHROPIC_API_KEY=$(op read "op://TerraphimPlatform/TruthForge.api-keys/anthropic-api-key")
export DEEPSEEK_API_KEY=$(op read "op://TerraphimPlatform/TruthForge.api-keys/deepseek-api-keys")

# Create a temporary multi-provider config to demonstrate routing across providers
CONFIG_FILE="config.multi-provider.tmp.toml"
cat > "$CONFIG_FILE" <<'EOF'
[proxy]
host = "127.0.0.1"
port = 3456
api_key = "sk_test_proxy_key_for_claude_code_testing_12345"
timeout_ms = 600000

[router]
# Default routing
default = "openrouter,anthropic/claude-sonnet-4.5"
# Thinking/reasoning: route directly to DeepSeek provider
think = "deepseek,deepseek-reasoner"
# Long context via OpenRouter → Gemini
long_context = "openrouter,google/gemini-2.5-flash-preview-09-2025"
long_context_threshold = 60000
# Web search via OpenRouter → Perplexity
web_search = "openrouter,perplexity/llama-3.1-sonar-large-128k-online"
# Image via OpenRouter → Anthropic
image = "openrouter,anthropic/claude-sonnet-4.5"
# Background via Ollama (optional)
background = "ollama,qwen2.5-coder:latest"

[security.rate_limiting]
enabled = false
requests_per_minute = 600
concurrent_requests = 100

[security.ssrf_protection]
enabled = true
allow_localhost = true
allow_private_ips = true

# Providers
[[providers]]
name = "openrouter"
api_base_url = "https://openrouter.ai/api/v1"
api_key = "$OPENROUTER_API_KEY"
models = [
  "anthropic/claude-sonnet-4.5",
  "google/gemini-2.5-flash-preview-09-2025",
  "deepseek/deepseek-v3.1-terminus",
  "perplexity/llama-3.1-sonar-large-128k-online"
]
transformers = ["openrouter"]

[[providers]]
name = "anthropic"
api_base_url = "https://api.anthropic.com"
api_key = "$ANTHROPIC_API_KEY"
models = ["claude-3-5-sonnet-20241022", "claude-3-5-haiku-20241022"]
transformers = ["anthropic"]

[[providers]]
name = "deepseek"
api_base_url = "https://api.deepseek.com/chat/completions"
api_key = "$DEEPSEEK_API_KEY"
models = ["deepseek-chat", "deepseek-reasoner"]
transformers = ["deepseek"]

[[providers]]
name = "ollama"
api_base_url = "http://127.0.0.1:11434"
api_key = "ollama"
models = ["qwen2.5-coder:latest", "llama3.2:latest"]
transformers = ["ollama"]
EOF

# Start the proxy in background with debug logging to file
LOG_FILE="proxy.log"
echo "Starting llm_proxy with $CONFIG_FILE (debug logs → $LOG_FILE)..."
env OPENROUTER_API_KEY="$OPENROUTER_API_KEY" \
    ANTHROPIC_API_KEY="$ANTHROPIC_API_KEY" \
    DEEPSEEK_API_KEY="$DEEPSEEK_API_KEY" \
    ./target/release/terraphim-llm-proxy --config "$CONFIG_FILE" --log-level debug > "$LOG_FILE" 2>&1 &
PROXY_PID=$!

# Wait for proxy health
for i in {1..20}; do
  if curl -sf http://127.0.0.1:3456/health | grep -q OK; then
    echo "Proxy is healthy."
    break
  fi
  sleep 1
  if [ "$i" -eq 20 ]; then
    echo "Proxy failed to become healthy. See $LOG_FILE"
    exit 1
  fi
done

# Configure Claude Code to use proxy (client auth to proxy)
export ANTHROPIC_BASE_URL=http://127.0.0.1:3456
export ANTHROPIC_API_KEY=sk_test_proxy_key_for_claude_code_testing_12345

echo "Testing 10 queries to demonstrate pattern-based routing..."
echo ""

# Test 1: Plan mode / Think routing
echo "Test 1: Plan Mode (should route to think_routing → deepseek-reasoner)"
echo "Query: 'I need to enter plan mode to think deeply about this architecture'"
echo "I need to enter plan mode to think deeply about this architecture" | claude --print 2>&1 | head -3
# Evidence: show routing decision and resolved target from logs
grep -E "Routing decision \(streaming\): using provider|Resolved service target" "$LOG_FILE" | tail -2 || true
echo ""
sleep 2

# Test 2: Background task
echo "Test 2: Background Task (should route to background_routing → ollama/qwen)"
echo "Query: 'Run this as a background task'"
echo "Run this as a background task" | claude --print 2>&1 | head -3
grep -E "Routing decision \(streaming\): using provider|Resolved service target" "$LOG_FILE" | tail -2 || true
echo ""
sleep 2

# Test 3: Web search
echo "Test 3: Web Search (should route to web_search_routing → perplexity)"
echo "Query: 'Search the web for latest Rust news'"
echo "Search the web for latest Rust news" | claude --print 2>&1 | head -3
grep -E "Routing decision \(streaming\): using provider|Resolved service target" "$LOG_FILE" | tail -2 || true
echo ""
sleep 2

# Test 4: Long context
echo "Test 4: Long Context (should route to long_context_routing → gemini)"
echo "Query: 'Analyze this extended context with high token count'"
echo "Analyze this extended context with high token count" | claude --print 2>&1 | head -3
grep -E "Routing decision \(streaming\): using provider|Resolved service target" "$LOG_FILE" | tail -2 || true
echo ""
sleep 2

# Test 5: Image analysis
echo "Test 5: Image Analysis (should route to image_routing → claude-sonnet)"
echo "Query: 'Analyze this screenshot with vision capabilities'"
echo "Analyze this screenshot with vision capabilities" | claude --print 2>&1 | head -3
grep -E "Routing decision \(streaming\): using provider|Resolved service target" "$LOG_FILE" | tail -2 || true
echo ""
sleep 2

# Test 6: Chain of thought
echo "Test 6: Chain of Thought (should route to think_routing → deepseek)"
echo "Query: 'Use chain-of-thought reasoning for this problem'"
echo "Use chain-of-thought reasoning for this problem" | claude --print 2>&1 | head -3
grep -E "Routing decision \(streaming\): using provider|Resolved service target" "$LOG_FILE" | tail -2 || true
echo ""
sleep 2

# Test 7: Offline processing
echo "Test 7: Offline Processing (should route to background_routing → ollama)"
echo "Query: 'Process this offline in batch mode'"
echo "Process this offline in batch mode" | claude --print 2>&1 | head -3
grep -E "Routing decision \(streaming\): using provider|Resolved service target" "$LOG_FILE" | tail -2 || true
echo ""
sleep 2

# Test 8: Online search
echo "Test 8: Online Search (should route to web_search_routing → perplexity)"
echo "Query: 'Find current information with internet search'"
echo "Find current information with internet search" | claude --print 2>&1 | head -3
grep -E "Routing decision \(streaming\): using provider|Resolved service target" "$LOG_FILE" | tail -2 || true
echo ""
sleep 2

# Test 9: Visual analysis
echo "Test 9: Visual Analysis (should route to image_routing → multimodal)"
echo "Query: 'Perform visual analysis on this image'"
echo "Perform visual analysis on this image" | claude --print 2>&1 | head -3
grep -E "Routing decision \(streaming\): using provider|Resolved service target" "$LOG_FILE" | tail -2 || true
echo ""
sleep 2

# Test 10: Default routing
echo "Test 10: Default (should route to default_routing → claude-sonnet)"
echo "Query: 'What is 2+2?'"
echo "What is 2+2?" | claude --print 2>&1 | head -3
grep -E "Routing decision \(streaming\): using provider|Resolved service target" "$LOG_FILE" | tail -2 || true
echo ""

# Test 11: Anthropic routing demonstration
echo "Test 11: Anthropic (should route to anthropic → claude-sonnet)"
echo "Query: 'Simple question for Claude'"
echo "Simple question for Claude" | claude --print 2>&1 | head -3
grep -E "Routing decision \(streaming\): using provider|Resolved service target" "$LOG_FILE" | tail -2 || true
echo ""
sleep 2

# Test 12: DeepSeek routing demonstration
echo "Test 12: DeepSeek (should route to deepseek → deepseek-reasoner)"
echo "Query: 'Use deep reasoning for this complex problem'"
echo "Use deep reasoning for this complex problem" | claude --print 2>&1 | head -3
grep -E "Routing decision \(streaming\): using provider|Resolved service target" "$LOG_FILE" | tail -2 || true
echo ""
sleep 2

# Test 13: OpenRouter routing demonstration
echo "Test 13: OpenRouter (should route to openrouter → perplexity)"
echo "Query: 'Search for latest AI news'"
echo "Search for latest AI news" | claude --print 2>&1 | head -3
grep -E "Routing decision \(streaming\): using provider|Resolved service target" "$LOG_FILE" | tail -2 || true
echo ""

echo "==================================================================="
echo "All 13 routing tests complete!"
echo "Check proxy logs for routing decisions"
echo "==================================================================="
echo ""
echo "Stopping proxy..."
kill $PROXY_PID
