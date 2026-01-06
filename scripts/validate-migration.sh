#!/bin/bash

set -e

echo "=== Cloudflare Pages Custom Domain Configuration ==="
echo ""

# Load credentials
source $HOME/.my_cloudflare.sh

echo "🔍 Checking Current Domain Configuration..."
echo ""

# Check if domains are already configured via API
echo "Using Cloudflare API to check project domains..."

# Get project info via Pages API
echo "📊 Project Analysis:"
curl -s -X GET "https://api.cloudflare.com/client/v4/accounts/$CLOUDFLARE_ACCOUNT_ID/pages/projects" \
  -H "Authorization: Bearer $CLOUDFLARE_API_TOKEN" \
  -H "Content-Type: application/json" | jq -r '.result[] | select(.name == "terraphim-ai") | {name, domains, latest_deployment}'

echo ""
echo "🌐 DNS Status Analysis:"
echo "Current Nameservers:"
dig NS terraphim.ai +short | while read ns; do
    echo "  - $ns"
done

echo ""
echo "A Records:"
dig A terraphim.ai +short | while read a; do
    echo "  - $a"
done

echo ""
echo "www CNAME:"
dig CNAME www.terraphim.ai +short | while read cname; do
    echo "  - $cname"
done

echo ""
echo "=== Current Status ==="
echo ""

# Check if we can access via custom domain
echo "🔍 Testing terraphim.ai accessibility..."
if curl -s -o /dev/null -w "%{http_code}" https://terraphim.ai | grep -q "200\|301\|302"; then
    echo "✅ terraphim.ai is accessible"
else
    echo "❌ terraphim.ai is not accessible"
fi

echo ""
echo "🔍 Testing www.terraphim.ai accessibility..."
if curl -s -o /dev/null -w "%{http_code}" https://www.terraphim.ai | grep -q "200\|301\|302"; then
    echo "✅ www.terraphim.ai is accessible"
else
    echo "❌ www.terraphim.ai is not accessible"
fi

echo ""
echo "=== Production Deployment Readiness ==="
echo ""

# Test production build
cd website
if zola build; then
    echo "✅ Production build successful"
else
    echo "❌ Production build failed"
    exit 1
fi

# Check total build size
total_size=$(du -sk public | cut -f1)
total_mb=$((total_size / 1024))
echo "📊 Build size: ${total_mb}MB"

if [[ $total_mb -lt 100 ]]; then
    echo "✅ Build size is reasonable"
else
    echo "⚠️  Build size is large (${total_mb}MB)"
fi

echo ""
echo "=== Next Actions ==="
echo ""

if curl -s -o /dev/null -w "%{http_code}" https://terraphim.ai | grep -q "200\|301\|302"; then
    echo "🎯 DOMAIN IS ALREADY WORKING"
    echo "Next steps:"
    echo "1. Deploy to production"
    echo "2. Verify all functionality"
    echo "3. Update monitoring"
    echo "4. Complete migration"
else
    echo "🌐 DOMAIN SETUP NEEDED"
    echo "Next steps:"
    echo "1. Add custom domain in Cloudflare dashboard"
    echo "2. Wait for DNS propagation"
    echo "3. Test domain accessibility"
    echo "4. Deploy to production"
fi

echo ""
echo "=== Production Deployment Command ==="
echo ""
echo "When ready, run:"
echo "source \$HOME/.my_cloudflare.sh && cd website && zola build && cd .. && wrangler pages deploy website/public --project-name=terraphim-ai --branch=main"
echo ""