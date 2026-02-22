#!/bin/bash
# TinyClaw Gateway Deployment Example
# This script demonstrates how to deploy TinyClaw in gateway mode
# with Telegram and Discord bot integration

set -e

echo "==================================="
echo "TinyClaw Gateway Deployment Example"
echo "==================================="
echo ""

# Step 1: Build TinyClaw
echo "Step 1: Building TinyClaw..."
cargo build -p terraphim_tinyclaw --release --features telegram,discord
echo "✓ Build complete"
echo ""

# Step 2: Create configuration directory
echo "Step 2: Setting up configuration..."
CONFIG_DIR="${HOME}/.config/terraphim"
mkdir -p "${CONFIG_DIR}/skills"
mkdir -p "${CONFIG_DIR}/sessions"
echo "✓ Configuration directory created: ${CONFIG_DIR}"
echo ""

# Step 3: Create example configuration
echo "Step 3: Creating example configuration..."
cat > "${CONFIG_DIR}/tinyclaw-config.toml" << 'EOF'
# TinyClaw Gateway Configuration Example
# Copy this file to ~/.config/terraphim/tinyclaw-config.toml and customize

[agent]
workspace = "~/.config/terraphim"
max_iterations = 20
system_prompt = "~/.config/terraphim/SYSTEM.md"

[llm.proxy]
base_url = "https://api.openai.com/v1"
api_key = "${OPENAI_API_KEY}"
model = "gpt-4"
timeout_ms = 60000
retry_after_secs = 5

[llm.direct]
provider = "ollama"
base_url = "http://127.0.0.1:11434"
model = "llama3.2"

# Telegram Bot Configuration
# Get your bot token from @BotFather on Telegram
[channels.telegram]
token = "${TELEGRAM_BOT_TOKEN}"
# List of allowed user IDs or usernames
allow_from = ["your_telegram_username", "123456789"]

# Discord Bot Configuration
# Create a bot at https://discord.com/developers/applications
[channels.discord]
token = "${DISCORD_BOT_TOKEN}"
# List of allowed user IDs or usernames
allow_from = ["your_discord_username", "123456789012345678"]
EOF

echo "✓ Example configuration created: ${CONFIG_DIR}/tinyclaw-config.toml"
echo ""

# Step 4: Create example system prompt
echo "Step 4: Creating example system prompt..."
cat > "${CONFIG_DIR}/SYSTEM.md" << 'EOF'
# TinyClaw System Prompt

You are TinyClaw, a helpful AI assistant integrated with Terraphim.
You can help users with:

- Executing shell commands
- Searching through documents
- Running skills (automated workflows)
- Answering questions

Be concise, helpful, and accurate.
EOF

echo "✓ System prompt created: ${CONFIG_DIR}/SYSTEM.md"
echo ""

# Step 5: Create example skills
echo "Step 5: Creating example skills..."

# Code review skill
cat > "${CONFIG_DIR}/skills/code-review.json" << 'EOF'
{
  "name": "code-review",
  "version": "1.0.0",
  "description": "Perform a code review on a file or directory",
  "author": "Terraphim",
  "inputs": [
    {
      "name": "path",
      "description": "Path to the file or directory to review",
      "required": true
    },
    {
      "name": "focus",
      "description": "Focus area (security, performance, style)",
      "required": false,
      "default": "general"
    }
  ],
  "steps": [
    {
      "type": "Shell",
      "command": "find {path} -name '*.rs' -o -name '*.py' -o -name '*.js' | head -20",
      "working_dir": null
    },
    {
      "type": "Llm",
      "prompt": "Review the following code for {focus} issues:\n\n{previous_output}",
      "use_context": true
    }
  ]
}
EOF

# Documentation generation skill
cat > "${CONFIG_DIR}/skills/generate-docs.json" << 'EOF'
{
  "name": "generate-docs",
  "version": "1.0.0",
  "description": "Generate documentation for a codebase",
  "author": "Terraphim",
  "inputs": [
    {
      "name": "path",
      "description": "Path to the codebase",
      "required": true
    }
  ],
  "steps": [
    {
      "type": "Shell",
      "command": "ls -la {path}",
      "working_dir": null
    },
    {
      "type": "Llm",
      "prompt": "Generate a README.md for this project based on the file structure:\n\n{previous_output}",
      "use_context": false
    }
  ]
}
EOF

echo "✓ Example skills created in: ${CONFIG_DIR}/skills/"
echo ""

# Step 6: Usage examples
echo "==================================="
echo "Usage Examples"
echo "==================================="
echo ""

cat << 'EOF'
1. Run in Agent Mode (CLI):
   -------------------------
   terraphim-tinyclaw agent
   
   This starts an interactive CLI session where you can chat with TinyClaw.

2. Run in Gateway Mode (Telegram + Discord):
   -----------------------------------------
   export TELEGRAM_BOT_TOKEN="your_token_here"
   export DISCORD_BOT_TOKEN="your_token_here"
   terraphim-tinyclaw gateway --config ~/.config/terraphim/tinyclaw-config.toml
   
   This starts the gateway server that listens for messages from Telegram
   and Discord bots.

3. Manage Skills:
   --------------
   # List all skills
   terraphim-tinyclaw skill list
   
   # Run a skill
   terraphim-tinyclaw skill run code-review path=./src focus=security
   
   # Save a new skill
   terraphim-tinyclaw skill save ./my-skill.json

4. Environment Variables:
   ----------------------
   TELEGRAM_BOT_TOKEN     - Telegram bot token (required for Telegram)
   DISCORD_BOT_TOKEN      - Discord bot token (required for Discord)
   OLLAMA_BASE_URL        - Ollama API URL (default: http://127.0.0.1:11434)
   OLLAMA_MODEL           - Ollama model (default: llama3.2)
   RUST_LOG               - Log level (info, debug, trace)

EOF

echo ""
echo "==================================="
echo "Next Steps"
echo "==================================="
echo ""
echo "1. Edit ~/.config/terraphim/tinyclaw-config.toml"
echo "2. Set up your Telegram bot via @BotFather"
echo "3. Set up your Discord bot at https://discord.com/developers/applications"
echo "4. Run: export TELEGRAM_BOT_TOKEN='your_token'"
echo "5. Run: export DISCORD_BOT_TOKEN='your_token'"
echo "6. Start: ./target/release/terraphim-tinyclaw gateway"
echo ""
echo "For more information, see:"
echo "  - crates/terraphim_tinyclaw/README.md"
echo "  - crates/terraphim_tinyclaw/docs/TELEGRAM_QUICKSTART.md"
echo ""
