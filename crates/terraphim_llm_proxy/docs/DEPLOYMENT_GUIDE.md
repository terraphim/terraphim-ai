# Production Deployment Guide - Terraphim LLM Proxy

**Version:** 0.1.0
**Phase:** 1 (MVP)
**Status:** Ready for deployment validation
**Last Updated:** 2025-10-12

---

## Overview

This guide provides step-by-step instructions for deploying the Terraphim LLM Proxy to production environments.

---

## Prerequisites

### System Requirements

**Minimum:**
- OS: Linux (Ubuntu 20.04+, Debian 11+, or similar)
- CPU: 2 cores
- RAM: 512 MB
- Disk: 100 MB
- Network: Internet access for API providers

**Recommended:**
- OS: Linux (Ubuntu 22.04 LTS)
- CPU: 4 cores
- RAM: 2 GB
- Disk: 1 GB
- Network: Low-latency connection

### Software Requirements

- Rust 1.70+ (for building from source)
- systemd (for service management)
- curl (for testing)
- jq (for JSON parsing in tests)

**Optional:**
- Ollama (for local model support)
- Docker (alternative deployment method)
- Nginx/Caddy (reverse proxy)

---

## Installation

### Option 1: Build from Source (Recommended)

```bash
# Clone repository
cd /opt
git clone https://github.com/terraphim/terraphim-llm-proxy.git
cd terraphim-llm-proxy

# Build release binary
cargo build --release

# Binary location
ls -lh target/release/terraphim-llm-proxy
# Expected: ~15 MB executable

# Verify build
./target/release/terraphim-llm-proxy --version
# Expected: terraphim-llm-proxy 0.1.0
```

### Option 2: Pre-built Binary (Future)

```bash
# Download latest release
wget https://github.com/terraphim/terraphim-llm-proxy/releases/latest/terraphim-llm-proxy

# Make executable
chmod +x terraphim-llm-proxy

# Move to system path
sudo mv terraphim-llm-proxy /usr/local/bin/
```

---

## Configuration

### 1. Create Configuration Directory

```bash
sudo mkdir -p /etc/terraphim-llm-proxy
sudo mkdir -p /var/log/terraphim-llm-proxy
```

### 2. Create Production Configuration

**File:** `/etc/terraphim-llm-proxy/config.toml`

```toml
[proxy]
host = "127.0.0.1"  # Localhost only (use reverse proxy for external access)
port = 3456
api_key = "$PROXY_API_KEY"  # Load from environment
timeout_ms = 600000  # 10 minutes

[router]
# Production routing configuration
default = "deepseek,deepseek-chat"
background = "ollama,qwen2.5-coder:latest"
think = "deepseek,deepseek-reasoner"
long_context = "openrouter,google/gemini-2.0-flash-exp"
long_context_threshold = 60000
web_search = "openrouter,perplexity/llama-3.1-sonar-large-128k-online"
image = "openrouter,anthropic/claude-3.5-sonnet"

[security.rate_limiting]
enabled = true
requests_per_minute = 60
concurrent_requests = 10

[security.ssrf_protection]
enabled = true
allow_localhost = false  # Security: block localhost in production
allow_private_ips = false  # Security: block private IPs

[[providers]]
name = "deepseek"
api_base_url = "https://api.deepseek.com/chat/completions"
api_key = "$DEEPSEEK_API_KEY"
models = ["deepseek-chat", "deepseek-reasoner"]
transformers = ["deepseek"]

[[providers]]
name = "ollama"
api_base_url = "http://localhost:11434/v1/chat/completions"
api_key = "ollama"
models = ["qwen2.5-coder:latest"]
transformers = ["ollama"]

[[providers]]
name = "openrouter"
api_base_url = "https://openrouter.ai/api/v1/chat/completions"
api_key = "$OPENROUTER_API_KEY"
models = [
    "google/gemini-2.0-flash-exp",
    "anthropic/claude-3.5-sonnet",
    "perplexity/llama-3.1-sonar-large-128k-online"
]
transformers = ["openrouter"]
```

### 3. Configure Environment Variables

**File:** `/etc/terraphim-llm-proxy/environment`

```bash
# Proxy API key (generate a secure key)
PROXY_API_KEY=sk_prod_<generate_32_character_random_string>

# Provider API keys
DEEPSEEK_API_KEY=sk-your-deepseek-api-key
OPENROUTER_API_KEY=sk-or-v1-your-openrouter-key

# Logging
RUST_LOG=info

# Optional: Ollama URL if not default
# OLLAMA_BASE_URL=http://localhost:11434
```

