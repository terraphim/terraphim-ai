# DNS Migration Guide: Terraphim.ai from Netlify to Cloudflare

## Current DNS Configuration

### Netlify Setup
- **Domain**: terraphim.ai
- **Nameservers**: Netlify's nameservers
- **A Records**: Point to Netlify's load balancers
- **CNAME**: www.terraphim.ai → Netlify
- **SSL**: Managed by Netlify

## Target DNS Configuration

### Cloudflare Pages Setup
- **Domain**: terraphim.ai
- **Nameservers**: Cloudflare's nameservers
- **A Records**: Point to Cloudflare Pages
- **CNAME**: www.terraphim.ai → Cloudflare Pages
- **SSL**: Managed by Cloudflare

## Migration Steps

### Phase 1: Preparation

#### 1.1 Current DNS Analysis
```bash
# Check current nameservers
dig NS terraphim.ai

# Check current A records
dig A terraphim.ai

# Check current CNAME
dig CNAME www.terraphim.ai

# Check SSL certificate
openssl s_client -connect terraphim.ai:443 -servername terraphim.ai
```

#### 1.2 Document Current Records
Create a backup of current DNS settings:

| Record Type | Name | Value | TTL |
|-------------|------|-------|-----|
| A | @ | Netlify IP | 300 |
| CNAME | www | netlify.app | 300 |
| MX | @ | mail.terraphim.ai | 300 |
| TXT | @ | Various verification | 300 |

### Phase 2: Cloudflare Setup

#### 2.1 Add Domain to Cloudflare
1. Log in to Cloudflare Dashboard
2. Add domain: `terraphim.ai`
3. Choose plan (Free is sufficient)
4. Scan existing DNS records
5. Update nameservers to Cloudflare

#### 2.2 Cloudflare Nameservers
After adding domain to Cloudflare, you'll get nameservers like:
- `dina.ns.cloudflare.com`
- `jim.ns.cloudflare.com`

#### 2.3 DNS Record Configuration
Once nameservers are updated, configure these records:

```
# A Records (for root domain)
A    @    192.0.2.1    # Cloudflare Pages IP
A    @    192.0.2.2    # Cloudflare Pages IP
A    @    192.0.2.3    # Cloudflare Pages IP

# CNAME Records
CNAME    www    terraphim-ai.pages.dev

# MX Records (if email is used)
MX    @    10    mail.terraphim.ai

# TXT Records
TXT    @    "v=spf1 include:_spf.google.com ~all"
TXT    @    "google-site-verification=..."
```

### Phase 3: Cloudflare Pages Configuration

#### 3.1 Custom Domain Setup
1. Go to Cloudflare Pages > terraphim-ai
2. Click "Custom domains"
3. Add `terraphim.ai`
4. Add `www.terraphim.ai`
5. Wait for DNS verification

#### 3.2 SSL Certificate
- Cloudflare automatically provisions SSL certificate
- Usually takes 5-10 minutes
- Certificate is valid for 1 year and auto-renews

### Phase 4: Migration Execution

#### 4.1 Pre-Migration Checklist
- [ ] Backup current DNS records
- [ ] Verify Cloudflare account access
- [ ] Test Cloudflare Pages deployment
- [ ] Prepare rollback plan
- [ ] Schedule maintenance window

#### 4.2 Migration Timeline
```
T-2 hours:  Final verification of all configurations
T-1 hour:  Notify users of scheduled maintenance
T-0:       Update nameservers to Cloudflare
T+5 min:   Verify nameserver propagation
T+15 min:  Check DNS resolution
T+30 min:  Verify SSL certificate
T+1 hour:  Test website functionality
T+2 hours: Monitor performance and analytics
T+24 hours: Delete Netlify project (if stable)
```

#### 4.3 Migration Commands
```bash
# Monitor nameserver propagation
watch dig NS terraphim.ai

# Check A record resolution
watch dig A terraphim.ai

# Test website accessibility
curl -I https://terraphim.ai

# Check SSL certificate
openssl s_client -connect terraphim.ai:443 -servername terraphim.ai
```

### Phase 5: Post-Migration

#### 5.1 Verification Tests
```bash
# Test all pages
curl -s https://terraphim.ai | grep -i "title"

# Test static assets
curl -I https://terraphim.ai/static/css/style.css

# Test navigation
curl -s https://terraphim.ai/posts | grep -i "title"

# Test forms (if any)
curl -X POST https://terraphim.ai/ -d "test=data"
```

#### 5.2 Performance Monitoring
- Cloudflare Analytics
- Google PageSpeed Insights
- GTmetrix performance tests
- Uptime monitoring

#### 5.3 SEO Considerations
- Verify all URLs are the same
- Check Google Search Console
- Monitor for 404 errors
- Verify sitemap accessibility

## Rollback Plan

### Immediate Rollback (if issues within 24 hours)
1. Revert nameservers to Netlify
2. Restore original DNS records
3. Verify website is accessible
4. Investigate Cloudflare issues

### Rollback Commands
```bash
# Revert nameservers (via domain registrar)
# Update back to Netlify nameservers

# Verify rollback
dig NS terraphim.ai
dig A terraphim.ai
curl -I https://terraphim.ai
```

## Troubleshooting

### Common Issues

#### DNS Propagation Delays
- **Issue**: Nameserver changes taking too long
- **Solution**: Wait up to 48 hours for full propagation
- **Check**: Use multiple DNS lookup tools

#### SSL Certificate Issues
- **Issue**: Certificate not provisioning
- **Solution**: Check DNS records, ensure CNAME is correct
- **Force**: Re-issue certificate in Cloudflare dashboard

#### Website Not Loading
- **Issue**: 404 errors or connection refused
- **Solution**: Verify Cloudflare Pages deployment
- **Check**: Build logs and deployment status

#### Performance Issues
- **Issue**: Slow load times
- **Solution**: Check Cloudflare caching rules
- **Optimize**: Enable Cloudflare features (Brotli, HTTP/2)

### Monitoring Commands
```bash
# Continuous monitoring
while true; do
    echo "$(date): Checking website..."
    curl -s -o /dev/null -w "%{http_code}" https://terraphim.ai
    echo ""
    sleep 60
done

# DNS propagation check
for ns in 8.8.8.8 1.1.1.1 208.67.222.222; do
    echo "Querying $ns:"
    dig @$ns A terraphim.ai
    echo ""
done
```

## Success Metrics

### Technical Metrics
- **DNS Propagation**: <2 hours
- **SSL Provisioning**: <15 minutes
- **Website Availability**: 99.9%+
- **Load Time**: <2 seconds globally

### Business Metrics
- **Zero Downtime**: During migration
- **SEO Stability**: No ranking changes
- **User Experience**: No reported issues
- **Performance Improvement**: 20%+ faster load times

## Maintenance

### Ongoing Tasks
- Monitor SSL certificate renewal
- Update DNS records as needed
- Optimize Cloudflare caching rules
- Regular performance audits

### Security Considerations
- Enable Cloudflare security features
- Monitor for DDoS attacks
- Keep DNS records updated
- Regular security audits

---

*This DNS migration guide ensures a smooth transition from Netlify to Cloudflare Pages while maintaining website availability and performance.*