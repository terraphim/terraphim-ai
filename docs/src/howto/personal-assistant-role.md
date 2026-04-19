# Personal Assistant Role: Search Email and Notes Together

The Personal Assistant role indexes a Fastmail JMAP mailbox and an Obsidian vault under a single Terraphim role, so one query returns both notes and email ordered by knowledge-graph relevance. This how-to shows the end-to-end setup.

## Why this exists

Most "personal AI" tools split your context across silos: one search box for email, another for notes, a third for chat history. Terraphim treats every source as a haystack on the same role, so a single query crosses them. The Personal Assistant role wires up the two most common personal sources -- email (JMAP) and notes (Obsidian) -- with deterministic, sub-millisecond ranking and no cloud round-trip.

## Prerequisites

1. **A locally built `terraphim-agent` with the `jmap` feature.** The crates.io binary does not include `haystack_jmap` (the dependency is not yet published), so `cargo install terraphim_agent` will index the Obsidian vault but silently skip JMAP. See [Crates Overview](../crates-overview.md) for the workspace layout.

   ```bash
   cd ~/projects/terraphim/terraphim-ai
   cargo build --release -p terraphim_agent --features jmap
   cp target/release/terraphim-agent ~/.cargo/bin/terraphim-agent
   ```

   To enable the feature in your local checkout (the published crates have it commented out):

   ```toml
   # crates/terraphim_middleware/Cargo.toml
   haystack_jmap = { path = "../haystack_jmap", version = "1.0.0", optional = true }
   # in [features]
   jmap = ["dep:haystack_jmap"]

   # crates/terraphim_agent/Cargo.toml -- in [features]
   jmap = ["terraphim_middleware/jmap"]
   ```

2. **An Obsidian vault on the local filesystem** -- any path containing markdown files works. This guide assumes `~/synced/ObsidianVault`.

3. **A Fastmail JMAP access token.** Generate one at <https://app.fastmail.com/settings/security/tokens> with the "Mail" scope. Store it in 1Password (or any secret manager that exposes it via env at runtime) -- never paste it into the role config on disk.

## Step 1 -- Add the role to `embedded_config.json`

Back up first:

```bash
cp ~/.config/terraphim/embedded_config.json{,.bak-$(date +%Y-%m-%d)}
```

Add the role under `roles` in `~/.config/terraphim/embedded_config.json`:

```json
"Personal Assistant": {
  "shortname": "PA",
  "name": "Personal Assistant",
  "relevance_function": "terraphim-graph",
  "terraphim_it": false,
  "theme": "lumen",
  "kg": {
    "automata_path": null,
    "knowledge_graph_local": {
      "input_type": "markdown",
      "path": "/Users/alex/synced/ObsidianVault"
    },
    "public": false,
    "publish": false
  },
  "haystacks": [
    {
      "location": "/Users/alex/synced/ObsidianVault",
      "service": "Ripgrep",
      "read_only": true
    },
    {
      "location": "https://api.fastmail.com/jmap/session",
      "service": "Jmap",
      "read_only": true,
      "extra_parameters": {
        "limit": "50"
      }
    }
  ],
  "llm_enabled": false
}
```

Notes on the choices:

- The Obsidian haystack uses `Ripgrep` because the vault is just markdown -- no Obsidian-specific service needed. `read_only: true` ensures the agent never edits notes.
- The JMAP haystack `location` defaults to Fastmail's session URL; override it for other JMAP providers.
- `kg.knowledge_graph_local.path` points at the same vault, so the role's knowledge graph is built from your own notes -- this gives the Aho-Corasick matcher synonyms specific to your project vocabulary, which boosts ranking on both notes and email.
- `extra_parameters.limit` caps the JMAP search to 50 hits per query; tune as needed.
- The token is **not** in the JSON. JMAP haystack reads `JMAP_ACCESS_TOKEN` from environment first, then falls back to `extra_parameters.access_token`. We use the env path so the secret never lands on disk.

Reload the agent's persisted config from the JSON file:

```bash
terraphim-agent config reload
```

Verify the role appears:

