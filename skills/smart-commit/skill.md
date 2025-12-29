# Smart Commit

Use this skill to enhance commit messages with knowledge graph concepts extracted from changed files.

## When to Use

- When creating commits with meaningful domain context
- To auto-tag commits with relevant concepts
- For better commit searchability and traceability
- When following semantic commit conventions

## How It Works

1. Extracts diff from staged changes
2. Identifies knowledge graph concepts in the diff
3. Appends relevant concepts to commit message
4. Maintains human-written message integrity

## Usage

### Enable Smart Commit

Set environment variable before committing:

```bash
export TERRAPHIM_SMART_COMMIT=1
git commit -m "Your message"
```

### One-Time Smart Commit

```bash
TERRAPHIM_SMART_COMMIT=1 git commit -m "Your message"
```

### Manual Concept Extraction

```bash
# Extract concepts from staged diff
git diff --cached | terraphim-agent hook --hook-type prepare-commit-msg --input '{"diff": "..."}'

# Or directly validate the diff
git diff --cached | terraphim-agent validate --json
```

## Example Output

**Before (original message):**
```
feat: add user authentication
```

**After (with smart commit enabled):**
```
feat: add user authentication

Concepts: authentication, security, user, login, session
```

## Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `TERRAPHIM_SMART_COMMIT` | `0` | Enable smart commit (set to `1`) |
| `TERRAPHIM_VERBOSE` | `0` | Show debug output |

### Concept Limit

By default, only the top 5 unique concepts are added. This keeps commit messages clean while providing useful context.

## Integration

### Git Hook Installation

```bash
# Install prepare-commit-msg hook
./scripts/install-terraphim-hooks.sh

# Or manually
cp scripts/hooks/prepare-commit-msg .git/hooks/
chmod +x .git/hooks/prepare-commit-msg
```

### Claude Code Integration

Smart commit works automatically when:
1. The prepare-commit-msg hook is installed
2. `TERRAPHIM_SMART_COMMIT=1` is set
3. terraphim-agent is available

## Best Practices

1. **Concise Messages First**: Write your commit message normally
2. **Review Concepts**: Check that extracted concepts are relevant
3. **Disable When Needed**: Unset `TERRAPHIM_SMART_COMMIT` for quick fixes
4. **Role Selection**: Concepts come from the current role's knowledge graph

## Troubleshooting

| Issue | Solution |
|-------|----------|
| No concepts added | Check `TERRAPHIM_SMART_COMMIT=1` is set |
| Wrong concepts | Try different role with `--role` flag |
| Hook not running | Verify `.git/hooks/prepare-commit-msg` exists and is executable |
| Agent not found | Build with `cargo build --release -p terraphim_agent` |

## Related Skills

- `pre-llm-validate` - Validate input before LLM calls
- `post-llm-check` - Validate LLM outputs against checklists
- `terraphim-hooks` - Full hooks documentation
