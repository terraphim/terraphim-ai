#!/bin/bash

# Ollama Llama Integration Test Runner
# Tests LLM integration using local Ollama instance with llama3.2:3b model

set -e

echo "🚀 Starting Ollama Llama Integration Tests"
echo "=========================================="

# Check if Ollama is running
echo "🔍 Checking Ollama connectivity..."
if ! curl -s --connect-timeout 5 "http://127.0.0.1:11434/api/tags" > /dev/null; then
    echo "❌ Ollama is not running on http://127.0.0.1:11434"
    echo "   Please start Ollama first:"
    echo "   ollama serve"
    echo ""
    echo "   Then pull the llama3.2:3b model:"
    echo "   ollama pull llama3.2:3b"
    exit 1
fi

echo "✅ Ollama is running"

# Check if llama3.2:3b model is available
echo "🔍 Checking for llama3.2:3b model..."
if ! ollama list | grep -q "llama3.2:3b"; then
    echo "⚠️  llama3.2:3b model not found"
    echo "   Pulling llama3.2:3b model (this may take a while)..."
    ollama pull llama3.2:3b
fi

echo "✅ llama3.2:3b model is available"

# Set environment variables
export OLLAMA_BASE_URL="http://127.0.0.1:11434"
export RUST_LOG="info"

echo "🧪 Running Ollama Llama integration tests..."
echo "   Base URL: $OLLAMA_BASE_URL"
echo "   Model: llama3.2:3b"
echo ""

# Run the comprehensive integration test
echo "📋 Running comprehensive integration test..."
cargo test --features ollama ollama_llama_integration_comprehensive -- --nocapture

echo ""
echo "📏 Running length constraint test..."
cargo test --features ollama ollama_llama_length_constraint_test -- --nocapture

echo ""
echo "📊 Running performance test..."
cargo test --features ollama ollama_llama_performance_test -- --nocapture

echo ""
echo "🎯 Running all Ollama-related tests..."
cargo test --features ollama ollama -- --nocapture

echo ""
echo "✅ All Ollama Llama integration tests completed successfully!"
echo ""
echo "📝 Test Summary:"
echo "   - Connectivity: ✅ Ollama instance reachable"
echo "   - Model: ✅ llama3.2:3b available"
echo "   - Integration: ✅ LLM client functionality"
echo "   - Role Config: ✅ Role-based configuration"
echo "   - E2E Search: ✅ Search with auto-summarization"
echo "   - Performance: ✅ Multi-request reliability"
echo ""
echo "🚀 Ollama LLM integration is ready for production use!"