**Security:** Ensure file is only readable by proxy user:
```bash
sudo chmod 600 /etc/terraphim-llm-proxy/environment
sudo chown terraphim-proxy:terraphim-proxy /etc/terraphim-llm-proxy/environment
```

---

## User Setup

### Create Dedicated User

```bash
# Create system user for running the proxy
sudo useradd -r -s /bin/false -d /opt/terraphim-llm-proxy terraphim-proxy

# Set ownership
sudo chown -R terraphim-proxy:terraphim-proxy /opt/terraphim-llm-proxy
sudo chown -R terraphim-proxy:terraphim-proxy /var/log/terraphim-llm-proxy
```

---

## Systemd Service

### Create Service File

**File:** `/etc/systemd/system/terraphim-llm-proxy.service`

```ini
[Unit]
Description=Terraphim LLM Proxy
Documentation=https://github.com/terraphim/terraphim-llm-proxy
After=network.target

[Service]
Type=simple
User=terraphim-proxy
Group=terraphim-proxy
WorkingDirectory=/opt/terraphim-llm-proxy

# Load environment variables
EnvironmentFile=/etc/terraphim-llm-proxy/environment

# Start proxy
ExecStart=/opt/terraphim-llm-proxy/target/release/terraphim-llm-proxy \
    --config /etc/terraphim-llm-proxy/config.toml \
    --log-json

# Restart on failure
Restart=on-failure
RestartSec=10s

# Resource limits
LimitNOFILE=65536
MemoryMax=1G
CPUQuota=200%

# Logging
StandardOutput=append:/var/log/terraphim-llm-proxy/proxy.log
StandardError=append:/var/log/terraphim-llm-proxy/error.log

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/log/terraphim-llm-proxy

[Install]
WantedBy=multi-user.target
```

### Enable and Start Service

```bash
# Reload systemd
sudo systemctl daemon-reload

# Enable service (start on boot)
sudo systemctl enable terraphim-llm-proxy

# Start service
sudo systemctl start terraphim-llm-proxy

# Check status
sudo systemctl status terraphim-llm-proxy

# View logs
sudo journalctl -u terraphim-llm-proxy -f
```

---

## Reverse Proxy Setup

### Option 1: Nginx

**File:** `/etc/nginx/sites-available/terraphim-llm-proxy`

```nginx
upstream llm_proxy {
    server 127.0.0.1:3456;
    keepalive 32;
}

server {
    listen 443 ssl http2;
    server_name llm-proxy.example.com;

    # SSL configuration
    ssl_certificate /etc/letsencrypt/live/llm-proxy.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/llm-proxy.example.com/privkey.pem;

    # Security headers
    add_header Strict-Transport-Security "max-age=31536000" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header X-Frame-Options "DENY" always;

    # Proxy settings
    location / {
        proxy_pass http://llm_proxy;
        proxy_http_version 1.1;
        proxy_set_header Connection "";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # Timeouts for long requests
        proxy_connect_timeout 10s;
        proxy_send_timeout 600s;
        proxy_read_timeout 600s;

        # Buffer settings for SSE
        proxy_buffering off;
        proxy_cache off;
    }

    # Access logging
    access_log /var/log/nginx/llm-proxy-access.log;
    error_log /var/log/nginx/llm-proxy-error.log;
}
```

**Enable:**
```bash
sudo ln -s /etc/nginx/sites-available/terraphim-llm-proxy /etc/nginx/sites-enabled/
sudo nginx -t
sudo systemctl reload nginx
```

### Option 2: Caddy (Simpler)

**File:** `/etc/caddy/Caddyfile`

```caddy
llm-proxy.example.com {
    reverse_proxy 127.0.0.1:3456 {
        # Timeouts for long requests
        timeout 600s

        # Health check
        health_uri /health
        health_interval 30s
    }

    # Logging
    log {
        output file /var/log/caddy/llm-proxy.log {
            roll_size 10MiB
            roll_keep 10
        }
    }

    # TLS (automatic with Let's Encrypt)
    tls {
        protocols tls1.2 tls1.3
    }
}
```

**Reload:**
```bash
sudo systemctl reload caddy
```

