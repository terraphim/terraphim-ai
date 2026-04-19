# Progressive Context Loading for Claude Code

**Status:** Design
**Branch:** `claude/progressive-context-loading-nGNbR`
**Owners:** terraphim-agent, terraphim_hooks, terraphim_file_search

## Problem

A project-level `CLAUDE.md` plus the Claude Code skill catalog can easily exceed
10k tokens (the `CLAUDE.md` in this repo is ~500 lines, ~6–7k tokens; the
skills catalog adds thousands more). All of it is injected into every turn,
regardless of whether the user is renaming a variable or refactoring the
persistence layer. This wastes context budget and degrades attention on the
bytes that actually matter for the current task.

We already have the ingredients to do better:

- `terraphim_automata` — Aho-Corasick automata over the project KG thesaurus,
  built to extract concepts from arbitrary text at line rate.
- `terraphim_rolegraph` — concept graph with PageRank-like scoring.
- `terraphim_agent` learnings store — captures past prompts, failures, and
  corrections; already queryable by pattern and semantic concept.
- `terraphim_hooks` + `.claude/hooks/*.sh` — existing entry points for
  `PreToolUse`, `UserPromptSubmit`, and `SessionStart`.
- `terraphim_file_search` — fast file finder (`fff`-style) with KG scoring.
- `ast-grep` — structural code search, ideal for inferring "what kind of change
  is happening" from the files the user is touching.

Progressive Context Loading (PCL) uses these to inject only the `CLAUDE.md`
fragments and skill bodies that are semantically relevant to the current turn.

## Goals

1. Cut steady-state project-context tokens by ≥60% on typical turns.
2. Zero regression on tasks that genuinely need broad context (catch-all
   fallback when KG coverage is low).
3. No model changes — works through Claude Code hooks + additionalContext.
4. Deterministic and debuggable — every injected fragment traces back to the
   concepts that pulled it in.

## Non-Goals

- Rewriting `CLAUDE.md`. PCL consumes the file as-is plus a sidecar index.
- Dynamic skill *authoring*. We load existing skills; we don't synthesise them.
- Replacing user-invoked `/skill` calls. Those remain explicit.

## Architecture

```
┌──────────────────────── Claude Code session ─────────────────────────┐
│                                                                      │
│  SessionStart hook ──► terraphim-agent context bootstrap             │
│       │                   ├─ loads minimal CLAUDE.md core            │
│       │                   └─ warms automata + fragment index         │
│       ▼                                                              │
│  UserPromptSubmit hook ──► terraphim-agent context resolve           │
│       │                   ├─ automata match on prompt  (fast)        │
│       │                   ├─ fff scan of cwd + recent files          │
│       │                   ├─ ast-grep pass on touched files          │
│       │                   ├─ learnings lookup (similar past prompts) │
│       │                   └─ emit additionalContext JSON             │
│       ▼                                                              │
│  PreToolUse(Read/Edit/Write) ──► terraphim-agent context file        │
│                           └─ inject per-file guidance (opt-in)       │
└──────────────────────────────────────────────────────────────────────┘
```

Key idea: the hook output uses Claude Code's `additionalContext` /
`hookSpecificOutput` channel to inject text without the user seeing it, and the
selection is a pure function of (prompt, cwd, recent edits, KG, learnings).

## Fragment Index

### CLAUDE.md segmentation

`CLAUDE.md` is split into fragments at H2/H3 boundaries. Each fragment carries
frontmatter the author can add once:

```markdown
<!-- pcl:
  id: async-channels
  concepts: [tokio, mpsc, broadcast, oneshot, backpressure, RwLock, Mutex]
  files: ["**/*.rs"]
  weight: 1.0
  always: false
-->
## Channels and Concurrency
- Use Rust's `tokio::sync::mpsc` ...
```

The index is a single JSON artefact written to
`target/pcl/claude_md_index.json`:

```json
{
  "core": ["project-overview", "key-conventions"],
  "fragments": [
    {
      "id": "async-channels",
      "file": "CLAUDE.md",
      "range": [145, 162],
      "concepts": ["tokio", "mpsc", "broadcast", "oneshot"],
      "file_globs": ["**/*.rs"],
      "weight": 1.0,
      "token_estimate": 240
    },
    ...
  ]
}
```

Fragments without frontmatter get concepts inferred automatically by running
the KG automata over the fragment body — this means existing `CLAUDE.md` works
on day one without hand-tagging.

### Skill index

`~/.claude/skills/*.md` and project-local skills are indexed the same way
(`target/pcl/skills_index.json`), plus the `TRIGGER`/`SKIP` hints the skill
already ships (see e.g. `claude-api`, `session-start-hook` in this repo's skill
list).

### Build

A new subcommand:

```bash
terraphim-agent context index           # rebuild both indices
terraphim-agent context index --watch   # incremental, using terraphim_file_search::watcher
```

Cache keyed on `(file_hash, thesaurus_hash)`. Rebuild is O(fragments) and
completes in <500ms for a 5k-line `CLAUDE.md`.

