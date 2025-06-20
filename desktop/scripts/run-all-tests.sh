#!/bin/bash

# Terraphim Desktop App - Comprehensive Test Runner
# This script runs all test suites in the correct order

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
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

# Function to check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Parse command line arguments
SKIP_INSTALL=false
SKIP_BACKEND=false
SKIP_FRONTEND=false
SKIP_E2E=false
SKIP_VISUAL=false
COVERAGE=false
HELP=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --skip-install)
            SKIP_INSTALL=true
            shift
            ;;
        --skip-backend)
            SKIP_BACKEND=true
            shift
            ;;
        --skip-frontend)
            SKIP_FRONTEND=true
            shift
            ;;
        --skip-e2e)
            SKIP_E2E=true
            shift
            ;;
        --skip-visual)
            SKIP_VISUAL=true
            shift
            ;;
        --coverage)
            COVERAGE=true
            shift
            ;;
        --help|-h)
            HELP=true
            shift
            ;;
        *)
            print_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

if [ "$HELP" = true ]; then
    echo "Terraphim Desktop App Test Runner"
    echo ""
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  --skip-install    Skip dependency installation"
    echo "  --skip-backend    Skip backend (Rust) tests"
    echo "  --skip-frontend   Skip frontend (Svelte) tests"
    echo "  --skip-e2e        Skip end-to-end tests"
    echo "  --skip-visual     Skip visual regression tests"
    echo "  --coverage        Generate coverage reports"
    echo "  --help, -h        Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0                           # Run all tests"
    echo "  $0 --coverage                # Run all tests with coverage"
    echo "  $0 --skip-e2e --skip-visual  # Run only unit tests"
    exit 0
fi

# Start time
START_TIME=$(date +%s)

print_status "Starting Terraphim Desktop App Test Suite..."
echo "================================================="

# Check prerequisites
print_status "Checking prerequisites..."

if ! command_exists "cargo"; then
    print_error "Rust/Cargo is not installed. Please install Rust first."
    exit 1
fi

if ! command_exists "yarn"; then
    print_error "Yarn is not installed. Please install Yarn first."
    exit 1
fi

if ! command_exists "node"; then
    print_error "Node.js is not installed. Please install Node.js first."
    exit 1
fi

print_success "Prerequisites check passed"

# Install dependencies
if [ "$SKIP_INSTALL" = false ]; then
    print_status "Installing dependencies..."
    
    # Install Node.js dependencies
    if yarn install --frozen-lockfile; then
        print_success "Node.js dependencies installed"
    else
        print_error "Failed to install Node.js dependencies"
        exit 1
    fi
    
    # Install Playwright browsers if needed
    if [ "$SKIP_E2E" = false ] || [ "$SKIP_VISUAL" = false ]; then
        if npx playwright install --with-deps; then
            print_success "Playwright browsers installed"
        else
            print_warning "Failed to install Playwright browsers (E2E/Visual tests may fail)"
        fi
    fi
else
    print_warning "Skipping dependency installation"
fi

# Test results tracking
BACKEND_RESULT=0
FRONTEND_RESULT=0
E2E_RESULT=0
VISUAL_RESULT=0

# Backend Tests (Rust/Tauri)
if [ "$SKIP_BACKEND" = false ]; then
    print_status "Running backend tests (Rust/Tauri)..."
    
    cd src-tauri
    
    if [ "$COVERAGE" = true ]; then
        # Install coverage tool if not present
        if ! command_exists "cargo-tarpaulin"; then
            print_status "Installing cargo-tarpaulin for coverage..."
            cargo install cargo-tarpaulin
        fi
        
        if cargo tarpaulin --out xml --output-dir ../coverage/backend; then
            print_success "Backend tests with coverage passed"
        else
            print_error "Backend tests failed"
            BACKEND_RESULT=1
        fi
    else
        if cargo test --verbose; then
            print_success "Backend tests passed"
        else
            print_error "Backend tests failed"
            BACKEND_RESULT=1
        fi
    fi
    
    cd ..
else
    print_warning "Skipping backend tests"
fi

# Frontend Tests (Svelte/Vitest)
if [ "$SKIP_FRONTEND" = false ]; then
    print_status "Running frontend tests (Svelte/Vitest)..."
    
    if [ "$COVERAGE" = true ]; then
        if yarn test:coverage; then
            print_success "Frontend tests with coverage passed"
        else
            print_error "Frontend tests failed"
            FRONTEND_RESULT=1
        fi
    else
        if yarn test; then
            print_success "Frontend tests passed"
        else
            print_error "Frontend tests failed"
            FRONTEND_RESULT=1
        fi
    fi
else
    print_warning "Skipping frontend tests"
fi

# Build application for E2E tests
if [ "$SKIP_E2E" = false ] || [ "$SKIP_VISUAL" = false ]; then
    print_status "Building application for E2E tests..."
    
    if yarn build; then
        print_success "Application built successfully"
    else
        print_error "Failed to build application"
        exit 1
    fi
fi

# End-to-End Tests (Playwright)
if [ "$SKIP_E2E" = false ]; then
    print_status "Running end-to-end tests (Playwright)..."
    
    if yarn e2e; then
        print_success "E2E tests passed"
    else
        print_error "E2E tests failed"
        E2E_RESULT=1
    fi
else
    print_warning "Skipping E2E tests"
fi

# Visual Regression Tests (Playwright)
if [ "$SKIP_VISUAL" = false ]; then
    print_status "Running visual regression tests..."
    
    if npx playwright test tests/visual; then
        print_success "Visual regression tests passed"
    else
        print_error "Visual regression tests failed"
        VISUAL_RESULT=1
    fi
else
    print_warning "Skipping visual regression tests"
fi

# Calculate total time
END_TIME=$(date +%s)
DURATION=$((END_TIME - START_TIME))

# Print summary
echo ""
echo "================================================="
print_status "Test Summary"
echo "================================================="

echo "Backend Tests:       $([ $BACKEND_RESULT -eq 0 ] && echo -e "${GREEN}PASSED${NC}" || echo -e "${RED}FAILED${NC}")"
echo "Frontend Tests:      $([ $FRONTEND_RESULT -eq 0 ] && echo -e "${GREEN}PASSED${NC}" || echo -e "${RED}FAILED${NC}")"
echo "E2E Tests:          $([ $E2E_RESULT -eq 0 ] && echo -e "${GREEN}PASSED${NC}" || echo -e "${RED}FAILED${NC}")"
echo "Visual Tests:       $([ $VISUAL_RESULT -eq 0 ] && echo -e "${GREEN}PASSED${NC}" || echo -e "${RED}FAILED${NC}")"

echo ""
echo "Total Duration:     ${DURATION}s"

# Coverage reports
if [ "$COVERAGE" = true ]; then
    echo ""
    print_status "Coverage reports generated:"
    [ -d "coverage/backend" ] && echo "  - Backend: coverage/backend/"
    [ -d "coverage" ] && echo "  - Frontend: coverage/"
fi

# Exit with error if any tests failed
TOTAL_FAILED=$((BACKEND_RESULT + FRONTEND_RESULT + E2E_RESULT + VISUAL_RESULT))

if [ $TOTAL_FAILED -eq 0 ]; then
    echo ""
    print_success "All tests passed! 🎉"
    exit 0
else
    echo ""
    print_error "$TOTAL_FAILED test suite(s) failed"
    exit 1
fi 