+++
title="Teaching AI Coding Agents with Knowledge Graph Hooks"
date=2025-12-28

[taxonomies]
categories = ["Technical"]
tags = ["Terraphim", "ai", "hooks", "knowledge-graph", "claude-code", "developer-tools", "bun", "anthropic"]
[extra]
toc = true
comments = true
+++

How we use Aho-Corasick automata and knowledge graphs to automatically enforce coding standards across AI coding agents like Claude Code, Cursor, and Aider.

<!-- more -->

## Anthropic Bought Bun. Claude Still Outputs `npm install`.

On December 3, 2025, [Anthropic announced its first-ever acquisition](https://www.anthropic.com/news/anthropic-acquires-bun-as-claude-code-reaches-usd1b-milestone): Bun, the blazing-fast JavaScript runtime. This came alongside Claude Code reaching [$1 billion in run-rate revenue](https://bun.com/blog/bun-joins-anthropic) just six months after public launch.

As Mike Krieger, Anthropic's Chief Product Officer, put it:

> "Bun represents exactly the kind of technical excellence we want to bring into Anthropic... bringing the Bun team into Anthropic means we can build the infrastructure to compound that momentum."

Claude Code [ships as a Bun executable](https://simonwillison.net/2025/Dec/2/anthropic-acquires-bun/) to millions of developers. Anthropic now owns the runtime their flagship coding tool depends on.

**And yet...**

Ask Claude to set up a Node.js project, and what do you get?

```bash
npm install express
yarn add lodash
pnpm install --save-dev jest
```

Yet Anthropic's own models still default to npm, yarn, and pnpm in their outputs. The training data predates the acquisition, and old habits die hard.

**So how do you teach your AI coding tools to consistently use Bun, regardless of what the underlying LLM insists on?**

## The Problem: LLMs Don't Know Your Preferences

AI coding agents are powerful, but they're trained on the internet's collective habits—which means npm everywhere. Your team might have standardized on Bun for its speed (25% monthly growth, [7.2 million downloads](https://devclass.com/2025/12/03/bun-javascript-runtime-acquired-by-anthropic-tying-its-future-to-ai-coding/) in October 2025), but every AI agent keeps suggesting the old ways.

Manually fixing these inconsistencies is tedious. What if your knowledge graph could automatically intercept and transform AI outputs?

## The Solution: Knowledge Graph Hooks

Terraphim provides a hook system that intercepts AI agent actions and applies knowledge graph-based transformations. The system uses:

1. **Aho-Corasick automata** for efficient multi-pattern matching
2. **LeftmostLongest strategy** ensuring specific patterns match before general ones
3. **Markdown-based knowledge graph** files that are human-readable and version-controlled

### How It Works

```
Input Text → Aho-Corasick Automata → Pattern Match → Knowledge Graph Lookup → Transformed Output
```

The knowledge graph is built from simple markdown files:

```markdown
# bun install

Fast package installation with Bun.

synonyms:: pnpm install, npm install, yarn install
```

When the automata encounter any synonym, they replace it with the canonical term (the heading).

## Real-World Example: npm → bun

Let's prove it works. Here's a live test:

```bash
$ echo "npm install" | terraphim-agent replace
bun install

$ echo "yarn install lodash" | terraphim-agent replace
bun install lodash

$ echo "pnpm install --save-dev jest" | terraphim-agent replace
bun install --save-dev jest
```

The LeftmostLongest matching ensures `npm install` matches the more specific pattern before standalone `npm` could match.

## Hook Integration Points

Terraphim hooks integrate at multiple points in the development workflow:

### 1. Claude Code PreToolUse Hooks

Intercept Bash commands before execution:

```json
{
  "hooks": {
    "PreToolUse": [{
      "matcher": "Bash",
      "hooks": [{
        "type": "command",
        "command": "terraphim-agent replace"
      }]
    }]
  }
}
```

When Claude Code tries to run `npm install express`, the hook transforms it to `bun install express` before execution.

### 2. Git prepare-commit-msg Hooks

Enforce attribution standards in commits:

```bash
#!/bin/bash
COMMIT_MSG_FILE=$1
ORIGINAL=$(cat "$COMMIT_MSG_FILE")
TRANSFORMED=$(echo "$ORIGINAL" | terraphim-agent replace)
echo "$TRANSFORMED" > "$COMMIT_MSG_FILE"
```

With a knowledge graph entry:

```markdown
# Terraphim AI

Attribution for AI-assisted development.

synonyms:: Claude Code, Claude, Anthropic Claude
```

Every commit message mentioning "Claude Code" becomes "Terraphim AI".

### 3. MCP Tools

The `replace_matches` MCP tool exposes the same functionality to any MCP-compatible client:

```json
{
  "tool": "replace_matches",
  "arguments": {
    "text": "Run npm install to setup"
  }
}
```

## Architecture

The hook system is built on three crates:

| Crate | Purpose |
|-------|---------|
| `terraphim_automata` | Aho-Corasick pattern matching, thesaurus building |
| `terraphim_hooks` | ReplacementService, HookResult, binary discovery |
| `terraphim_agent` | CLI with `replace` subcommand |

### Performance

- **Pattern matching**: O(n) where n is input length (not pattern count)
- **Startup**: ~50ms to load knowledge graph and build automata
- **Memory**: Automata are compact finite state machines

## Extending the Knowledge Graph

Adding new patterns is simple. Create a markdown file in the mdBook source tree under `docs/src/kg/` (published at https://docs.terraphim.ai/src/kg/).

```markdown
# pytest

Python testing framework.

synonyms:: python -m unittest, unittest, nose
```

The system automatically rebuilds the automata on startup.

### Pattern Priority

The LeftmostLongest strategy means:
- `npm install` matches before `npm`
- `python -m pytest` matches before `python`
- Longer, more specific patterns always win

## Installation

### Quick Setup

```bash
# Install all hooks
./scripts/install-terraphim-hooks.sh --easy-mode

# Test the replacement
echo "npm install" | ./target/release/terraphim-agent replace
```

### Manual Setup

1. Build the agent:
```bash
cargo build -p terraphim_agent --features repl-full --release
```

2. Configure Claude Code hooks in `.claude/settings.local.json`

3. Install Git hooks:
```bash
cp scripts/hooks/prepare-commit-msg .git/hooks/
chmod +x .git/hooks/prepare-commit-msg
```

## Use Cases

| Use Case | Pattern | Replacement |
|----------|---------|-------------|
| Package manager standardization | npm, yarn, pnpm | bun |
| AI attribution | Claude Code, Claude | Terraphim AI |
| Framework migration | React.Component | React functional components |
| API versioning | /api/v1 | /api/v2 |
| Deprecated function replacement | moment() | dayjs() |

## Claude Code Skills Plugin

For AI agents that support skills, we provide a dedicated plugin:

```bash
claude plugin install terraphim-engineering-skills@terraphim-ai
```

The `terraphim-hooks` skill teaches agents how to:
- Use the replace command correctly
- Extend the knowledge graph
- Debug hook issues

## Conclusion

Knowledge graph hooks provide a powerful, declarative way to enforce coding standards across AI agents. By defining patterns in simple markdown files, you can:

- Standardize package managers across your team
- Ensure consistent attribution in commits
- Migrate deprecated patterns automatically
- Keep your knowledge graph version-controlled and human-readable

The Aho-Corasick automata ensure efficient matching regardless of pattern count, making this approach scale to large knowledge graphs.

## Resources

- [Terraphim AI Repository](https://github.com/terraphim/terraphim-ai)
- [Claude Code Skills Plugin](https://github.com/terraphim/terraphim-claude-skills)
- [Hook Installation Guide](https://docs.terraphim.ai/hooks/)
- [Knowledge Graph Documentation](https://docs.terraphim.ai/knowledge-graph/)