## Resolver

`terraphim-agent context resolve` is the hook-callable core. Input is a JSON
event from Claude Code; output is `additionalContext` text plus a manifest.

### Signal sources

1. **Prompt automata match** — run Aho-Corasick (LeftmostLongest, same engine
   as `replace`) over the prompt. Returns a multiset of concepts with match
   positions. This is the primary signal.
2. **CWD / recent files via fff** — `terraphim_file_search` scans the working
   directory and the last-N edited files (read from
   `.claude/sessions/<id>/transcript.jsonl`) and scores them against the KG.
   Returns a set of "active concepts" with weights.
3. **Structural pass via ast-grep** — for files in the active set, run a small
   battery of ast-grep patterns that map to KG concepts. Examples:

   ```yaml
   # rules/async-tokio.yml
   id: uses-tokio-channel
   language: rust
   rule:
     any:
       - pattern: tokio::sync::mpsc::$$$
       - pattern: tokio::sync::broadcast::$$$
       - pattern: tokio::spawn($$$)
   concepts: [tokio, async-channels]
   ```

   Rules live under `crates/terraphim_agent/rules/astgrep/` and are shipped with
   the binary. Users can add project-local rules under `.terraphim/astgrep/`.

4. **Learnings lookup** — query the skill store with the prompt plus the
   concepts from (1). `terraphim-agent learn query --semantic` already supports
   this. A past successful turn for a similar prompt contributes its fragment
   manifest with decayed weight.

### Scoring & selection

Each fragment `f` gets a score:

```
score(f) = Σ_c  w_c · (match_prompt(c) + α·match_files(c) + β·match_ast(c))
        + γ · learning_prior(f)
        - δ · token_cost(f)
```

Weights are role-scoped (`terraphim_config::Role`) and default to
`α=0.7, β=0.5, γ=1.2, δ=0.002`. Selection is a knapsack with a per-turn budget
(default 1500 tokens, configurable via env `TERRAPHIM_PCL_BUDGET`).

Fragments marked `always: true` are injected unconditionally and don't count
against budget. The "core" slice (project overview, safety rules, commit
conventions) is always-on.

### Output

Stdout is a Claude Code hook JSON payload:

```json
{
  "hookSpecificOutput": {
    "hookEventName": "UserPromptSubmit",
    "additionalContext": "## Relevant project conventions\n\n<fragment bodies>\n"
  },
  "systemMessage": "PCL loaded 3 fragments, 2 skills (1120/1500 tokens). Concepts: tokio, mpsc, rolegraph."
}
```

The `systemMessage` appears in the transcript so the user can always see *why*
a fragment was loaded, which is essential for debugging false negatives.

## Integration points

### astgrep

Dependency: the `ast-grep` CLI (already used elsewhere in the Terraphim
workflow). The resolver shells out with
`ast-grep scan --rule-file <bundled> --json <files>`. On machines without
`ast-grep`, the structural signal is skipped silently — the automata and file
signals still work.

Output is folded into the concept multiset with a distinct `src: "ast"` tag so
downstream code can down-weight it if the user opts out.

### fff (terraphim_file_search)

Already present as a crate. We call it in-process, not by shelling out:

```rust
use terraphim_file_search::{config::SearchConfig, kg_scorer::KgScorer};

let hits = SearchConfig::for_cwd()
    .with_kg_scorer(KgScorer::from_role(&role))
    .scan_recent(30)              // last 30 edited files in the session
    .await?;
```

### Hooks

Three shell scripts under `.claude/hooks/`:

- `pcl-session-start.sh` — calls `terraphim-agent context bootstrap --json`,
  writes `target/pcl/session_<id>.json` with the resolved core.
- `pcl-user-prompt.sh` — reads event JSON from stdin, invokes
  `terraphim-agent context resolve --event -`, prints hook JSON to stdout.
- `pcl-pre-tool.sh` — opt-in; for `Read`/`Edit`/`Write` injects file-level
  guidance via `terraphim-agent context file <path>`.

Wired up in `.claude/settings.json` under `hooks`.

### Reusing existing machinery

- `terraphim_hooks` crate already has pattern-discovery + replacement
  infrastructure. PCL reuses `terraphim_hooks::discovery` for detecting the
  project's `CLAUDE.md`, hooks dir, and skills dir.
- `terraphim_agent::learnings::hook` already parses the Claude Code hook JSON
  schema; PCL reuses that parser so we don't drift.
- `terraphim_agent::kg_validation` already wraps automata match + connectivity;
  PCL layers on top of it instead of reimplementing concept extraction.

## Implementation plan

### Phase 1 — Index and resolver core (foundation)

Crate: `crates/terraphim_agent`, new module `context/`.

1. `context::index` — segmentation + frontmatter parsing + concept inference.
   Add `SegmentedDoc`, `FragmentIndex`. Reuse `terraphim-markdown-parser` for
   headings and frontmatter.
