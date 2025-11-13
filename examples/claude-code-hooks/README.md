# Claude Code Hooks with Terraphim-TUI

This guide shows how to use Terraphim-TUI and its knowledge graph capabilities as a hook for Claude Code CLI to automatically enforce coding preferences, such as replacing package manager commands.

## Overview

Claude Code supports "hooks" - shell commands that execute in response to events like user prompt submission. This example demonstrates how to use Terraphim's knowledge graph and text replacement features to automatically convert package manager commands (npm, yarn, pnpm) to your preferred tool (bun).

## Why Use Terraphim as a Hook?

- **Knowledge Graph-Based**: Uses Terraphim's semantic matching for context-aware replacements
- **Configurable**: Define your own synonym mappings in markdown files
- **Fast**: Sub-100ms replacement using Aho-Corasick automata
- **Case Insensitive**: Works with any capitalization
- **Longest Match First**: Handles "npm install" before "npm" for precise replacements

## Quick Start

### 1. Build Terraphim-TUI

```bash
cargo build --release -p terraphim_tui
```

This creates the binary at `target/release/terraphim-tui`.

### 2. Test the Hook

```bash
cd examples/claude-code-hooks
./test-hook.sh
```

This runs a test suite to verify the hook works correctly.

### 3. Configure Claude Code

Add the hook to your Claude Code settings. The location depends on your setup:

**For Claude Code CLI (local):**
```bash
# Edit ~/.config/claude-code/settings.json
mkdir -p ~/.config/claude-code
```

Add this configuration:

```json
{
  "hooks": {
    "user-prompt-submit": {
      "command": "bash",
      "args": [
        "/full/path/to/terraphim-ai/examples/claude-code-hooks/terraphim-package-manager-hook.sh"
      ],
      "enabled": true,
      "description": "Replace package manager commands with bun"
    }
  }
}
```

**Important**: Replace `/full/path/to/` with the actual absolute path to your terraphim-ai directory.

### 4. Set Environment Variables (Optional)

```bash
# Use a specific terraphim-tui binary
export TERRAPHIM_TUI_BIN=/path/to/terraphim-tui

# Choose a different role from your terraphim config
export TERRAPHIM_ROLE="My Custom Role"

# Set the hook mode
export HOOK_MODE=replace  # replace, suggest, or passive
```

### 5. Test with Claude Code

Start a Claude Code session and try commands like:

```
"npm install the dependencies"
```

The hook will automatically replace it with:

```
"bun install the dependencies"
```

## How It Works

### 1. Knowledge Graph

The knowledge graph is defined in markdown files at `docs/src/kg/`:

**`docs/src/kg/bun.md`:**
```markdown
# Bun

Bun is a modern JavaScript runtime and package manager.

synonyms:: pnpm, npm, yarn
```

**`docs/src/kg/bun_install.md`:**
```markdown
# bun install

Fast package installation with Bun.

synonyms:: pnpm install, npm install, yarn install
```

These files create a thesaurus where:
- `npm` → `bun`
- `yarn` → `bun`
- `pnpm` → `bun`
- `npm install` → `bun install`
- `yarn install` → `bun install`
- `pnpm install` → `bun install`

### 2. The Hook Script

The hook script (`terraphim-package-manager-hook.sh`) does the following:

1. Reads the user's input from stdin
2. Checks if it contains package manager commands
3. Calls `terraphim-tui replace` to perform replacements
4. Returns the modified text

### 3. Terraphim-TUI Replace Command

```bash
terraphim-tui replace "npm install" --role "Terraphim Engineer"
# Output: bun install
```

The replace command:
- Loads the knowledge graph for the specified role
- Uses Aho-Corasick automata for fast pattern matching
- Replaces all matches with normalized terms
- Returns the transformed text

## Hook Modes

The hook supports three modes via the `HOOK_MODE` environment variable:

### Replace Mode (default)

```bash
export HOOK_MODE=replace
```

Automatically replaces package manager commands:
```
Input:  "npm install dependencies"
Output: "bun install dependencies"
```

### Suggest Mode

```bash
export HOOK_MODE=suggest
```

Shows suggestions but keeps the original:
```
Input:  "npm install dependencies"
Output: "npm install dependencies"
Stderr: "[Terraphim Hook] Suggestion: bun install dependencies"
```

### Passive Mode

```bash
export HOOK_MODE=passive
```

Only logs what would be replaced without modifying:
```
Input:  "npm install dependencies"
Output: "npm install dependencies"
Stderr: "[Terraphim Hook] Would replace with: bun install dependencies"
```

## Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `TERRAPHIM_TUI_BIN` | `terraphim-tui` | Path to terraphim-tui binary |
| `TERRAPHIM_ROLE` | `Terraphim Engineer` | Role name from terraphim config |
| `HOOK_MODE` | `replace` | Hook behavior mode |

