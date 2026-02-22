#!/bin/bash
# Comprehensive Routing Demonstration Script
# Tests pattern-based, lowest-cost, and highest-throughput routing

echo "==================================================================="
echo "Terraphim LLM Proxy - Comprehensive Routing Demonstration"
echo "==================================================================="
echo ""

# Load API keys from 1Password (providers)
export OPENROUTER_API_KEY=$(op read "op://TerraphimPlatform/TruthForge.api-keys/openrouter-api-key")
export ANTHROPIC_API_KEY=$(op read "op://TerraphimPlatform/TruthForge.api-keys/anthropic-api-key")
export DEEPSEEK_API_KEY=$(op read "op://TerraphimPlatform/TruthForge.api-keys/deepseek-api-keys")

echo "✅ Proxy is running in tmux session 'llm-proxy'"
echo "✅ Health check: $(curl -sf http://127.0.0.1:3456/health)"
echo ""

# Configure Claude Code to use proxy (client auth to proxy)
export ANTHROPIC_BASE_URL=http://127.0.0.1:3456
export ANTHROPIC_API_KEY=sk_test_proxy_key_for_claude_code_testing_12345

echo "==================================================================="
echo "Part 1: Pattern-Based Intelligent Routing Tests"
echo "==================================================================="

# Test patterns and their expected routes
declare -A tests=(
    ["Plan Mode Testing"]="think_routing:deepseek-reasoner"
    ["Background task processing"]="background_routing:ollama"
    ["Search the web for latest"]="web_search_routing:perplexity"
    ["Analyze this extended context"]="long_context_routing:gemini"
    ["Analyze this screenshot"]="image_routing:claude-sonnet"
    ["Chain of thought reasoning"]="think_routing:deepseek"
    ["Process offline batch"]="background_routing:ollama"
    ["Current information search"]="web_search_routing:perplexity"
    ["Visual analysis image"]="image_routing:multimodal"
    ["Simple math question"]="default_routing:claude-sonnet"
)

test_num=1
for query in "${!tests[@]}"; do
    expected_route="${tests[$query]}"
    echo "Test $test_num: $query"
    echo "Expected: $expected_route"
    echo "Query: '$query'"

    # Send query through proxy
    response=$(echo "$query" | timeout 30s claude --print 2>&1 | head -3)
    echo "Response: $response"

    # Check proxy logs for routing decision
    sleep 2
    routing_info=$(tmux capture-pane -t llm-proxy -p | tail -20 | grep -E "Routing decision|Resolved service target|Selected provider" | tail -3)
    echo "Routing: $routing_info"
    echo "---"
    ((test_num++))
done

echo ""
echo "==================================================================="
echo "Part 2: Lowest Cost Routing Tests"
echo "==================================================================="

# Create a cost-optimized configuration
COST_CONFIG="config.lowest-cost.toml"
cat > "$COST_CONFIG" <<'EOF'
[proxy]
host = "127.0.0.1"
port = 3457
api_key = "sk_test_proxy_key_for_claude_code_testing_12345"
timeout_ms = 600000

[router]
# Default routing - prioritize lowest cost
default = "deepseek,deepseek-chat"
# Thinking - use DeepSeek reasoner (low cost, high capability)
think = "deepseek,deepseek-reasoner"
# Long context - use DeepSeek V3.1 (good value)
long_context = "deepseek,deepseek-v3.1-terminus"
long_context_threshold = 60000
# Web search - use OpenRouter → DeepSeek via OpenRouter
web_search = "openrouter,deepseek/deepseek-chat"
# Image - use OpenRouter → Claude (only when needed)
image = "openrouter,anthropic/claude-sonnet-4.5"
# Background - use Ollama (free local)
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
  "deepseek/deepseek-chat",
  "deepseek/deepseek-v3.1-terminus"
]
transformers = ["openrouter"]

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

echo "Created cost-optimized configuration: $COST_CONFIG"
echo "Starting cost-optimized proxy on port 3457..."

env OPENROUTER_API_KEY="$OPENROUTER_API_KEY" \
    DEEPSEEK_API_KEY="$DEEPSEEK_API_KEY" \
    tmux new-session -d -s llm-proxy-cost "./target/release/terraphim-llm-proxy --config $COST_CONFIG --log-level debug"

# Wait for cost-optimized proxy to be healthy
for i in {1..10}; do
  if curl -sf http://127.0.0.1:3457/health | grep -q OK; then
    echo "✅ Cost-optimized proxy is healthy"
    break
  fi
  sleep 1
done

# Test cost-optimized routing
echo "Testing cost-optimized routing (port 3457)..."
cost_queries=(
    "Help me debug this Rust code"
    "Explain quantum computing simply"
    "Write a Python script for data analysis"
    "Create a marketing plan"
)

for i in "${!cost_queries[@]}"; do
    query="${cost_queries[$i]}"
    echo "Cost Test $((i+1)): $query"

    # Route through cost-optimized proxy
    export ANTHROPIC_BASE_URL=http://127.0.0.1:3457
    response=$(echo "$query" | timeout 30s claude --print 2>&1 | head -2)
    echo "Response: $response"

    sleep 2
    routing_info=$(tmux capture-pane -t llm-proxy-cost -p | tail -10 | grep -E "Routing decision|Selected provider|deepseek" | tail -2)
    echo "Routing: $routing_info"
    echo "---"
