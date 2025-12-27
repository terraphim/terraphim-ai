#!/bin/bash

set -e

echo "Setting up 1Password integration for Terraphim.ai website..."

# Check if op CLI is installed
if ! command -v op &> /dev/null; then
    echo "Error: 1Password CLI is not installed. Please install it first."
    exit 1
fi

# Check if user is authenticated with 1Password
if ! op account get &> /dev/null; then
    echo "Error: Not authenticated with 1Password. Please run 'op account login' first."
    exit 1
fi

# Create 1Password items if they don't exist
echo "Creating 1Password items for Cloudflare integration..."

# Create Workers API Token item (reference existing token)
if ! op item get "terraphim-ai-cloudflare-workers-api-token" --vault Terraphim &> /dev/null; then
    echo "Creating Workers API Token item (referencing existing token)..."
    op item create --vault Terraphim --category "API Credential" \
      --title "Terraphim AI Cloudflare Workers API Token" \
      credential="op://Terraphim/Terraphim.io.cloudflare.token/credential"
else
    echo "Workers API Token item already exists"
fi

# Create Account ID item
if ! op item get "terraphim-ai-cloudflare-account-id" --vault Terraphim &> /dev/null; then
    echo "Creating Account ID item..."
    op item create --vault Terraphim --category "Database" \
      --title "Terraphim AI Cloudflare Account ID" \
      Account="4a345f44f6a673abdaf28eea80da7588"
else
    echo "Account ID item already exists"
fi

# Create Zone ID item
if ! op item get "terraphim-ai-cloudflare-zone-id" --vault Terraphim &> /dev/null; then
    echo "Creating Zone ID item..."
    op item create --vault Terraphim --category "Database" \
      --title "Terraphim AI Cloudflare Zone ID" \
      Zone="b489b841cea3c6a7270890a7e2310e5d"
else
    echo "Zone ID item already exists"
fi

echo ""
echo "1Password setup completed!"
echo ""
echo "Next steps:"
echo "1. Fill in the actual values for the created 1Password items:"
echo "   - terraphim-ai-cloudflare-workers-api-token: Your Cloudflare API token"
echo "   - terraphim-ai-cloudflare-account-id: Your Cloudflare account ID"
echo "   - terraphim-ai-cloudflare-zone-id: Your terraphim.ai zone ID"
echo ""
echo "2. Test the deployment:"
echo "   op run --env-file=website/.env.1password -- ./scripts/deploy-website.sh preview"
echo ""
echo "3. Configure GitHub Actions with OP_SERVICE_ACCOUNT_TOKEN secret"