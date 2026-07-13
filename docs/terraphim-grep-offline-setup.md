# Enabling terraphim-grep Offline (lessons learned)

**Date:** 2026-06-29
**Scope:** Getting `terraphim-grep` to return real results offline, with a project-specific knowledge graph, no server required.

This is an operational lessons-learned note, not a design doc. It captures the
exact failure mode that makes a freshly-installed `terraphim-grep` return zero
results for every query, and the minimal setup to get KG-driven search working
against any project.

---

## TL;DR — the one thing that bites everyone

A stock `cargo install terraphim_grep` binary **does not search code**.

`terraphim_grep`'s `Cargo.toml` gates the actual code scanner behind an opt-in
feature:

```toml
[features]
code-search = ["dep:fff-search"]   # fff-search = the code scanner
default     = ["llm"]              # <-- code-search is NOT default
```

The default install compiles `search_code()` down to a stub that returns
`Ok(vec![])`:

```rust
// terraphim_grep/src/hybrid_searcher.rs
async fn search_code(..) {
    #[cfg(feature = "code-search")] { /* real fff-search */ }
    #[cfg(not(feature = "code-search"))] { let _ = (..); Ok(vec![]) }  // <- you are here
}
```

So **every query returns `chunks_returned: 0` in `search_latency_ms: 0`**, with
no error. Rebuild with the feature enabled:

```bash
cargo install terraphim_grep --version 1.20.5 --features code-search --force
```

That is the fix. Everything below is diagnosis and project setup.

---

## Symptom → diagnosis → fix

| Symptom | Meaning | Fix |
|--------|---------|-----|
| `chunks_returned: 0`, `search_latency_ms: 0`, no error | fff-search is compiled out (no `code-search` feature) | rebuild with `--features code-search` |
| `chunks_returned > 0` but `kg_hits: 0` | fff-search works; thesaurus not loaded / not matching | pass `--thesaurus`; check synonym terms |
| `Graph has no nodes yet - no documents have been indexed` | rolegraph has no document nodes | **ignore for grep** — the thesaurus-only fallback still boosts; only matters for `terraphim-agent` |
| `No thesaurus specified and could not find default` | `--thesaurus` missing and no auto-discovered file | pass `--thesaurus`, or place `.terraphim/thesaurus-<role>.json` |
| `Failed to compute source hash for ...kg` | broken symlinks in the global KG dir poison the build | `find ~/.config/terraphim/kg -xtype l -delete` |

**The single reliable signal** that fff-search never ran is
`search_latency_ms: 0` combined with `chunks_returned: 0` for a query that
*must* exist in the codebase. Real scans take single-digit-to-hundreds of ms.

---

## Architecture: why it works offline (and what fooled me)

`terraphim-grep` is **offline-first**. The `terraphim-agent` help calls itself
"server-backed", which mislead me into thinking grep also needed the server. It
does not.

```
terraphim-grep "query" --paths . --thesaurus t.json
        │
   HybridSearcher::search()        (parallel tokio tasks)
        ├── search_code()   → fff-search over --paths   → code_results  (THE chunks)
        └── search_kg()     → rolegraph query
                              └─ if graph empty → thesaurus-only fallback
        │
   boost_chunks_with_kg(code_results, kg_concepts)   ← KG only RE-RANKS
        │
   results
```

Two consequences worth internalising:

1. **Chunks come from fff-search, not from the KG.** The thesaurus/rolegraph
   only *boosts* (re-ranks) chunks. So an empty rolegraph is **not** fatal —
   there is an explicit thesaurus-only fallback in `search_kg` so boosting still
   fires when no documents are indexed.

2. **The rolegraph rabbit hole is a trap.** I burned real time believing the
   `Graph has no nodes` message (and the missing `terraphim_server`) was the
   blocker. It was a red herring. The real blocker was the feature flag. When
   grep returns zero, look at the *code-search feature* first, the graph last.

Source of truth (in the published crate, ~`~/.cargo/registry/src/.../terraphim_grep-<ver>/`):
`src/hybrid_searcher.rs` (`search`, `search_code`, `search_kg`) and `Cargo.toml`.