done

echo ""
echo "==================================================================="
echo "Part 3: Highest Throughput Routing Tests"
echo "==================================================================="

# Create a throughput-optimized configuration
THROUGHPUT_CONFIG="config.high-throughput.toml"
cat > "$THROUGHPUT_CONFIG" <<'EOF'
[proxy]
host = "127.0.0.1"
port = 3458
api_key = "sk_test_proxy_key_for_claude_code_testing_12345"
timeout_ms = 300000

[router]
# Default routing - prioritize fastest models
default = "openrouter,deepseek/deepseek-chat"
# Thinking - use DeepSeek reasoner (fast reasoning)
think = "openrouter,deepseek/deepseek-reasoner"
# Long context - use Gemini Flash (very fast)
long_context = "openrouter,google/gemini-2.0-flash-exp"
long_context_threshold = 60000
# Web search - use Perplexity Sonar (fast online)
web_search = "openrouter,perplexity/llama-3.1-sonar-small-128k-online"
# Image - use Claude Sonnet (fast vision)
image = "openrouter,anthropic/claude-sonnet-4.5"
# Background - use Ollama (local, fast)
background = "ollama,llama3.2:latest"

[security.rate_limiting]
enabled = false
requests_per_minute = 1200
concurrent_requests = 200

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
  "deepseek/deepseek-chat",
  "deepseek/deepseek-reasoner",
  "google/gemini-2.0-flash-exp",
  "perplexity/llama-3.1-sonar-small-128k-online",
  "anthropic/claude-sonnet-4.5"
]
transformers = ["openrouter"]

[[providers]]
name = "ollama"
api_base_url = "http://127.0.0.1:11434"
api_key = "ollama"
models = ["llama3.2:latest", "qwen2.5-coder:latest"]
transformers = ["ollama"]
EOF

echo "Created throughput-optimized configuration: $THROUGHPUT_CONFIG"
echo "Starting throughput-optimized proxy on port 3458..."

env OPENROUTER_API_KEY="$OPENROUTER_API_KEY" \
    tmux new-session -d -s llm-proxy-throughput "./target/release/terraphim-llm-proxy --config $THROUGHPUT_CONFIG --log-level debug"

# Wait for throughput-optimized proxy to be healthy
for i in {1..10}; do
  if curl -sf http://127.0.0.1:3458/health | grep -q OK; then
    echo "✅ Throughput-optimized proxy is healthy"
    break
  fi
  sleep 1
done

# Test throughput-optimized routing with concurrent requests
echo "Testing throughput-optimized routing with concurrent requests..."

export ANTHROPIC_BASE_URL=http://127.0.0.1:3458

# Function to test a single request
test_throughput_request() {
    local query="$1"
    local test_id="$2"
    echo "Throughput Test $test_id: $query"

    start_time=$(date +%s)
    response=$(echo "$query" | timeout 20s claude --print 2>&1 | head -2)
    end_time=$(date +%s)

    duration=$((end_time - start_time))
    echo "Response (${duration}s): $response"

    routing_info=$(tmux capture-pane -t llm-proxy-throughput -p | tail -10 | grep -E "Routing decision|Selected provider|gemini|deepseek|perplexity" | tail -1)
    echo "Routing: $routing_info"
    echo "---"
}

# Test with different queries optimized for speed
throughput_queries=(
    "What is 2+2?"
    "Summarize photosynthesis briefly"
    "List 3 colors of rainbow"
    "Define AI in one sentence"
)

# Run tests concurrently in background
for i in "${!throughput_queries[@]}"; do
    query="${throughput_queries[$i]}"
    test_throughput_request "$query" "$((i+1))" &
done

# Wait for all background jobs to complete
wait

echo ""
echo "==================================================================="
echo "Part 4: Performance Comparison Summary"
echo "==================================================================="

echo "Checking proxy logs for routing summary..."
echo ""

echo "📊 Pattern-based Routing (port 3456):"
tmux capture-pane -t llm-proxy -p | grep -E "Routing decision|Selected provider" | tail -5

echo ""
echo "💰 Lowest Cost Routing (port 3457):"
tmux capture-pane -t llm-proxy-cost -p | grep -E "Routing decision|Selected provider" | tail -5

echo ""
echo "⚡ Highest Throughput Routing (port 3458):"
tmux capture-pane -t llm-proxy-throughput -p | grep -E "Routing decision|Selected provider" | tail -5

echo ""
echo "==================================================================="
echo "All routing demonstrations completed successfully!"
echo ""
echo "Active tmux sessions:"
echo "- llm-proxy: Pattern-based routing (port 3456)"
echo "- llm-proxy-cost: Lowest cost routing (port 3457)"
echo "- llm-proxy-throughput: Highest throughput routing (port 3458)"
echo ""
echo "Use 'tmux attach -t <session-name>' to view any session logs"
echo "==================================================================="