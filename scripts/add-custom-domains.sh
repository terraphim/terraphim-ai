#!/bin/bash

set -e

echo "=== Adding Custom Domain to Cloudflare Pages ==="
echo ""

# Load credentials
source $HOME/.my_cloudflare.sh

PROJECT_NAME="terraphim-ai"
DOMAINS='["terraphim.ai", "www.terraphim.ai"]'

echo "🔧 Project Configuration:"
echo "Project: $PROJECT_NAME"
echo "Account ID: $CLOUDFLARE_ACCOUNT_ID"
echo "Domains to add: ${DOMAINS[*]}"
echo ""

# Get current project details
echo "📊 Getting current project configuration..."
PROJECT_INFO=$(curl -s -X GET "https://api.cloudflare.com/client/v4/accounts/$CLOUDFLARE_ACCOUNT_ID/pages/projects/$PROJECT_NAME" \
  -H "Authorization: Bearer $CLOUDFLARE_API_TOKEN" \
  -H "Content-Type: application/json")

echo "Current domains:"
echo "$PROJECT_INFO" | jq -r '.result.domains[]' 2>/dev/null || echo "  - No custom domains configured yet"

echo ""

# Add custom domains via API
for domain in terraphim.ai www.terraphim.ai; do
    echo "🌐 Adding domain: $domain"

    ADD_DOMAIN_RESPONSE=$(curl -s -X POST "https://api.cloudflare.com/client/v4/accounts/$CLOUDFLARE_ACCOUNT_ID/pages/projects/$PROJECT_NAME/domains" \
      -H "Authorization: Bearer $CLOUDFLARE_API_TOKEN" \
      -H "Content-Type: application/json" \
      -d "{\"name\": \"$domain\"}")

    SUCCESS=$(echo "$ADD_DOMAIN_RESPONSE" | jq -r '.success // false')
    MESSAGE=$(echo "$ADD_DOMAIN_RESPONSE" | jq -r '.message // "Unknown error"')

    if [[ "$SUCCESS" == "true" ]]; then
        echo "✅ Successfully added: $domain"

        # Wait a moment for processing
        sleep 2

        # Check domain status
        DOMAIN_STATUS=$(curl -s -X GET "https://api.cloudflare.com/client/v4/accounts/$CLOUDFLARE_ACCOUNT_ID/pages/projects/$PROJECT_NAME/domains/$domain" \
          -H "Authorization: Bearer $CLOUDFLARE_API_TOKEN" \
          -H "Content-Type: application/json")

        STATUS=$(echo "$DOMAIN_STATUS" | jq -r '.result.status // "unknown"')
        DNS_NEEDED=$(echo "$DOMAIN_STATUS" | jq -r '.result.dns_needed // false')

        echo "  Status: $STATUS"
        if [[ "$DNS_NEEDED" == "true" ]]; then
            echo "  ⚠️  DNS configuration needed"
        else
            echo "  ✅ DNS configuration OK"
        fi
    else
        echo "❌ Failed to add: $domain"
        echo "  Error: $MESSAGE"

        # Show detailed errors if available
        ERRORS=$(echo "$ADD_DOMAIN_RESPONSE" | jq -r '.errors[] | "- \(.code): \(.message)"' 2>/dev/null)
        if [[ -n "$ERRORS" ]]; then
            echo "  Details:"
            echo "$ERRORS"
        fi
    fi

    echo ""
done

echo "=== DNS Configuration Instructions ==="
echo ""
echo "The following DNS records may be needed:"
echo ""

# Get DNS records for custom domains
for domain in terraphim.ai www.terraphim.ai; do
    echo "🔍 DNS for $domain:"

    DNS_RECORDS=$(curl -s -X GET "https://api.cloudflare.com/client/v4/accounts/$CLOUDFLARE_ACCOUNT_ID/pages/projects/$PROJECT_NAME/domains/$domain/dns-records" \
      -H "Authorization: Bearer $CLOUDFLARE_API_TOKEN" \
      -H "Content-Type: application/json")

    echo "$DNS_RECORDS" | jq -r '.result[] | "  \(.type): \(.name) -> \(.content) (TTL: \(.ttl))"' 2>/dev/null || echo "  No special DNS records required"
    echo ""
done

echo "=== Verification ==="
echo ""
echo "After DNS propagation, verify with:"
echo ""
echo "1. DNS lookup:"
echo "   dig A terraphim.ai"
echo "   dig CNAME www.terraphim.ai"
echo ""
echo "2. HTTP access:"
echo "   curl -I https://terraphim.ai"
echo "   curl -I https://www.terraphim.ai"
echo ""
echo "3. Browser test:"
echo "   https://terraphim.ai"
echo "   https://www.terraphim.ai"
echo ""

echo "=== Current Project Status ==="
echo ""

# Get updated project info
FINAL_PROJECT_INFO=$(curl -s -X GET "https://api.cloudflare.com/client/v4/accounts/$CLOUDFLARE_ACCOUNT_ID/pages/projects/$PROJECT_NAME" \
  -H "Authorization: Bearer $CLOUDFLARE_API_TOKEN" \
  -H "Content-Type: application/json")

echo "Project domains:"
echo "$FINAL_PROJECT_INFO" | jq -r '.result.domains[]' 2>/dev/null || echo "  No custom domains found"

echo ""
echo "Project aliases:"
echo "$FINAL_PROJECT_INFO" | jq -r '.result.aliases[]' 2>/dev/null || echo "  No aliases found"

echo ""
echo "🎯 Domain configuration completed!"