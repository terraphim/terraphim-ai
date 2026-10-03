# Howto: Learning Capture for opencode

## What it does

opencode automatically captures failed bash commands and stores them as learnings. When you run the same failing command again, you get a warning with the previous error and any recorded correction. Both opencode and Claude Code share the same learning store.

## Architecture

```
opencode plugin (~/.config/opencode/plugin/terraphim-hooks.js)
    |
    |--- tool.execute.before
    |       |---> terraphim-agent guard --json              (safety, can throw)
    |       |---> terraphim-agent learn hook ...pre-tool-use (warn on past failures)
    |       |---> terraphim-agent replace --role ... --json  (rewrite git commits / npm->bun)
    |
    |--- tool.execute.after
    |       |---> terraphim-agent learn hook --format opencode
    |
                                                     |
                                                     v
                                         Shared learning store
                                      (~/.config/terraphim/learnings/)
```

## Prerequisites

- `terraphim-agent` installed at `~/.cargo/bin/terraphim-agent`
- opencode v1.4.6+ installed (`bun install -g opencode-ai`)

## Setup

The plugin is already installed at `~/.config/opencode/plugin/terraphim-hooks.js`. No additional configuration needed — opencode auto-loads all `.js` files from `~/.config/opencode/plugin/` and `~/.config/opencode/plugins/`.

### Verify it works

```bash
# 1. Trigger a failing command inside opencode
opencode --prompt "Run: nonexistent-test-command-xyz"

# 2. Check the learning was captured
terraphim-agent learn query "nonexistent-test-command-xyz"
```

Expected output:
```
Learnings matching 'nonexistent-test-command-xyz'.
  [G] [cmd] nonexistent-test-command-xyz (exit: 127)
```

## How learning capture works

### Capture (automatic)

When any bash command fails (non-zero exit code) inside opencode, the `tool.execute.after` hook fires and sends the command, output, and exit code to `terraphim-agent learn hook --format opencode`.

The hook:
1. Checks `input.tool` is `"bash"` (lowercase)
2. Extracts the command from `input.args.command`
3. Gets the raw output from `output.output` (string)
4. Parses exit code from output text (opencode doesn't provide exit code directly)
5. Constructs structured JSON and pipes to `terraphim-agent learn hook`

### Pre-tool-use warnings (automatic)

When you run a command that previously failed, the `tool.execute.before` hook queries past learnings. Instead of printing warnings to the terminal (which interrupts the user), hints are silently written to `~/.local/share/terraphim/session-hints.txt` so they can be consumed by the LLM in subsequent turns without cluttering the UI.

This design keeps the user interface clean while still surfacing past failures and corrections to the model.

### Correction (manual)

Add corrections to teach the system what to do instead:

```bash
# Record that 'npm install' should be 'bun install'
terraphim-agent learn correction \
  --original "npm install" \
  --corrected "bun install" \
  --correction-type "tool-preference"

# Record that a specific command has a known fix
terraphim-agent learn correction \
  --original "cargo test --all" \
  --corrected "cargo test --workspace" \
  --correction-type "tool-preference"
```

### Query (manual)

Search past learnings:

```bash
# Search by keyword
terraphim-agent learn query "docker"

# Search exactly
terraphim-agent learn query "npm install" --exact

# List recent learnings
terraphim-agent learn list --recent 10
```

### Safety guard (automatic)

The `tool.execute.before` hook runs `terraphim-agent guard --json` on every bash command before it executes. If the command matches a destructive pattern (`git reset --hard`, `rm -rf`, `docker rm -f`, etc.) the guard returns `{"decision":"block","reason":"..."}` and the plugin throws, preventing opencode from executing it.

To bypass a legitimate block, run the command directly in your terminal outside opencode.

### Command rewriting (automatic)

The `tool.execute.before` hook also runs `terraphim-agent replace` to apply knowledge-graph-driven rewrites:

- Git commit messages are rewritten using the role's thesaurus.
- Package manager commands (`npm`, `yarn`, `pnpm`, `pip`, `pip3`, `pipx`) are rewritten if the knowledge graph has a mapping (e.g. `npm install` to `bun add`).

Rewrite mode is controlled by `TERRAPHIM_REWRITE_MODE`:
- `suggest` (default): logs the rewrite to `~/Library/Application Support/terraphim/rewrites.log`; only git commits are applied.
- `apply`: all rewrites are applied automatically.

Rewrite role is set by `TERRAPHIM_REWRITE_ROLE` (default: `"Terraphim Engineer"`).

## Key implementation details

### Why Bun.spawnSync, not Bun `$` shell

The plugin MUST use `Bun.spawnSync()` for all terraphim-agent calls, NOT Bun's `$` shell template. The `$` shell corrupts bash tool execution in opencode, causing every bash command to fail with "expected a command or assignment but got: Redirect".

### Case sensitivity

opencode passes `input.tool` as `"bash"` (lowercase). The plugin uses `input.tool?.toLowerCase() !== "bash"` for matching.

### Exit code extraction

opencode's `tool.execute.after` hook provides `output.output` as a raw string (no structured exit code). The plugin extracts exit codes by:
1. Checking `output.metadata?.exitCode` or `output.metadata?.exit_code`
2. Matching `exit code: N` patterns in output text
3. Falling back to heuristic (error indicators like `command not found`, `Error:`, `FAILED` in output)

### Disabled plugins

Plugin files ending in `.disabled` (e.g. `terraphim-hooks.js.disabled`, `learning-capture.js.disabled`) are not loaded by opencode. Rename to `.js` to re-enable.

## Plugin file structure

```
~/.config/opencode/
  plugin/
    terraphim-hooks.js           # Main plugin: safety + learning + KG replacement
    terraphim-hooks.js.disabled  # Previous version (disabled)
    subagent-start.js            # Injects .docs/summary.md into subagent context
  plugins/
    advisory-guard.js            # Advisory-level safety guard (non-blocking warnings)
    fff.js                       # Fast file finder
    safety-guard.js              # Additional safety guard
    learning-capture.js.disabled # Earlier standalone learning capture (superseded)
```

## Troubleshooting

### Learnings not being captured

1. Check terraphim-agent is installed:
   ```bash
   ~/.cargo/bin/terraphim-agent learn list --recent 5
   ```

2. Check the plugin loads without errors:
   ```bash
   bun -e "const m = require('${HOME}/.config/opencode/plugin/terraphim-hooks.js'); console.log(Object.keys(m))"
   ```
   Expected: `[ "TerraphimHooks", "default" ]`

3. Verify with a diagnostic plugin:
   ```bash
   cat > /tmp/test-plugin.js << 'EOF'
   export const Test = async () => ({
     "tool.execute.after": async (input, output) => {
       Bun.write("/tmp/hook-fired.txt", `${new Date().toISOString()} tool=${input.tool}\n`).catch(() => {})
     }
   })
   export default Test
   EOF
   cp /tmp/test-plugin.js ~/.config/opencode/plugins/test-learn.js
   # Run opencode, trigger a bash command, then check:
   cat /tmp/hook-fired.txt
   # Clean up when done:
   rm ~/.config/opencode/plugins/test-learn.js
   ```

### All bash commands fail with "Redirect" error

This means a plugin is using Bun's `$` shell template, which corrupts bash tool execution. Ensure all terraphim-agent calls use `Bun.spawnSync()` instead.

## See also

- [Learning Capture for Claude Code](./learning-capture-claude-code.md) — same learning store, different hook registration (shell scripts vs Bun plugins)
- [Command Rewriting How-To](../command-rewriting-howto.md) — deeper walkthrough of the knowledge-graph-driven `replace` pipeline
