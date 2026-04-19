# Plug Terraphim Search into Claude Code and opencode

Two integration paths. CLI is the recommended starting point: zero new binaries, faster cold start, works in every host that can shell out. MCP earns its keep when you also want autocomplete-as-you-type or `update_config_tool` exposed to the model. Both paths use the **same** `~/.config/terraphim/embedded_config.json`, so you get all six roles (Terraphim Engineer, Personal Assistant, System Operator, Context Engineering Author, Rust Engineer, Default) in either case.

## Path A -- CLI via slash command (recommended)

`terraphim-agent search` already exists, takes `--role` and `--limit`, and returns ranked results to stdout. A two-line slash command in either host wraps it.

### Claude Code

Create `~/.claude/commands/tsearch.md`:

```markdown
---
description: Terraphim search across configured roles. Usage: /tsearch [role] <query>
allowed-tools: Bash(terraphim-agent search:*), Bash(terraphim-agent-pa search:*)
---
Run `terraphim-agent search --role "<role>" --limit 5 "<query>"` (or
`terraphim-agent-pa search ...` if the role is "Personal Assistant" and
the query needs the JMAP haystack). Return the top results as a numbered
list with title, source path/URL, and a 120-char snippet.
```

That is the entire integration. The `allowed-tools` line auto-approves the two CLI invocations so the model does not have to ask permission for each call.

### opencode

Same file, drop it at `~/.config/opencode/command/tsearch.md`. opencode reads the same frontmatter shape; no extra setup.

### Why this is fast enough

`terraphim-agent` reads the persisted role state at start (low ms), runs the query against the role's haystacks (Aho-Corasick on local KG plus ripgrep over the haystack folders), and returns. For a typical knowledge-graph query against the Terraphim Engineer role on a laptop, the round trip is well under a second from slash command to formatted output. The agent already has the typed CLI (`--role`, `--limit`, `--format json`) -- no need to layer MCP on top for the common case.

## Path B -- MCP server (when you want typed tools)

If you want the model to call `search` as a first-class tool with structured JSON output -- alongside `autocomplete_terms`, `autocomplete_with_snippets`, `fuzzy_autocomplete_search`, `build_autocomplete_index`, and `update_config_tool` -- register `terraphim_mcp_server` instead. It reads the same config so the role list is identical.

### Build the binary

```bash
cd ~/projects/terraphim/terraphim-ai
cargo build --release -p terraphim_mcp_server --features jmap
cp target/release/terraphim_mcp_server ~/.cargo/bin/terraphim_mcp_server
```

For the Personal Assistant role (JMAP needs `JMAP_ACCESS_TOKEN` from 1Password), wrap the binary so the secret never lands on disk. Mirror the existing `terraphim-agent-pa` pattern at `~/bin/terraphim_mcp_server-pa`:

```bash
#!/usr/bin/env bash
exec op run --account my.1password.com \
  --env-file=<(echo 'JMAP_ACCESS_TOKEN=op://VAULT/ITEM/credential') \
  -- /Users/alex/.cargo/bin/terraphim_mcp_server "$@"
```

### Register in opencode

Add two entries under `mcp` in `~/.config/opencode/opencode.json`:

```jsonc
"terraphim":    { "type": "local", "command": ["/Users/alex/.cargo/bin/terraphim_mcp_server"] },
"terraphim-pa": { "type": "local", "command": ["/Users/alex/bin/terraphim_mcp_server-pa"] }
```

Restart opencode. Tools appear as `mcp__terraphim__search`, `mcp__terraphim_pa__search`, etc.

### Register in Claude Code

```bash
claude mcp add terraphim    /Users/alex/.cargo/bin/terraphim_mcp_server
claude mcp add terraphim-pa /Users/alex/bin/terraphim_mcp_server-pa
claude mcp list
```

The list output should show both servers as Connected.

## SessionStart primer (both paths)

Slash commands are useless if the model does not know the roles exist. Extend the SessionStart hook in `~/.claude/settings.json` (and the equivalent in opencode) to print a one-screen role index:

```bash
printf '\n--- Terraphim search via /tsearch [role] <query> ---\n'
printf '  Terraphim Engineer  (Rust/agent KG)\n'
printf '  Personal Assistant  (Obsidian + Fastmail JMAP, use terraphim-agent-pa for email)\n'
printf '  System Operator     (INCOSE/MBSE Logseq KG)\n'
printf '  Context Engineering Author, Rust Engineer, Default\n'
```