---

## Optional: Ollama Setup (For Local Models)

### Install Ollama

```bash
# Install Ollama
curl -fsSL https://ollama.ai/install.sh | sh

# Start Ollama service
sudo systemctl start ollama

# Pull required model
ollama pull qwen2.5-coder:latest

# Verify
ollama list
```

### Configure Ollama for Proxy

```bash
# Ensure Ollama is accessible on localhost:11434
curl http://localhost:11434/api/tags

# Expected: JSON with list of models
```

---

## Validation

### 1. Service Health Check

```bash
# Check service status
sudo systemctl status terraphim-llm-proxy

# Expected: active (running)
```

### 2. API Health Check

```bash
# Direct connection
curl http://localhost:3456/health
# Expected: OK

# Through reverse proxy (if configured)
curl https://llm-proxy.example.com/health
# Expected: OK
```

### 3. Functional Test

```bash
# Set API key
export PROXY_API_KEY="your-production-proxy-api-key"

# Test token counting
curl -X POST http://localhost:3456/v1/messages/count_tokens \
  -H "x-api-key: $PROXY_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-3-5-sonnet-20241022",
    "messages": [{"role": "user", "content": "Hello!"}]
  }'

# Expected: {"input_tokens":6}
```

### 4. Routing Validation

```bash
# Enable debug logging temporarily
sudo systemctl stop terraphim-llm-proxy
sudo RUST_LOG=debug /opt/terraphim-llm-proxy/target/release/terraphim-llm-proxy \
    --config /etc/terraphim-llm-proxy/config.toml

# Send test request (in another terminal)
curl -X POST http://localhost:3456/v1/messages \
  -H "x-api-key: $PROXY_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-3-5-haiku-20241022",
    "messages": [{"role": "user", "content": "Quick task"}],
    "stream": false
  }'

# Check logs for routing decision
# Expected: "Routing decision: provider=ollama, model=qwen2.5-coder:latest, scenario=Background"

# Restart service
sudo systemctl start terraphim-llm-proxy
```

---

## Monitoring

### Log Files

**Systemd logs:**
```bash
# View all logs
sudo journalctl -u terraphim-llm-proxy

# Follow logs in real-time
sudo journalctl -u terraphim-llm-proxy -f

# Last 100 lines
sudo journalctl -u terraphim-llm-proxy -n 100

# Filter by priority
sudo journalctl -u terraphim-llm-proxy -p err
```

**Application logs:**
```bash
# View proxy logs
sudo tail -f /var/log/terraphim-llm-proxy/proxy.log

# View error logs
sudo tail -f /var/log/terraphim-llm-proxy/error.log

# Search for routing decisions
sudo grep "Routing decision" /var/log/terraphim-llm-proxy/proxy.log

# Search for errors
sudo grep -i error /var/log/terraphim-llm-proxy/error.log
```

### Key Metrics to Monitor

**Request Metrics:**
- Total requests per minute
- Requests per routing scenario
- Token usage per provider
- Response latency (P50, P95, P99)

**Error Metrics:**
- Authentication failures
- Provider errors
- Timeout errors
- Rate limit hits

**Resource Metrics:**
- Memory usage (target: <500 MB)
- CPU usage (target: <50%)
- Network bandwidth
- Connection count

### Log Rotation

**File:** `/etc/logrotate.d/terraphim-llm-proxy`

```
/var/log/terraphim-llm-proxy/*.log {
    daily
    rotate 30
    compress
    delaycompress
    notifempty
    create 0640 terraphim-proxy terraphim-proxy
    sharedscripts
    postrotate
        systemctl reload terraphim-llm-proxy > /dev/null 2>&1 || true
    endscript
}
```

---

## Security Hardening

### 1. Firewall Configuration

```bash
# Allow only from specific IPs (if applicable)
sudo ufw allow from 10.0.0.0/8 to any port 3456 proto tcp

# Or allow from localhost only (with reverse proxy)
sudo ufw deny 3456
```

### 2. File Permissions

```bash
# Configuration files
sudo chmod 600 /etc/terraphim-llm-proxy/config.toml
sudo chmod 600 /etc/terraphim-llm-proxy/environment
sudo chown -R terraphim-proxy:terraphim-proxy /etc/terraphim-llm-proxy

# Log directory
sudo chmod 750 /var/log/terraphim-llm-proxy
sudo chown -R terraphim-proxy:terraphim-proxy /var/log/terraphim-llm-proxy

# Binary
sudo chmod 755 /opt/terraphim-llm-proxy/target/release/terraphim-llm-proxy
```

