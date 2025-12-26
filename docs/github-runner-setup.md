# Terraphim GitHub Runner - Setup Guide

Complete guide for setting up and deploying the Terraphim GitHub Runner.

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Installation](#installation)
3. [Configuration](#configuration)
4. [GitHub Integration](#github-integration)
5. [Firecracker Setup](#firecracker-setup)
6. [LLM Configuration](#llm-configuration)
7. [Testing](#testing)
8. [Deployment](#deployment)
9. [Troubleshooting](#troubleshooting)

## Prerequisites

### System Requirements

- **OS**: Linux (Ubuntu 20.04+ recommended)
- **RAM**: 4GB+ minimum
- **CPU**: 2+ cores recommended
- **Disk**: 10GB+ free space

### Software Dependencies

```bash
# Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Firecracker (via fcctl-web)
# See Firecracker Setup section below

# Ollama (optional, for LLM features)
curl -fsSL https://ollama.com/install.sh | sh

# GitHub CLI (optional, for setup)
curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg | sudo dd of=/usr/share/keyrings/githubcli-archive-keyring.gpg
echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" | sudo tee /etc/apt/sources.list.d/github-cli.list > /dev/null
sudo apt update
sudo apt install gh
```

## Installation

### 1. Clone Repository

```bash
git clone https://github.com/terraphim/terraphim-ai.git
cd terraphim-ai
```

### 2. Build Server

```bash
# Build with Ollama support (recommended)
cargo build --release -p terraphim_github_runner_server --features ollama

# Or build without LLM features
cargo build --release -p terraphim_github_runner_server
```

### 3. Verify Installation

```bash
./target/release/terraphim_github_runner_server --version
```

## Configuration

### Environment Variables

Create `/etc/terraphim/github-runner.env`:

```bash
# Server Configuration
PORT=3000
HOST=0.0.0.0

# GitHub Integration
GITHUB_WEBHOOK_SECRET=your_webhook_secret_here
GITHUB_TOKEN=ghp_your_github_token_here

# Firecracker Integration
FIRECRACKER_API_URL=http://127.0.0.1:8080
FIRECRACKER_AUTH_TOKEN=your_jwt_token_here

# LLM Configuration
USE_LLM_PARSER=true
OLLAMA_BASE_URL=http://127.0.0.1:11434
OLLAMA_MODEL=gemma3:4b

# Repository
REPOSITORY_PATH=/var/lib/terraphim/repos
```

### Load Environment

```bash
source /etc/terraphim/github-runner.env
```

## GitHub Integration

### 1. Create Webhook Secret

```bash
# Generate secure secret
openssl rand -hex 32
```

### 2. Configure GitHub Repository

```bash
# Set webhook
gh api repos/OWNER/REPO/hooks \
  --method POST \
  -f name=terraphim-runner \
  -f active=true \
  -f events='[pull_request,push]' \
  -f config='{
    "url": "https://your-server.com/webhook",
    "content_type": "json",
    "secret": "YOUR_WEBHOOK_SECRET",
    "insecure_ssl": false
  }'
```

### 3. Create Test Workflow

Create `.github/workflows/test.yml`:

```yaml
name: Terraphim Test

on:
  pull_request:
    branches: [ main ]
  push:
    branches: [ main ]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - name: Environment
        run: |
          echo "Running in Terraphim Firecracker VM"
          uname -a

      - name: List Workspace
        run: ls -la /workspace

      - name: Run Commands
        run: |
          echo "✓ Step 1 passed"
          echo "✓ Step 2 passed"
```

## Firecracker Setup

### Option 1: Using fcctl-web (Recommended)

```bash
# Clone fcctl-web
git clone https://github.com/firecracker-microvm/fcctl-web.git
cd fcctl-web

# Build and run
cargo build --release
./target/release/fcctl-web \
  --firecracker-binary /usr/bin/firecracker \
  --socket-path /tmp/fcctl-web.sock \
  --api-socket /tmp/fcctl-web-api.sock
```

### Option 2: Direct Firecracker

```bash
# Install Firecracker
wget https://github.com/firecracker-microvm/firecracker/releases/download/v1.5.0/firecracker-v1.5.0
chmod +x firecracker-v1.5.0
sudo mv firecracker-v1.5.0 /usr/local/bin/firecracker

# Test Firecracker
firecracker --version
```

### Verify Firecracker API

```bash
curl http://127.0.0.1:8080/health
```

Expected response:
```json
{"status":"ok"}
```

## LLM Configuration

### Option 1: Ollama (Local, Free)

```bash
# Install Ollama
curl -fsSL https://ollama.com/install.sh | sh

# Start Ollama service
ollama serve &

# Pull model
ollama pull gemma3:4b

# Verify
ollama list
```

### Option 2: OpenRouter (Cloud, Paid)

```bash
# Get API key from https://openrouter.ai/keys

# Configure environment
export OPENROUTER_API_KEY=sk-your-key-here
export OPENROUTER_MODEL=openai/gpt-3.5-turbo
```

### Test LLM Integration

```bash
# Start server with LLM
USE_LLM_PARSER=true \
OLLAMA_BASE_URL=http://127.0.0.1:11434 \
OLLAMA_MODEL=gemma3:4b \
./target/release/terraphim_github_runner_server
```

## Testing

### 1. Start Server

```bash
GITHUB_WEBHOOK_SECRET=test_secret \
FIRECRACKER_API_URL=http://127.0.0.1:8080 \
USE_LLM_PARSER=true \
OLLAMA_BASE_URL=http://127.0.0.1:11434 \
OLLAMA_MODEL=gemma3:4b \
RUST_LOG=info \
./target/release/terraphim_github_runner_server
```

### 2. Send Test Webhook

```python
import hmac
import hashlib
import json
import subprocess

secret = b"test_secret"
payload = json.dumps({
    "action": "opened",
    "number": 1,
    "repository": {
        "full_name": "test/repo",
        "clone_url": "https://github.com/test/repo.git"
    },
    "pull_request": {
        "title": "Test PR",
        "html_url": "https://github.com/test/repo/pull/1"
    }
}, separators=(',', ':'))

signature = hmac.new(secret, payload.encode(), hashlib.sha256).hexdigest()

result = subprocess.run([
    'curl', '-s', '-X', 'POST', 'http://localhost:3000/webhook',
    '-H', 'Content-Type: application/json',
    '-H', f'X-Hub-Signature-256: sha256={signature}',
    '-d', payload
], capture_output=True, text=True)

print(f"Status: {result.returncode}")
print(f"Response: {result.stdout}")
```

### 3. Check Logs

```bash
# Should show:
# ✅ Webhook received
# 🤖 LLM-based workflow parsing enabled
# 🔧 Initializing Firecracker VM provider
# ⚡ Creating VmCommandExecutor
# 🎯 Creating SessionManager
# Allocated VM fc-vm-<UUID>
# Executing command in Firecracker VM
# Workflow completed successfully
```

## Deployment

### Systemd Service

Create `/etc/systemd/system/terraphim-github-runner.service`:

```ini
[Unit]
Description=Terraphim GitHub Runner Server
After=network.target fcctl-web.service
Requires=fcctl-web.service

[Service]
Type=simple
User=terraphim
Group=terraphim
WorkingDirectory=/opt/terraphim-github-runner
EnvironmentFile=/etc/terraphim/github-runner.env
ExecStart=/opt/terraphim-github-runner/terraphim_github_runner_server
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

Enable and start:

```bash
sudo systemctl daemon-reload
sudo systemctl enable terraphim-github-runner
sudo systemctl start terraphim-github-runner
sudo systemctl status terraphim-github-runner
```

### Docker Deployment

Create `Dockerfile`:

```dockerfile
FROM rust:1.75 as builder

WORKDIR /app
COPY . .

RUN cargo build --release -p terraphim_github_runner_server --features ollama

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/terraphim_github_runner_server /usr/local/bin/

EXPOSE 3000
ENV PORT=3000
ENV HOST=0.0.0.0

ENTRYPOINT ["terraphim_github_runner_server"]
```

Build and run:

```bash
docker build -t terraphim-github-runner .
docker run -d \
  -p 3000:3000 \
  -e GITHUB_WEBHOOK_SECRET=${SECRET} \
  -e FIRECRACKER_API_URL=http://host.docker.internal:8080 \
  terraphim-github-runner
```

### Nginx Reverse Proxy

Create `/etc/nginx/sites-available/terraphim-runner`:

```nginx
server {
    listen 443 ssl http2;
    server_name your-server.com;

    ssl_certificate /etc/ssl/certs/your-cert.pem;
    ssl_certificate_key /etc/ssl/private/your-key.pem;

    location /webhook {
        proxy_pass http://localhost:3000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

Enable:

```bash
sudo ln -s /etc/nginx/sites-available/terraphim-runner /etc/nginx/sites-enabled/
sudo nginx -t
sudo systemctl reload nginx
```

## Troubleshooting

### Server Won't Start

```bash
# Check logs
journalctl -u terraphim-github-runner -n 50

# Common issues:
# - Port already in use: Change PORT variable
# - Missing environment: Check all required vars are set
# - Firecracker not running: Start fcctl-web first
```

### Webhook Returns 403

```bash
# Verify secret matches
echo $GITHUB_WEBHOOK_SECRET

# Check GitHub webhook settings
gh api repos/OWNER/REPO/hooks

# Test signature manually
python3 << 'EOF'
import hmac, hashlib
secret = b"test"
msg = b"test"
sig = hmac.new(secret, msg, hashlib.sha256).hexdigest()
print(f"sha256={sig}")
EOF
```

### LLM Parsing Fails

```bash
# Check Ollama is running
curl http://127.0.0.1:11434/api/tags

# Pull required model
ollama pull gemma3:4b

# Test LLM directly
curl http://127.0.0.1:11434/api/chat -d '{
  "model": "gemma3:4b",
  "messages": [{"role": "user", "content": "test"}]
}'
```

### Firecracker VM Fails

```bash
# Check Firecracker logs
journalctl -u fcctl-web -n 50

# Verify API accessibility
curl http://127.0.0.1:8080/health

# Check available resources
free -h
df -h
```

### High Memory Usage

```bash
# Monitor processes
htop

# Check VM count
curl http://127.0.0.1:8080/vms 2>/dev/null | jq '. | length'

# Release stuck VMs
curl -X DELETE http://127.0.0.1:8080/vms/stuck
```

## Monitoring

### Logs

```bash
# Real-time logs
journalctl -u terraphim-github-runner -f

# Last 100 lines
journalctl -u terraphim-github-runner -n 100

# Logs from current boot
journalctl -u terraphim-github-runner -b
```

### Metrics

Consider adding Prometheus metrics:

```rust
use prometheus::{Counter, Histogram, Registry};

lazy_static! {
    static ref WEBHOOK_RECEIVED: Counter = register_counter!(
        "github_runner_webhooks_total",
        "Total webhooks received"
    ).unwrap();
}
```

### Alerts

Configure alerts for:

- Server down (heartbeat failure)
- High error rate (>5% failures)
- Slow execution (>60s per workflow)
- VM exhaustion (no available VMs)

## Support

- **Issues**: https://github.com/terraphim/terraphim-ai/issues
- **Docs**: https://github.com/terraphim/terraphim-ai/tree/main/docs
- **Discord**: [Join our Discord](https://discord.gg/terraphim)

## Next Steps

1. ✅ Install Firecracker and Ollama
2. ✅ Build and configure server
3. ✅ Set up GitHub webhook
4. ✅ Test with sample workflow
5. 🔄 Deploy to production
6. 🔄 Configure monitoring
7. 🔄 Optimize performance

See [Architecture Documentation](../docs/github-runner-architecture.md) for deep dive into system design.
