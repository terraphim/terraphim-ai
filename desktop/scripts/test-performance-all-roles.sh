#!/bin/bash

# Performance Validation Test for All Roles
# This script runs comprehensive tests to validate that all three roles work correctly
# and that the performance optimizations are effective.

set -e

echo "🚀 Starting Performance Validation Test for All Roles"
echo "=================================================="

# Check if we're in the right directory
if [ ! -f "package.json" ]; then
    echo "❌ Error: Please run this script from the desktop directory"
    exit 1
fi

# Check if dependencies are installed
if [ ! -d "node_modules" ]; then
    echo "📦 Installing dependencies..."
    yarn install
fi

# Check if the server is running
echo "🔍 Checking if Terraphim server is running..."
if ! curl -s http://localhost:8000/health > /dev/null 2>&1; then
    echo "⚠️  Warning: Terraphim server not running on localhost:8000"
    echo "   Please start the server with: cargo run --bin terraphim_server"
    echo "   Or run: cd .. && cargo run --bin terraphim_server"
    echo ""
    echo "   Continuing with tests anyway (they may fail)..."
fi

# Check if dev server is running
echo "🔍 Checking if dev server is running..."
if ! curl -s http://localhost:5173 > /dev/null 2>&1; then
    echo "🚀 Starting dev server..."
    yarn run dev &
    DEV_PID=$!

    # Wait for dev server to start
    echo "⏳ Waiting for dev server to start..."
    for i in {1..30}; do
        if curl -s http://localhost:5173 > /dev/null 2>&1; then
            echo "✅ Dev server started successfully"
            break
        fi
        sleep 1
    done

    if ! curl -s http://localhost:5173 > /dev/null 2>&1; then
        echo "❌ Failed to start dev server"
        exit 1
    fi
else
    echo "✅ Dev server is already running"
    DEV_PID=""
fi

# Run the performance validation tests
echo ""
echo "🧪 Running Performance Validation Tests..."
echo "=========================================="

# Run the specific performance test
npx playwright test tests/e2e/performance-validation-all-roles.spec.ts --headed --timeout=60000

# Check test results
if [ $? -eq 0 ]; then
    echo ""
    echo "✅ All Performance Validation Tests PASSED!"
    echo "=========================================="
    echo "🎉 Summary:"
    echo "  ✅ All three roles (Default, Rust Engineer, Terraphim Engineer) working"
    echo "  ✅ Search performance optimized (< 2 seconds)"
    echo "  ✅ No UI freeze during search"
    echo "  ✅ Role switching working correctly"
    echo "  ✅ Search responsiveness maintained during rapid typing"
    echo ""
    echo "🚀 Performance optimizations are working correctly!"
else
    echo ""
    echo "❌ Some Performance Validation Tests FAILED!"
    echo "=========================================="
    echo "Please check the test output above for details."
    exit 1
fi

# Cleanup
if [ ! -z "$DEV_PID" ]; then
    echo "🧹 Cleaning up dev server..."
    kill $DEV_PID 2>/dev/null || true
fi

echo ""
echo "🎯 Performance validation complete!"
