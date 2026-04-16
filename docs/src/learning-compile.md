# Learning Compile

`learn compile` closes the feedback loop between user corrections and live command rewriting.
It scans the learnings directory for captured corrections, converts `ToolPreference` entries
into thesaurus mappings, and writes a JSON file that the `replace` command loads immediately.

Source: `crates/terraphim_agent/src/learnings/compile.rs`

## The feedback loop

```
user says "use bun instead of npm"
        |
        v
correction-*.md saved by post-tool-use hook
        |
        v
terraphim-agent learn compile --output compiled-corrections.json
        |
        v
replace command loads compiled-corrections.json
        |
        v
"npm install" -> "bun install" in future commands
```

## CLI usage

Compile to a standalone file:

```bash
terraphim-agent learn compile --output compiled-corrections.json
```

Merge with an existing curated thesaurus (compiled entries override curated on conflict):

```bash
terraphim-agent learn compile \
  --output merged.json \
  --merge-with docs/src/kg/bun.json
```

Output:

```
Compiled 3 correction(s), merged with 12 curated entries -> 14 total entries.
```

## What gets compiled

Only `ToolPreference` corrections are compiled into the thesaurus. Other correction types
(`Naming`, `CodePattern`, `WorkflowStep`, `FactCorrection`) are parsed but silently skipped.

A `ToolPreference` correction contains:
- `original` — the pattern to match (becomes the thesaurus key)
- `corrected` — the replacement term (becomes the nterm value)

Example: a correction captured when the user typed `use bun instead of npm install` produces:

```json
{
  "npm install": { "id": 1, "nterm": "bun install", "url": null }
}
```

## Merge precedence

When `--merge-with` is supplied, compiled corrections take precedence over curated entries
that share the same key. This means user preferences learned from actual failures override
the defaults shipped with the knowledge graph.

## Output format

The output file is a standard thesaurus JSON that `terraphim_automata::load_thesaurus()`
accepts:

```json
{
  "name": "compiled_corrections",
  "data": {
    "npm install": { "id": 1, "nterm": "bun install", "url": null },
    "yarn add":    { "id": 2, "nterm": "bun add",     "url": null }
  }
}
```

Pass this file to `terraphim-agent replace` via the `--thesaurus` flag to activate the
compiled corrections.