### Claude Code Hook Configuration

```json
{
  "hooks": {
    "user-prompt-submit": {
      "command": "bash",
      "args": ["/path/to/hook.sh"],
      "enabled": true,
      "description": "Hook description"
    }
  }
}
```

- **command**: The shell command to execute
- **args**: Arguments passed to the command
- **enabled**: Whether the hook is active
- **description**: Human-readable description

## Adding Custom Replacements

To add your own replacements, create or edit markdown files in `docs/src/kg/`:

### Example: Enforce Deno over Node.js

Create `docs/src/kg/deno.md`:
```markdown
# Deno

Deno is a modern JavaScript runtime.

synonyms:: node, nodejs
```

Create `docs/src/kg/deno_run.md`:
```markdown
# deno run

Run a script with Deno.

synonyms:: node run, npm run
```

Restart terraphim-tui, and the hook will now replace `node` with `deno`.

### Example: Enforce Rust over Python

Create `docs/src/kg/rust.md`:
```markdown
# Rust

A systems programming language.

synonyms:: python, py
```

Now `python script.py` becomes `rust script.py` (though you'd want more sophisticated replacements in practice).

## Advanced Usage

### Multiple Knowledge Graphs

You can organize replacements into different domains:

```
docs/src/kg/
├── bun.md              # Package manager
├── typescript.md       # Language preferences
├── databases.md        # Database tools
└── frameworks.md       # Framework preferences
```

All files are loaded together into one knowledge graph.

### Role-Specific Replacements

Use different roles for different projects:

```bash
# For Node.js projects
export TERRAPHIM_ROLE="Node.js Engineer"

# For Deno projects
export TERRAPHIM_ROLE="Deno Engineer"
```

Each role can have its own knowledge graph and preferences.

### Chaining Multiple Hooks

You can chain hooks by making one hook call another:

```bash
#!/usr/bin/env bash
# First hook
OUTPUT=$(cat | /path/to/first-hook.sh)
# Second hook
echo "$OUTPUT" | /path/to/second-hook.sh
```

## Testing

### Manual Testing

Test the hook directly:

```bash
echo "npm install dependencies" | ./terraphim-package-manager-hook.sh
# Output: bun install dependencies
```

### Automated Testing

Run the test suite:

```bash
./test-hook.sh
```

This tests:
- ✓ npm install → bun install
- ✓ yarn build → bun build
- ✓ pnpm test → bun test
- ✓ Case insensitivity
- ✓ Multiple commands
- ✓ Pass-through for non-package-manager commands

### Unit Tests

The underlying functionality is tested in the Rust codebase:

```bash
cargo test -p terraphim_tui --test replace_feature_tests
```

## Troubleshooting

### Hook Not Working

1. **Check if terraphim-tui is built:**
   ```bash
   ls target/release/terraphim-tui
   ```
   If not, build it:
   ```bash
   cargo build --release -p terraphim_tui
   ```

2. **Verify the hook script is executable:**
   ```bash
   chmod +x examples/claude-code-hooks/terraphim-package-manager-hook.sh
   ```

3. **Test the hook directly:**
   ```bash
   echo "npm install" | ./examples/claude-code-hooks/terraphim-package-manager-hook.sh
   ```

4. **Check Claude Code settings path:**
   Ensure the path in your settings.json is absolute, not relative.

### Knowledge Graph Not Loading

1. **Verify KG files exist:**
   ```bash
   ls docs/src/kg/bun.md
   ls docs/src/kg/bun_install.md
   ```

2. **Check role name:**
   ```bash
   terraphim-tui roles list
   ```

3. **Test replacement directly:**
   ```bash
   terraphim-tui replace "npm install" --role "Terraphim Engineer"
   ```

### Replacements Not Accurate

The Aho-Corasick matcher uses:
- **Case insensitive** matching: "NPM" = "npm"
- **Leftmost longest** match: "npm install" matched before "npm"
- **Non-overlapping**: Each position matched only once

To debug:
```bash
# Enable verbose mode (if supported)
terraphim-tui replace "npm install" --role "Terraphim Engineer" --verbose
```

## Performance

- **Hook execution**: ~10-50ms per invocation
- **Pattern matching**: Uses Aho-Corasick, O(n + m) where n = text length, m = number of matches
- **Memory**: ~5-10MB for typical knowledge graphs
- **Startup cost**: ~100-200ms to load knowledge graph (cached after first run)

For large inputs (>1MB), consider processing in chunks.

## Security Considerations

1. **Arbitrary Command Execution**: Hooks execute shell commands. Only use trusted scripts.
2. **Input Validation**: The hook script validates input before processing.
3. **Error Handling**: Failed hooks won't break Claude Code (exit 0 on error).
4. **Permissions**: Hook scripts should not require elevated permissions.

## Examples

### Example 1: Convert Package.json Scripts

**Input:**
```json
{
  "scripts": {
    "install": "npm install",
    "build": "yarn build",
    "test": "pnpm test"
  }
}
```

**After hook processes the prompt:**
```json
{
  "scripts": {
    "install": "bun install",
    "build": "bun build",
    "test": "bun test"
  }
}
```

### Example 2: Convert Shell Scripts

**Input:**
```bash
#!/bin/bash
npm install
npm run build
npm test
```

**After hook:**
```bash
#!/bin/bash
bun install
bun run build
bun test
```

### Example 3: Convert Documentation

**Input:**
```markdown
# Installation

Run `npm install` to install dependencies.

For development:
```bash
yarn dev
```

**After hook:**
```markdown
# Installation

Run `bun install` to install dependencies.

For development:
```bash
bun dev
```

## Integration with Other Tools

### Git Hooks

You can use the same Terraphim hook in git commit messages:

```bash
# .git/hooks/commit-msg
#!/bin/bash
MSG_FILE=$1
CONTENT=$(cat "$MSG_FILE")
REPLACED=$(echo "$CONTENT" | /path/to/terraphim-package-manager-hook.sh)
echo "$REPLACED" > "$MSG_FILE"
```

### CI/CD Pipelines

Validate that package.json uses preferred tools:

```yaml
# .github/workflows/validate.yml
- name: Validate package manager
  run: |
    if grep -q "npm\\|yarn\\|pnpm" package.json; then
      echo "❌ Use bun instead of npm/yarn/pnpm"
      exit 1
    fi
```

### IDE Integration

Many IDEs support external formatters. Point them to terraphim-tui:

```json
{
  "editor.formatOnSave": true,
  "editor.defaultFormatter": "custom",
  "custom.formatter.command": "terraphim-tui replace ${file} --role 'Engineer'"
}
```

## Extending the Hook

### Add Logging

```bash
# Add to hook script
echo "$(date): Replaced '$INPUT' with '$REPLACED'" >> /tmp/terraphim-hook.log
```

### Add Metrics

```bash
# Add to hook script
if [ "$REPLACED" != "$INPUT" ]; then
    echo "hook.replacement.count:1|c" | nc -u -w1 localhost 8125  # StatsD
fi
```

### Add Notifications

```bash
# Add to hook script
if [ "$REPLACED" != "$INPUT" ]; then
    notify-send "Terraphim Hook" "Replaced package manager command"
fi
```

## Best Practices

1. **Test Before Enabling**: Always test hooks with `test-hook.sh` before enabling in Claude Code
2. **Use Specific Roles**: Create roles for different projects with appropriate knowledge graphs
3. **Version Control KG Files**: Keep `docs/src/kg/` in version control for team consistency
4. **Document Replacements**: Add comments in KG files explaining why synonyms exist
5. **Start with Suggest Mode**: Use `HOOK_MODE=suggest` initially to verify behavior
6. **Monitor Performance**: Check hook execution time with `time` command
7. **Handle Errors Gracefully**: Hooks should never block Claude Code (exit 0 on error)

## FAQ

**Q: Can I use this with Claude Code on the web?**
A: No, hooks are only available in the Claude Code CLI (local) version.

**Q: Will this work with other AI assistants?**
A: Yes! The hook script is generic and can be adapted for any tool that supports shell hooks.

**Q: Can I disable the hook temporarily?**
A: Yes, set `"enabled": false` in settings.json or `export HOOK_MODE=passive`.

**Q: How do I update the knowledge graph?**
A: Edit files in `docs/src/kg/` and rebuild terraphim-tui. Changes are picked up automatically.

**Q: Can I use multiple hooks?**
A: Yes, chain them by calling one from another (see Advanced Usage).

**Q: What if terraphim-tui crashes?**
A: The hook script catches errors and falls back to the original input.

## Related Documentation

- [Terraphim TUI Documentation](../../crates/terraphim_tui/README.md)
- [Knowledge Graph System](../../docs/src/kg/knowledge-graph-system.md)
- [Package Manager Replacement Guide](../../docs/src/kg/PACKAGE_MANAGER_REPLACEMENT.md)
- [Thesaurus Documentation](../../docs/src/kg/thesaurus.md)

## Contributing

To contribute improvements to this hook:

1. Test your changes with `test-hook.sh`
2. Update this README with new features
3. Add tests for new functionality
4. Submit a PR with a clear description

## License

This example is part of the Terraphim AI project and follows the same license (Apache-2.0).

## Support

For issues or questions:
- Open an issue: https://github.com/terraphim/terraphim-ai/issues
- Documentation: https://docs.terraphim.ai
- Community: https://discord.gg/terraphim
