# How-To: Learning-Driven Command Rewriting

This guide shows how to use terraphim-agent to rewrite shell commands before
execution -- for example `npm install` -> `bun add` or `pip install` -> `uv
add` -- by plugging a knowledge-graph-backed thesaurus into your AI coding
agent's tool-execution hook.

The mechanism composes three pieces that already exist in terraphim-agent:

1. A Logseq-style knowledge graph of command synonyms under
   `~/.config/terraphim/docs/src/kg/` (or any role-configured path).
2. `terraphim-agent replace` -- Aho-Corasick replacement that rewrites text
   using a role's compiled thesaurus.
3. A plugin hook in your AI agent (OpenCode, Claude Code, etc.) that
   intercepts every Bash tool call, pipes the command through `replace`,
   and writes the result back into the tool's args.

## Prerequisites

- `terraphim-agent` on `PATH` (any recent release; 1.16.33 or later).
- A role whose KG directory you control. The default ships with a
  `Terraphim Engineer` role pointing at `~/.config/terraphim/docs/src/kg/`.
- An AI agent that exposes a `tool.execute.before` style plugin API
  (OpenCode has one; Claude Code exposes equivalent hooks via shell scripts).

## 1. Curate the knowledge graph

Each concept is one markdown file. The filename stem becomes the concept
key; the H1 heading provides the display name used as the replacement; the
`synonyms::` line lists terms that should be rewritten to it.

Example `~/.config/terraphim/docs/src/kg/bun install.md`:

```markdown
# bun add

Install dependencies using Bun package manager.

synonyms:: npm install, yarn install, pnpm install, npm i, yarn add, pnpm add
```

Conventions that matter in practice:

- **Filename uses spaces, not underscores** if the concept has multiple
  words. The matcher compares against the filename stem (`bun install`).
- **Multi-word synonyms are supported.** `python -m pip install` is a valid
  synonym and is matched as a whole phrase; the Aho-Corasick automaton uses
  LeftmostLongest, so the longer phrase wins when a shorter one would also
  match.
- **Do not overlap synonyms across files.** If both `uv.md` and `uv add.md`
  claim `pip install`, the behaviour becomes non-deterministic at rebuild
  time. Keep single-token synonyms in the short file (`pip` -> `uv`) and
  multi-token phrases in the specific file (`pip install` -> `uv add`).
- **Keep domain vocabulary separate from command vocabulary** if you need
  both. Create a dedicated role with its own KG path rather than bleeding
  domain terms into shell commands.

### Seed set shipped in this repo

The `Terraphim Engineer` role's KG now ships with these command files:

| File                                 | Maps to  | Covers                                   |
| ------------------------------------ | -------- | ---------------------------------------- |
| `bun.md`                             | bun      | npm, yarn, pnpm                          |
| `bun install.md`                     | bun add  | npm install, yarn install, pnpm install, npm i, yarn add, pnpm add |
| `bun run.md`                         | bun run  | npm run, yarn run, pnpm run              |
| `bunx.md`                            | bunx     | npx, pnpx, yarn dlx                      |
| `uv.md`                              | uv       | pip, pip3, pipx                          |
| `uv add.md`                          | uv add   | pip install, pip3 install, pip add, pipx install, python -m pip install |
| `uv sync.md`                         | uv sync  | pip install -r requirements.txt          |

## 2. Verify with the CLI

```bash
printf "npm install express" \
  | terraphim-agent replace --role "Terraphim Engineer" --fail-open --json
```

Expected output:

```json
{"result":"bun add express","original":"npm install express","replacements":1,"changed":true}
```

Flags worth knowing:

- `--fail-open` -- on any error, emits the input unchanged. Mandatory in
  hooks so a misconfigured terraphim-agent never wedges the agent.
- `--json` -- structured output with `result`, `changed`, `replacements`.
  Use this if the hook needs to branch on whether anything changed.
