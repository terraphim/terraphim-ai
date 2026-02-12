# TinyClaw - Multi-channel AI Assistant

TinyClaw is a multi-channel AI assistant powered by Terraphim, supporting Telegram, Discord, CLI, and other messaging platforms.

## Features

- **Multi-Channel Support**: Interact via Telegram, Discord, CLI, or Matrix/WhatsApp (via bridge)
- **Tool System**: Extensible tools for filesystem, web, shell, and code operations
- **Skills System**: Create and execute reusable JSON-defined workflows
- **Session Management**: Persistent conversation history
- **Hybrid LLM Router**: Intelligent routing between local and cloud LLM providers

## Installation

```bash
cargo install terraphim-tinyclaw
```

## Usage

### CLI Mode

Run TinyClaw in interactive CLI mode:

```bash
terraphim-tinyclaw agent --system-prompt /path/to/SYSTEM.md
```

### Gateway Mode

Run as a gateway server with all enabled channels:

```bash
terraphim-tinyclaw gateway
```

## Skills System

Skills are JSON-defined workflows that combine tool calls, LLM prompts, and shell commands into reusable automation scripts.

### Quick Start

```bash
# Save an example skill
terraphim-tinyclaw skill save examples/skills/analyze-repo.json

# Run the skill
terraphim-tinyclaw skill run analyze-repo repo_path=/path/to/repo

# List all skills
terraphim-tinyclaw skill list
```

### Creating Skills

Skills are defined as JSON files:

```json
{
  "name": "my-skill",
  "version": "1.0.0",
  "description": "What this skill does",
  "author": "Your Name",
  "inputs": [
    {
      "name": "input_name",
      "description": "Input description",
      "required": true,
      "default": null
    }
  ],
  "steps": [
    {
      "type": "tool",
      "tool": "shell",
      "args": {
        "command": "echo {input_name}"
      }
    }
  ]
}
```

See [examples/skills/](examples/skills/) for complete examples.

### Skill Commands

- `skill save <path>` - Save a skill from JSON file
- `skill load <name>` - Display skill details
- `skill list` - List all saved skills
- `skill run <name> [key=value...]` - Execute a skill
- `skill cancel` - Cancel running skill

### Skill Storage

Skills are stored in:
- Linux/macOS: `~/.config/terraphim/skills/`
- Windows: `%APPDATA%\terraphim\skills\`

## Configuration

Configuration is loaded from `~/.config/terraphim/tinyclaw.toml`:

```toml
[agent]
workspace = "/path/to/workspace"
max_iterations = 10

[llm.proxy]
base_url = "https://api.openai.com/v1"
api_key = "${OPENAI_API_KEY}"
model = "gpt-4"

[channels.telegram]
token = "${TELEGRAM_BOT_TOKEN}"
allow_from = ["@yourusername"]

[channels.discord]
token = "${DISCORD_BOT_TOKEN}"
allow_from = ["your_user_id"]
```

## Available Tools

- **filesystem**: Read, write, and list files
- **shell**: Execute shell commands (with safety guards)
- **edit**: Search and replace in files
- **web_search**: Search the web
- **web_fetch**: Fetch web pages
- **voice_transcribe**: Transcribe voice messages (requires `voice` feature)

## Development

### Running Tests

```bash
# Unit tests
cargo test -p terraphim_tinyclaw

# Integration tests
cargo test -p terraphim_tinyclaw --test skills_integration

# All tests
cargo test -p terraphim_tinyclaw --all
```

### Adding New Skills

1. Create a JSON file in `examples/skills/`
2. Test locally: `terraphim-tinyclaw skill save your-skill.json`
3. Run: `terraphim-tinyclaw skill run your-skill key=value`

### Architecture

See [docs/plans/tinyclaw-phase2-design.md](../docs/plans/tinyclaw-phase2-design.md) for detailed architecture.

## License

MIT OR Apache-2.0
