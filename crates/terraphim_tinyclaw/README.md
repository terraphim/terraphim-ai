# TinyClaw - Multi-channel AI Assistant

TinyClaw is a multi-channel AI assistant powered by Terraphim, supporting Telegram, Discord, CLI, and other messaging platforms. It can execute shell commands, run skills (automated workflows), and interact with various LLM providers.

## Features

- **Multi-Channel Support**: Interact via Telegram, Discord, CLI, or Matrix (via bridge)
- **Tool System**: Extensible tools for filesystem, web, shell, and code operations
- **Skills System**: Create and execute reusable JSON-defined workflows
- **Session Management**: Persistent conversation history
- **Hybrid LLM Router**: Intelligent routing between local (Ollama) and cloud (OpenAI) LLM providers
- **Security**: Execution guards, allowlists, and safe command execution

## Quick Start

### Installation

```bash
cargo install terraphim-tinyclaw
```

Or build from source:

```bash
git clone https://github.com/terraphim/terraphim-ai
cd terraphim-ai
cargo build -p terraphim_tinyclaw --release
```

### 1. CLI Mode (Interactive)

Run TinyClaw in interactive CLI mode:

```bash
# Start interactive session
terraphim-tinyclaw agent

# With custom system prompt
terraphim-tinyclaw agent --system-prompt ~/SYSTEM.md
```

### 2. Gateway Mode (Telegram + Discord)

Run as a gateway server with bot integrations:

```bash
# Set required environment variables
export TELEGRAM_BOT_TOKEN="your_telegram_bot_token"
export DISCORD_BOT_TOKEN="your_discord_bot_token"

# Start gateway
terraphim-tinyclaw gateway
```

