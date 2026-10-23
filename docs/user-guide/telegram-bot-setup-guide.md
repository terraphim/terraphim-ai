# Setting Up Telegram Bot with TinyClaw

**A Complete Guide to Configuring terraphim-tinyclaw with Telegram Bot API**

---

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Creating Your Telegram Bot](#creating-your-telegram-bot)
3. [Configuring TinyClaw](#configuring-tinyclaw)
4. [Running TinyClaw](#running-tinyclaw)
5. [Using Your Bot](#using-your-bot)
6. [Advanced Configuration](#advanced-configuration)
7. [Deployment Options](#deployment-options)
8. [Troubleshooting](#troubleshooting)
9. [Security Best Practices](#security-best-practices)

---

## Prerequisites

Before you begin, ensure you have:

- A **Telegram account** (mobile or desktop app)
- **Rust toolchain** installed (for building from source)
- **TinyClaw** installed: `cargo install terraphim-tinyclaw`
- A **server or VPS** (for 24/7 bot operation)

---

## Creating Your Telegram Bot

### Step 1: Talk to BotFather

1. Open Telegram and search for **@BotFather**
2. Start a conversation with `/start`
3. Create a new bot with `/newbot`
4. Follow the prompts:
   - **Name**: Display name (e.g., "Terraphim Assistant")
   - **Username**: Unique identifier ending in "bot" (e.g., "my_terraphim_bot")

### Step 2: Get Your API Token

After creation, BotFather will provide your **HTTP API Token**:

```
Use this token to access the HTTP API:
123456789:ABCdefGHIjklMNOpqrSTUvwxyz123456789
```

**⚠️ IMPORTANT**: Keep this token secret! Anyone with this token can control your bot.

### Step 3: Set Bot Commands (Optional)

Configure command suggestions:

```
/setcommands
```

Then send the command list:

```
start - Start the bot
help - Show help
status - Check bot status
skill - Run a skill
session - Manage sessions
```

### Step 4: Configure Privacy Mode

Check privacy settings:

```
/mybots → Select your bot → Bot Settings → Group Privacy
```

**Recommendation**: Turn OFF privacy mode if you want the bot to read all messages in groups.

---

## Configuring TinyClaw

### Step 1: Create Config Directory

```bash
mkdir -p ~/.config/terraphim
```

### Step 2: Create Configuration File

Create `~/.config/terraphim/tinyclaw.toml`:

```toml
# TinyClaw Configuration for Telegram Bot

[agent]
# Workspace directory for temporary files
workspace = "/tmp/tinyclaw"

# System prompt (optional)
# system_prompt = "/path/to/SYSTEM.md"

# LLM Configuration
[llm]
provider = "ollama"
model = "llama3.2:3b"
base_url = "http://localhost:11434"

# Telegram Channel Configuration
[channels.telegram]
# Your bot token from BotFather (use environment variable!)
token = "${TELEGRAM_BOT_TOKEN}"

# Optional: Webhook configuration (recommended for production)
# webhook_url = "https://your-domain.com/webhook"
# webhook_port = 8443

# Optional: Allowed users (empty = all users allowed)
# allowed_users = ["@your_telegram_username"]

# Optional: Admin users
# admin_users = ["@your_admin_username"]

# Session timeout (seconds)
session_timeout = 3600

# Rate limiting
[channels.telegram.rate_limit]
# Max messages per minute per user
messages_per_minute = 30
# Cooldown after rate limit (seconds)
cooldown_seconds = 60
```

### Step 3: Set Environment Variable

**Option A: Export in shell**

```bash
export TELEGRAM_BOT_TOKEN="123456789:ABCdefGHIjklMNOpqrSTUvwxyz123456789"
```

**Option B: Use .env file**

Create `.env` in your working directory:

```bash
TELEGRAM_BOT_TOKEN=123456789:ABCdefGHIjklMNOpqrSTUvwxyz123456789
```

Load it:

```bash
source .env
```

**Option C: Systemd service (production)**

Create `/etc/systemd/system/tinyclaw-telegram.service`:

```ini
[Unit]
Description=TinyClaw Telegram Bot
After=network.target

[Service]
Type=simple
User=tinyclaw
Environment="TELEGRAM_BOT_TOKEN=123456789:ABCdefGHIjklMNOpqrSTUvwxyz123456789"
Environment="RUST_LOG=info"
ExecStart=/usr/local/bin/terraphim-tinyclaw gateway
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

---

## Running TinyClaw

### Local Testing

```bash
# Run in gateway mode (includes Telegram)
terraphim-tinyclaw gateway

# Or run Telegram channel only
terraphim-tinyclaw telegram

# With debug logging
RUST_LOG=debug terraphim-tinyclaw gateway
```

You should see:

```
[INFO] terraphim-tinyclaw starting
[INFO] Loading configuration from /home/user/.config/terraphim/tinyclaw.toml
[INFO] Telegram bot connected: @my_terraphim_bot
[INFO] Bot is running. Press Ctrl+C to stop.
```

### Testing Your Bot

1. Open Telegram
2. Search for your bot (@my_terraphim_bot)
3. Click **Start** or send `/start`
4. The bot should respond with a welcome message

---

## Using Your Bot

### Basic Commands

```
/start - Initialize conversation
/help - Show available commands
/status - Check bot status
/session - View current session info
/skill list - List available skills
/skill run <name> - Run a skill
```

### Example Conversations

**Simple Query:**
```
You: Hello!
Bot: Hello! I'm your Terraphim assistant. How can I help you today?
```

**Running a Skill:**
```
You: /skill run analyze-repo repo_path=/home/user/myproject
Bot: 🔍 Analyzing repository...
Bot: Analysis complete!
    Total files: 42
    Languages: Rust, Python
    Issues found: 3
```

**Asking Questions:**
```
You: What is the best way to handle errors in Rust?
Bot: In Rust, error handling is primarily done through the Result<T, E> type...
[Detailed explanation follows]
```

---

## Advanced Configuration

### Webhook Mode (Production)

For production, use webhooks instead of polling:

1. **Set up HTTPS** (required by Telegram)
2. **Update config**:

```toml
[channels.telegram]
token = "${TELEGRAM_BOT_TOKEN}"
webhook_url = "https://api.yourdomain.com/webhook"
webhook_port = 8443
```

3. **Configure SSL certificate**

### Multiple LLM Providers

```toml
[llm]
# Primary provider
provider = "openrouter"
# api_key loaded from OPENROUTER_API_KEY environment variable
model = "anthropic/claude-3.5-sonnet"

# Fallback provider
[llm.fallback]
provider = "ollama"
model = "llama3.2:3b"
```

### Custom Skills for Telegram

Create `~/.config/terraphim/skills/telegram-welcome.json`:

```json
{
  "name": "telegram-welcome",
  "version": "1.0.0",
  "description": "Welcome message for Telegram users",
  "inputs": [
    {
      "name": "username",
      "description": "User's Telegram username",
      "required": true
    }
  ],
  "steps": [
    {
      "type": "llm",
      "prompt": "Generate a friendly welcome message for Telegram user @{username}. Keep it under 200 characters."
    }
  ]
}
```

Use it:

```
You: /skill run telegram-welcome username=john_doe
Bot: 👋 Welcome @john_doe! I'm your AI assistant...
```

---

## Deployment Options

### Docker Deployment

**Dockerfile**:

```dockerfile
FROM rust:1.75-slim as builder
WORKDIR /app
COPY . .
RUN cargo build --release -p terraphim_tinyclaw

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates
COPY --from=builder /app/target/release/terraphim-tinyclaw /usr/local/bin/
COPY tinyclaw.toml /etc/terraphim/
ENV TELEGRAM_BOT_TOKEN=${TELEGRAM_BOT_TOKEN}
CMD ["terraphim-tinyclaw", "gateway"]
```

**docker-compose.yml**:

```yaml
version: '3.8'
services:
  tinyclaw:
    build: .
    environment:
      - TELEGRAM_BOT_TOKEN=${TELEGRAM_BOT_TOKEN}
      - RUST_LOG=info
    volumes:
      - ./data:/tmp/tinyclaw
    restart: unless-stopped
```

**Run**:

```bash
docker-compose up -d
```

### VPS Deployment

1. **Upload binary**:

```bash
scp target/release/terraphim-tinyclaw user@your-vps:/usr/local/bin/
```

2. **Create systemd service**:

```bash
sudo nano /etc/systemd/system/tinyclaw.service
```

Paste the service config from Step 3 above.

3. **Enable and start**:

```bash
sudo systemctl daemon-reload
sudo systemctl enable tinyclaw
sudo systemctl start tinyclaw
```

4. **Check status**:

```bash
sudo systemctl status tinyclaw
sudo journalctl -u tinyclaw -f
```

---

## Troubleshooting

### Bot Doesn't Respond

**Check 1: Verify token**

```bash
echo $TELEGRAM_BOT_TOKEN
# Should show your token
```

**Check 2: Test with curl**

```bash
curl -s "https://api.telegram.org/bot${TELEGRAM_BOT_TOKEN}/getMe"
# Should return bot info
```

**Check 3: Check logs**

```bash
RUST_LOG=debug terraphim-tinyclaw gateway
```

### Connection Timeout

**Problem**: Bot can't connect to Telegram API

**Solution**:

```toml
# Add to tinyclaw.toml
[channels.telegram]
token = "${TELEGRAM_BOT_TOKEN}"
# Increase timeout
request_timeout = 60
```

### Rate Limiting

**Problem**: "Too Many Requests" error

**Solution**: Implement rate limiting in config:

```toml
[channels.telegram.rate_limit]
messages_per_minute = 20
cooldown_seconds = 60
```

### Webhook Issues

**Problem**: Webhook not receiving updates

**Checklist**:
- [ ] HTTPS certificate valid
- [ ] Webhook URL accessible from internet
- [ ] Port 8443 (or your port) open in firewall
- [ ] Correct webhook URL set in config

**Debug**:

```bash
# Check webhook status
curl "https://api.telegram.org/bot${TELEGRAM_BOT_TOKEN}/getWebhookInfo"
```

### Session Not Persisting

**Problem**: Bot forgets context between messages

**Solution**: Ensure session storage is configured:

```toml
[agent]
workspace = "/tmp/tinyclaw"
session_timeout = 3600  # 1 hour
```

---

## Security Best Practices

### 1. Protect Your Token

**DON'T**:
- ❌ Hardcode token in config files
- ❌ Commit token to git
- ❌ Share token in public channels

**DO**:
- ✅ Use environment variables
- ✅ Use secret management (HashiCorp Vault, AWS Secrets Manager)
- ✅ Rotate tokens periodically

### 2. User Access Control

```toml
[channels.telegram]
token = "${TELEGRAM_BOT_TOKEN}"
# Only allow specific users
allowed_users = ["@trusted_user1", "@trusted_user2"]
# Admin users can manage bot
admin_users = ["@your_admin_username"]
```

### 3. Rate Limiting

Always enable rate limiting to prevent abuse:

```toml
[channels.telegram.rate_limit]
messages_per_minute = 30
cooldown_seconds = 60
```

### 4. Input Validation

Skills should validate inputs:

```json
{
  "inputs": [
    {
      "name": "file_path",
      "description": "Path to file",
      "required": true,
      "validation": "^/safe/directory/.*$"
    }
  ]
}
```

### 5. Secure Deployment

- Use non-root user
- Enable firewall (ufw)
- Use HTTPS for webhooks
- Regular security updates

---

## Next Steps

1. **Create Custom Skills**: Build skills specific to your use case
2. **Add More Channels**: Enable Discord, Matrix, etc.
3. **Monitor Usage**: Check logs and metrics
4. **Scale Up**: Deploy to multiple regions if needed

## Resources

- **Telegram Bot API**: https://core.telegram.org/bots/api
- **TinyClaw README**: `crates/terraphim_tinyclaw/README.md`
- **BotFather**: https://t.me/botfather

---

**You're all set!** Your TinyClaw bot is now running on Telegram and ready to assist users.
