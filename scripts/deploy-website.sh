#!/bin/bash

set -e

ENVIRONMENT=${1:-preview}
PROJECT_NAME="terraphim-ai"

echo "Deploying Terraphim.ai website to $ENVIRONMENT..."

# Check if Zola is installed
if ! command -v zola &> /dev/null; then
    echo "Error: Zola is not installed. Please install Zola first."
    exit 1
fi

# Check if Wrangler is installed
if ! command -v wrangler &> /dev/null; then
    echo "Error: Wrangler is not installed. Please install Wrangler first."
    exit 1
fi

# Build the site
echo "Building website with Zola..."
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_ROOT/website"
zola build

# Check if build was successful
if [ ! -d "public" ]; then
    echo "Error: Build failed - public directory not found"
    exit 1
fi

# Deploy to Cloudflare Pages
echo "Deploying to Cloudflare Pages..."
if [ "$ENVIRONMENT" = "production" ]; then
    echo "Deploying to production..."
    wrangler pages deploy public --project-name=$PROJECT_NAME --branch=main
else
    echo "Deploying to preview..."
    wrangler pages deploy public --project-name=$PROJECT_NAME --branch=preview
fi

echo "Deployment completed successfully!"
echo "Preview URL: https://terraphim-ai.pages.dev"