### 3. API Key Security

**Generate secure API key:**
```bash
# Generate 32-byte random key, base64 encoded
openssl rand -base64 32 | tr -d '\n' && echo
# Example: sk_prod_gX9kP2mQ8vL5nR7jW4tY6hB3fC1dE0sA9zK8xM7u
```

**Store securely:**
- Use environment files with restricted permissions (600)
- Consider using secrets management (HashiCorp Vault, AWS Secrets Manager)
- Rotate keys regularly (every 90 days)

---

## High Availability Setup

### Multi-Instance Deployment

**Load Balancer Configuration (Nginx):**

```nginx
upstream llm_proxy_cluster {
    least_conn;  # Route to least busy instance
    server 10.0.1.10:3456 max_fails=3 fail_timeout=30s;
    server 10.0.1.11:3456 max_fails=3 fail_timeout=30s;
    server 10.0.1.12:3456 max_fails=3 fail_timeout=30s;
    keepalive 32;
}

server {
    listen 443 ssl http2;
    server_name llm-proxy.example.com;

    location / {
        proxy_pass http://llm_proxy_cluster;
        # ... rest of proxy config
    }
}
```

### Health Checks

**Endpoint:** `GET /health`

**Load balancer health check:**
```bash
# Add to cron for monitoring
*/1 * * * * curl -sf http://localhost:3456/health || systemctl restart terraphim-llm-proxy
```

---

## Backup and Recovery

### Configuration Backup

```bash
# Backup script
#!/bin/bash
BACKUP_DIR="/var/backups/terraphim-llm-proxy"
DATE=$(date +%Y%m%d-%H%M%S)

mkdir -p $BACKUP_DIR
tar -czf $BACKUP_DIR/config-$DATE.tar.gz \
    /etc/terraphim-llm-proxy/config.toml \
    /etc/terraphim-llm-proxy/environment

# Keep last 30 days
find $BACKUP_DIR -name "config-*.tar.gz" -mtime +30 -delete
```

### Restore Configuration

```bash
# Stop service
sudo systemctl stop terraphim-llm-proxy

# Restore from backup
tar -xzf /var/backups/terraphim-llm-proxy/config-YYYYMMDD-HHMMSS.tar.gz -C /

# Verify configuration
/opt/terraphim-llm-proxy/target/release/terraphim-llm-proxy \
    --config /etc/terraphim-llm-proxy/config.toml 2>&1 | head -10

# Restart service
sudo systemctl start terraphim-llm-proxy
```

---

## Troubleshooting

### Service Won't Start

**Check logs:**
```bash
sudo journalctl -u terraphim-llm-proxy -n 50
```

**Common issues:**
1. **Port already in use**
   ```bash
   sudo lsof -i :3456
   # Kill conflicting process or change port
   ```

2. **Configuration error**
   ```bash
   # Validate config manually
   /opt/terraphim-llm-proxy/target/release/terraphim-llm-proxy \
       --config /etc/terraphim-llm-proxy/config.toml
   ```

3. **Missing environment variables**
   ```bash
   # Check environment file
   sudo cat /etc/terraphim-llm-proxy/environment
   # Ensure all required variables are set
   ```

4. **Permission denied**
   ```bash
   # Check file ownership
   ls -la /etc/terraphim-llm-proxy
   ls -la /opt/terraphim-llm-proxy/target/release/terraphim-llm-proxy
   ```

### Provider Errors

**Check provider connectivity:**
```bash
# DeepSeek
curl https://api.deepseek.com/v1/models

# OpenRouter
curl -H "Authorization: Bearer $OPENROUTER_API_KEY" \
    https://openrouter.ai/api/v1/models

# Ollama (if using)
curl http://localhost:11434/api/tags
```

### High Memory Usage

**Check memory:**
```bash
# Current usage
sudo systemctl status terraphim-llm-proxy | grep Memory

# Detailed memory info
sudo ps aux | grep terraphim-llm-proxy
```

**If memory usage high:**
1. Check for memory leaks in logs
2. Reduce concurrent_requests in config
3. Restart service periodically (e.g., daily)

---

## Performance Tuning

### 1. Optimize for Latency

