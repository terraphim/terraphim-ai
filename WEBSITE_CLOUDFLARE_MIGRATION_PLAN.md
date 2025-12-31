# Terraphim.ai Migration: Netlify to Cloudflare Pages

## Migration Overview

This document outlines the complete migration plan for moving terraphim.ai from Netlify to Cloudflare Pages, leveraging existing infrastructure and patterns established for docs.terraphim.ai.

## Current State Analysis

### Netlify Configuration
- **Build Command**: `zola build`
- **Publish Directory**: `public`
- **Node Version**: Not specified (static site)
- **Environment Variables**: None required
- **Custom Domain**: terraphim.ai
- **SSL**: Managed by Netlify

### Cloudflare Requirements
- **Build Tool**: Zola 0.21.0 (static site generator)
- **Output**: Static files in `public/` directory
- **Custom Domain**: terraphim.ai
- **SSL**: Automatic via Cloudflare
- **Build Time**: ~92ms (well within limits)

## Migration Plan

### Phase 1: Preparation & Configuration

#### 1.1 Cloudflare Project Setup
- [ ] Create Cloudflare Pages project: `terraphim-ai`
- [ ] Connect to GitHub repository
- [ ] Set build framework: `Zola`
- [ ] Configure build settings:
  - **Build command**: `zola build`
  - **Build output directory**: `public`
  - **Root directory**: `/website`

#### 1.2 Environment Configuration
- [ ] Set up 1Password secrets (following docs pattern):
  - `op://TerraphimPlatform/terraphim-ai-cloudflare/workers-api-token`
  - `op://TerraphimPlatform/terraphim-ai-cloudflare/account-id`
  - `op://TerraphimPlatform/terraphim-ai-cloudflare/zone-id`

#### 1.3 Configuration Files
- [ ] Create `website/wrangler.toml`
- [ ] Update `website/.gitignore` for Cloudflare
- [ ] Remove `website/netlify.toml`

### Phase 2: DNS & Domain Migration

#### 2.1 DNS Preparation
- [ ] Verify terraphim.ai DNS records
- [ ] Document current Netlify IPs
- [ ] Prepare Cloudflare DNS records

#### 2.2 Domain Migration
- [ ] Add custom domain in Cloudflare Pages
- [ ] Update DNS nameservers to Cloudflare
- [ ] Verify SSL certificate provisioning
- [ ] Test domain resolution

#### 2.3 Migration Timing
- [ ] Schedule migration during low-traffic period
- [ ] Prepare rollback plan
- [ ] Monitor for DNS propagation

### Phase 3: Build & Deployment Configuration

#### 3.1 Wrangler Configuration
```toml
# website/wrangler.toml
name = "terraphim-ai"
compatibility_date = "2024-01-01"
compatibility_flags = ["nodejs_compat"]

[build]
command = "zola build"
cwd = "/website"
watch_dir = "/website"

[env.production]
name = "terraphim-ai"

[env.preview]
name = "terraphim-ai-preview"

# Security headers
[[headers]]
for = "/*"
[headers.values]
X-Frame-Options = "DENY"
X-Content-Type-Options = "nosniff"
X-XSS-Protection = "1; mode=block"
Referrer-Policy = "strict-origin-when-cross-origin"

# Cache control
[[headers]]
for = "*.css"
[headers.values]
Cache-Control = "public, max-age=31536000, immutable"

[[headers]]
for = "*.js"
[headers.values]
Cache-Control = "public, max-age=31536000, immutable"

[[headers]]
for = "*.png"
[headers.values]
Cache-Control = "public, max-age=31536000, immutable"

[[headers]]
for = "*.jpg"
[headers.values]
Cache-Control = "public, max-age=31536000, immutable"

[[headers]]
for = "*.svg"
[headers.values]
Cache-Control = "public, max-age=31536000, immutable"

[[headers]]
for = "*.ico"
[headers.values]
Cache-Control = "public, max-age=31536000, immutable"

[[headers]]
for = "*.woff"
[headers.values]
Cache-Control = "public, max-age=31536000, immutable"

[[headers]]
for = "*.woff2"
[headers.values]
Cache-Control = "public, max-age=31536000, immutable"
```

#### 3.2 GitHub Actions Workflow
- [ ] Create `.github/workflows/deploy-website.yml`
- [ ] Configure 1Password integration
- [ ] Set up preview deployments for PRs
- [ ] Configure production deployments for main

### Phase 4: Deployment Scripts

#### 4.1 Manual Deployment Script
```bash
#!/bin/bash
# scripts/deploy-website.sh

set -e

ENVIRONMENT=${1:-preview}
PROJECT_NAME="terraphim-ai"

echo "Deploying Terraphim.ai website to $ENVIRONMENT..."

# Build the site
cd website
zola build

# Deploy to Cloudflare Pages
if [ "$ENVIRONMENT" = "production" ]; then
    wrangler pages deploy public --project-name=$PROJECT_NAME --branch=main
else
    wrangler pages deploy public --project-name=$PROJECT_NAME --branch=preview
fi

echo "Deployment completed successfully!"
```