---

## Project setup: a self-contained `.terraphim/`

Drop this into any project root. No global config edits required.

```
.terraphim/
├── kg/                # KG concepts as markdown
│   ├── provider.md
│   ├── session.md
│   └── ...
├── thesaurus.json     # compiled synonym -> {id, nterm} map
└── config.json        # (optional) Role for --role-config
```

### KG markdown format

One concept per file. The `synonyms::` line is what the thesaurus is built from:

```markdown
# Provider

Abstract LLM backend implementing the `Provider` trait (`src/provider.rs`).
Description, key files, related concepts.

synonyms:: provider, llm backend, anthropic, openai, gemini, cohere, azure, bedrock, vertex, copilot, kimi
```

### Thesaurus format (must match exactly)

```json
{
  "name": "My Project Engineer",
  "data": {
    "provider":        { "id": 100, "nterm": "provider" },
    "llm backend":     { "id": 100, "nterm": "provider" },
    "anthropic":       { "id": 100, "nterm": "provider" }
  }
}
```

Each synonym maps to `{id, nterm}` where `nterm` is the normalised concept.
All synonyms of one concept share its `id`/`nterm`. Generate it from the
markdown by parsing the `# Title` and the `synonyms::` line — a ~15-line
Python script is enough (lowercase keys, dedupe, sequential ids).

### Generate from KG markdown

```bash
python3 - <<'PY'
import os, json, glob
data, cid = {}, 100
for f in sorted(glob.glob("kg/*.md")):
    nterm = os.path.splitext(os.path.basename(f))[0]
    txt = open(f, encoding="utf-8").read()
    syn = next((l.split("::",1)[1] for l in txt.splitlines()
                if l.strip().lower().startswith("synonyms::")), "")
    for t in dict.fromkeys([nterm] + [s.strip().lower() for s in syn.split(",") if s.strip()]):
        data.setdefault(t.lower(), {"id": cid, "nterm": nterm})
    cid += 1
json.dump({"name": "My Project Engineer", "data": data}, open("thesaurus.json","w"), indent=2)
PY
```

---

## Run it

```bash
cd /path/to/project
terraphim-grep "session persistence" \
  --paths . --thesaurus .terraphim/thesaurus.json -n 8 -C 2

# structured:
terraphim-grep "provider" --paths . --thesaurus .terraphim/thesaurus.json --json
```

A healthy result:

```json
{ "stats": { "search_latency_ms": 306, "chunks_returned": 5, "kg_hits": 3 },
  "concepts": [ {"name":"provider"}, {"name":"sse"} ], "sufficiency": "SearchOnly" }
```

KG-boosted chunks score above the fff baseline (1.0); a chunk whose path/content
matches a thesaurus concept rises to ~3.0, so your project vocabulary directly
shapes ranking.

---

## OpenRouter / RLM fallback configuration

By default `terraphim-grep` runs in search-only mode. To let the sufficiency
judge fall back to an LLM when local retrievers do not have enough signal, you
need a wired LLM client. The easiest way is OpenRouter.

### 1. Build with the OpenRouter feature

The default `cargo install terraphim_grep` does not include the OpenRouter
adapter. Rebuild with the feature enabled:

```bash
cargo install terraphim_grep --features "code-search openrouter" --force
```

If you are building from the workspace:

```bash
cargo build -p terraphim_grep --features "code-search openrouter" --release
```

### 2. Provide credentials via environment variables

`terraphim-grep` builds a minimal `Role` from these env vars when no
`--role-config` is passed:

```bash
export OPENROUTER_API_KEY=<your-openrouter-key>
export OPENROUTER_MODEL="qwen/qwen3-coder:free"   # optional; default is qwen/qwen3-coder:free
```

Then run with `--answer` (LLM answer when needed) or `--force-rlm` (always
synthesise):

```bash
terraphim-grep "retry policy" --paths . \
  --thesaurus .terraphim/thesaurus.json --answer --json
```

### 3. Provide credentials via role config

For a persistent, project-level setup, drop a role JSON file into
`.terraphim/`:

