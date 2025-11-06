#!/bin/bash
# Test GitHub Actions workflows locally using act
# Requires: brew install act

set -e

echo "Testing GitHub Actions workflows locally..."
echo "=========================================="

# Check if act is installed
if ! command -v act &> /dev/null; then
    echo "❌ 'act' is not installed. Install it with: brew install act"
    exit 1
fi

# Test build-binaries job for Linux
echo "Testing Linux binary build..."
act -j build-binaries \
    -P ubuntu-20.04=ghcr.io/catthehacker/ubuntu:act-20.04 \
    --matrix os:ubuntu-20.04 \
    --matrix target:x86_64-unknown-linux-gnu \
    --dryrun

# Test Tauri desktop build
echo "Testing Tauri desktop build..."
act -j build-tauri-desktop \
    -P ubuntu-20.04=ghcr.io/catthehacker/ubuntu:act-20.04 \
    --matrix platform:ubuntu-20.04 \
    --dryrun

# Test Docker multi-arch build
echo "Testing Docker multi-architecture build..."
act workflow_call \
    -W .github/workflows/docker-multiarch.yml \
    --input platforms=linux/amd64 \
    --input ubuntu-versions='["20.04","22.04"]' \
    --dryrun

echo "✅ Workflow dry-run tests completed successfully!"
echo ""
echo "To run actual tests (without --dryrun), edit this script."
echo "Note: Full workflow testing requires Docker and significant resources."