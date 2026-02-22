#!/bin/bash
# Demonstrate routing logic by showing log analysis

echo "=== Terraphim LLM Proxy - Routing Logic Demonstration ==="
echo ""

# Clear previous logs
tmux send-keys -t proxy 'clear' C-m

echo "Testing different routing scenarios..."
echo ""

# Test 1: Default routing
echo "Test 1: Default/General Query"
echo "Query: 'Explain quantum computing'"
curl -s -X POST http://127.0.0.1:3456/v1/messages \
  -H "Content-Type: application/json" \
  -H "x-api-key: sk_test_proxy_key_for_claude_code_testing_12345" \
  -d '{
    "model": "claude-3-5-sonnet-20241022",
    "max_tokens": 50,
    "messages": [{"role": "user", "content": "Explain quantum computing"}]
  }' >/dev/null 2>&1
echo "✓ Request sent"
sleep 1

# Test 2: Background task (would route to Ollama if RoleGraph worked)
echo "Test 2: Background Task (Pattern matching)"
echo "Query: 'Process this data in background mode'"
curl -s -X POST http://127.0.0.1:3456/v1/messages \
  -H "Content-Type: application/json" \
  -H "x-api-key: sk_test_proxy_key_for_claude_code_testing_12345" \
  -d '{
    "model": "claude-3-5-sonnet-20241022",
    "max_tokens": 50,
    "messages": [{"role": "user", "content": "Process this data in background mode"}]
  }' >/dev/null 2>&1
echo "✓ Request sent"
sleep 1

# Test 3: Thinking/reasoning (would route to DeepSeek if RoleGraph worked)
echo "Test 3: Complex Reasoning (Pattern matching)"
echo "Query: 'I need to plan this complex architecture carefully'"
curl -s -X POST http://127.0.0.1:3456/v1/messages \
  -H "Content-Type: application/json" \
  -H "x-api-key: sk_test_proxy_key_for_claude_code_testing_12345" \
  -d '{
    "model": "claude-3-5-sonnet-20241022",
    "max_tokens": 50,
    "messages": [{"role": "user", "content": "I need to plan this complex architecture carefully"}]
  }' >/dev/null 2>&1
echo "✓ Request sent"
sleep 1

echo ""
echo "=== Routing Decisions from Logs ==="
echo ""

# Show the routing decisions
echo "Recent routing decisions:"
tmux capture-pane -t proxy -p | grep -E "Routing decision made|Resolved routing decision" | tail -6

echo ""
echo "=== How Intelligent Routing Works ==="
echo ""
echo "1. REQUEST ANALYSIS:"
echo "   - Token count estimation"
echo "   - Content analysis (thinking, web search, images)"
echo "   - Pattern matching against RoleGraph taxonomy"
echo ""
echo "2. ROUTING DECISIONS:"
echo "   - Default: OpenRouter Claude Sonnet (balanced cost/performance)"
echo "   - Background: Ollama (lowest cost, local processing)"
echo "   - Thinking: OpenRouter DeepSeek (best for reasoning)"
echo "   - Web Search: OpenRouter Perplexity (internet access)"
echo "   - Long Context: OpenRouter Gemini (large context windows)"
echo ""
echo "3. COST OPTIMIZATION:"
echo "   - Background tasks → Ollama (free, local)"
echo "   - Simple queries → Claude Sonnet (good balance)"
echo "   - Complex reasoning → DeepSeek (cost-effective reasoning)"
echo ""
echo "4. THROUGHPUT OPTIMIZATION:"
echo "   - Fast local responses → Ollama"
echo "   - Parallel processing → Multiple providers"
echo "   - Caching → Session management"
echo ""
echo "=== Attach to proxy session to see live logs ==="
echo "Run: tmux attach -t proxy"
echo ""
echo "=== Attach to claude session for interactive testing ==="
echo "Run: tmux attach -t claude"
echo ""
