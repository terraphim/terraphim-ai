#!/bin/bash
# Demonstrate intelligent routing through the proxy

echo "=== Terraphim LLM Proxy - Intelligent Routing Demonstration ==="
echo ""

# Test 1: Default routing (should go to OpenRouter Claude Sonnet)
echo "Test 1: Default Routing (General query)"
echo "Query: 'What is the capital of France?'"
curl -s -X POST http://127.0.0.1:3456/v1/messages \
  -H "Content-Type: application/json" \
  -H "x-api-key: sk_test_proxy_key_for_claude_code_testing_12345" \
  -d '{
    "model": "claude-3-5-sonnet-20241022",
    "max_tokens": 100,
    "messages": [{"role": "user", "content": "What is the capital of France?"}]
  }' | jq -r '.content[0].text' 2>/dev/null || echo "Request failed"
echo ""

# Test 2: Background task routing (should go to Ollama)
echo "Test 2: Background Task Routing (Pattern: 'background task')"
echo "Query: 'Run this background task to process data'"
curl -s -X POST http://127.0.0.1:3456/v1/messages \
  -H "Content-Type: application/json" \
  -H "x-api-key: sk_test_proxy_key_for_claude_code_testing_12345" \
  -d '{
    "model": "claude-3-5-sonnet-20241022",
    "max_tokens": 100,
    "messages": [{"role": "user", "content": "Run this background task to process data"}]
  }' | jq -r '.content[0].text' 2>/dev/null || echo "Request failed"
echo ""

# Test 3: Thinking/reasoning routing (should go to OpenRouter DeepSeek)
echo "Test 3: Thinking/Reasoning Routing (Pattern: 'plan mode')"
echo "Query: 'I need to enter plan mode to think deeply about this architecture'"
curl -s -X POST http://127.0.0.1:3456/v1/messages \
  -H "Content-Type: application/json" \
  -H "x-api-key: sk_test_proxy_key_for_claude_code_testing_12345" \
  -d '{
    "model": "claude-3-5-sonnet-20241022",
    "max_tokens": 100,
    "messages": [{"role": "user", "content": "I need to enter plan mode to think deeply about this architecture"}]
  }' | jq -r '.content[0].text' 2>/dev/null || echo "Request failed"
echo ""

# Test 4: Web search routing (should go to OpenRouter Perplexity)
echo "Test 4: Web Search Routing (Pattern: 'web search')"
echo "Query: 'Search the web for latest Rust news'"
curl -s -X POST http://127.0.0.1:3456/v1/messages \
  -H "Content-Type: application/json" \
  -H "x-api-key: sk_test_proxy_key_for_claude_code_testing_12345" \
  -d '{
    "model": "claude-3-5-sonnet-20241022",
    "max_tokens": 100,
    "messages": [{"role": "user", "content": "Search the web for latest Rust news"}]
  }' | jq -r '.content[0].text' 2>/dev/null || echo "Request failed"
echo ""

echo "=== Check proxy logs for routing decisions ==="
echo "Run: tmux attach -t proxy"
echo ""