## Verify

CLI path:

```bash
terraphim-agent search --role "Terraphim Engineer" --limit 3 "rolegraph"
terraphim-agent search --role "System Operator"   --limit 3 "RFP"
terraphim-agent-pa search --role "Personal Assistant" --limit 3 "invoice"
```

MCP path:

```bash
claude mcp list | grep terraphim
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}' \
  | ~/.cargo/bin/terraphim_mcp_server
```

Inside a fresh Claude Code or opencode session, ask the model to list available tools; both `mcp__terraphim__search` and `mcp__terraphim_pa__search` should appear if the MCP path is configured.

## Auto-routing (CLI and MCP)

Both paths now auto-route the search when no role is specified. The agent scores every configured role's knowledge graph against the query and picks the highest-rank match.

CLI: skip `--role` and the picked role is printed once on stderr:

```
[auto-route] picked role "System Operator" (score=128, candidates=4); to override, pass --role
```

stdout (including `--robot` and `--format json` payloads) is unchanged.

MCP: omit the `role` parameter on the `terraphim_search` tool. The CallToolResult prepends one text content of the form `[auto-route] picked role "<name>" (score=<n>, candidates=<m>); pass role parameter to override` so MCP clients can surface the routing decision; the resource contents that follow are unchanged in count and order.

Pass an explicit role to short-circuit auto-routing -- the routing line is suppressed entirely.

## Three example queries (one per role)

- **Terraphim Engineer**: `/tsearch "Terraphim Engineer" rolegraph` -- returns hits from `~/.config/terraphim/docs/src/`.
- **System Operator**: `/tsearch "System Operator" RFP` -- KG normalises `RFP` to `acquisition need` (INCOSE canonical term) and returns `Acquisition need.md` near the top with a high rank score.
- **Personal Assistant**: `/tsearch "Personal Assistant" invoice` -- mixes Obsidian notes with `jmap:///email/<id>` hits from your Fastmail mailbox. The wrapper script injects `JMAP_ACCESS_TOKEN` once per call.

## When to pick which path

| | CLI (Path A) | MCP (Path B) |
| --- | --- | --- |
| New binaries needed | None | `terraphim_mcp_server` + wrapper |
| Cold start | ~50-200 ms per call | ~10-50 ms per call (long-lived process) |
| Tools exposed | `search` only | `search` + 4 autocomplete + `build_autocomplete_index` + `update_config_tool` |
| Works in any host | Yes -- anything that can run a slash command | Only hosts that speak MCP |
| Token handling | Wrapper script (`terraphim-agent-pa`) | Wrapper script (`terraphim_mcp_server-pa`) |

For the search-across-roles flow, CLI is enough. Add MCP when the model needs autocomplete-as-you-type or you want it to manage role configuration without leaving the conversation.

## Troubleshooting

- **`terraphim-agent: command not found`** in slash command output -- the host is shelling out without your shell environment. Either install via `cargo install terraphim_agent` to a globally-visible path, or absolute-path the binary in the slash command (`/Users/alex/.cargo/bin/terraphim-agent ...`).
- **`mcp__terraphim_pa__search` returns zero email hits** -- the `op` CLI needs an active session (`op signin`); biometric prompts may not surface from non-interactive MCP health checks. Run `op signin` once per terminal session before launching the host.
- **No matches from any role** -- run `terraphim-agent config reload` to rebuild the persisted role index from `embedded_config.json`. Most empty-result confusion is a stale persisted snapshot.
- **Personal Assistant only shows notes, never email** -- you are calling `terraphim-agent` instead of `terraphim-agent-pa`, so `JMAP_ACCESS_TOKEN` is unset. The bare CLI is fine for the other roles.

## Related

- [Personal Assistant role](./personal-assistant-role.md) -- the JMAP + Obsidian role this integration exposes
- [System Operator README](../../terraphim_server/README_SYSTEM_OPERATOR.md) -- the Logseq MBSE KG
- [Command Rewriting How-To](../command-rewriting-howto.md) -- the hooks-based knowledge-graph integration that runs alongside this search integration
