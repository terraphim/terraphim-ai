# Listener Shell Dispatch

The listener can execute `terraphim-agent` subcommands triggered by `@adf:<agent-name>` mentions
on Gitea issues and comments. The result is posted back as a markdown comment with the command
output in a code block.

Source: `crates/terraphim_agent/src/shell_dispatch.rs`, `crates/terraphim_agent/src/listener.rs`

## How it works

1. An issue comment contains `@adf:worker search automata --role engineer`.
2. The listener extracts the context after the `@adf:` mention.
3. The context is parsed into `(subcommand, args)` and validated through three security layers.
4. The agent binary runs `terraphim-agent search automata --role engineer --robot`.
5. The output is formatted as a markdown comment and posted to the Gitea issue.

## Security layers

All three checks must pass before the process is spawned:

| Layer | Mechanism | Example rejection |
|-------|-----------|-------------------|
| Shell metachar rejection | Scans raw input for `\|`, `;`, `&`, `` ` ``, `$`, `(`, `)`, `<`, `>` | `search \| cat` |
| Subcommand allowlist | Must be in built-in list or `extra_allowed_subcommands` | `foobar` |
| Subcommand denylist | Explicitly blocked regardless of allowlist | `listen`, `repl`, `update` |
| CommandGuard | Pattern-based guard blocks destructive operations | `guard git reset --hard` |

### Allowed subcommands

`search`, `extract`, `replace`, `validate`, `suggest`, `graph`, `roles`, `config`, `learn`,
`chat`, `check-update`, `guard`, `hook`, `evaluate`

### Denied subcommands

`listen`, `repl`, `interactive`, `setup`, `update`, `sessions`

The deny list prevents the listener from spawning another listener, entering interactive mode,
or running self-update operations.

## Configuration

Add a `dispatch` block to your `ListenerConfig` JSON:

```json
{
  "identity": { "agent_name": "worker" },
  "gitea": {
    "base_url": "https://git.terraphim.cloud",
    "owner": "terraphim",
    "repo": "terraphim-ai"
  },
  "dispatch": {
    "timeout_secs": 300,
    "max_output_bytes": 48000,
    "extra_allowed_subcommands": [],
    "specialist_routes": {
      "search": "search-specialist",
      "evaluate": "eval-bot"
    }
  }
}
```

`specialist_routes` routes specific subcommands to a named agent instead of executing locally.
If the map is empty, all allowed subcommands execute on the current agent.

## Output format

The markdown comment posted to Gitea looks like:

````markdown
## `search` -- exit 0 (0.4s)

```
[result JSON lines]
```

agent=worker session=ses:abc event=evt:def
````

Stderr is included in a collapsible `<details>` block. Output is capped at 48 KB. A
5-minute timeout kills the process and adds a `**TIMED OUT**` marker.

The `--robot` flag is always appended to every dispatched command, so output is
machine-readable JSON regardless of the user's default output mode.
