+++
title = "Quickstart"
description = "Get started with Terraphim AI in 5 minutes"
date = 2026-01-27
+++

# Quickstart Guide

Get up and running with Terraphim AI in just 5 minutes.

## Step 1: Install Terraphim

Choose your preferred installation method:

### Option A: Universal Installer (Recommended)

\`\`\`bash
# Single command installation with platform detection
curl -fsSL https://raw.githubusercontent.com/terraphim/terraphim-ai/main/scripts/install.sh | bash
\`\`\`

### Option B: Homebrew (macOS/Linux)

\`\`\`bash
# Add Terraphim tap
brew tap terraphim/terraphim

# Install both server and CLI tools
brew install terraphim-server terraphim-agent
\`\`\`

### Option C: Cargo

\`\`\`bash
# Install REPL with interactive TUI (11 commands)
cargo install terraphim-repl

# Install CLI for automation (8 commands)
cargo install terraphim-cli
\`\`\`

[Need more options?](/docs/installation)

## Step 2: Start Server

Terraphim server provides HTTP API and knowledge graph backend.

\`\`\`bash
terraphim-server
\`\`\`

By default, server runs on \`http://localhost:8080\`.

You should see output like:
\`\`\`
[INFO] Terraphim Server v1.5.2 starting...
[INFO] Server listening on http://localhost:8080
[INFO] Knowledge graph initialized
\`\`\`

## Step 3: Use REPL

In a new terminal, start the interactive REPL (Read-Eval-Print Loop):

\`\`\`bash
terraphim-repl
\`\`\`

You'll see a welcome message and can start typing commands:

\`\`\`
Terraphim AI REPL v1.5.2
Type 'help' for available commands

> search rust async
Found 12 results for 'rust async'

> role engineer
Role set to: Engineer (optimizing for technical depth)

> search patterns
Found 8 results for 'patterns'
\`\`\`

## Common REPL Commands

Here are the most useful commands to get started:

\`\`\`bash
> search <query>              # Search knowledge graph
> role <name>                 # Set search role (engineer, architect, etc.)
> connect <term1> <term2>    # Link two terms in knowledge graph
> import <file>                # Import markdown file into knowledge graph
> export <format>              # Export knowledge graph (json, csv)
> status                      # Show server status and statistics
> help                        # Show all available commands
\`\`\`

## Step 4: Import Your Content

Import your markdown files or documentation:

\`\`\`bash
# Import a single file
import ~/notes/project-a.md

# Import entire directory
import ~/Documents/knowledge-base/
\`\`\`

## Step 5: Configure Data Sources

Configure Terraphim to search different sources:

\`\`\`bash
# Search GitHub repositories
source add github https://github.com/terraphim/terraphim-ai

# Search StackOverflow
source add stackoverflow rust tokio

# Search local filesystem
source add filesystem ~/code/ --recursive
\`\`\`

## Step 6: Explore Features

### Semantic Search

\`\`\`bash
> search how to implement async channels in rust
\`\`\`

### Role-Based Filtering

\`\`\`bash
> role architect
> search system design patterns
\`\`\`

### Knowledge Graph Exploration

\`\`\`bash
> connect tokio async
> show tokio
\`\`\`

## CLI Automation

For automation and scripting, use the CLI instead of REPL:

\`\`\`bash
# Search and get JSON output
terraphim-cli search "async patterns" --format json

# Import files programmatically
terraphim-cli import ~/notes/*.md --recursive

# Set role and search
terraphim-cli search "rust error handling" --role engineer
\`\`\`

## Example Workflow

Here's a complete example workflow:

\`\`\`bash
# 1. Start the server (in one terminal)
terraphim-server &

# 2. Import your codebase (in another terminal)
terraphim-repl
> import ~/my-project/src/

# 3. Search for information
> search error handling patterns

# 4. Set role for better results
> role senior-engineer

# 5. Search again with role context
> search error handling patterns

# 6. Export results
> export json > search-results.json
\`\`\`

## Next Steps

- [Full Documentation](https://docs.terraphim.ai) - Comprehensive user guide and API reference
- [Installation Guide](/docs/installation) - More installation options and troubleshooting
- [Configuration Guide](/docs/terraphim_config) - Customize Terraphim to your needs
- [Contribution Guide](/docs/contribution) - Contribute to Terraphim development
- [Community](https://discord.gg/VPJXB6BGuY) - Join our Discord for support

## Getting Help

If you run into issues:

1. Check [troubleshooting section](https://docs.terraphim.ai/troubleshooting.html)
2. Search existing [GitHub issues](https://github.com/terraphim/terraphim-ai/issues)
3. [Create a new issue](https://github.com/terraphim/terraphim-ai/issues/new)
4. Join [Discord community](https://discord.gg/VPJXB6BGuY) for support
5. Contact us at [alex@terraphim.ai](mailto:alex@terraphim.ai)
