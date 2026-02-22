# TinyClaw Quick Start Guide

This guide will get you up and running with TinyClaw in 5 minutes.

## Prerequisites

- Rust toolchain installed
- (Optional) Ollama for local LLM support

## Step 1: Build TinyClaw

```bash
cd /path/to/terraphim-ai
cargo build -p terraphim_tinyclaw --release
```

## Step 2: Try CLI Mode

```bash
# Run in agent mode
./target/release/terraphim-tinyclaw agent

# You'll see:
# TinyClaw Agent Mode
# ===================
# > 
```

Type your messages and press Enter. TinyClaw will respond!

**Example interaction:**

```
> Hello!
Hello! How can I help you today?

> What files are in the current directory?
I'll check the current directory for you.

Executing: ls -la
[output appears]

> Thank you!
You're welcome! Let me know if you need anything else.
```

Press Ctrl+C to exit.

## Step 3: Create Your First Skill

1. Create a skill file:

```bash
cat > hello-world.json << 'EOF'
{
  "name": "hello-world",
  "version": "1.0.0",
  "description": "A simple hello world skill",
  "author": "You",
  "inputs": [
    {
      "name": "name",
      "description": "Name to greet",
      "required": false,
      "default": "World"
    }
  ],
  "steps": [
    {
      "type": "Shell",
      "command": "echo 'Hello, {name}!'",
      "working_dir": null
    }
  ]
}
EOF
```

2. Save and run the skill:

```bash
# Save the skill
./target/release/terraphim-tinyclaw skill save hello-world.json

# Run it with default
./target/release/terraphim-tinyclaw skill run hello-world

# Run it with custom input
./target/release/terraphim-tinyclaw skill run hello-world name=Alice
```

## Step 4: Set Up a Telegram Bot (Optional)

1. Message [@BotFather](https://t.me/botfather) on Telegram
2. Create a bot: `/newbot`
3. Save your bot token
4. Run TinyClaw with the token:

```bash
export TELEGRAM_BOT_TOKEN="your_token_here"
./target/release/terraphim-tinyclaw gateway
```

5. Message your bot on Telegram!

## Next Steps

- Read the [full README](../README.md)
- Explore [example skills](skills/)
- See [Gateway Deployment](../examples/deploy-gateway.sh)
- Check out [Telegram Quickstart](TELEGRAM_QUICKSTART.md)

## Common Commands

```bash
# List all skills
terraphim-tinyclaw skill list

# Get help
terraphim-tinyclaw --help
terraphim-tinyclaw agent --help
terraphim-tinyclaw gateway --help

# Run with verbose logging
RUST_LOG=debug terraphim-tinyclaw agent

# Run with custom config
terraphim-tinyclaw gateway --config /path/to/config.toml
```

## Troubleshooting

**Issue**: Command not found
- Solution: Make sure the binary is in your PATH or use full path

**Issue**: Permission denied
- Solution: Make binary executable: `chmod +x terraphim-tinyclaw`

**Issue**: Bot doesn't respond
- Solution: Check that TELEGRAM_BOT_TOKEN is set correctly

**Issue**: Skills not found
- Solution: Check that skill JSON is valid: `python3 -m json.tool skill.json`

## Help

Need more help? Open an issue at:
https://github.com/terraphim/terraphim-ai/issues