```bash
.terraphim/role-rust-engineer.json
```

The file must deserialise into a `terraphim_config::Role`. The fields that
govern OpenRouter are:

```json
{
  "name": "rust-engineer",
  "llm_enabled": true,
  "llm_api_key": "sk-or-v1-...",
  "llm_model": "qwen/qwen3-coder:free",
  "extra": {
    "llm_provider": "openrouter"
  }
}
```

`llm_enabled` must be `true`, and both `llm_api_key` and `llm_model` must be
set. Adding `"llm_provider": "openrouter"` in `extra` forces the
`build_llm_from_role` path to try OpenRouter first; without it, OpenRouter is
still tried as a fallback when `llm_enabled` + key/model are present.

Auto-discovery works as follows:

- `--role-config <path>` uses the exact file you pass.
- Otherwise, if `.terraphim/role-<role>.json` exists, grep loads it
  automatically (the role name is resolved from `--role` or the project
  config, defaulting to `default`).
- Otherwise, grep falls back to env vars.

### 4. Rate limits and model choice

OpenRouter's free tier is capped at 20 requests/min and 200 requests/day
(1000/day after a $10 deposit). If you see:

```
RLM execution failed: Config error: Rate limit exceeded
```

...the fallback is working but the key/model has hit the limit. Options:

- Switch to a free model with a higher quota, e.g.
  `liquid/lfm-2.5-1.2b-instruct:free`.
- Deposit credits to raise the daily cap.
- Use a local Ollama instance instead (`OLLAMA_BASE_URL=http://localhost:11434`).

If you have a small paid credit balance, the best "fast + cheap" combination
validated with `terraphim-grep --answer` is:

| Model | Price (prompt) | RLM latency | Quality |
|---|---|---|---|
| `amazon/nova-micro-v1` | ~$0.035 / M tokens | ~2 s | Good, real synthesis |
| `mistralai/mistral-nemo` | ~$0.020 / M tokens | ~18 s | Good, but slower |
| `meta-llama/llama-3.2-1b-instruct` | ~$0.027 / M tokens | ~1 s | Returns placeholder text for this task |

Free models are cheapest but rate-limited; the small paid models above avoid
the free-tier queue.

When `terraphim_rlm` auto-configures OpenRouter directly, it also honours
`RLM_OPENROUTER_MODEL` as a model override.

---

## Gotchas checklist

- [ ] Rebuilt with `--features code-search`? (Default install is useless for search.)
- [ ] `--thesaurus` passed and parsed? (`Loaded thesaurus with N entries` in debug log.)
- [ ] `search_latency_ms > 0`? (Zero = fff-search never ran.)
- [ ] Global KG broken symlinks cleaned? (`find ~/.config/terraphim/kg -xtype l -delete`.)
- [ ] Query terms lowercased in the thesaurus? (Aho-Corasick matching is case-sensitive on the keys.)
- [ ] OpenRouter wired for RLM fallback? (`OPENROUTER_API_KEY` set or `.terraphim/role-<name>.json` has `llm_enabled` + `llm_api_key` + `llm_model`; run with `--answer` or `--force-rlm`.)

---

## Enriching the KG with ast-grep (code anchors)

Prose synonyms only get you so far — a query for `ReadTool` or `AnthropicProvider`
won't boost the right chunks unless those exact code identifiers are in the
thesaurus. Use **ast-grep** to mine real identifiers and add them as anchors.

### Why it helps

KG-boosting (`boost_chunks_with_kg`) raises a chunk's score when its path or
content contains a thesaurus key. Adding real code tokens (`ReadTool`,
`ModelRegistry`, `AuthStorage`, `SseParser`, `AnthropicProvider`) means a query
for that token matches the concept **and** ranks the actual definition file
first. Observed: `ReadTool` → concept `tool`, `tools.rs` boosted to 3.0 vs 1.0
baseline; `AnthropicProvider` → concept `provider`, `providers/anthropic.rs` to
4.0 (three concept hits).

### Extract with ast-grep

