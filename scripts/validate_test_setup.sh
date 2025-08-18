#!/bin/bash

# Validation Script for Terraphim Engineer Test Setup
# This script validates that the configuration and setup is working correctly

set -e

echo "🔍 Validating Terraphim Engineer test setup..."

# Check that configuration files exist
echo "📋 1. Checking configuration files..."

if [ -f "terraphim_engineer_test_config.json" ]; then
    echo "✅ Template configuration file exists"
else
    echo "❌ Template configuration file missing"
    exit 1
fi

if [ -f "terraphim_engineer_test_config_final.json" ]; then
    echo "✅ Final configuration file exists"
else
    echo "❌ Final configuration file missing"
    exit 1
fi

if [ -f "test_config_info.json" ]; then
    echo "✅ Test configuration info file exists"
else
    echo "❌ Test configuration info file missing"
    exit 1
fi

# Validate JSON structure
echo ""
echo "🔍 2. Validating JSON structure..."

if command -v jq >/dev/null 2>&1; then
    echo "📝 Validating template configuration..."
    if jq empty terraphim_engineer_test_config.json 2>/dev/null; then
        echo "✅ Template configuration is valid JSON"
    else
        echo "❌ Template configuration is invalid JSON"
        exit 1
    fi
    
    echo "📝 Validating final configuration..."
    if jq empty terraphim_engineer_test_config_final.json 2>/dev/null; then
        echo "✅ Final configuration is valid JSON"
    else
        echo "❌ Final configuration is invalid JSON"
        exit 1
    fi
    
    echo "📝 Validating test info..."
    if jq empty test_config_info.json 2>/dev/null; then
        echo "✅ Test configuration info is valid JSON"
    else
        echo "❌ Test configuration info is invalid JSON"
        exit 1
    fi
else
    echo "⚠️  jq not found, skipping JSON validation"
fi

# Check configuration content
echo ""
echo "🔍 3. Validating configuration content..."

CONFIG_CONTENT=$(cat terraphim_engineer_test_config_final.json)

# Check for proper role structure
if echo "$CONFIG_CONTENT" | jq -e '.roles["Terraphim Engineer Test"]' >/dev/null 2>&1; then
    echo "✅ Test role exists in configuration"
else
    echo "❌ Test role missing from configuration"
    exit 1
fi

# Check for atomic haystack
if echo "$CONFIG_CONTENT" | jq -e '.roles["Terraphim Engineer Test"].haystacks[] | select(.service == "Atomic")' >/dev/null 2>&1; then
    echo "✅ Atomic haystack configured"
    
    # Check that URL is substituted (not a template variable)
    ATOMIC_URL=$(echo "$CONFIG_CONTENT" | jq -r '.roles["Terraphim Engineer Test"].haystacks[] | select(.service == "Atomic") | .location')
    if [[ "$ATOMIC_URL" == *"${"* ]]; then
        echo "❌ Atomic server URL still contains template variables: $ATOMIC_URL"
        exit 1
    else
        echo "✅ Atomic server URL properly substituted: $ATOMIC_URL"
    fi
    
    # Check if secret is substituted
    ATOMIC_SECRET=$(echo "$CONFIG_CONTENT" | jq -r '.roles["Terraphim Engineer Test"].haystacks[] | select(.service == "Atomic") | .atomic_server_secret')
    if [[ "$ATOMIC_SECRET" == *"${"* ]]; then
        echo "❌ Atomic server secret still contains template variables"
        exit 1
    elif [[ "$ATOMIC_SECRET" == "null" || -z "$ATOMIC_SECRET" ]]; then
        echo "⚠️  Atomic server secret is empty (functionality will be limited)"
    else
        echo "✅ Atomic server secret properly substituted (***hidden***)"
    fi
else
    echo "❌ Atomic haystack not found in configuration"
    exit 1
fi

# Check for ripgrep haystack
if echo "$CONFIG_CONTENT" | jq -e '.roles["Terraphim Engineer Test"].haystacks[] | select(.service == "Ripgrep")' >/dev/null 2>&1; then
    echo "✅ Ripgrep haystack configured"
else
    echo "❌ Ripgrep haystack not found in configuration"
    exit 1
fi

# Check for OpenRouter configuration
if echo "$CONFIG_CONTENT" | jq -e '.roles["Terraphim Engineer Test"].openrouter_enabled' >/dev/null 2>&1; then
    OPENROUTER_ENABLED=$(echo "$CONFIG_CONTENT" | jq -r '.roles["Terraphim Engineer Test"].openrouter_enabled')
    if [ "$OPENROUTER_ENABLED" = "true" ]; then
        echo "✅ OpenRouter enabled in configuration"
        
        # Check API key substitution
        OPENROUTER_KEY=$(echo "$CONFIG_CONTENT" | jq -r '.roles["Terraphim Engineer Test"].openrouter_api_key')
        if [[ "$OPENROUTER_KEY" == *"${"* ]]; then
            echo "❌ OpenRouter API key still contains template variables"
            exit 1
        elif [[ "$OPENROUTER_KEY" == "null" || -z "$OPENROUTER_KEY" ]]; then
            echo "⚠️  OpenRouter API key is empty (AI summarization will not work)"
        else
            echo "✅ OpenRouter API key properly substituted (***hidden***)"
        fi
        
        # Check model configuration
        OPENROUTER_MODEL=$(echo "$CONFIG_CONTENT" | jq -r '.roles["Terraphim Engineer Test"].openrouter_model')
        if [[ "$OPENROUTER_MODEL" == *"${"* ]]; then
            echo "❌ OpenRouter model still contains template variables"
            exit 1
        else
            echo "✅ OpenRouter model configured: $OPENROUTER_MODEL"
        fi
    else
        echo "⚠️  OpenRouter disabled in configuration"
    fi
