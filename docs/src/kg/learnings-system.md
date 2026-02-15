# Learning Capture System

The Learning Capture System automatically captures failed commands and their error output as learning documents. This helps build a personal knowledge base of common mistakes and their corrections.

## Overview

When you run a command that fails, the Learning Capture System:

1. **Captures** the command and error output
2. **Redacts** any secrets (API keys, passwords, etc.)
3. **Stores** the learning in Markdown format
4. **Indexes** it for later search and retrieval

## Architecture

```text
Failed Command → Secret Redaction → Storage → Indexing → Search
                     ↓
              Auto-Suggest (future)
```

## Storage Locations

Learnings are stored in two locations:

### Project-Specific (`.terraphim/learnings/`)
- Captured when you're in a project directory
- Shared with project-specific knowledge graph
- Priority location for queries

### Global (`~/.local/share/terraphim/learnings/`)
- Fallback when not in a project
- Common patterns across all projects
- Always searched as secondary source

## Usage

### Capturing a Failed Command

```bash
terraphim-agent learn capture "git push -f" \
  --error "remote: rejected" \
  --exit-code 1
```

This creates a Markdown file in your learnings directory with:
- The command that failed
- The error output (with secrets redacted)
- Exit code
- Timestamp and context
- Unique ID for reference

### Listing Recent Learnings

```bash
# Show last 10 learnings (default)
terraphim-agent learn list

# Show more
terraphim-agent learn list --recent 20

# Show global learnings
terraphim-agent learn list --global
```

Output shows:
```
Recent learnings:
  1. [P] git push -f (exit: 1)
  2. [G] npm install (exit: 1)
  3. [P] cargo build (exit: 101)
```

`[P]` = Project-specific, `[G]` = Global

### Querying Learnings

```bash
# Search by substring (default)
terraphim-agent learn query "git push"

# Exact match
terraphim-agent learn query "git push -f" --exact

# Search global learnings
terraphim-agent learn query "npm" --global
```

## Automatic Capture with Hooks

The Learning Capture System can automatically capture failed commands via hooks.

### PostToolUse Hook

Add to your Claude Code configuration to automatically capture failed Bash commands:

```json
{
  "hooks": {
    "PostToolUse": ".claude/hooks/learning-capture.sh"
  }
}
```

The hook:
- Only captures **failed** commands (non-zero exit)
- Automatically **redacts secrets** before storage
- Is **fail-open** - doesn't block if capture fails
- Works transparently in the background

### Debug Mode

Enable debug output to see what's being captured:

```bash
export TERRAPHIM_LEARN_DEBUG=true
```

## Secret Redaction

Before storing, the system automatically redacts:

- **AWS keys** (`AKIA...`)
- **API tokens** (`sk-...`, `ghp_...`, `xoxb-...`)
- **Connection strings** (`postgresql://...`, `mysql://...`)
- **Environment variables** (`VAR=value` patterns)

Example:
```
Before: postgresql://user:secret@localhost/db
After:  postgresql://[REDACTED]@localhost/db
```

## Ignored Commands

The following commands are automatically ignored (not captured):

- `cargo test*` - Test commands
- `npm test*` - Test commands  
- `pytest*` - Test commands
- `yarn test*` - Test commands

This prevents test failures from cluttering your learnings.

## Learning Document Format

Learnings are stored as Markdown files with YAML frontmatter:

```markdown
---
id: abc123-1708012345678
command: git push -f
exit_code: 1
source: Project
captured_at: 2024-01-15T10:30:00Z
working_dir: /home/user/myproject
---

## Command

`git push -f`

## Error Output

```
remote: rejected
```
```

## Future Features

Coming in future updates:

- **Auto-suggest corrections** - Query existing KG for suggested fixes
- **Add corrections** - `terraphim-agent learn correct <id> --correction "..."`
- **Web UI** - Browse and manage learnings visually
- **Export/Import** - Share learnings between machines
- **Team sharing** - Share common patterns with team

## Configuration

The Learning Capture System uses sensible defaults but can be configured via environment:

```bash
# Enable debug output
export TERRAPHIM_LEARN_DEBUG=true

# Custom directories (future feature)
export TERRAPHIM_LEARN_PROJECT_DIR=".myproject/learnings"
export TERRAPHIM_LEARN_GLOBAL_DIR="~/.myapp/learnings"
```

## Troubleshooting

### Learnings not being captured

1. Check that `terraphim-agent` is in your PATH
2. Enable debug mode: `export TERRAPHIM_LEARN_DEBUG=true`
3. Verify storage directory is writable
4. Check if command matches ignore patterns

### Hook not working

1. Ensure hook script is executable: `chmod +x learning-capture.sh`
2. Check Claude Code hook configuration
3. Verify `terraphim-agent` binary exists

### Storage location

Find where learnings are stored:

```bash
# Check project location
ls -la .terraphim/learnings/

# Check global location
ls -la ~/.local/share/terraphim/learnings/
```

## See Also

- [Knowledge Graph Documentation](knowledge-graph.md)
- [Terraphim Agent CLI](../reference/cli.md)
- [Hook System](../reference/hooks.md)
