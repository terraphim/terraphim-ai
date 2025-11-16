# Cloudflare Pages Deployment for Terraphim AI Documentation

This document describes how to deploy the Terraphim AI documentation to Cloudflare Pages.

## Overview

The documentation is built using [mdBook](https://rust-lang.github.io/mdBook/) and deployed to Cloudflare Pages. The migration from Netlify to Cloudflare Pages provides:

- **Global CDN**: Content served from 200+ edge locations
- **Automatic SSL**: Free SSL certificates with automatic renewal
- **Preview deployments**: Automatic previews for pull requests
- **Fast builds**: Quick build and deployment times
- **Security headers**: Built-in security headers configuration

## Prerequisites

### For Local Development

1. **Install mdBook**:
   ```bash
   cargo install mdbook
   ```

2. **Install mdBook Mermaid plugin** (optional, for diagrams):
   ```bash
   cargo install mdbook-mermaid
   ```

3. **Install Wrangler CLI**:
   ```bash
   npm install -g wrangler
   ```

4. **Authenticate with Cloudflare**:
   ```bash
   wrangler login
   ```

### For CI/CD (GitHub Actions)

Configure the following secrets in your GitHub repository:

- `CLOUDFLARE_API_TOKEN`: API token with "Cloudflare Pages — Edit" permission
- `CLOUDFLARE_ACCOUNT_ID`: Your Cloudflare account ID
- `CLOUDFLARE_ZONE_ID`: (Optional) Zone ID for cache purging

## Local Development

### Build Documentation

```bash
cd docs
mdbook build
```

The built documentation will be in `docs/book/`.

### Serve Locally

```bash
cd docs
mdbook serve --open
```

This starts a local server at `http://localhost:3000`.

### Watch for Changes

```bash
cd docs
mdbook serve --open
```

The `serve` command automatically watches for changes and rebuilds.

## Deployment

### Automatic Deployment (GitHub Actions)

The documentation is automatically deployed when:

1. **Production** (https://doc.terraphim.ai):
   - Push to `main` branch with changes in `docs/`

2. **Preview**:
   - Pull requests with changes in `docs/`
   - Preview URL is automatically commented on the PR

3. **Manual**:
   - Trigger via GitHub Actions workflow dispatch

### Manual Deployment

Use the deployment script:

```bash
# Preview deployment
./scripts/deploy-docs.sh preview

# Production deployment
./scripts/deploy-docs.sh production
```

Or use Wrangler directly:

```bash
# Build
cd docs
mdbook build

# Deploy to preview
wrangler pages deploy book --project-name=terraphim-docs --branch=preview

# Deploy to production
wrangler pages deploy book --project-name=terraphim-docs --branch=main
```

## Configuration

### wrangler.toml

The `docs/wrangler.toml` file configures:

- Project name and compatibility
- Security headers
- Cache control rules
- Redirects

### book.toml

The `docs/book.toml` file configures mdBook:

- Book metadata (title, authors)
- Output settings
- Preprocessors (Mermaid)
- Additional JavaScript files

## Custom Domain Setup

1. Go to Cloudflare Dashboard > Pages > terraphim-docs > Custom domains
2. Add `doc.terraphim.ai`
3. Follow DNS configuration instructions
4. SSL certificate is automatically provisioned

## Monitoring

### Cloudflare Dashboard

- **Analytics**: View page views, unique visitors, bandwidth
- **Functions**: Monitor any edge functions
- **Deployments**: View deployment history and rollback if needed

### GitHub Actions

- Check workflow runs in Actions tab
- View deployment summaries
- Debug failed deployments

## Troubleshooting

### Build Failures

1. **Missing preprocessor**:
   ```bash
   cargo install mdbook-mermaid
   mdbook-mermaid install docs/
   ```

2. **Broken links**:
   ```bash
   cd docs
   mdbook build 2>&1 | grep "ERROR"
   ```

3. **Syntax errors in SUMMARY.md**:
   Verify all links in `docs/src/SUMMARY.md` are valid.

### Deployment Failures

1. **Authentication issues**:
   ```bash
   wrangler login
   ```

2. **Missing secrets**: Ensure GitHub secrets are configured

3. **Project doesn't exist**: Create the project first in Cloudflare Dashboard

### Cache Issues

Clear the Cloudflare cache:
```bash
# Via API
curl -X POST "https://api.cloudflare.com/client/v4/zones/ZONE_ID/purge_cache" \
  -H "Authorization: Bearer API_TOKEN" \
  -H "Content-Type: application/json" \
  --data '{"purge_everything":true}'

# Via Dashboard
# Cloudflare Dashboard > Caching > Configuration > Purge Everything
```

## Migration from Netlify

The migration involves:

1. **Removing Netlify configuration** (if any)
2. **Adding Cloudflare Pages configuration** (wrangler.toml)
3. **Setting up GitHub Actions** (deploy-docs.yml)
4. **Configuring DNS** (point doc.terraphim.ai to Cloudflare)
5. **Testing** deployment pipeline

## File Structure

```
terraphim-ai/
├── docs/
│   ├── book.toml              # mdBook configuration
│   ├── wrangler.toml          # Cloudflare Pages configuration
│   ├── CLOUDFLARE_DEPLOYMENT.md  # This file
│   └── src/
│       ├── SUMMARY.md         # Table of contents
│       └── *.md               # Documentation pages
├── scripts/
│   └── deploy-docs.sh         # Manual deployment script
└── .github/
    └── workflows/
        └── deploy-docs.yml    # CI/CD workflow
```

## Security Considerations

- API tokens should have minimal permissions (Pages Edit only)
- Use GitHub Environments with protection rules for production
- Review preview deployments before merging to main
- Security headers are configured in wrangler.toml

## Performance Optimization

- Static assets (CSS, JS, images) cached for 1 year
- HTML pages have `must-revalidate` for instant updates
- Cloudflare's global CDN provides low-latency access
- Automatic compression (gzip, brotli)

## Cost

Cloudflare Pages is free for:
- Unlimited sites
- Unlimited bandwidth
- 500 builds per month
- 1 concurrent build (free tier)

This is more generous than Netlify's free tier for most use cases.
