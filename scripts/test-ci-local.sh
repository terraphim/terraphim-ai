#!/bin/bash

# Local CI testing script using act
# Usage: ./scripts/test-ci-local.sh [workflow-name]

set -e

WORKFLOW=${1:-"earthly-runner"}
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_ROOT"

echo "🔧 Testing GitHub Actions workflow: $WORKFLOW"

# Create act configuration if it doesn't exist
if [[ ! -f .actrc ]]; then
    echo "📋 Creating .actrc configuration..."
    cat > .actrc << EOF
# Use nektos/act Docker images for consistency
-P ubuntu-latest=catthehacker/ubuntu:act-latest
-P ubuntu-22.04=catthehacker/ubuntu:act-22.04
-P ubuntu-20.04=catthehacker/ubuntu:act-20.04

# Reuse Docker containers for speed
--reuse

# Bind mount Docker socket for Earthly
--bind

# Verbose output for debugging
--verbose
EOF
fi

# Create secrets file for act if needed
if [[ ! -f .secrets ]]; then
    echo "🔐 Creating .secrets file template..."
    cat > .secrets << EOF
# GitHub token for API access (optional for local testing)
GITHUB_TOKEN=your_token_here

# Earthly token (optional)
EARTHLY_TOKEN=your_earthly_token_here
EOF
    echo "⚠️  Please edit .secrets file with actual tokens if needed"
fi

echo "🚀 Running act for workflow: $WORKFLOW"

case "$WORKFLOW" in
    "earthly-runner"|"earthly")
        echo "Testing Earthly CI/CD workflow..."
        act -W .github/workflows/earthly-runner.yml \
            --secret-file .secrets \
            --env-file <(echo "EARTHLY_ORG=") \
            --env-file <(echo "EARTHLY_SATELLITE=") \
            push
        ;;
    "ci-native"|"native")
        echo "Testing Native CI workflow..."
        act -W .github/workflows/ci-native.yml \
            --secret-file .secrets \
            push
        ;;
    "frontend")
        echo "Testing Frontend Build workflow..."
        act -W .github/workflows/frontend-build.yml \
            --secret-file .secrets \
            workflow_call
        ;;
    "rust")
        echo "Testing Rust Build workflow..."
        act -W .github/workflows/rust-build.yml \
            --secret-file .secrets \
            workflow_call \
            --input target=x86_64-unknown-linux-gnu
        ;;
    "lint"|"setup")
        echo "Testing individual jobs..."
        act -W .github/workflows/ci-native.yml \
            --secret-file .secrets \
            -j lint-and-format \
            push
        ;;
    *)
        echo "❌ Unknown workflow: $WORKFLOW"
        echo "Available workflows:"
        echo "  earthly-runner (or earthly)"
        echo "  ci-native (or native)"
        echo "  frontend"
        echo "  rust"
        echo "  lint (or setup)"
        exit 1
        ;;
esac

echo "✅ Local CI test completed for: $WORKFLOW"
