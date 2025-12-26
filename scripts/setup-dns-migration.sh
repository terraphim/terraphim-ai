#!/bin/bash

set -e

echo "=== Terraphim.ai DNS Migration to Cloudflare ==="
echo ""
echo "Current DNS Analysis..."
echo ""

# Check current nameservers
echo "🔍 Current Nameservers:"
dig NS terraphim.ai +short | sort

echo ""
echo "🌐 Current A Records:"
dig A terraphim.ai +short

echo ""
echo "🔗 Current www CNAME:"
dig CNAME www.terraphim.ai +short

echo ""
echo "📊 Cloudflare Account Analysis:"
source $HOME/.my_cloudflare.sh
echo "Account ID: $CLOUDFLARE_ACCOUNT_ID"
echo "Zone ID: b489b841cea3c6a7270890a7e2310e5d"

echo ""
echo "=== Migration Steps ==="
echo ""
echo "1. 🌐 Add Custom Domain in Cloudflare:"
echo "   - Go to: https://dash.cloudflare.com/pages"
echo "   - Select project: terraphim-ai"
echo "   - Click 'Custom domains'"
echo "   - Add: terraphim.ai"
echo "   - Add: www.terraphim.ai"
echo ""

echo "2. 🔄 Update Nameservers:"
echo "   Current registrar needs nameserver update"
echo "   Target nameservers (from Cloudflare):"
echo "   - dina.ns.cloudflare.com"
echo "   - jim.ns.cloudflare.com"
echo ""

echo "3. ⏱️  Wait for DNS Propagation:"
echo "   - Usually takes 1-24 hours"
echo "   - Monitor with: dig NS terraphim.ai"
echo ""

echo "4. 🔒 SSL Certificate:"
echo "   - Automatic provisioning by Cloudflare"
echo "   - Usually 5-10 minutes after DNS update"
echo "   - Check status in Cloudflare dashboard"
echo ""

echo "5. ✅ Validation:"
echo "   - HTTP/HTTPS accessibility"
echo "   - Certificate validity"
echo "   - Website functionality"
echo ""

echo "=== Pre-Migration Checklist ==="
echo ""
echo "Before proceeding, verify:"
echo "✅ Cloudflare Pages project created"
echo "✅ Preview deployment working"
echo "✅ All files under 25MB limit"
echo "✅ Backup of current configuration"
echo "✅ DNS access at domain registrar"
echo "✅ Maintenance window scheduled"
echo ""

echo "=== Rollback Plan ==="
echo ""
echo "If migration fails:"
echo "1. Revert nameservers to original"
echo "2. Restore Netlify configuration"
echo "3. Verify website accessibility"
echo "4. Investigate failure points"
echo ""

echo "=== Ready for Migration ==="
echo ""
echo "Execute the above steps in Cloudflare dashboard and domain registrar."
echo "After completion, run: ./scripts/validate-migration.sh"