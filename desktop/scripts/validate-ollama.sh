#!/bin/bash

# Ollama Model Validation Script
# This script validates that Ollama is properly configured and working
# for the Terraphim AI test suite

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Configuration
OLLAMA_BASE_URL="http://127.0.0.1:11434"
REQUIRED_MODELS=("llama3.2:3b" "llama3:8b")
TEST_PROMPTS=(
    "What is Rust programming language?"
    "Explain async/await in one sentence."
    "List three benefits of WebAssembly."
)

print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

echo -e "${BLUE}🤖 Ollama Model Validation${NC}"
echo "=================================================="

# Check if Ollama is installed
if ! command -v ollama >/dev/null 2>&1; then
    print_error "Ollama is not installed"
    echo "Please install from: https://ollama.ai/"
    exit 1
fi

print_success "Ollama is installed"

# Check if Ollama service is running
print_status "Checking Ollama service..."
if ! curl -s "$OLLAMA_BASE_URL/api/tags" >/dev/null 2>&1; then
    print_error "Ollama service is not running"
    echo "Please start Ollama service: ollama serve"
    exit 1
fi

print_success "Ollama service is running"

# Get available models
print_status "Checking available models..."
AVAILABLE_MODELS=$(curl -s "$OLLAMA_BASE_URL/api/tags" | jq -r '.models[].name' 2>/dev/null || echo "")

if [ -z "$AVAILABLE_MODELS" ]; then
    # Try without jq
    AVAILABLE_MODELS=$(curl -s "$OLLAMA_BASE_URL/api/tags" | grep -o '"name":"[^"]*"' | cut -d':' -f2 | tr -d '"' || echo "")
fi

if [ -z "$AVAILABLE_MODELS" ]; then
    print_error "Could not retrieve model list"
    exit 1
fi

echo "Available models:"
echo "$AVAILABLE_MODELS" | while read -r model; do
    [ -n "$model" ] && echo "  • $model"
done

# Check required models
echo ""
print_status "Validating required models..."

MISSING_MODELS=()
PRIMARY_MODEL=""

for model in "${REQUIRED_MODELS[@]}"; do
    if echo "$AVAILABLE_MODELS" | grep -q "$model"; then
        print_success "Model $model is available"
        if [ -z "$PRIMARY_MODEL" ]; then
            PRIMARY_MODEL="$model"
        fi
    else
        print_warning "Model $model is not available"
        MISSING_MODELS+=("$model")
    fi
done