```bash
ast-grep run -l Rust -p 'pub struct $NAME { $$$BODY }' src --json=compact > structs.json
ast-grep run -l Rust -p 'pub enum $NAME { $$$BODY }'   src --json=compact > enums.json
ast-grep run -l Rust -p 'pub trait $NAME { $$$BODY }'  src --json=compact > traits.json
ast-grep run -l Rust -p 'impl $T for $NAME { $$$BODY }' src --json=compact > impls.json
```

Captured names live at `match["metaVariables"]["single"]["NAME"]["text"]` (the
top-level JSON is a **list** of matches; `metaVariables.single`, not a bare
`NAME`).

### Bucket by concept — selectively

Map each identifier to a concept with **name-based** rules. Do **not** use
file-wide catch-alls like `f.startswith("src/providers/")` — they flood the
thesaurus with peripheral request/response types and metrics noise.

```python
rules = {
    "provider":        lambda n,f: n.endswith("Provider"),
    "session":         lambda n,f: "Session" in n,
    "extension":       lambda n,f: "Hostcall" in n or n.startswith("Extension") or "Capability" in n,
    "model-registry":  lambda n,f: "Model" in n,
    "sse":             lambda n,f: "Sse" in n or n == "StreamEvent",
    "acp":             lambda n,f: "Acp" in n,
    "auth":            lambda n,f: "Auth" in n or "OAuth" in n,
    "interactive-tui": lambda n,f: ("Picker" in n or "Selector" in n or n == "PiApp") and "src/interactive" in f,
    "tool":            lambda n,f: n.endswith("Tool") or n == "ToolRegistry",
    "hashline-edit":   lambda n,f: n == "HashlineEditTool",
}
```

Append each bucket to the matching `kg/<concept>.md` `synonyms::` line
(idempotently — only add lowercased terms not already present), then regenerate
`thesaurus.json`. A working, idempotent version of this whole loop lives at
`pi_agent_rust`'s `.terraphim/scripts/refresh-anchors.sh` — run it whenever the
code changes to keep anchors aligned.

### Verify

```bash
terraphim-grep 'ReadTool' --paths src --thesaurus .terraphim/thesaurus.json -n 3 --json
# expect: concepts include 'tool', top chunk = tools.rs at score > 1.0
```

---

## Lessons, bluntly

1. **A feature-gated no-op stub is indistinguishable from "working but empty"** unless you read the source. `Ok(vec![])` behind `cfg(not(feature))` returns success with zero items — no error, no warning. Always confirm a search tool actually *scans* (non-zero latency) before debugging results.
2. **"Server-backed" in a sibling tool's tagline does not mean every CLI in the family needs the server.** terraphim-grep is offline-capable; the rolegraph-empty messages are advisory, not blocking.
3. **Match the diagnostic to the layer.** Zero chunks → code scanner. Zero boost → thesaurus. Both are independent; fix them independently.
4. **Keep the project KG in the repo** (`.terraphim/`), version-controlled. It is documentation that doubles as a search index.
5. **The best thesaurus terms are the code's own identifiers.** Prose synonyms match intent; ast-grep-mined struct/trait/impl names match the actual text in chunks. Enrich the KG structurally and keep it fresh with a repeatable script — a hand-maintained thesaurus rots.
6. **Selectivity beats coverage.** File-wide bucketing rules (`f.startswith("src/providers/")`) drown the thesaurus in noise; name-based rules (`n.endswith("Provider")`) keep it signal-rich.

---

## Companion documents

- **Skill:** `terraphim-skills/skills/terraphim-grep-offline/SKILL.md` -- agent-facing skill covering offline search, thesaurus setup, ast-grep enrichment, and the OpenRouter `--answer` LLM fallback (model selection, env vars, 1Password key loading).
- **Usage guide:** `terraphim-skills/docs/terraphim-grep-usage.md` -- end-user quick reference with the OpenRouter model cost table.
- **OpenRouter fallback:** pass `--answer` (optionally `--force-rlm`) and export `OPENROUTER_API_KEY` / `OPENROUTER_MODEL`; see the skill's "OpenRouter model setup" section.