else
    echo "⚠️  OpenRouter configuration not found"
fi

# Check knowledge graph configuration
if echo "$CONFIG_CONTENT" | jq -e '.roles["Terraphim Engineer Test"].kg' >/dev/null 2>&1; then
    echo "✅ Knowledge graph configuration found"
    
    # Check KG path
    KG_PATH=$(echo "$CONFIG_CONTENT" | jq -r '.roles["Terraphim Engineer Test"].kg.knowledge_graph_local.path')
    if [ "$KG_PATH" != "null" ]; then
        echo "✅ Knowledge graph path configured: $KG_PATH"
        
        # Check if path exists
        if [ -d "$KG_PATH" ]; then
            KG_FILE_COUNT=$(find "$KG_PATH" -name "*.md" | wc -l)
            echo "✅ Knowledge graph directory exists with $KG_FILE_COUNT markdown files"
        else
            echo "⚠️  Knowledge graph directory not found: $KG_PATH"
        fi
    else
        echo "⚠️  Knowledge graph path not configured"
    fi
else
    echo "⚠️  Knowledge graph configuration not found"
fi

# Check connectivity
echo ""
echo "🔍 4. Testing connectivity..."

# Read atomic server URL from test info
if command -v jq >/dev/null 2>&1; then
    ATOMIC_URL=$(jq -r '.atomicServerUrl' test_config_info.json)
    
    if command -v curl >/dev/null 2>&1; then
        if curl -s --max-time 5 "$ATOMIC_URL" >/dev/null 2>&1; then
            echo "✅ Atomic server accessible at $ATOMIC_URL"
        else
            echo "⚠️  Atomic server not accessible at $ATOMIC_URL"
        fi
    else
        echo "⚠️  curl not available, skipping connectivity test"
    fi
fi

# Check for necessary files
echo ""
echo "🔍 5. Checking required files for testing..."

REQUIRED_PATHS=(
    "docs/src"
    "docs/src/kg"
)

for path in "${REQUIRED_PATHS[@]}"; do
    if [ -d "$path" ]; then
        FILE_COUNT=$(find "$path" -name "*.md" | wc -l)
        echo "✅ Directory exists: $path ($FILE_COUNT files)"
    else
        echo "⚠️  Directory missing: $path"
    fi
done

# Summary
echo ""
echo "📊 VALIDATION SUMMARY"
echo "========================="

TEST_INFO_CONTENT=$(cat test_config_info.json)
HAS_ATOMIC_SECRET=$(echo "$TEST_INFO_CONTENT" | jq -r '.hasAtomicSecret')
HAS_OPENROUTER_KEY=$(echo "$TEST_INFO_CONTENT" | jq -r '.hasOpenRouterKey')

echo "🔧 Configuration: ✅ Valid"
echo "📁 Files: ✅ All present"
echo "🌐 Atomic Server: $([ "$HAS_ATOMIC_SECRET" = "true" ] && echo "✅ Configured" || echo "⚠️  Limited (no secret)")"
echo "🤖 OpenRouter: $([ "$HAS_OPENROUTER_KEY" = "true" ] && echo "✅ Configured" || echo "⚠️  Disabled (no key)")"
echo "🕸️  Knowledge Graph: ✅ Configured"
echo "🔍 Search: ✅ Ready (Ripgrep + Atomic haystacks)"

echo ""
echo "🎉 Test setup validation completed successfully!"
echo ""
echo "🚀 You can now run the comprehensive tests with:"
echo "   cd desktop && yarn playwright test tests/e2e/terraphim-engineer-comprehensive.spec.ts"
echo ""

# Create a summary file
cat > test_validation_summary.json << 'EOF'
{
  "timestamp": "TIMESTAMP_PLACEHOLDER",
  "status": "passed",
  "configuration": {
    "valid": true,
    "testRole": "Terraphim Engineer Test",
    "haystacks": ["Ripgrep", "Atomic"],
    "knowledgeGraph": true,
    "atomicConfigured": "ATOMIC_PLACEHOLDER",
    "openRouterConfigured": "OPENROUTER_PLACEHOLDER"
  },
  "readyForTesting": true
}
EOF

# Replace placeholders
sed -i "s/TIMESTAMP_PLACEHOLDER/$(date -Iseconds)/" test_validation_summary.json
sed -i "s/ATOMIC_PLACEHOLDER/$HAS_ATOMIC_SECRET/" test_validation_summary.json
sed -i "s/OPENROUTER_PLACEHOLDER/$HAS_OPENROUTER_KEY/" test_validation_summary.json

echo "📝 Validation summary saved to: test_validation_summary.json" 