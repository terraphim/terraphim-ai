# TinyClaw Telegram Bot - Quick Setup

## 5-Minute Setup

### 1. Get Bot Token (2 minutes)

1. Open Telegram, search **@BotFather**
2. Send `/newbot`
3. Name your bot (e.g., "Terraphim Assistant")
4. Username ending in "bot" (e.g., "my_terraphim_bot")
5. **Copy the token** (looks like: `123456789:ABCdef...`)

### 2. Configure TinyClaw (2 minutes)

```bash
# Create config directory
mkdir -p ~/.config/terraphim

# Create config file
cat > ~/.config/terraphim/tinyclaw.toml << 'EOF'
[agent]
workspace = "/tmp/tinyclaw"

[llm]
provider = "ollama"
model = "llama3.2:3b"
base_url = "http://localhost:11434"

[channels.telegram]
token = "${TELEGRAM_BOT_TOKEN}"
EOF

# Set token as environment variable
export TELEGRAM_BOT_TOKEN="YOUR_TOKEN_HERE"
```

### 3. Run TinyClaw (1 minute)

```bash
terraphim-tinyclaw gateway
```

Done! Your bot is running.

## Test Your Bot

1. Open Telegram
2. Search for your bot (@my_terraphim_bot)
3. Click **Start**
4. Type: `Hello!`

You should get a response.

## Common Commands

```
/start - Start conversation
/help - Show help
/status - Bot status
/skill list - List skills
/skill run analyze-repo repo_path=/path/to/repo
```

## Production Deployment

### Using Docker

```bash
# Create docker-compose.yml
cat > docker-compose.yml << 'EOF'
version: '3'
services:
  tinyclaw:
    image: terraphim/tinyclaw:latest
    environment:
      - TELEGRAM_BOT_TOKEN=${TELEGRAM_BOT_TOKEN}
    volumes:
      - ./tinyclaw.toml:/etc/terraphim/tinyclaw.toml
    restart: always
EOF

# Run
docker-compose up -d
```

### Using systemd

```bash
# Create service
sudo tee /etc/systemd/system/tinyclaw.service > /dev/null << 'EOF'
[Unit]
Description=TinyClaw Telegram Bot
After=network.target

[Service]
Type=simple
Environment="TELEGRAM_BOT_TOKEN=YOUR_TOKEN"
ExecStart=/usr/local/bin/terraphim-tinyclaw gateway
Restart=always

[Install]
WantedBy=multi-user.target
EOF

# Enable and start
sudo systemctl daemon-reload
sudo systemctl enable tinyclaw
sudo systemctl start tinyclaw
```

## Troubleshooting

**Bot not responding?**
```bash
# Check if token is set
echo $TELEGRAM_BOT_TOKEN

# Test token
curl "https://api.telegram.org/bot${TELEGRAM_BOT_TOKEN}/getMe"

# Run with debug logging
RUST_LOG=debug terraphim-tinyclaw gateway
```

**Connection issues?**
- Check internet connection
- Verify firewall allows outbound connections
- Try webhook mode for production

## Security Checklist

- [ ] Token stored as environment variable (not in code)
- [ ] Config file has restricted permissions (`chmod 600`)
- [ ] Rate limiting enabled
- [ ] Only trusted users allowed (optional)

## Next Steps

1. **Create custom skills**: See `examples/skills/`
2. **Add system prompt**: Create `SYSTEM.md` file
3. **Enable other channels**: Discord, Matrix
4. **Monitor logs**: `journalctl -u tinyclaw -f`

## Full Guide

For detailed setup instructions, see: `docs/telegram-bot-setup-guide.md`
