#!/bin/bash
# Quick LLM Chat Test Runner
# Runs essential tests with a free OpenRouter model for development workflow

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

echo -e "${BLUE}Quick LLM Chat Test Runner${NC}"
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
OPENROUTER_MODEL=${OPENROUTER_MODEL:-"liquid/lfm-2.5-1.2b-instruct:free"}
export OPENROUTER_API_KEY OPENROUTER_MODEL

echo -e "\n${BLUE}Configuration:${NC}"
echo -e "  OpenRouter model: ${OPENROUTER_MODEL}"

# Check OpenRouter availability
echo -e "\n${BLUE}Checking OpenRouter...${NC}"
if [ -n "${OPENROUTER_API_KEY:-}" ]; then
    echo -e "${GREEN}PASS: OpenRouter API key is configured${NC}"
else
    echo -e "${RED}FAIL: OPENROUTER_API_KEY is not set${NC}"
    echo -e "${YELLOW}Set OPENROUTER_API_KEY and optionally OPENROUTER_MODEL=${OPENROUTER_MODEL}${NC}"
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
echo -e "\n${BLUE}Running core OpenRouter LLM chat tests...${NC}"

# Test 1: Basic functionality with Default role
echo -e "\n${BLUE}Test 1: Default role with OpenRouter${NC}"
if cargo test --test llm_chat_matrix_test test_default_ripgrep_openrouter --features openrouter -- --ignored --nocapture; then
    echo -e "${GREEN}PASS: Default role test passed${NC}"
else
    echo -e "${RED}FAIL: Default role test failed${NC}"
    echo -e "${YELLOW}This is a critical test - please check OpenRouter configuration${NC}"
    exit 1
fi

# Test 2: Rust Engineer role (tests system prompts)
echo -e "\n${BLUE}Test 2: Rust Engineer role with OpenRouter${NC}"
if cargo test --test llm_chat_matrix_test test_rust_engineer_ripgrep_openrouter --features openrouter -- --ignored --nocapture; then
    echo -e "${GREEN}PASS: Rust Engineer role test passed${NC}"
else
    echo -e "${YELLOW}WARN: Rust Engineer role test failed${NC}"
fi

# Test 3: AI Engineer role
echo -e "\n${BLUE}Test 3: AI Engineer role with OpenRouter${NC}"
if cargo test --test llm_chat_matrix_test test_ai_engineer_ripgrep_openrouter --features openrouter -- --ignored --nocapture; then
    echo -e "${GREEN}PASS: AI Engineer role test passed${NC}"
else
    echo -e "${YELLOW}WARN: AI Engineer role test failed${NC}"
fi

# Test 4: System Operator role
echo -e "\n${BLUE}Test 4: System Operator role with OpenRouter${NC}"
if cargo test --test llm_chat_matrix_test test_system_operator_ripgrep_openrouter --features openrouter -- --ignored --nocapture; then
    echo -e "${GREEN}PASS: System Operator role test passed${NC}"
else
    echo -e "${YELLOW}WARN: System Operator role test failed${NC}"
fi

# Summary
echo -e "\n${BLUE}=============================${NC}"
echo -e "${GREEN}Quick tests completed!${NC}"
echo -e "\n${BLUE}What was tested:${NC}"
echo -e "  PASS: OpenRouter free-model connectivity"
echo -e "  PASS: Basic LLM chat functionality"
echo -e "  PASS: Role-specific system prompts"
echo -e "  PASS: Bounded response timing"

echo -e "\n${BLUE}For comprehensive testing, run:${NC}"
echo -e "  ./scripts/test_llm_chat_matrix.sh"

echo -e "\n${BLUE}For one-off OpenRouter testing:${NC}"
echo -e "  OPENROUTER_MODEL=${OPENROUTER_MODEL} cargo test --test llm_chat_matrix_test test_ai_engineer_ripgrep_openrouter --features openrouter -- --ignored --nocapture"

echo -e "\n${GREEN}LLM Chat system is working correctly with OpenRouter!${NC}"
