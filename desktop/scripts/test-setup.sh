#!/bin/bash

# Terraphim AI E2E Test Setup Script
# This script prepares the environment for running comprehensive e2e tests
# including Ollama setup, backend server, and required dependencies

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
OLLAMA_MODEL="llama3.2:3b"
OLLAMA_BASE_URL="http://127.0.0.1:11434"
BACKEND_CONFIG="terraphim_server/default/ollama_llama_config.json"
TEST_TIMEOUT=120
STARTUP_WAIT=10

echo -e "${BLUE}🚀 Terraphim AI E2E Test Setup${NC}"
echo "=================================================="

# Function to print status messages
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

# Function to check if a command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Function to check if a service is running on a port
port_is_open() {
    nc -z localhost "$1" >/dev/null 2>&1
}

# Function to wait for a service to be ready
wait_for_service() {
    local url=$1
    local service_name=$2
    local timeout=$3
    
    print_status "Waiting for $service_name to be ready..."
    
    for i in $(seq 1 $timeout); do
        if curl -s "$url" >/dev/null 2>&1; then
            print_success "$service_name is ready!"
            return 0
        fi
        echo -n "."
        sleep 1
    done
    
    echo ""
    print_error "$service_name failed to start within $timeout seconds"
    return 1
}

# Check prerequisites
echo "📋 Checking prerequisites..."

if ! command_exists curl; then
    print_error "curl is required but not installed"
    exit 1
fi

if ! command_exists nc; then
    print_error "nc (netcat) is required but not installed"
    exit 1
fi

if ! command_exists node; then
    print_error "Node.js is required but not installed"
    exit 1
fi

if ! command_exists cargo; then
    print_error "Rust/Cargo is required but not installed"
    exit 1
fi

print_success "Prerequisites check passed"

# Check if we're in the right directory
if [ ! -f "package.json" ] || [ ! -d "../terraphim_server" ]; then
    print_error "This script must be run from the desktop directory of the Terraphim AI project"
    exit 1
fi

print_success "Project structure verified"

# Setup environment variables
echo ""
echo "🔧 Setting up environment..."

# Copy .env.example to .env if it doesn't exist
if [ ! -f ".env" ]; then
    if [ -f ".env.example" ]; then
        cp .env.example .env
        print_success "Created .env from .env.example"
        print_warning "Please edit .env with your actual API keys and secrets"
    else
        print_warning ".env.example not found, skipping .env setup"
    fi
else
    print_status ".env file already exists"
fi

# Check Ollama installation and setup
echo ""
echo "🤖 Setting up Ollama..."

if ! command_exists ollama; then
    print_error "Ollama is not installed"
    echo "Please install Ollama from https://ollama.ai/"
    echo "Or run: curl https://ollama.ai/install.sh | sh"
    exit 1
fi

print_success "Ollama is installed"

# Check if Ollama service is running
if ! port_is_open 11434; then
    print_status "Starting Ollama service..."
    
    # Try to start Ollama in background
    ollama serve &
    OLLAMA_PID=$!
    
    # Wait for Ollama to start
    if wait_for_service "$OLLAMA_BASE_URL/api/tags" "Ollama" 30; then
        print_success "Ollama service started (PID: $OLLAMA_PID)"
        echo $OLLAMA_PID > .ollama_pid
    else
        print_error "Failed to start Ollama service"
        exit 1
    fi
else
    print_success "Ollama service is already running"
fi

# Check if required model is available
echo ""
print_status "Checking Ollama model: $OLLAMA_MODEL"

if ollama list | grep -q "$OLLAMA_MODEL"; then
    print_success "Model $OLLAMA_MODEL is available"
else
    print_status "Pulling model $OLLAMA_MODEL (this may take several minutes)..."
    
    if ollama pull "$OLLAMA_MODEL"; then
        print_success "Model $OLLAMA_MODEL pulled successfully"
    else
        print_error "Failed to pull model $OLLAMA_MODEL"
        exit 1
    fi
fi

# Test Ollama API
print_status "Testing Ollama API..."
if curl -s "$OLLAMA_BASE_URL/api/tags" | jq -e '.models' >/dev/null 2>&1; then
    print_success "Ollama API is working"
else
    # Try without jq
    if curl -s "$OLLAMA_BASE_URL/api/tags" | grep -q "models"; then
        print_success "Ollama API is working"
    else
        print_error "Ollama API test failed"
        exit 1
    fi
fi

# Setup backend server
echo ""
echo "🔧 Setting up backend server..."

# Check if backend config exists
BACKEND_CONFIG_PATH="../$BACKEND_CONFIG"
if [ ! -f "$BACKEND_CONFIG_PATH" ]; then
    print_error "Backend config not found: $BACKEND_CONFIG_PATH"
    exit 1
fi

print_success "Backend config found: $BACKEND_CONFIG"

# Build backend if needed
print_status "Building backend server..."
cd ..
if cargo build --release; then
    print_success "Backend built successfully"