- `--format plain|markdown|wiki|html` -- how the replacement is wrapped.
  Hooks want `plain`.

## 3. Flush the cache after KG edits

Terraphim caches compiled thesauri in a SQLite database at
`/tmp/terraphim_sqlite/terraphim.db` (path configured by
`crates/terraphim_settings/default/settings.toml`). Editing a KG markdown
file does **not** invalidate this cache; `replace` keeps returning the old
mapping until you flush it.

```bash
sqlite3 /tmp/terraphim_sqlite/terraphim.db \
  "DELETE FROM terraphim_kv WHERE key LIKE 'thesaurus_%' OR key LIKE 'document_ripgrep_%';"
```

Because `/tmp/` is wiped on reboot, a fresh boot always gives the
up-to-date thesaurus.

## 4. Wire up the hook (OpenCode example)

OpenCode plugins expose `tool.execute.before(input, output)` where
`output.args.command` is the mutable shell command about to run. The same
pattern works in Claude Code via the `PreToolUse` hook script, just with
shell-stdin instead of a JS closure.

```js
// ~/.config/opencode/plugin/terraphim-hooks.js
const REWRITE_MODE = process.env.TERRAPHIM_REWRITE_MODE || "suggest"
const REWRITE_ROLE = process.env.TERRAPHIM_REWRITE_ROLE || "Terraphim Engineer"
const AUDIT_LOG    = `${process.env.HOME}/Library/Application Support/terraphim/rewrites.log`

// Narrow whitelist of commands whose argument grammar survives a synonym swap.
const REWRITEABLE_HEADS =
  /^\s*(npm|yarn|pnpm|npx|pnpx|pip|pip3|pipx|python\s+-m\s+pip|python3\s+-m\s+pip)\b/i

export const TerraphimHooks = async ({ $ }) => ({
  "tool.execute.before": async (input, output) => {
    if (input.tool !== "Bash" || !output.args?.command) return
    const command = output.args.command

    const agent = `${process.env.HOME}/.cargo/bin/terraphim-agent`

    // Always run the destructive-command guard first.
    const g = await $`${agent} guard ${command} --json --fail-open 2>/dev/null || echo '{"decision":"allow"}'`
    const guard = JSON.parse(g.stdout)
    if (guard.decision === "block") {
      throw new Error(`BLOCKED: ${guard.reason}`)
    }

    const isGitCommit   = /git\s+(-C\s+\S+\s+)?commit/i.test(command)
    const isRewriteable = REWRITEABLE_HEADS.test(command)
    if (!isGitCommit && !isRewriteable) return

    const res     = await $`echo ${command} | ${agent} replace --role ${REWRITE_ROLE} --fail-open --json 2>/dev/null`
    const parsed  = JSON.parse(res.stdout)
    const rewrite = (parsed.result || "").trim()
    if (!parsed.changed || !rewrite || rewrite === command) return

    const line = [
      new Date().toISOString(), REWRITE_MODE,
      isGitCommit ? "git-commit" : "pkg-mgr",
      command.replace(/[\t\n\r]/g, " "),
      rewrite.replace(/[\t\n\r]/g, " "),
    ].join("\t") + "\n"
    await $`mkdir -p "$(dirname ${AUDIT_LOG})" < /dev/null && printf %s ${line} >> ${AUDIT_LOG}`

    if (REWRITE_MODE === "apply" || isGitCommit) {
      output.args.command = rewrite
    }
  },
})
```

Design notes:

- **Whitelist, not blacklist.** Arbitrary shell is never rewritten. Only
  commands whose head matches `REWRITEABLE_HEADS` are candidates.
- **Suggest mode by default.** Set `TERRAPHIM_REWRITE_MODE=apply` once you
  trust the diffs. Git commit rewriting always applies because commit
  messages are prose, not syntax.
- **Audit log.** Every rewrite is logged tab-separated to
  `~/Library/Application Support/terraphim/rewrites.log` so you can diff
  before flipping modes.
