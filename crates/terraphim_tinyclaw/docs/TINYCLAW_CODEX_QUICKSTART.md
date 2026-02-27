# TinyClaw with OpenAI Codex - Quick Setup

Multi-channel AI assistant with intelligent LLM routing via terraphim-llm-proxy.

**Version:** 1.0
**Last Updated:** 2026-02-26

---

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Architecture](#architecture)
3. [Option A: Codex CLI (File-based OAuth)](#option-a-codex-cli-file-based-oauth)
4. [Option B: Browser OAuth Flow](#option-b-browser-oauth-flow)
5. [Docker Deployment](#docker-deployment)
6. [Configuration Reference](#configuration-reference)
7. [Troubleshooting](#troubleshooting)

---

## Prerequisites

- [ ] Telegram Bot Token (from [@BotFather](https://t.me/BotFather))
- [ ] terraphim-llm-proxy built (`cargo build --release`)
- [ ] terraphim-tinyclaw built
- [ ] (Option A only) Codex CLI installed and authenticated
- [ ] (Option B only) Browser for OAuth flow

### Build Components

```bash
# Build terraphim-llm-proxy
cd /path/to/terraphim-llm-proxy
cargo build --release

# Build terraphim-tinyclaw
cd /path/to/terraphim-ai
cargo build -p terraphim_tinyclaw --release
```

---

## Architecture

```
┌─────────────┐     ┌─────────────────────┐     ┌─────────────────────┐
│  Telegram   │────▶│    terraphim-llm-   │────▶│  OpenAI Codex       │
│  User       │     │    proxy            │     │  (chatgpt.com)      │
└─────────────┘     └─────────────────────┘     └─────────────────────┘
                           │
                           │                      ┌─────────────────────┐
                    ┌──────┴──────┐               │  Claude (Anthropic) │
                    │  terraphim- │──────────────▶│  Gemini (Google)    │
                    │  tinyclaw   │               │  OpenAI (API Key)   │
                    └─────────────┘               └─────────────────────┘
```

**Data Flow:**
1. User sends message via Telegram/Discord
2. TinyClaw receives message and processes through agent loop
3. For LLM requests, TinyClaw forwards to terraphim-llm-proxy
4. Proxy routes to appropriate provider (Codex, Claude, Gemini, etc.)
5. Response flows back through proxy to TinyClaw to user

---

## Option A: Codex CLI (File-based OAuth)

This is the quickest setup if you already have Codex CLI authenticated.

### Step 1: Verify Codex Authentication

```bash
# Check Codex status
codex auth status

# Verify tokens exist
ls -la ~/.codex/auth.json

# View token info (optional)
cat ~/.codex/auth.json | python3 -c "import json,sys; d=json.load(sys.stdin); print('Account:', d['tokens'].get('account_id', 'N/A'))"
```

### Step 2: Configure terraphim-llm-proxy

Create proxy configuration:

```bash
mkdir -p ~/.config/terraphim
cat > ~/.config/terraphim/proxy.toml << 'EOF'
# Server Configuration
[server]
host = "0.0.0.0"
port = 3456

# OAuth Configuration - Codex auto-import
[oauth]
# Codex tokens will be auto-imported from ~/.codex/auth.json on startup
[oauth.openai]
enabled = true

# Router Configuration - Default to Codex
[router]
# Format: "primary|fallback1,fallback2"
# Codex provides best tool-calling capabilities
default = "openai-codex|gpt-4.5"

# Optional: Model-specific routing
# patterns = [
#   { pattern = "code", provider = "openai-codex" },
#   { pattern = "creative", provider = "claude" },
# ]

[logging]
level = "info"
EOF
```

### Step 3: Configure TinyClaw

```bash
cat > ~/.config/terraphim/tinyclaw.toml << 'EOF'
[agent]
workspace = "/tmp/tinyclaw"

# LLM Configuration - Use proxy for tool calls, direct for compression
[llm.proxy]
base_url = "http://localhost:3456"
api_key = OAUTH_NOT_REQUIRED

[llm.direct]
provider = "ollama"
model = "llama3.2:3b"
base_url = "http://localhost:11434"
timeout = 120

# Telegram Channel
[channels.telegram]
token = "${TELEGRAM_BOT_TOKEN}"
allow_from = ["YOUR_TELEGRAM_ID"]
update_mode = "polling"
session_timeout = 3600

# Logging
[log]
level = "info"
console = true
EOF
```

### Step 4: Run the Services

**Terminal 1 - Start Proxy:**

```bash
cd /path/to/terraphim-llm-proxy
./target/release/terraphim-llm-proxy --config ~/.config/terraphim/proxy.toml
```

Expected output:
```
[INFO] terraphim-llm-proxy starting
[INFO] Loading configuration from ~/.config/terraphim/proxy.toml
[INFO] Importing Codex tokens from ~/.codex/auth.json
[INFO] OAuth: Successfully imported tokens for account_id
[INFO] Server listening on 0.0.0.0:3456
```

**Terminal 2 - Start TinyClaw:**

```bash
export TELEGRAM_BOT_TOKEN="YOUR_TELEGRAM_TOKEN"
cd /path/to/terraphim-ai
./target/release/terraphim-tinyclaw -c ~/.config/terraphim/tinyclaw.toml gateway
```

### Step 5: Test

1. Open Telegram
2. Find your bot (@your_bot_username)
3. Send a message

---

## Option B: Browser OAuth Flow

Use this if you want to authenticate without Codex CLI, or want to use Claude/Gemini directly.

### Step 1: Configure terraphim-llm-proxy

```bash
cat > ~/.config/terraphim/proxy-oauth.toml << 'EOF'
# Server Configuration
[server]
host = "0.0.0.0"
port = 3456

# OAuth Configuration - Browser-based flows
[oauth]
storage_backend = "file"
token_path = "~/.terraphim-llm-proxy/tokens"

# Claude OAuth (Anthropic)
[oauth.claude]
enabled = true
callback_port = 54545
client_id = "${ANTHROPIC_CLIENT_ID}"
client_secret = "${ANTHROPIC_CLIENT_SECRET}"

# OpenAI OAuth (for GPT models)
[oauth.openai]
enabled = true

# Gemini OAuth (Google)
[oauth.gemini]
enabled = false
callback_port = 54546
client_id = "${GEMINI_CLIENT_ID}"
client_secret = "${GEMINI_CLIENT_SECRET}"

# Router Configuration
[router]
default = "claude|gpt-4.5"

[logging]
level = "info"
EOF
```

### Step 2: Start Proxy

```bash
cd /path/to/terraphim-llm-proxy
export ANTHROPIC_CLIENT_ID="your-client-id"
export ANTHROPIC_CLIENT_SECRET="your-client-secret"
./target/release/terraphim-llm-proxy --config ~/.config/terraphim/proxy-oauth.toml
```

### Step 3: Authenticate (First Time Only)

**For Claude:**

```bash
# Start OAuth flow
curl -X POST http://localhost:3456/oauth/claude/start
```

Response:
```json
{
  "authorization_url": "https://auth.anthropic.com/...",
  "state": "random-state-token",
  "callback_port": 54545
}
```

1. Open the `authorization_url` in your browser
2. Complete authentication
3. The proxy automatically handles the callback

**Check status:**

```bash
curl "http://localhost:3456/oauth/claude/status?state=random-state-token"
```

Response:
```json
{
  "status": "completed",
  "account_id": "your-email@example.com",
  "error": null
}
```

### Step 4: Configure TinyClaw

Same as Option A - see [Configure TinyClaw](#step-3-configure-tinyclaw)

### Step 5: Run

Same as Option A - see [Run the Services](#step-4-run-the-services)

---

## Docker Deployment

Deploy both services using Docker Compose.

### Create docker-compose.yml

```bash
cat > docker-compose.yml << 'EOF'
version: '3.8'

services:
  proxy:
    image: terraphim/llm-proxy:latest
    build:
      context: ./terraphim-llm-proxy
      dockerfile: Dockerfile
    ports:
      - "3456:3456"
    volumes:
      - ./proxy-config.toml:/etc/terraphim/proxy.toml:ro
      - codex-auth:/root/.codex:ro
    environment:
      - ANTHROPIC_CLIENT_ID=${ANTHROPIC_CLIENT_ID}
      - ANTHROPIC_CLIENT_SECRET=${ANTHROPIC_CLIENT_SECRET}
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:3456/health"]
      interval: 30s
      timeout: 10s
      retries: 3

  tinyclaw:
    image: terraphim/tinyclaw:latest
    build:
      context: ./terraphim-ai
      dockerfile: Dockerfile
    ports:
      - "8080:8080"
    volumes:
      - ./tinyclaw-config.toml:/etc/terraphim/tinyclaw.toml:ro
    environment:
      - TELEGRAM_BOT_TOKEN=${TELEGRAM_BOT_TOKEN}
    depends_on:
      - proxy
    restart: unless-stopped

volumes:
  codex-auth:
    driver: local
    driver_opts:
      type: none
      o: bind
      device: ${HOME}/.codex

networks:
  default:
    name: terraphim-network
EOF
```

### Create proxy-config.toml

```bash
cat > proxy-config.toml << 'EOF'
[server]
host = "0.0.0.0"
port = 3456

[oauth]
[oauth.openai]
enabled = true

[router]
default = "openai-codex|gpt-4.5"

[logging]
level = "info"
EOF
```

### Create tinyclaw-config.toml

```bash
cat > tinyclaw-config.toml << 'EOF'
[agent]
workspace = "/tmp/tinyclaw"

[llm.proxy]
base_url = "http://proxy:3456"

[llm.direct]
provider = "ollama"
model = "llama3.2:3b"
base_url = "http://host.docker.internal:11434"

[channels.telegram]
token = "${TELEGRAM_BOT_TOKEN}"
allow_from = ["YOUR_TELEGRAM_ID"]

[log]
level = "info"
console = true
EOF
```

### Create .env file

```bash
cat > .env << 'EOF'
# Telegram Bot Token
TELEGRAM_BOT_TOKEN=your_telegram_bot_token_here

# OAuth (only for browser OAuth flow)
# ANTHROPIC_CLIENT_ID=your_anthropic_client_id
# ANTHROPIC_CLIENT_SECRET=your_anthropic_client_secret
EOF
```

### Run with Docker

```bash
# Build and start
docker-compose up -d

# View logs
docker-compose logs -f

# Check status
docker-compose ps
```

### Using Docker Volumes for Codex Auth

To use Codex CLI tokens in Docker:

```bash
# Option 1: Bind mount (shown above)
volumes:
  - ${HOME}/.codex:/root/.codex:ro

# Option 2: Copy tokens to container
docker cp ~/.codex/auth.json terraphim-proxy-1:/root/.codex/auth.json
```

---

## Configuration Reference

### terraphim-llm-proxy

#### Server Section

```toml
[server]
host = "0.0.0.0"      # Bind address
port = 3456           # HTTP port
```

#### OAuth Section

```toml
[oauth]
storage_backend = "file"  # or "redis"
token_path = "~/.terraphim-llm-proxy/tokens"

# File-based: tokens stored in ~/.terraphim-llm-proxy/tokens/
# Redis: share tokens across multiple proxy instances
```

#### Provider OAuth

```toml
[oauth.claude]
enabled = true
callback_port = 54545
client_id = "your-client-id"
client_secret = "your-client-secret"
scopes = ["openid", "email"]

[oauth.openai]
enabled = true

[oauth.gemini]
enabled = false
callback_port = 54546
client_id = "your-client-id"
client_secret = "your-client-secret"
```

#### Router Section

```toml
[router]
# Default provider chain (primary|fallback1,fallback2)
default = "openai-codex|gpt-4.5,claude"

# Pattern-based routing (optional)
# [[router.patterns]]
# pattern = "code"
# provider = "openai-codex"
# [[router.patterns]]
# pattern = "creative|write"
# provider = "claude"
```

### terraphim-tinyclaw

#### LLM Section

```toml
[llm.proxy]
base_url = "http://localhost:3456"
api_key = OAUTH_NOT_REQUIRED  # Not needed when using OAuth

[llm.direct]
provider = "ollama"  # or "openai", "anthropic"
model = "llama3.2:3b"
base_url = "http://localhost:11434"
timeout = 120
```

#### Telegram Channel

```toml
[channels.telegram]
token = "${TELEGRAM_BOT_TOKEN}"
allow_from = ["123456789", "@username"]  # Allowed users
update_mode = "polling"  # or "webhook"
poll_interval = 1000
session_timeout = 3600
request_timeout = 60

[channels.telegram.rate_limit]
messages_per_minute = 30
cooldown_seconds = 60
```

---

## Troubleshooting

### Proxy Issues

**"No OpenAI OAuth tokens found"**
```bash
# Verify Codex tokens exist
cat ~/.codex/auth.json

# Check token permissions
ls -la ~/.codex/auth.json

# Import tokens manually (if using OAuth)
curl -X POST http://localhost:3456/oauth/openai/start
```

**Proxy not starting**
```bash
# Check port availability
lsof -i :3456

# Verify config syntax
./terraphim-llm-proxy --config /path/to/config.toml --validate
```

**"Token expired" error**
```bash
# Refresh tokens via OAuth flow
curl -X POST http://localhost:3456/oauth/claude/start
```

### TinyClaw Issues

**"Connection refused" to proxy**
```bash
# Check proxy is running
curl http://localhost:3456/health

# Verify config URL
cat tinyclaw.toml | grep base_url
```

**"Proxy marked as unhealthy"**
```bash
# This is normal if proxy was down - it will auto-recover
# Check proxy logs for issues
tail -f /var/log/terraphim-proxy.log
```

**Bot not responding**
```bash
# Verify Telegram token
curl "https://api.telegram.org/bot${TELEGRAM_BOT_TOKEN}/getMe"

# Check user is allowed
# Add your Telegram ID to allow_from list
```

### OAuth Flow Issues

**Browser callback not working**
```bash
# Check callback port
netstat -an | grep 54545

# Ensure firewall allows localhost
sudo iptables -A INPUT -i lo -j ACCEPT
```

**State token mismatch**
```bash
# State tokens expire after 10 minutes
# Restart the OAuth flow from the beginning
curl -X POST http://localhost:3456/oauth/claude/start
```

### Docker Issues

**"No such container"**
```bash
# Rebuild images
docker-compose build --no-cache

# Start fresh
docker-compose down -v
docker-compose up -d
```

**Can't connect to proxy from tinyclaw container**
```bash
# Use Docker network name instead of localhost
# In tinyclaw config:
base_url = "http://proxy:3456"

# Verify network
docker network ls
docker network inspect terraphim-network
```

---

## Security Best Practices

1. **Never commit tokens** - Use environment variables
2. **Restrict permissions** - `chmod 600` on config files
3. **Limit users** - Use `allow_from` in Telegram config
4. **Enable rate limiting** - Prevent abuse
5. **Use HTTPS in production** - Configure TLS for OAuth callbacks

---

## Next Steps

1. [Add Discord channel](./TELEGRAM_QUICKSTART.md#add-discord)
2. [Create custom skills](./examples/skills/README.md)
3. [Set up monitoring](./docs/MONITORING.md)
4. [Configure system prompt](./docs/SYSTEM_PROMPT.md)

---

## Related Documentation

- [TinyClaw Telegram Quickstart](./TELEGRAM_QUICKSTART.md)
- [terraphim-llm-proxy OAuth Guide](../terraphim-llm-proxy/docs/OAUTH_GUIDE.md)
- [Configuration Reference](./CONFIGURATION.md)
- [Skills System](./examples/skills/README.md)