#### 4.2 1Password Setup Script
```bash
#!/bin/bash
# scripts/setup-1password-website.sh

set -e

echo "Setting up 1Password integration for Terraphim.ai website..."

# Create 1Password items if they don't exist
op item create --vault TerraphimPlatform --category "API Credential" \
  --title "Terraphim AI Cloudflare Workers API Token" \
  --fields label=API,type=concurrent,generate=true

op item create --vault TerraphimPlatform --category "Database" \
  --title "Terraphim AI Cloudflare Account ID" \
  --fields label=Account,type=concurrent

op item create --vault TerraphimPlatform --category "Database" \
  --title "Terraphim AI Cloudflare Zone ID" \
  --fields label=Zone,type=concurrent

echo "1Password setup completed!"
```

### Phase 5: Testing & Validation

#### 5.1 Pre-Migration Testing
- [ ] Test local build with `zola build`
- [ ] Verify all static assets are present
- [ ] Test navigation and links
- [ ] Validate HTML/CSS/JS functionality

#### 5.2 Cloudflare Testing
- [ ] Deploy to preview environment
- [ ] Test all pages and functionality
- [ ] Verify SSL certificate
- [ ] Check performance metrics

#### 5.3 Production Validation
- [ ] Monitor DNS propagation
- [ ] Test live site functionality
- [ ] Verify analytics tracking
- [ ] Check form submissions (if any)

### Phase 6: Migration Execution

#### 6.1 Pre-Migration Checklist
- [ ] Backup current Netlify configuration
- [ ] Document all DNS records
- [ ] Prepare rollback procedure
- [ ] Notify stakeholders of maintenance window

#### 6.2 Migration Steps
1. **Deploy to Cloudflare Pages preview**
2. **Validate preview deployment**
3. **Update DNS to Cloudflare nameservers**
4. **Add custom domain in Cloudflare Pages**
5. **Wait for DNS propagation (1-24 hours)**
6. **Verify SSL certificate**
7. **Test live site functionality**
8. **Update monitoring and analytics**

#### 6.3 Post-Migration
- [ ] Delete Netlify project (after 48 hours)
- [ ] Update documentation references
- [ ] Configure Cloudflare analytics
- [ ] Set up monitoring alerts

## Benefits of Migration

### Performance Improvements
- **Global CDN**: 200+ edge locations vs Netlify's limited CDN
- **Faster builds**: 92ms build time well within Cloudflare's limits
- **Better caching**: Configurable cache headers and rules
- **Lower latency**: Cloudflare's optimized network

### Cost Benefits
- **Free tier advantages**:
  - Unlimited bandwidth (vs 100GB on Netlify)
  - 500 builds/month (vs 300 on Netlify)
  - No concurrent build limits on paid tiers
  - Better performance analytics

### Feature Enhancements
- **Preview deployments**: Automatic for PRs
- **Better security**: Enhanced security headers
- **Analytics**: Built-in performance analytics
- **Edge functions**: Future serverless capabilities

## Risk Mitigation

### Technical Risks
- **DNS propagation delays**: Mitigate with proper timing
- **SSL certificate issues**: Cloudflare auto-provisions
- **Build failures**: Test thoroughly in preview
- **Performance regression**: Cloudflare's CDN is superior

### Business Risks
- **Downtime**: Minimize with careful migration timing
- **SEO impact**: Use proper redirects and maintain URLs
- **User experience**: Thorough testing prevents issues

## Rollback Plan

If migration fails:
1. **Immediate**: Revert DNS to Netlify nameservers
2. **Temporary**: Keep Netlify project active for 48 hours
3. **Investigation**: Analyze failure points and fix
4. **Retry**: Schedule new migration attempt

## Timeline

### Week 1: Preparation
- Day 1-2: Cloudflare project setup
- Day 3-4: Configuration and scripts
- Day 5: Testing and validation

### Week 2: Migration
- Day 1: Pre-migration testing
- Day 2: Migration execution
- Day 3-5: Monitoring and optimization

### Week 3: Cleanup
- Day 1-2: Post-migration validation
- Day 3-4: Netlify cleanup
- Day 5: Documentation updates

## Success Metrics

- **Zero downtime** during migration
- **Performance improvement**: <2s load time globally
- **Build success rate**: 100% in first week
- **SEO stability**: No ranking changes
- **User experience**: No reported issues

## Next Steps

1. **Approve migration plan** with stakeholders
2. **Schedule migration window**
3. **Execute Phase 1**: Preparation and configuration
4. **Test thoroughly** before DNS changes
5. **Execute migration** during low-traffic period
6. **Monitor and optimize** post-migration

---

*This migration leverages existing Cloudflare infrastructure and patterns established for docs.terraphim.ai, ensuring consistency and reliability across the Terraphim AI platform.*