- **Fail-open.** Each external call is wrapped in try/catch with `||`
  fallbacks. If terraphim-agent is missing, commands pass through unchanged.

## 5. Confirm end-to-end

With the hook installed and the cache flushed, open your agent, ask it to
run `npm install express`, and inspect the audit log:

```bash
tail -n 5 ~/Library/Application\ Support/terraphim/rewrites.log
```

You should see a line like:

```
2026-04-15T11:32:51.129Z    suggest    pkg-mgr    npm install express    bun add express
```

In `suggest` mode the command still executes as `npm install express`; in
`apply` mode the agent actually runs `bun add express`.

## 6. Phase 2: user-prompt-submit wiring

`terraphim-agent learn hook --learn-hook-type user-prompt-submit` scans
the user's prompt for tool-preference patterns and records a
`CorrectionType::ToolPreference` correction under
`~/.local/share/terraphim/learnings/correction-*.md` (or the configured
project fallback). Supported patterns:

| Pattern | Example | Captured |
|---------|---------|----------|
| use X instead of Y | "use uv instead of pip" | (pip -> uv) |
| use X not Y | "use cargo not make" | (make -> cargo) |
| prefer X over Y | "prefer bunx over npx" | (npx -> bunx) |

Patterns that start with a personal pronoun (e.g. "I prefer tea over
coffee") are intentionally ignored so that personal preferences do not
pollute the tool-preference corpus.

### 6.1 Claude Code shell hook

Copy `examples/claude-code-hooks/user-prompt-submit-hook.sh` into your
Claude Code hooks directory and reference it in `claude-settings.json`:

```json
{
  "userPromptSubmit": {
    "command": "/path/to/user-prompt-submit-hook.sh"
  }
}
```

The script pipes the prompt JSON into `terraphim-agent learn hook
--learn-hook-type user-prompt-submit` and always passes the original
input through unchanged (fail-open).

### 6.2 OpenCode plugin

Copy `examples/opencode-plugin/user-prompt-submit.js` into
`~/.config/opencode/plugin/` (or `plugins/`). The plugin normalises the
OpenCode user-prompt payload to `{"user_prompt":"..."}` and invokes the
same CLI entry point.

### 6.3 Verify capture

After running one of the hooks with a tool-preference prompt, check the
learnings directory:

```bash
ls ~/.local/share/terraphim/learnings/correction-*.md
```

Each file is a markdown document with front-matter containing the
original tool, the corrected tool, and the `tool-preference` type tag.
These files feed Phase 3 (`terraphim-agent learn compile`) which
aggregates them into a thesaurus JSON for the `replace` command.

## Troubleshooting

**`replace` returns the original unchanged.**
Run `terraphim-agent search "<synonym>" --role "<role>"` -- if the concept
appears, the KG is loaded but the synonym is not. Confirm the synonym is on
the `synonyms::` line (case-insensitive; commas separate entries). Flush
the cache (section 3) and retry.

**`Failed to load thesaurus: NotFound("thesaurus_...")` in stderr.**
Cosmetic. The agent looked for a pre-compiled JSON thesaurus first, didn't
find one, and fell back to building from markdown. Expected on first run.

**Hook does nothing in OpenCode.**
Check the plugin loaded: `grep terraphim-hooks
~/.local/share/opencode/log/$(ls -t ~/.local/share/opencode/log/ | head -1)`.
You should see a line like `service=plugin path=...terraphim-hooks.js
loading plugin`. If absent, the plugin file is in the wrong directory --
OpenCode autoloads from `~/.config/opencode/plugin/` and
`~/.config/opencode/plugins/`.

**Commands get double-rewritten on retry.**
The hook only touches `tool.execute.before`; the agent does not loop back
through the hook on its own retries. If you see double rewrites, check
whether `input.tool === "Bash"` is spelt exactly -- OpenCode passes
`"Bash"`, not `"bash"`.