2. `context::resolve` — takes `ResolveEvent`, returns `ResolveOutcome`
   (`fragments`, `skills`, `systemMessage`, `token_budget`). No I/O in this
   function; dependency-inject the KG, fragment index, and learnings store.
3. CLI: `terraphim-agent context {index,resolve,bootstrap,file}`.
4. Unit tests in `crates/terraphim_agent/src/context/tests.rs` — golden
   fragments, deterministic selection under fixed weights, budget respected.

Exit criteria: `terraphim-agent context resolve` produces a manifest for a
fixture prompt on this repo's real `CLAUDE.md` in <50ms (warm).

### Phase 2 — Signal fan-out (fff + ast-grep)

1. Wire `terraphim_file_search` into the resolver behind a `FileSignal` trait.
2. Add `AstGrepSignal` shelling out to `ast-grep`; bundle starter rules.
3. Add `LearningSignal` reading from `terraphim_agent::learnings`.
4. Make each signal independently togglable (`--no-ast`, `--no-files`,
   `--no-learnings`) so CI and offline runs stay deterministic.

Exit criteria: on a benchmark of 20 real prompts captured from past sessions,
PCL selects the "obviously relevant" fragment ≥90% of the time.

### Phase 3 — Hook wiring

1. Ship `pcl-session-start.sh`, `pcl-user-prompt.sh`, `pcl-pre-tool.sh` under
   `.claude/hooks/`.
2. Extend `scripts/install-terraphim-hooks.sh` with `--enable-pcl`.
3. Update `.claude/settings.json` template in `terraphim_hooks::discovery`
   (registration code already there for existing hooks).
4. Integration test: spawn a mock Claude Code event stream (we already have
   `terraphim_agent::learnings::hook` test harness), assert
   `additionalContext` contains the expected fragment ids.

Exit criteria: end-to-end test passes locally and in CI.

### Phase 4 — Observability + tuning

1. `target/pcl/trace.jsonl` — one record per resolve, containing concepts,
   per-signal scores, selected fragments, and budget usage.
2. `terraphim-agent context explain <trace-id>` — pretty-prints the scoring
   table for a past resolve. Essential for iterating weights.
3. Role-scoped weight overrides via existing `Role::extra`.
4. Procedure capture: wire into `terraphim-agent learn procedure` so successful
   PCL manifests get recorded and replayed for similar future prompts.

### Phase 5 — Opt-in progressive expansion

A follow-up: let the model *request* more context mid-turn by emitting a
sentinel (e.g. tool call `context_expand(topic: "persistence")`). The hook
watches for it and injects the matching fragments on the next turn. This keeps
the floor low while giving an escape hatch.

## Testing strategy

- **Unit**: fragment index parsing; selection under fixed weights; budget
  enforcement; frontmatter inference fallback.
- **Property**: for any prompt and fragment set, `|selected|` is stable when
  the same prompt is resolved twice (determinism) and total tokens ≤ budget.
- **Golden**: record resolver output for a curated prompt corpus under
  `crates/terraphim_agent/tests/fixtures/pcl/` and diff on CI.
- **Integration**: end-to-end hook round-trip using the
  `terraphim_agent::learnings::hook` event harness.
- **Live** (ignored, env-gated): run against a real Claude Code session with
  `TERRAPHIM_PCL_TRACE=1` and inspect `target/pcl/trace.jsonl`.

## Risks & mitigations

| Risk | Mitigation |
|---|---|
| Resolver misses a needed fragment | Always-on core + explicit `systemMessage` listing loaded fragments so the user spots gaps early. `/pcl expand <topic>` slash command as manual override. |
| Frontmatter drift from `CLAUDE.md` edits | Concept inference fallback + `context index --watch` keeps the index fresh. |
| ast-grep unavailable on the host | Structural signal is feature-detected and silently skipped; other signals cover. |
| Hook latency on every keystroke | `UserPromptSubmit` fires once per submitted message, not per keystroke; cold resolve target 50ms, warm 10ms. |
| Learnings poisoning selection | Weight on `learning_prior` is bounded and decayed; `terraphim-agent learn correct` already exists to retract. |

## Success metrics

1. **Token footprint**: median project-context tokens per turn drops from
   ~7k to ≤2.5k on a representative prompt corpus.
2. **Fragment recall**: on the labelled corpus (Phase 2), ≥90% of prompts get
   the fragment a human would have highlighted.
3. **Latency**: p95 resolve ≤60ms on this repo; no perceptible hook delay.
4. **Determinism**: repeated resolves of the same prompt produce identical
   manifests.

## Open questions

- Should skill bodies be injected verbatim or summarised? Current plan is
  verbatim for skills under 600 tokens, summary (first paragraph + headings)
  otherwise.
- Per-file PCL (`PreToolUse` path) is powerful but loud; default off and let
  users opt in via `.claude/settings.json`.
- Cross-repo learnings via `shared-learning` feature: in-scope for Phase 4 but
  gated on trust level.