```toml
[router]
# Use faster models for default
default = "deepseek,deepseek-chat"  # Fast and cheap

# Use local Ollama for background tasks
background = "ollama,qwen2.5-coder:latest"  # No network latency

[security.rate_limiting]
# Increase concurrent requests
concurrent_requests = 50
```

### 2. Optimize for Throughput

```toml
[proxy]
# Increase timeout for high-concurrency scenarios
timeout_ms = 300000  # 5 minutes

[security.rate_limiting]
requests_per_minute = 300  # Higher throughput
concurrent_requests = 100
```

### 3. Optimize for Cost

```toml
[router]
# Use cheapest models where possible
default = "deepseek,deepseek-chat"  # $0.14/1M tokens
background = "ollama,qwen2.5-coder:latest"  # Free

# Reserve expensive models for specific needs only
image = "openrouter,anthropic/claude-3.5-sonnet"
```

---

## Updates and Maintenance

### Updating the Proxy

```bash
# Stop service
sudo systemctl stop terraphim-llm-proxy

# Backup current binary
sudo cp /opt/terraphim-llm-proxy/target/release/terraphim-llm-proxy \
    /opt/terraphim-llm-proxy/target/release/terraphim-llm-proxy.backup

# Pull latest code
cd /opt/terraphim-llm-proxy
sudo -u terraphim-proxy git pull

# Build new version
sudo -u terraphim-proxy cargo build --release

# Test new binary
./target/release/terraphim-llm-proxy --version
./target/release/terraphim-llm-proxy --config /etc/terraphim-llm-proxy/config.toml &
sleep 2
curl http://localhost:3456/health
pkill terraphim-llm-proxy

# Start service
sudo systemctl start terraphim-llm-proxy

# Verify
sudo systemctl status terraphim-llm-proxy
curl http://localhost:3456/health
```

### Configuration Updates

```bash
# Edit configuration
sudo nano /etc/terraphim-llm-proxy/config.toml

# Validate configuration
sudo -u terraphim-proxy /opt/terraphim-llm-proxy/target/release/terraphim-llm-proxy \
    --config /etc/terraphim-llm-proxy/config.toml &
sleep 2
pkill terraphim-llm-proxy

# Reload service (graceful restart)
sudo systemctl reload terraphim-llm-proxy
```

---

## Disaster Recovery

### Emergency Procedures

**1. Service Down:**
```bash
# Quick restart
sudo systemctl restart terraphim-llm-proxy

# If still failing, check logs
sudo journalctl -u terraphim-llm-proxy -n 100

# Fallback: Use backup binary
sudo systemctl stop terraphim-llm-proxy
sudo cp /opt/terraphim-llm-proxy/target/release/terraphim-llm-proxy.backup \
    /opt/terraphim-llm-proxy/target/release/terraphim-llm-proxy
sudo systemctl start terraphim-llm-proxy
```

**2. Configuration Corruption:**
```bash
# Restore from backup
sudo cp /var/backups/terraphim-llm-proxy/config-LATEST.tar.gz /tmp/
cd /tmp
tar -xzf config-LATEST.tar.gz
sudo cp etc/terraphim-llm-proxy/config.toml /etc/terraphim-llm-proxy/
sudo systemctl restart terraphim-llm-proxy
```

**3. Provider API Down:**
```bash
# Temporarily change routing to use alternative provider
sudo nano /etc/terraphim-llm-proxy/config.toml

# Change default provider
[router]
default = "ollama,llama3.2:3b"  # Fallback to local

# Reload
sudo systemctl reload terraphim-llm-proxy
```

---

## Deployment Checklist

### Pre-Deployment

- [ ] Binary built successfully (`cargo build --release`)
- [ ] All tests passing (`cargo test`)
- [ ] Configuration file created and validated
- [ ] Environment variables set securely
- [ ] API keys obtained and tested
- [ ] User account created (terraphim-proxy)
- [ ] File permissions set correctly
- [ ] Systemd service file created

### Deployment

- [ ] Binary copied to /opt/terraphim-llm-proxy
- [ ] Configuration copied to /etc/terraphim-llm-proxy
- [ ] Service enabled (`systemctl enable`)
- [ ] Service started (`systemctl start`)
- [ ] Service status verified (active running)
- [ ] Health endpoint responding
- [ ] Logs show successful startup

### Post-Deployment

