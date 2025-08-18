#!/bin/bash

# Setup Test Configuration Script
# This script populates the test configuration with environment variables
# and ensures the test environment is ready

set -e

echo "🔧 Setting up Terraphim Engineer test configuration..."

# Set default values if environment variables are not set
ATOMIC_SERVER_URL=${ATOMIC_SERVER_URL:-"http://localhost:9883"}
ATOMIC_SERVER_SECRET=${ATOMIC_SERVER_SECRET:-""}
OPENROUTER_API_KEY=${OPENROUTER_API_KEY:-""}
OPENROUTER_MODEL=${OPENROUTER_MODEL:-"openai/gpt-3.5-turbo"}

# Check if required environment variables are set
if [ -z "$ATOMIC_SERVER_SECRET" ]; then
    echo "⚠️  Warning: ATOMIC_SERVER_SECRET is not set. Atomic save functionality will not work."
fi

if [ -z "$OPENROUTER_API_KEY" ]; then
    echo "⚠️  Warning: OPENROUTER_API_KEY is not set. AI summarization will not work."
fi

# Template file and output file
TEMPLATE_FILE="terraphim_engineer_test_config.json"
OUTPUT_FILE="terraphim_engineer_test_config_final.json"

echo "📄 Reading template configuration from: $TEMPLATE_FILE"

# Check if template file exists
if [ ! -f "$TEMPLATE_FILE" ]; then
    echo "❌ Template file $TEMPLATE_FILE not found!"
    exit 1
fi

# Substitute environment variables in the configuration
echo "🔄 Substituting environment variables..."

# Use envsubst to replace variables, but handle the case where envsubst might not be available
if command -v envsubst >/dev/null 2>&1; then
    envsubst < "$TEMPLATE_FILE" > "$OUTPUT_FILE"
else
    echo "📝 envsubst not found, using sed for substitution..."
    
    # Manual substitution using sed
    cp "$TEMPLATE_FILE" "$OUTPUT_FILE"
    sed -i.bak "s|\${ATOMIC_SERVER_URL}|$ATOMIC_SERVER_URL|g" "$OUTPUT_FILE"
    sed -i.bak "s|\${ATOMIC_SERVER_SECRET}|$ATOMIC_SERVER_SECRET|g" "$OUTPUT_FILE"
    sed -i.bak "s|\${OPENROUTER_API_KEY}|$OPENROUTER_API_KEY|g" "$OUTPUT_FILE"
    sed -i.bak "s|\${OPENROUTER_MODEL}|$OPENROUTER_MODEL|g" "$OUTPUT_FILE"
    
    # Clean up backup files
    rm -f "$OUTPUT_FILE.bak"
fi

echo "✅ Configuration file created: $OUTPUT_FILE"

# Display the substituted values (mask secrets)
echo ""
echo "📊 Configuration Summary:"
echo "  🌐 Atomic Server URL: $ATOMIC_SERVER_URL"
echo "  🔐 Atomic Server Secret: $([ -n "$ATOMIC_SERVER_SECRET" ] && echo "***SET***" || echo "NOT SET")"
echo "  🤖 OpenRouter API Key: $([ -n "$OPENROUTER_API_KEY" ] && echo "***SET***" || echo "NOT SET")"
echo "  🎯 OpenRouter Model: $OPENROUTER_MODEL"
echo ""

# Validate the generated JSON
echo "🔍 Validating generated JSON..."

if command -v jq >/dev/null 2>&1; then
    if jq empty "$OUTPUT_FILE" 2>/dev/null; then
        echo "✅ Generated configuration is valid JSON"
    else
        echo "❌ Generated configuration is not valid JSON!"
        exit 1
    fi
else
    echo "⚠️  jq not found, skipping JSON validation"
fi

# Check if atomic server is running
echo "🔍 Checking atomic server connectivity..."

if command -v curl >/dev/null 2>&1; then
    if curl -s --max-time 5 "$ATOMIC_SERVER_URL" >/dev/null 2>&1; then
        echo "✅ Atomic server is accessible at $ATOMIC_SERVER_URL"
    else
        echo "⚠️  Atomic server is not accessible at $ATOMIC_SERVER_URL"
        echo "   Make sure the atomic server is running with: cargo install atomic-server && atomic-server"
    fi
else
    echo "⚠️  curl not found, skipping connectivity check"
fi

# Create a test script that can be used by Playwright tests
cat > "test_config_info.json" << EOF
{
  "configFile": "$OUTPUT_FILE",
  "atomicServerUrl": "$ATOMIC_SERVER_URL",
  "hasAtomicSecret": $([ -n "$ATOMIC_SERVER_SECRET" ] && echo "true" || echo "false"),
  "hasOpenRouterKey": $([ -n "$OPENROUTER_API_KEY" ] && echo "true" || echo "false"),
  "openRouterModel": "$OPENROUTER_MODEL",
  "testRole": "Terraphim Engineer Test"
}
EOF

echo "📝 Created test configuration info: test_config_info.json"

echo ""
echo "🎉 Test configuration setup complete!"
echo "   Use the following file for testing: $OUTPUT_FILE"
echo "   Test info available in: test_config_info.json"
echo "" 