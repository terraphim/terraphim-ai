# Howto: Learning Capture for Claude Code

## What it does

Claude Code automatically captures failed bash commands and stores them as learnings. When you run the same failing command again, the pre-tool-use hook warns you. Both Claude Code and opencode share the same learning store.

## Architecture

```
Claude Code hooks (~/.claude/hooks/)
    |
    |--- PreToolUse  (pre_tool_use.sh)
    |       |---> terraphim-agent guard --json                   (safety)
    |       |---> terraphim-agent replace --role ... --json      (knowledge graph rewrites)
    |       |---> terraphim-agent hook --hook-type pre-tool-use  (generic pre handler)
    |
    |--- PostToolUse (post_tool_use.sh)
    |       |---> terraphim-agent learn hook --format claude     (capture failed commands)
    |       |---> terraphim-agent hook --hook-type post-tool-use (generic post handler)
    |
                                              |
                                              v
                                     Shared learning store
                                  (~/.config/terraphim/learnings/)
```

## Prerequisites

- `terraphim-agent` installed at `~/.cargo/bin/terraphim-agent`
- Claude Code v2.1+ installed (`npm install -g @anthropic-ai/claude-code`)

## Setup

### 1. Hook scripts (already installed)

The hook scripts live at:
- `~/.claude/hooks/pre_tool_use.sh` — runs before every Bash tool call
- `~/.claude/hooks/post_tool_use.sh` — runs after every Bash tool call

### 2. Hook registration (already configured)

Hooks can be registered in either `~/.claude/settings.json` (shared, version-controlled) or `~/.claude/settings.local.json` (personal, git-ignored). This setup uses `settings.local.json` so per-developer paths do not leak into the repo. On teams that want everyone to share the same hooks, move the block into `settings.json`.

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "~/.claude/hooks/pre_tool_use.sh"
          }
        ]
      }
    ],
    "PostToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "~/.claude/hooks/post_tool_use.sh"
          }
        ]
      }
    ]
  }
}
```

### 3. Verify it works

```bash
# Simulate what Claude Code sends when a command fails
echo '{"tool_name":"Bash","tool_input":{"command":"nonexistent-test-cmd"},"tool_result":{"exit_code":127,"stdout":"","stderr":"command not found"}}' \
  | ~/.claude/hooks/post_tool_use.sh

# Check the learning was captured
terraphim-agent learn query "nonexistent-test-cmd"
```

Expected output:
```
Learnings matching 'nonexistent-test-cmd'.
  [G] [cmd] nonexistent-test-cmd (exit: 127)
```

## How learning capture works

### Capture (automatic)

When Claude Code runs a Bash command that fails, Claude Code sends JSON to the PostToolUse hook via stdin:

```json
{
  "tool_name": "Bash",
  "tool_input": { "command": "the-failed-command" },
  "tool_result": { "exit_code": 1, "stdout": "...", "stderr": "error output" }
}
```

The `post_tool_use.sh` script:
1. Locates `terraphim-agent` binary (4 fallback paths)
2. Reads JSON from stdin
3. Pipes to `terraphim-agent learn hook --format claude`
4. Also runs `terraphim-agent hook --hook-type post-tool-use --json`
5. Always fails open (`|| true`) — never blocks Claude Code

### Pre-tool-use warnings (automatic)

Before each Bash command, `pre_tool_use.sh` runs `terraphim-agent hook --hook-type pre-tool-use --json` which checks the command against past learnings and the role-configured knowledge graph. If it matches a past failure or a recorded correction, Claude Code receives a warning (or a rewritten command, if the `replace` step returned one) via the hook's structured JSON response.

### Safety guard (automatic)

`pre_tool_use.sh` also runs `terraphim-agent guard` which blocks destructive commands like `git reset --hard`, `rm -rf`, etc. If blocked, the hook returns:

```json
{
  "hookSpecificOutput": {
    "hookEventName": "PreToolUse",
    "permissionDecision": "deny",
    "permissionDecisionReason": "BLOCKED: <reason>"
  }
}
```

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

# Capture a user correction inline (used in conversation)
terraphim-agent learn correction \
  --original "pip install" \
  --corrected "uv pip install" \
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

# Query and see corrections
terraphim-agent learn query "npm"
```

## Hook script details

### post_tool_use.sh

```
stdin -> JSON { tool_name, tool_input, tool_result }
         |
         v
    Find terraphim-agent binary
    (~/.cargo/bin/ -> target/release/ -> target/debug/ -> PATH)
         |
         v
    cd ~/.config/terraphim  (for KG access)
         |
         v
    terraphim-agent learn hook --format claude  (capture failed commands)
         |
         v
    terraphim-agent hook --hook-type post-tool-use --json  (generic handler)
         |
         v
    stdout passthrough (always succeeds, || true)
```

### pre_tool_use.sh

```
stdin -> JSON { tool_name, tool_input }
         |
         v
    Filter: only Bash commands
         |
         v
    Step 1: terraphim-agent guard --json  (block destructive commands)
         |   If blocked -> return deny JSON
         v
    Step 2: terraphim-agent replace --role "Terraphim Engineer" --json
         |   (knowledge graph text replacement for git commits, package managers)
         v
    Step 3: terraphim-agent hook --hook-type pre-tool-use --json
         (generic pre-tool handler)
```

## Key differences from opencode hooks

| Aspect | Claude Code | opencode |
|--------|-------------|----------|
| Hook type | Shell scripts (bash) | JavaScript plugins (Bun) |
| Registration | `settings.local.json` | Auto-loaded from `plugin/` dir |
| Input format | JSON on stdin | JS objects `(input, output)` |
| Tool name | `"Bash"` (capital B) | `"bash"` (lowercase) |
| Exit code | Structured `{ exit_code: N }` | Raw string (parse from output) |
| Blocking | Return `permissionDecision: "deny"` | `throw new Error()` |
| Fail-open | `\|\| true` | `catch {}` |

## Troubleshooting

### Learnings not being captured

1. Check terraphim-agent is installed:
   ```bash
   ~/.cargo/bin/terraphim-agent learn list --recent 5
   ```

2. Test the hook directly:
   ```bash
   echo '{"tool_name":"Bash","tool_input":{"command":"test-verify-hook"},"tool_result":{"exit_code":1,"stdout":"","stderr":"error"}}' \
     | ~/.claude/hooks/post_tool_use.sh
   terraphim-agent learn query "test-verify-hook"
   ```

3. Check hook is registered:
   ```bash
   cat ~/.claude/settings.local.json | grep -A5 "PostToolUse"
   ```

### Hook script not executable

```bash
chmod +x ~/.claude/hooks/pre_tool_use.sh ~/.claude/hooks/post_tool_use.sh
```

### Safety guard blocking legitimate commands

The safety guard uses `terraphim-agent guard` which checks for destructive patterns. If a legitimate command is blocked, the guard can be bypassed by running the command outside Claude Code (directly in terminal).

## See also

- [Learning Capture for opencode](./learning-capture-opencode.md) — same learning store, Bun plugin instead of shell scripts
- [Command Rewriting How-To](../command-rewriting-howto.md) — deeper walkthrough of the knowledge-graph-driven `replace` pipeline
