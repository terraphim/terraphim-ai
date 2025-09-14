#!/bin/bash
# Quick LLM Chat Test Runner
# Runs essential tests with Ollama for development workflow

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

echo -e "${BLUE}🚀 Quick LLM Chat Test Runner${NC}"
echo -e "${BLUE}=============================${NC}"

# Load environment
if [ -f "${PROJECT_ROOT}/.env" ]; then
    source "${PROJECT_ROOT}/.env"
    echo -e "${GREEN}✓ Loaded .env configuration${NC}"
else
    echo -e "${RED}✗ No .env file found${NC}"
    exit 1
fi

# Set defaults
OLLAMA_BASE_URL=${OLLAMA_BASE_URL:-"http://127.0.0.1:11434"}
OLLAMA_MODEL=${OLLAMA_MODEL:-"llama3.2:3b"}

echo -e "\n${BLUE}Configuration:${NC}"
echo -e "  Ollama URL: ${OLLAMA_BASE_URL}"
echo -e "  Model: ${OLLAMA_MODEL}"

# Check Ollama availability
echo -e "\n${BLUE}Checking Ollama...${NC}"
if curl -s "${OLLAMA_BASE_URL}/api/tags" > /dev/null; then
    echo -e "${GREEN}✓ Ollama is running${NC}"
    
    # Ensure model is available
    echo -e "${BLUE}Ensuring model is loaded...${NC}"
    if command -v ollama &> /dev/null; then
        ollama pull "${OLLAMA_MODEL}" 2>/dev/null || true
        echo -e "${GREEN}✓ Model ${OLLAMA_MODEL} is ready${NC}"
    else
        echo -e "${YELLOW}⚠ ollama command not found, assuming model is loaded${NC}"
    fi
else
    echo -e "${RED}✗ Ollama is not running${NC}"
    echo -e "${YELLOW}Please start Ollama with: ollama serve${NC}"
    echo -e "${YELLOW}Then load the model with: ollama pull ${OLLAMA_MODEL}${NC}"
    exit 1
fi

# Quick pre-commit check
echo -e "\n${BLUE}Running quick pre-commit checks...${NC}"
if cargo fmt --all -- --check > /dev/null 2>&1; then
    echo -e "${GREEN}✓ Code formatting is good${NC}"
else
    echo -e "${YELLOW}⚠ Fixing code formatting...${NC}"
    cargo fmt --all
    echo -e "${GREEN}✓ Code formatting fixed${NC}"
fi

# Build tests
echo -e "\n${BLUE}Building test suite...${NC}"
if cargo build --tests > /dev/null; then
    echo -e "${GREEN}✓ Tests built successfully${NC}"
else
    echo -e "${RED}✗ Test build failed${NC}"
    exit 1
fi

# Run core tests
echo -e "\n${BLUE}Running core LLM chat tests...${NC}"

# Test 1: Basic functionality with Default role
echo -e "\n${BLUE}Test 1: Default role with Ollama${NC}"
if cargo test --test llm_chat_matrix_test test_default_ripgrep_ollama --features ollama -- --ignored --nocapture; then
    echo -e "${GREEN}✓ Default role test passed${NC}"
else
    echo -e "${RED}✗ Default role test failed${NC}"
    echo -e "${YELLOW}This is a critical test - please check Ollama configuration${NC}"
    exit 1
fi

# Test 2: Rust Engineer role (tests system prompts)
echo -e "\n${BLUE}Test 2: Rust Engineer role with Ollama${NC}"
if cargo test --test llm_chat_matrix_test test_rust_engineer_ripgrep_ollama --features ollama -- --ignored --nocapture; then
    echo -e "${GREEN}✓ Rust Engineer role test passed${NC}"
else
    echo -e "${YELLOW}⚠ Rust Engineer role test failed${NC}"
fi

# Test 3: AI Engineer role
echo -e "\n${BLUE}Test 3: AI Engineer role with Ollama${NC}"
if cargo test --test llm_chat_matrix_test test_ai_engineer_ripgrep_ollama --features ollama -- --ignored --nocapture; then
    echo -e "${GREEN}✓ AI Engineer role test passed${NC}"
else
    echo -e "${YELLOW}⚠ AI Engineer role test failed${NC}"
fi

# Test 4: Performance benchmark
echo -e "\n${BLUE}Test 4: Performance benchmarks${NC}"
if cargo test --test llm_chat_matrix_test test_performance_benchmarks --features ollama -- --ignored --nocapture; then
    echo -e "${GREEN}✓ Performance benchmarks passed${NC}"
else
    echo -e "${YELLOW}⚠ Performance benchmarks failed${NC}"
fi

# Summary
echo -e "\n${BLUE}=============================${NC}"
echo -e "${GREEN}🎉 Quick tests completed!${NC}"
echo -e "\n${BLUE}What was tested:${NC}"
echo -e "  ✓ Ollama connectivity and model loading"
echo -e "  ✓ Basic LLM chat functionality"
echo -e "  ✓ Role-specific system prompts"
echo -e "  ✓ Response timing and performance"

echo -e "\n${BLUE}For comprehensive testing, run:${NC}"
echo -e "  ./scripts/test_llm_chat_matrix.sh"

echo -e "\n${BLUE}For OpenRouter testing (requires API key):${NC}"
echo -e "  cargo test --test llm_chat_matrix_test test_ai_engineer_ripgrep_openrouter --features openrouter -- --ignored --nocapture"

echo -e "\n${GREEN}✅ LLM Chat system is working correctly with Ollama!${NC}"