# Claude Code Skills Integration

Terraphim provides a set of Claude Code skills that teach AI coding agents how to use Terraphim's knowledge graph capabilities. These skills are available as a Claude Code plugin.

## Installation

### From GitHub

```bash
# Add the Terraphim marketplace
claude plugin marketplace add terraphim/terraphim-claude-skills

# Install the engineering skills plugin
claude plugin install terraphim-engineering-skills@terraphim-ai
```

### From Local Clone

```bash
# Clone the repository
git clone https://github.com/terraphim/terraphim-claude-skills.git

# Add as local marketplace
claude plugin marketplace add ./terraphim-claude-skills

# Install the plugin
claude plugin install terraphim-engineering-skills@terraphim-ai
```

## Terraphim-Specific Skills

### terraphim-hooks

Knowledge graph-based text replacement using Terraphim hooks. This skill teaches Claude Code how to:

- **PreToolUse Hooks**: Intercept commands before execution (e.g., replace `npm install` with `bun install`)
- **Git Hooks**: Transform commit messages (e.g., replace "Claude Code" attribution with "Terraphim AI")
- **CLI Replace Command**: Use `terraphim-agent replace` for text transformation

**Example Usage:**

```bash
# Replace npm with bun using knowledge graph
echo "npm install react" | terraphim-agent replace
# Output: bun install react

# JSON output for programmatic use
echo "npm install" | terraphim-agent replace --json
# Output: {"result":"bun install","original":"npm install","replacements":1,"changed":true}
```

**Hook Configuration:**

Add to `.claude/settings.local.json`:
```json
{
  "hooks": {
    "PreToolUse": [{
      "matcher": "Bash",
      "hooks": [{
        "type": "command",
        "command": ".claude/hooks/npm_to_bun_guard.sh"
      }]
    }]
  }
}
```

### session-search

Search and analyze AI coding assistant session history. This skill teaches Claude Code how to:

- **Search Sessions**: Find past work by query, concept, or related sessions
- **Import History**: Load sessions from Claude Code, Cursor, Aider, and other assistants
- **Analyze Patterns**: Discover agent usage patterns and productivity trends
- **Export Sessions**: Save sessions to JSON or Markdown

**REPL Commands:**

| Command | Description |
|---------|-------------|
| `/sessions sources` | Detect available session sources |
| `/sessions import` | Import sessions from all sources |
| `/sessions search <query>` | Full-text search |
| `/sessions concepts <term>` | Knowledge graph concept search |
| `/sessions related <id>` | Find related sessions |
| `/sessions timeline` | Timeline visualization |
| `/sessions export` | Export to file |

**Example Workflow:**

```bash
# Launch REPL with session support
./target/release/terraphim-agent

# In REPL:
/sessions sources          # Detect available sources
/sessions import           # Import from Claude Code
/sessions search "rust"    # Find sessions about Rust
/sessions concepts "error handling"  # Concept-based search
```

## Engineering Skills

The plugin also includes general engineering skills:

| Skill | Description |
|-------|-------------|
| `architecture` | System design, ADRs, API planning |
| `implementation` | Production code with tests |
| `testing` | Unit, integration, property-based tests |
| `debugging` | Systematic root cause analysis |
| `rust-development` | Idiomatic Rust patterns |
| `rust-performance` | Profiling, SIMD, optimization |
| `code-review` | Thorough review for bugs/security |
| `documentation` | API docs, README, guides |
| `devops` | CI/CD, Docker, deployment |

## Disciplined Development Workflow

For complex features, use the three-phase approach:

```
Phase 1: Research          Phase 2: Design           Phase 3: Implementation
┌─────────────────┐       ┌─────────────────┐       ┌─────────────────┐
│disciplined-     │  →    │disciplined-     │   →   │disciplined-     │
│research         │       │design           │       │implementation   │
│                 │       │                 │       │                 │
│ • Problem scope │       │ • File changes  │       │ • Test first    │
│ • System mapping│       │ • API signatures│       │ • Small commits │
│ • Constraints   │       │ • Test strategy │       │ • Quality checks│
└─────────────────┘       └─────────────────┘       └─────────────────┘
```

## Knowledge Graph Integration

Skills leverage Terraphim's knowledge graph for:

### Text Replacement

Define replacement patterns in `docs/src/kg/`:

```markdown
# bun

Modern JavaScript runtime and package manager.

synonyms:: npm, yarn, pnpm, npx
```

### Concept Search

Sessions are enriched with knowledge graph concepts for semantic search:

```rust
use terraphim_sessions::{SessionEnricher, EnrichmentConfig};

let enricher = SessionEnricher::new(config)?;
let enriched = enricher.enrich(&session)?;

// Find sessions by concept
let results = search_by_concept(&sessions, "error handling")?;
```

## Quick Setup

Install all Terraphim hooks and skills:

```bash
# In terraphim-ai repository
./scripts/install-terraphim-hooks.sh --easy-mode

# Test hooks are working
./scripts/test-terraphim-hooks.sh

# Build with session support
cargo build -p terraphim_agent --features repl-full --release
```

## Repository

- **Skills Repository**: [github.com/terraphim/terraphim-claude-skills](https://github.com/terraphim/terraphim-claude-skills)
- **Main Repository**: [github.com/terraphim/terraphim-ai](https://github.com/terraphim/terraphim-ai)

## Related Documentation

- [MCP Integration](./mcp-integration.md) - MCP server for AI tool integration
- [TUI Documentation](./tui.md) - Terminal UI with REPL commands
- [Knowledge Graph](./kg/knowledge-graph.md) - Building knowledge graphs
