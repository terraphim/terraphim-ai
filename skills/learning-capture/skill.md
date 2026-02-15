# Learning Capture

Use this skill to automatically capture failed commands and their error output as learning documents. Build a personal knowledge base of common mistakes and solutions.

## When to Use

- When commands fail and you want to remember the fix
- To build up a searchable database of common errors
- For tracking patterns across multiple projects
- When onboarding team members (share common pitfalls)
- To reduce time spent debugging the same issues

## How It Works

1. **Capture** - Records failed commands with error context
2. **Redact** - Automatically removes secrets (API keys, passwords)
3. **Store** - Saves as Markdown in `.terraphim/learnings/` or `~/.local/share/terraphim/learnings/`
4. **Index** - Makes learnings searchable via knowledge graph
5. **Retrieve** - Query past failures to find solutions

## Quick Start

### Manual Capture

```bash
# After a command fails, capture it manually
terraphim-agent learn capture "git push -f" \
  --error "remote: rejected" \
  --exit-code 1

# Output: Captured learning: .terraphim/learnings/learning-abc123.md
```

### Automatic Capture via Hook

Enable automatic capture of all failed Bash commands:

```bash
# Add to .claude/config.json
{
  "hooks": {
    "PostToolUse": ".claude/hooks/learning-capture.sh"
  }
}
```

From then on, every failed command is automatically captured.

## Usage

### Capture a Failed Command

```bash
terraphim-agent learn capture <command> --error <errmsg> [--exit-code N]
```

**Parameters:**
- `command` - The command that failed (required)
- `--error` - Error message or stderr output (required)
- `--exit-code` - Exit code (default: 1)
- `--debug` - Show debug output

**Example:**
```bash
terraphim-agent learn capture "npm install" \
  --error "EACCES: permission denied, mkdir '/node_modules'" \
  --exit-code 243
```

### List Recent Learnings

```bash
# Show last 10 learnings (default)
terraphim-agent learn list

# Show more
terraphim-agent learn list --recent 20

# Show global learnings
terraphim-agent learn list --global
```

**Output:**
```
Recent learnings:
  1. [P] git push -f (exit: 1)
  2. [G] npm install (exit: 243)
  3. [P] cargo build (exit: 101)
     Correction: cargo build --release
```

`[P]` = Project-specific | `[G]` = Global

### Query Learnings

```bash
# Search by substring
terraphim-agent learn query "git push"

# Exact match
terraphim-agent learn query "git push -f" --exact

# Search global only
terraphim-agent learn query "npm" --global
```

## Storage Locations

Learnings are stored as Markdown files:

### Project-Specific (`.terraphim/learnings/`)
- Created when in a project directory
- Priority for project-specific queries
- Committed with project (optional)

### Global (`~/.local/share/terraphim/learnings/`)
- Fallback location
- Common patterns across all projects
- Never committed

## Secret Redaction

Before storage, the system automatically redacts:

| Pattern | Example | Redacted |
|---------|---------|----------|
| AWS keys | `AKIAIOSFODNN7EXAMPLE` | `[AWS_KEY_REDACTED]` |
| OpenAI keys | `sk-proj-abc123` | `[OPENAI_KEY_REDACTED]` |
| GitHub tokens | `ghp_abc123` | `[GITHUB_TOKEN_REDACTED]` |
| Slack tokens | `xoxb-abc123` | `[SLACK_TOKEN_REDACTED]` |
| DB connection | `postgresql://user:pass@host` | `postgresql://[REDACTED]@host` |
| Env vars | `DATABASE_URL=postgres://...` | `DATABASE_URL=[ENV_REDACTED]` |

## Ignored Commands

The following are automatically ignored (not captured):

- `cargo test*` - Rust tests
- `npm test*` - Node tests
- `pytest*` - Python tests
- `yarn test*` - Yarn tests

## Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `TERRAPHIM_LEARN_DEBUG` | `false` | Show debug output |

### Hook Configuration

Create or edit `.claude/config.json`:

```json
{
  "hooks": {
    "PostToolUse": ".claude/hooks/learning-capture.sh"
  }
}
```

## Learning Document Format

Each learning is stored as Markdown with YAML frontmatter:

```markdown
---
id: abc123-1708012345678
command: git push -f
exit_code: 1
source: Project
captured_at: 2024-01-15T10:30:00Z
working_dir: /home/user/myproject
tags:
  - learning
  - exit-1
---

## Command

`git push -f`

## Error Output

```
remote: rejected
```
```

## Best Practices

1. **Review Before Capture** - Check that error output doesn't contain sensitive info
2. **Add Corrections** - When you find the fix, add it to the learning
3. **Query Regularly** - Before asking for help, search your learnings
4. **Clean Up** - Periodically remove outdated learnings
5. **Share Common Patterns** - Commit project learnings to help teammates

## Troubleshooting

| Issue | Solution |
|-------|----------|
| Capture fails silently | Enable debug: `TERRAPHIM_LEARN_DEBUG=true` |
| Hook not capturing | Check `terraphim-agent` is in PATH |
| Storage full | Remove old learnings or change location |
| Wrong directory captured | Check current working directory |
| Secrets in output | Verify redaction patterns cover your secrets |

## Integration Examples

### With Shell Aliases

```bash
# Add to .bashrc or .zshrc
alias learn='terraphim-agent learn'
alias learnc='terraphim-agent learn capture'
alias learnl='terraphim-agent learn list'
alias learnq='terraphim-agent learn query'

# Usage
learn capture "cmd" --error "msg"
learnl --recent 5
learnq "git push"
```

### With Git

```bash
# Show learnings for current project
git rev-parse --show-toplevel | xargs -I {} terraphim-agent learn list

# Commit learnings with project
git add .terraphim/learnings/
git commit -m "docs: add common error patterns"
```

### With Makefile

```makefile
# Capture failed commands in CI
learn:
	@terraphim-agent learn list --recent 10

.PHONY: learn
```

## Future Enhancements

Coming in future releases:

- **Auto-suggest corrections** - Query KG for suggested fixes
- **Web UI** - Browse and manage learnings visually
- **Export/Import** - Share between machines
- **Team sharing** - Common patterns for teams
- **Statistics** - Most common errors, success rates

## Related Skills

- `smart-commit` - Enhance commits with knowledge graph concepts
- `pre-llm-validate` - Validate input before LLM calls
- `post-llm-check` - Validate LLM outputs against checklists
- `terraphim-hooks` - Full hooks documentation

## See Also

- [Learning System Documentation](../../docs/src/kg/learnings-system.md)
- [Knowledge Graph Overview](../../docs/src/kg/knowledge-graph.md)
- [CLI Reference](../../docs/src/reference/cli.md)