else
    print_error "Backend build failed"
    exit 1
fi
cd desktop

# Start backend server in background
print_status "Starting backend server with Ollama config..."
cd ..
cargo run --release -- --config "$BACKEND_CONFIG" &
BACKEND_PID=$!
echo $BACKEND_PID > desktop/.backend_pid
cd desktop

print_status "Backend server started (PID: $BACKEND_PID)"

# Wait for backend to be ready
if wait_for_service "http://localhost:8080/health" "Backend server" 60; then
    print_success "Backend server is ready"
else
    # Try to find the actual port
    print_status "Trying to find backend server port..."
    sleep 5
    
    # Check common ports
    for port in 8080 3000 8000 8081; do
        if port_is_open $port; then
            if curl -s "http://localhost:$port/health" >/dev/null 2>&1; then
                print_success "Backend server found on port $port"
                export VITE_API_URL="http://localhost:$port"
                break
            fi
        fi
    done
fi

# Install frontend dependencies
echo ""
echo "📦 Installing frontend dependencies..."

if npm install; then
    print_success "Frontend dependencies installed"
else
    print_error "Frontend dependency installation failed"
    exit 1
fi

# Install Playwright browsers if needed
print_status "Setting up Playwright browsers..."
if npx playwright install; then
    print_success "Playwright browsers installed"
else
    print_warning "Playwright browser installation failed, but continuing..."
fi

# Create test data directory if needed
echo ""
print_status "Setting up test environment..."

mkdir -p test-data
mkdir -p test-results

# Run a quick smoke test
echo ""
echo "🧪 Running smoke tests..."

print_status "Testing Ollama connectivity..."
if curl -s "$OLLAMA_BASE_URL/api/generate" -d '{
    "model": "'$OLLAMA_MODEL'",
    "prompt": "Hello",
    "stream": false,
    "options": {"num_predict": 5}
}' | grep -q "response"; then
    print_success "Ollama model test passed"
else
    print_warning "Ollama model test failed, but continuing..."
fi

# Create test runner script
cat > run-e2e-tests.sh << 'EOF'
#!/bin/bash

# Run specific test suites
echo "🧪 Running Terraphim AI E2E Tests"

# Set test environment
export NODE_ENV=test
export PLAYWRIGHT_HEADLESS=true

# Run all tests or specific suites based on arguments
if [ $# -eq 0 ]; then
    echo "Running all e2e tests..."
    npx playwright test
else
    echo "Running specific test suite: $1"
    npx playwright test "$1"
fi
EOF

chmod +x run-e2e-tests.sh

# Create cleanup script
cat > cleanup-test-env.sh << 'EOF'
#!/bin/bash

echo "🧹 Cleaning up test environment..."

# Stop backend server
if [ -f .backend_pid ]; then
    BACKEND_PID=$(cat .backend_pid)
    if kill -0 $BACKEND_PID 2>/dev/null; then
        echo "Stopping backend server (PID: $BACKEND_PID)"
        kill $BACKEND_PID
    fi
    rm .backend_pid
fi

# Stop Ollama if we started it
if [ -f .ollama_pid ]; then
    OLLAMA_PID=$(cat .ollama_pid)
    if kill -0 $OLLAMA_PID 2>/dev/null; then
        echo "Stopping Ollama service (PID: $OLLAMA_PID)"
        kill $OLLAMA_PID
    fi
    rm .ollama_pid
fi

echo "Cleanup complete"
EOF

chmod +x cleanup-test-env.sh

# Final status report
echo ""
echo "✅ Setup Complete!"
echo "=================================================="
echo -e "${GREEN}Environment is ready for e2e testing${NC}"
echo ""
echo "📋 What was set up:"
echo "  • Ollama service with model: $OLLAMA_MODEL"
echo "  • Backend server with Ollama configuration"
echo "  • Frontend dependencies and Playwright browsers"
echo "  • Test helper scripts"
echo ""
echo "🚀 Next steps:"
echo "  • Run all tests: ./run-e2e-tests.sh"
echo "  • Run specific tests: ./run-e2e-tests.sh tests/e2e/chat-functionality.spec.ts"
echo "  • Clean up: ./cleanup-test-env.sh"
echo ""
echo "🔧 Available test suites:"
echo "  • chat-functionality.spec.ts - Complete chat system tests"
echo "  • summarization.spec.ts - Document summarization tests"
echo "  • ollama-integration.spec.ts - Ollama connectivity and model tests"
echo "  • config-wizard-complete.spec.ts - Configuration management tests"
echo ""

# Store setup info for later reference
cat > .test-setup-info << EOF
OLLAMA_BASE_URL=$OLLAMA_BASE_URL
OLLAMA_MODEL=$OLLAMA_MODEL
BACKEND_CONFIG=$BACKEND_CONFIG
SETUP_TIME=$(date)
BACKEND_PID=$BACKEND_PID
EOF

print_success "Setup information saved to .test-setup-info"

echo -e "${BLUE}Happy testing! 🎉${NC}"