```bash
terraphim-agent roles list
```

The output should include `Personal Assistant (PA)` alongside the existing roles.

## Step 2 -- Wrapper script for token injection

Because `JMAP_ACCESS_TOKEN` must be set in the agent's environment for every Personal Assistant query, the cleanest pattern is a small wrapper that uses your secret manager (here, `op run` from the 1Password CLI) to inject the token at exec time:

```bash
mkdir -p ~/bin
cat > ~/bin/terraphim-agent-pa <<'SH'
#!/usr/bin/env bash
exec op run --account my.1password.com \
  --env-file=<(echo 'JMAP_ACCESS_TOKEN=op://VAULT/ITEM/credential') \
  -- /Users/alex/.cargo/bin/terraphim-agent "$@"
SH
chmod +x ~/bin/terraphim-agent-pa
```

Replace `VAULT/ITEM` with the path to your Fastmail token in 1Password. After this, `terraphim-agent-pa` behaves exactly like `terraphim-agent` for the Personal Assistant role; the bare `terraphim-agent` continues to work for the other roles without paying the 1Password unlock cost.

The token only ever exists inside the running process. Verify nothing leaked to disk:

```bash
grep -r "JMAP_ACCESS_TOKEN\|fmu1-" ~/.config/terraphim/
```

The grep should return nothing.

## Step 3 -- Verify search

Notes-only query (no token needed; `terraphim-agent` is fine):

```bash
terraphim-agent search --role "Personal Assistant" --limit 3 "todo"
```

Each hit should have a path under your Obsidian vault.

Email query (use the wrapper):

```bash
terraphim-agent-pa search --role "Personal Assistant" --limit 3 "invoice"
```

Each hit should have a `jmap:///email/<id>` URL and the sender's address in the description.

Cross-source query -- a term that appears in both notes and email, e.g. a project name:

```bash
terraphim-agent-pa search --role "Personal Assistant" --limit 6 "<your project>"
```

You should see notes and emails interleaved, ordered by `terraphim-graph` rank.

## Auto-routing

When you call `terraphim-agent search "query"` without `--role`, the agent now scores every configured role's knowledge graph against the query and picks the highest-ranked match. The decision is printed once on stderr:

```
[auto-route] picked role "Personal Assistant" (score=42, candidates=4); to override, pass --role
```

stdout is untouched, so `--robot` and `--format json` output remain pure JSON. Pass `--role "Some Role"` to short-circuit auto-routing.

When `JMAP_ACCESS_TOKEN` is not set, the Personal Assistant's score is multiplied by 0.5 (it loses the JMAP half of its corpus). The role still competes -- a clearly PA-flavoured query like `invoice tax` still wins over local-only roles when only PA matches.

## Troubleshooting

- **No email hits, no error** -- the warning `JMAP haystack support not enabled. Skipping haystack:` in stderr means your binary lacks the `jmap` feature. Rebuild from local source per the Prerequisites.
- **No email hits, no warning** -- run the wrapper with `op run --no-masking` once and confirm `JMAP_ACCESS_TOKEN` is non-empty inside the subshell. If empty, the 1Password reference is wrong.
- **`401 Unauthorized` from Fastmail** -- the token has been revoked or scoped without "Mail" access. Regenerate at <https://app.fastmail.com/settings/security/tokens>.
- **Ranking feels off** -- the role's knowledge graph indexes the whole vault on first use; subsequent edits to notes need a `terraphim-agent config reload` (which rebuilds the role's KG within ~20 ms).
- **UTF-8 panic in CLI output** -- some snippets containing fancy quotes can trip a known truncation bug at `crates/terraphim_agent/src/main.rs:1414`. The search itself succeeds; only the trailing display crashes. Pipe through `head -n N` to bound the output until the upstream fix lands.

## Related

- [Command Rewriting How-To](../command-rewriting-howto.md) -- generic pattern for adding haystacks to any role.
- [Architecture](../Architecture.md) -- how roles, haystacks, and the knowledge graph compose.
- [Crates Overview](../crates-overview.md) -- where `haystack_jmap` and `terraphim_middleware` live.
