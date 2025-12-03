#!/bin/bash

# Simple test script to verify Rust setup
echo "🔧 Testing setup_zen_claude_config..."

cd setup_zen_claude_config

# Test if we can at least check the code
echo "📦 Running cargo check..."
if cargo check --target aarch64-apple-darwin --quiet; then
    echo "✅ Code compiles successfully"
else
    echo "❌ Code has compilation errors"
    exit 1
fi

echo "🎯 Testing basic functionality..."