# Pull missing models
if [ ${#MISSING_MODELS[@]} -gt 0 ]; then
    echo ""
    print_status "Pulling missing models..."

    for model in "${MISSING_MODELS[@]}"; do
        print_status "Pulling $model (this may take several minutes)..."

        if ollama pull "$model"; then
            print_success "Successfully pulled $model"
            if [ -z "$PRIMARY_MODEL" ]; then
                PRIMARY_MODEL="$model"
            fi
        else
            print_error "Failed to pull $model"
        fi
    done
fi

if [ -z "$PRIMARY_MODEL" ]; then
    print_error "No suitable model available for testing"
    echo "Please pull at least one model: ollama pull llama3.2:3b"
    exit 1
fi

print_success "Using primary model: $PRIMARY_MODEL"

# Test model functionality
echo ""
print_status "Testing model functionality..."

for i in "${!TEST_PROMPTS[@]}"; do
    prompt="${TEST_PROMPTS[$i]}"
    test_num=$((i + 1))

    print_status "Test $test_num: Testing prompt generation..."
    echo "Prompt: $prompt"

    # Create test request
    REQUEST_JSON=$(cat << EOF
{
    "model": "$PRIMARY_MODEL",
    "prompt": "$prompt",
    "stream": false,
    "options": {
        "num_predict": 50,
        "temperature": 0.7,
        "top_p": 0.9
    }
}
EOF
)

    # Send request and measure time
    start_time=$(date +%s%N)

    RESPONSE=$(curl -s -X POST "$OLLAMA_BASE_URL/api/generate" \
        -H "Content-Type: application/json" \
        -d "$REQUEST_JSON")

    end_time=$(date +%s%N)
    duration_ms=$(( (end_time - start_time) / 1000000 ))

    # Parse response
    if echo "$RESPONSE" | grep -q '"response"'; then
        # Extract response text
        RESPONSE_TEXT=$(echo "$RESPONSE" | jq -r '.response' 2>/dev/null || \
                       echo "$RESPONSE" | grep -o '"response":"[^"]*"' | cut -d':' -f2 | tr -d '"')

        if [ -n "$RESPONSE_TEXT" ] && [ "$RESPONSE_TEXT" != "null" ]; then
            print_success "Test $test_num passed (${duration_ms}ms)"
            echo "Response: ${RESPONSE_TEXT:0:100}..."

            # Check response quality
            word_count=$(echo "$RESPONSE_TEXT" | wc -w)
            if [ "$word_count" -gt 5 ]; then
                echo "  ✓ Response quality: Good ($word_count words)"
            else
                print_warning "  ⚠ Response quality: Short ($word_count words)"
            fi

            # Check response time
            if [ "$duration_ms" -lt 10000 ]; then
                echo "  ✓ Response time: Fast (${duration_ms}ms)"
            elif [ "$duration_ms" -lt 30000 ]; then
                echo "  ✓ Response time: Acceptable (${duration_ms}ms)"
            else
                print_warning "  ⚠ Response time: Slow (${duration_ms}ms)"
            fi
        else
            print_error "Test $test_num failed: Empty response"
        fi
    else
        print_error "Test $test_num failed: Invalid response format"
        echo "Response: $RESPONSE"
    fi

    echo ""
done

# Test streaming capability
print_status "Testing streaming capability..."

STREAM_REQUEST=$(cat << 'EOF'
{
    "model": "PRIMARY_MODEL_PLACEHOLDER",
    "prompt": "Explain the concept of ownership in Rust programming language.",
    "stream": true,
    "options": {
        "num_predict": 30
    }
}
EOF
)

STREAM_REQUEST=$(echo "$STREAM_REQUEST" | sed "s/PRIMARY_MODEL_PLACEHOLDER/$PRIMARY_MODEL/")

start_time=$(date +%s%N)
STREAM_RESPONSE=$(curl -s -X POST "$OLLAMA_BASE_URL/api/generate" \
    -H "Content-Type: application/json" \
    -d "$STREAM_REQUEST")
end_time=$(date +%s%N)
stream_duration_ms=$(( (end_time - start_time) / 1000000 ))

if echo "$STREAM_RESPONSE" | grep -q '"response"'; then
    print_success "Streaming test passed (${stream_duration_ms}ms)"

    # Count streaming chunks
    chunk_count=$(echo "$STREAM_RESPONSE" | grep -c '"response"' || echo "1")
    echo "  ✓ Streaming chunks: $chunk_count"
else
    print_warning "Streaming test failed or not supported"
fi

# Performance summary
echo ""
echo "📊 Performance Summary"
echo "=================================================="
echo "Primary model: $PRIMARY_MODEL"
echo "Service endpoint: $OLLAMA_BASE_URL"

# Calculate average response time from tests
if [ ${#TEST_PROMPTS[@]} -gt 0 ]; then
    echo "Test results: ${#TEST_PROMPTS[@]} prompts tested"
    echo "Streaming support: $(echo "$STREAM_RESPONSE" | grep -q '"response"' && echo "Yes" || echo "No")"
fi

# Memory usage check
if command -v ps >/dev/null 2>&1; then
    OLLAMA_MEMORY=$(ps aux | grep '[o]llama' | awk '{sum+=$6} END {print sum/1024}' 2>/dev/null || echo "unknown")
    if [ "$OLLAMA_MEMORY" != "unknown" ] && [ "${OLLAMA_MEMORY%.*}" -gt 0 ]; then
        echo "Ollama memory usage: ${OLLAMA_MEMORY%.*} MB"
    fi
fi

# Model size information
echo ""
print_status "Model information..."
MODEL_INFO=$(curl -s "$OLLAMA_BASE_URL/api/show" -d "{\"name\":\"$PRIMARY_MODEL\"}" 2>/dev/null || echo "")

if [ -n "$MODEL_INFO" ]; then
    # Try to extract model size
    MODEL_SIZE=$(echo "$MODEL_INFO" | jq -r '.details.parameter_size // empty' 2>/dev/null || echo "")
    if [ -n "$MODEL_SIZE" ]; then
        echo "Model size: $MODEL_SIZE"
    fi

    # Try to extract quantization
    QUANTIZATION=$(echo "$MODEL_INFO" | jq -r '.details.quantization_level // empty' 2>/dev/null || echo "")
    if [ -n "$QUANTIZATION" ]; then
        echo "Quantization: $QUANTIZATION"
    fi
fi

# Health check endpoint
echo ""
print_status "Health check..."
HEALTH_RESPONSE=$(curl -s "$OLLAMA_BASE_URL/api/tags" | head -c 100)
if [ -n "$HEALTH_RESPONSE" ]; then
    print_success "Health check passed"
else
    print_warning "Health check unclear"
fi

# Final validation
echo ""
echo "✅ Validation Summary"
echo "=================================================="

if [ -n "$PRIMARY_MODEL" ]; then
    print_success "Ollama is properly configured and ready for testing"
    echo ""
    echo "🎯 Configuration for tests:"
    echo "  • Model: $PRIMARY_MODEL"
    echo "  • Endpoint: $OLLAMA_BASE_URL"
    echo "  • Status: Ready"
    echo ""
    echo "📝 Environment variables to set:"
    echo "  export OLLAMA_BASE_URL=\"$OLLAMA_BASE_URL\""
    echo "  export OLLAMA_MODEL=\"$PRIMARY_MODEL\""
else
    print_error "Ollama validation failed"
    exit 1
fi

# Save validation results
cat > .ollama-validation-results << EOF
VALIDATION_DATE=$(date)
OLLAMA_BASE_URL=$OLLAMA_BASE_URL
PRIMARY_MODEL=$PRIMARY_MODEL
AVAILABLE_MODELS=$AVAILABLE_MODELS
VALIDATION_STATUS=PASSED
EOF

print_success "Validation results saved to .ollama-validation-results"
echo ""
echo -e "${GREEN}🎉 Ollama is ready for Terraphim AI testing!${NC}"