- [ ] Reverse proxy configured (if external access needed)
- [ ] SSL/TLS certificates configured
- [ ] Firewall rules configured
- [ ] Monitoring configured
- [ ] Log rotation configured
- [ ] Backup script configured
- [ ] Documentation updated with deployment details
- [ ] Team trained on operations

### Validation

- [ ] Health check passing
- [ ] Token counting accurate
- [ ] All routing scenarios tested
- [ ] Authentication working
- [ ] Error handling verified
- [ ] Performance acceptable
- [ ] Logs clean (no errors)
- [ ] Resource usage normal

---

## Support and Maintenance

### Regular Maintenance Tasks

**Daily:**
- Check service status
- Review error logs
- Monitor resource usage

**Weekly:**
- Review request statistics
- Check provider API status
- Verify backup completion

**Monthly:**
- Update dependencies (`cargo update`)
- Rotate API keys
- Review and optimize configuration
- Test disaster recovery procedures

### Getting Help

**Documentation:**
- README.md - Quick start
- CLAUDE_CODE_SETUP.md - Setup guide
- E2E_TESTING_GUIDE.md - Testing procedures
- STATUS.md - Current status

**Logs:**
- Service logs: `journalctl -u terraphim-llm-proxy`
- Application logs: `/var/log/terraphim-llm-proxy/`

**Common Issues:**
- See TROUBLESHOOTING.md (to be created)
- Check GitHub issues
- Review SECURITY.md for security concerns

---

## Rollback Procedure

If deployment fails or issues are discovered:

```bash
# Stop new version
sudo systemctl stop terraphim-llm-proxy

# Restore previous binary
sudo cp /opt/terraphim-llm-proxy/target/release/terraphim-llm-proxy.backup \
    /opt/terraphim-llm-proxy/target/release/terraphim-llm-proxy

# Restore previous configuration (if changed)
sudo tar -xzf /var/backups/terraphim-llm-proxy/config-PREVIOUS.tar.gz -C /

# Start service
sudo systemctl start terraphim-llm-proxy

# Verify rollback successful
curl http://localhost:3456/health
sudo journalctl -u terraphim-llm-proxy -n 20
```

---

## Appendix

### A. Example Deployment Script

**File:** `scripts/deploy-production.sh`

```bash
#!/bin/bash
# Production deployment script
set -e

echo "=== Terraphim LLM Proxy Deployment ==="

# Build
echo "Building release binary..."
cargo build --release

# Backup current deployment
echo "Backing up current deployment..."
sudo systemctl stop terraphim-llm-proxy
sudo cp /opt/terraphim-llm-proxy/target/release/terraphim-llm-proxy \
    /opt/terraphim-llm-proxy/target/release/terraphim-llm-proxy.backup

# Deploy new binary
echo "Deploying new binary..."
sudo cp target/release/terraphim-llm-proxy /opt/terraphim-llm-proxy/target/release/

# Start service
echo "Starting service..."
sudo systemctl start terraphim-llm-proxy

# Wait for startup
sleep 3

# Verify
echo "Verifying deployment..."
curl -sf http://localhost:3456/health || {
    echo "Health check failed! Rolling back..."
    sudo systemctl stop terraphim-llm-proxy
    sudo cp /opt/terraphim-llm-proxy/target/release/terraphim-llm-proxy.backup \
        /opt/terraphim-llm-proxy/target/release/terraphim-llm-proxy
    sudo systemctl start terraphim-llm-proxy
    exit 1
}

echo "✅ Deployment successful!"
sudo systemctl status terraphim-llm-proxy
```

### B. Configuration Validation Script

**File:** `scripts/validate-config.sh`

```bash
#!/bin/bash
# Validate configuration before deployment

CONFIG_FILE="${1:-/etc/terraphim-llm-proxy/config.toml}"

echo "Validating configuration: $CONFIG_FILE"

# Test configuration loading
/opt/terraphim-llm-proxy/target/release/terraphim-llm-proxy \
    --config "$CONFIG_FILE" 2>&1 | head -20 | grep -q "Configuration validated successfully"

if [ $? -eq 0 ]; then
    echo "✅ Configuration valid"
    exit 0
else
    echo "❌ Configuration invalid"
    exit 1
fi
```

---

**Deployment Guide Version:** 1.0
**Status:** Ready for production deployment validation
**Next Update:** After Week 4 E2E testing