See [Gateway Deployment Guide](#gateway-deployment) below for detailed setup.

### 3. Skills System

Skills are JSON-defined workflows:

```bash
# Save a skill
terraphim-tinyclaw skill save examples/skills/code-review.json

# Run the skill
terraphim-tinyclaw skill run code-review path=./src focus=security

# List all skills
terraphim-tinyclaw skill list
```

## Gateway Deployment

### Prerequisites

1. **Telegram Bot**:
   - Message [@BotFather](https://t.me/botfather) on Telegram
   - Create a new bot: `/newbot`
   - Save the bot token

2. **Discord Bot**:
   - Go to [Discord Developer Portal](https://discord.com/developers/applications)
   - Create New Application → Bot → Add Bot
   - Enable "Message Content Intent"
   - Copy the bot token

### Configuration

Create `~/.config/terraphim/tinyclaw.toml`:

```toml
[agent]
workspace = "~/.config/terraphim"
max_iterations = 20

[llm.proxy]
base_url = "https://api.openai.com/v1"
api_key = "${OPENAI_API_KEY}"
model = "gpt-4"
timeout_ms = 60000

[llm.direct]
provider = "ollama"
base_url = "http://127.0.0.1:11434"
model = "llama3.2"

[channels.telegram]
token = "${TELEGRAM_BOT_TOKEN}"
allow_from = ["your_telegram_username"]

[channels.discord]
token = "${DISCORD_BOT_TOKEN}"
allow_from = ["your_discord_user_id"]
```

### Running the Gateway

```bash
# Development mode with verbose logging
export RUST_LOG=debug
terraphim-tinyclaw gateway --config ~/.config/terraphim/tinyclaw.toml

# Production mode
export RUST_LOG=info
./target/release/terraphim-tinyclaw gateway
```

### Systemd Service (Linux)

Create `/etc/systemd/system/tinyclaw.service`:

```ini
[Unit]
Description=TinyClaw Multi-Channel AI Gateway
After=network.target

[Service]
Type=simple
User=tinyclaw
Environment="TELEGRAM_BOT_TOKEN=your_token"
Environment="DISCORD_BOT_TOKEN=your_token"
Environment="OLLAMA_BASE_URL=http://127.0.0.1:11434"
Environment="RUST_LOG=info"
ExecStart=/usr/local/bin/terraphim-tinyclaw gateway
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

Enable and start:

```bash
sudo systemctl enable tinyclaw
sudo systemctl start tinyclaw
sudo systemctl status tinyclaw
```

### Docker Deployment

```dockerfile
FROM rust:latest as builder
WORKDIR /app
COPY . .
RUN cargo build -p terraphim_tinyclaw --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates
COPY --from=builder /app/target/release/terraphim-tinyclaw /usr/local/bin/
ENTRYPOINT ["terraphim-tinyclaw"]
CMD ["gateway"]
```

Run:

```bash
docker build -t tinyclaw .
docker run -d \
  -e TELEGRAM_BOT_TOKEN="$TELEGRAM_BOT_TOKEN" \
  -e DISCORD_BOT_TOKEN="$DISCORD_BOT_TOKEN" \
  -v ~/.config/terraphim:/config \
  tinyclaw gateway --config /config/tinyclaw.toml
```

## Skills System

Skills combine tool calls, LLM prompts, and shell commands into reusable automation.

### Example: Code Review Skill

```json
{
  "name": "code-review",
  "version": "1.0.0",
  "description": "Perform a security-focused code review",
  "author": "Terraphim",
  "inputs": [
    {
      "name": "path",
      "description": "Path to review",
      "required": true
    },
    {
      "name": "focus",
      "description": "Review focus (security, performance, style)",
      "required": false,
      "default": "general"
    }
  ],
  "steps": [
    {
      "type": "Shell",
      "command": "find {path} -type f \( -name '*.rs' -o -name '*.py' -o -name '*.js' \) | head -20",
      "working_dir": null
    },
    {
      "type": "Llm",
      "prompt": "Review this code for {focus} issues:\n\n{previous_output}",
      "use_context": true
    }
  ]
}
```

Run it:

```bash
terraphim-tinyclaw skill save code-review.json
terraphim-tinyclaw skill run code-review path=./src focus=security
```

### Skill Storage

- Linux/macOS: `~/.config/terraphim/skills/`
- Windows: `%APPDATA%\terraphim\skills\`

## Available Tools

| Tool | Description | Example |
|------|-------------|---------|
| **filesystem** | Read/write/list files | `Read: /path/to/file` |
| **shell** | Execute commands (guarded) | `Execute: cargo build` |
| **edit** | Search and replace | `Replace "old" with "new"` |
| **web_search** | Web search | `Search: rust async tutorial` |
| **web_fetch** | Fetch web pages | `Fetch: https://example.com` |
| **voice_transcribe** | Transcribe audio | Requires `voice` feature |

## Architecture

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   Telegram Bot  │     │   Discord Bot   │     │      CLI        │
└────────┬────────┘     └────────┬────────┘     └────────┬────────┘
         │                       │                       │
         └───────────────────────┼───────────────────────┘
                                 │
                    ┌────────────▼────────────┐
                    │      MessageBus         │
                    └────────────┬────────────┘
                                 │
                    ┌────────────▼────────────┐
                    │     Agent Loop          │
                    │  (Tool Calling)         │
                    └────────────┬────────────┘
                                 │
         ┌───────────────────────┼───────────────────────┐
         │                       │                       │
┌────────▼────────┐     ┌────────▼────────┐     ┌────────▼────────┐
│  Tool Registry  │     │  LLM Router     │     │ Session Manager │
└─────────────────┘     └─────────────────┘     └─────────────────┘
```

## Configuration Reference

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `TELEGRAM_BOT_TOKEN` | Telegram bot token | - |
| `DISCORD_BOT_TOKEN` | Discord bot token | - |
| `OLLAMA_BASE_URL` | Ollama API URL | `http://127.0.0.1:11434` |
| `OLLAMA_MODEL` | Ollama model name | `llama3.2` |
| `RUST_LOG` | Log level | `info` |
| `TERRAPHIM_CONFIG` | Config file path | `~/.config/terraphim/tinyclaw.toml` |

### Config File Options

See [examples/tinyclaw-config.toml](examples/tinyclaw-config.toml) for complete example.

## Development

### Running Tests

```bash
# Unit tests
cargo test -p terraphim_tinyclaw

# Integration tests
cargo test -p terraphim_tinyclaw --test skills_integration
cargo test -p terraphim_tinyclaw --test gateway_dispatch

# All tests with all features
cargo test -p terraphim_tinyclaw --all-features
```

### Building

```bash
# Debug build
cargo build -p terraphim_tinyclaw

# Release build
cargo build -p terraphim_tinyclaw --release

# With all features
cargo build -p terraphim_tinyclaw --release --all-features

# Specific channels only
cargo build -p terraphim_tinyclaw --release --features telegram
cargo build -p terraphim_tinyclaw --release --features discord
```

### Adding New Features

1. **New Tool**: Implement in `src/tools/`, add to `ToolRegistry`
2. **New Channel**: Implement `Channel` trait in `src/channels/`
3. **New Skill**: Create JSON in `examples/skills/`

## Examples

See the [examples/](examples/) directory for:
- Complete configuration files
- Example skills (code review, documentation, security scan)
- Deployment scripts

## Troubleshooting

### Bot Not Responding

1. Check bot tokens are correct
2. Verify bot is added to Telegram group / Discord server
3. Check logs: `RUST_LOG=debug terraphim-tinyclaw gateway`
4. Ensure allowlist includes your user ID

### LLM Not Working

1. Verify Ollama is running: `curl http://localhost:11434/api/tags`
2. Check model is available: `ollama list`
3. For cloud LLM, verify API key is set

### Skills Not Found

1. Check skill file is valid JSON
2. Verify skill is saved: `terraphim-tinyclaw skill list`
3. Check skill directory: `~/.config/terraphim/skills/`

## Health Check

When running in gateway mode, a health check endpoint is available:

```bash
curl http://localhost:8080/health
```

Response:

```json
{
  "status": "healthy",
  "version": "1.8.0",
  "timestamp": "2026-02-20T12:00:00Z",
  "components": {
    "message_bus": true,
    "session_storage": true,
    "telegram": true,
    "discord": true
  }
}
```

## Security

- **Token Safety**: Bot tokens are never logged
- **Execution Guards**: Dangerous commands are blocked
- **Allowlists**: Only authorized users can interact with bots
- **Session Isolation**: Each chat has isolated session context

## License

MIT OR Apache-2.0

## Contributing

See [CONTRIBUTING.md](../../CONTRIBUTING.md) for contribution guidelines.

## Support

- GitHub Issues: https://github.com/terraphim/terraphim-ai/issues
- Discussions: https://github.com/terraphim/terraphim-ai/discussions
