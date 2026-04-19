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

The Terraphim workspace already contains almost every piece we need — the job
is composition, not new subsystems.

| Capability | Existing crate | Role in PCL |
|---|---|---|
| Concept extraction from text | `terraphim_automata` | Match prompts and fragments against the thesaurus (Aho-Corasick, LeftmostLongest). |
| Concept graph + ranking | `terraphim_rolegraph` + `TerraphimGraph` relevance function | The scoring function for fragments — don't reinvent it. |
| Document model | `terraphim_types::Document` | Fragments and skills are `Document`s; no custom `FragmentIndex` type needed. |
| Search service | `terraphim_service::TerraphimService::search` | Resolver is literally a search against a "claude-md" haystack with the `TerraphimGraph` scorer. |
| Haystack abstraction | `terraphim_middleware` (new `FragmentHaystack`) | Indexes `CLAUDE.md` fragments + skills the same way `Ripgrep` / `AtomicServer` do. |
| KG-based concept inference | `terraphim_kg_linter` | Runs on each fragment at index time to populate `tags`/`concepts` when frontmatter is absent. |
| Multi-backend cached index | `terraphim_persistence` | Stores the fragment `Document`s with write-back warm-up (memory → sqlite) per CLAUDE.md §Persistence. |
| Session transcripts + recent files | `terraphim_sessions` + `terraphim-session-analyzer` | Parses Claude Code / Cursor / Aider transcripts → recent-files and learning-prior signals. |
| Fast filesystem scan with KG scoring | `terraphim_file_search` (fff) | Scores cwd + recent files against the role's KG. |
| Learnings store | `terraphim_agent::learnings` | Past prompts → manifest priors via `learn query --semantic`. |
| Hook discovery + install | `terraphim_hooks` | Locates `.claude/hooks/` and `CLAUDE.md`; installs PCL hooks. |
| Hook JSON schema parser | `terraphim_agent::learnings::hook` | Reuse for `UserPromptSubmit` / `PreToolUse` parsing. |
| KG validation | `terraphim_agent::kg_validation` + `terraphim_validation` | Sanity-check fragment concept tags. |
| Markdown parsing | `terraphim-markdown-parser` | Segments `CLAUDE.md` at H2/H3; extracts frontmatter. |
| MCP surface | `terraphim_mcp_server` | Exposes `context.resolve` / `context.expand` as MCP tools so the model can request more context mid-turn. |
| Recursive model loop + budget | `terraphim_rlm` | Phase 5 model-initiated expansion reuses RLM's `BudgetTracker` and trajectory logger. |
| Usage telemetry | `terraphim_ccusage` + `terraphim_usage` + `terraphim_tracker` | Real Claude Code token/cost data feeds per-role budget calibration. |
| Config + roles | `terraphim_config::Role` | A dedicated `claude-md` role owns the haystack, scorer, weights, and budget. |
| Structural code signal | `ast-grep` (external) | Structural patterns → concept multiset; feature-detected, optional. |

Progressive Context Loading (PCL) is a thin resolver on top of this stack that
returns, per turn, only the `CLAUDE.md` fragments and skill bodies the current
prompt / files / recent session activity actually need.

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

## Fragment store = haystack

The single biggest simplification: **we don't build a new index format.** A
`CLAUDE.md` fragment is a `terraphim_types::Document` and the fragment store is
a haystack registered with `terraphim_middleware`. This gives us indexing,
persistence, search, and ranking for free — identical to how the Ripgrep and
AtomicServer haystacks work today.

```
┌──── "claude-md" Role (terraphim_config) ─────────────────────────────┐
│  relevance_function: TerraphimGraph                                  │
│  haystacks:                                                          │
│    - FragmentHaystack{ source: CLAUDE.md }                           │
│    - FragmentHaystack{ source: ~/.claude/skills/**/*.md }            │
│    - FragmentHaystack{ source: .claude/skills/**/*.md }              │
│  extra:                                                              │
│    pcl_budget_tokens: 1500                                           │
│    pcl_always: [project-overview, commit-conventions]                │
│    pcl_weights: { prompt: 1.0, files: 0.7, ast: 0.5, learnings: 1.2 }│
└──────────────────────────────────────────────────────────────────────┘
```

The `FragmentHaystack` indexer (new, small: 1 file in
`terraphim_middleware/src/haystack/fragment.rs`) does two things:

1. Segment `CLAUDE.md` / skill files via `terraphim-markdown-parser`.
2. For each fragment, emit a `Document` whose `body` is the fragment text,
   `tags` are concepts from frontmatter or — when absent — inferred by
   `terraphim_kg_linter` over the fragment body.

Then `terraphim_service::TerraphimService::search` against this haystack with
the `TerraphimGraph` scorer already returns fragments ranked by rolegraph
concept overlap. The "resolver" is now a thin layer on top that adds non-text
signals (files, ast-grep, learnings) by mutating the `SearchQuery`'s term
weights.

### CLAUDE.md segmentation

`CLAUDE.md` is split into fragments at H2/H3 boundaries. Each fragment carries
optional frontmatter the author can add once:

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

Fragments without frontmatter get concepts inferred automatically by
`terraphim_kg_linter` over the fragment body — existing `CLAUDE.md` works on
day one without hand-tagging.

### Persistence

No new on-disk format. The fragment `Document`s are persisted through
`terraphim_persistence::DeviceStorage` exactly like any other Terraphim
document, which means:

- Multi-backend ordering (memory → dashmap → sqlite) with fire-and-forget
  cache write-back, per `CLAUDE.md §Persistence Layer Cache Warm-up`.
- Schema-evolution resilience: a stale cached fragment that fails to
  deserialize is dropped and re-indexed, not an error.
- Observability via the existing tracing spans (`load_from_operator`,
  `cache_writeback`).

Cache key: `(file_path, file_hash, thesaurus_hash)`.

### Skill index

`~/.claude/skills/*.md` and project-local skills are indexed through the same
`FragmentHaystack`, with the skill's existing `TRIGGER` / `SKIP` hints parsed
into additional tags. No second index — one haystack, multiple sources.

### Build

A new subcommand:

```bash
terraphim-agent context index           # forces (re)index via TerraphimService
terraphim-agent context index --watch   # incremental via terraphim_file_search::watcher
```

Both just drive `terraphim_middleware`'s existing index pipeline for the
`claude-md` role. Rebuild is O(fragments) and completes in <500ms for a
5k-line `CLAUDE.md`.

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

Don't reinvent. Build a `SearchQuery` whose `term` multiset is the union of the
four signal sources, each tagged with a weight:

```rust
let mut terms = prompt_concepts;                    // from automata
terms.extend(file_concepts.scaled(w.files));        // from fff
terms.extend(ast_concepts.scaled(w.ast));           // from ast-grep
terms.extend(learning_concepts.scaled(w.learn));    // from learnings

let docs = service.search(&SearchQuery {
    role: Some("claude-md".into()),
    terms,
    operator: LogicalOperator::Or,
    limit: 64,
}).await?;
```

`TerraphimGraph`'s existing rolegraph scorer then ranks fragments by
PageRank-weighted concept overlap — the exact machinery used for every other
Terraphim search. Selection from the ranked list is a knapsack under a token
budget sourced from `terraphim_ccusage` / `terraphim_usage` (falling back to
the role's `pcl_budget_tokens`, default 1500).

Fragments in `pcl_always` (project overview, safety rules, commit conventions)
are injected unconditionally and don't count against budget.

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

### Sessions and learning priors

`terraphim_sessions` already imports Claude Code / Cursor / Aider transcripts
(see `desktop/` `/sessions` REPL commands and `crates/terraphim-session-analyzer`).
The resolver uses it for two signals:

1. **Recent files** — the last N `Edit`/`Write`/`Read` paths from the active
   session → feeds `terraphim_file_search` for scoring.
2. **Prior manifests** — for each past turn whose prompt semantically matches
   the current one (via `terraphim-session-analyzer::analyzer`), the fragments
   that were loaded and didn't trigger a correction become a `learning_prior`.
   `terraphim_agent::learnings` already tracks correction events, so we down-
   weight priors from turns that got corrected.

### Model-initiated expansion (Phase 5) via RLM

`terraphim_rlm` already implements a recursive language model loop with a
`BudgetTracker` and JSONL trajectory logger. Phase 5 wires PCL's `context.expand`
MCP tool into an RLM step: when the model calls the tool, RLM adds the
resulting fragments to the next turn's context and charges the budget. No new
budget or logging infrastructure.

### MCP surface

`terraphim_mcp_server` exposes resolver operations as tools — so any Claude
Code session (or other MCP client) can call them directly, independent of the
shell hooks:

- `context.resolve(prompt, cwd, recent_files[]) -> {fragments, manifest}`
- `context.expand(topic) -> {fragments}`
- `context.explain(trace_id) -> {score_table}`
- `context.index(source?) -> {indexed_count}`

This means Phase 5 (model asks for more context) becomes a native MCP tool
call instead of a sentinel-string hack.

### Reusing existing machinery

- `terraphim_hooks::discovery` detects the project's `CLAUDE.md`, hooks dir,
  and skills dir. PCL registers its three hooks through the same installer.
- `terraphim_agent::learnings::hook` already parses the Claude Code hook JSON
  schema; PCL reuses the parser so the schema stays in one place.
- `terraphim_agent::kg_validation` + `terraphim_validation` already wrap
  automata match + connectivity; PCL uses them for fragment tag validation at
  index time.
- `terraphim_config::Role` owns weights, budget, and the `pcl_always` list —
  PCL ships no global config file.

## Implementation plan

### Phase 1 — FragmentHaystack + role

Crate: `terraphim_middleware`, add `haystack/fragment.rs` (~200 lines).

1. `FragmentHaystack` implementing the haystack trait: segments markdown via
   `terraphim-markdown-parser`, emits `Document`s, runs `terraphim_kg_linter`
   when frontmatter is absent.
2. Ship a `claude-md` role preset in `terraphim_server/default/` with
   `relevance_function: TerraphimGraph` and the three default sources.
3. Persist via the existing `terraphim_persistence` stack — nothing new.
4. Unit tests: segmentation, frontmatter parsing, linter fallback.

Exit criteria: `terraphim-agent search --role claude-md "tokio mpsc"` returns
the async-channels fragment first on this repo's real `CLAUDE.md`.

### Phase 2 — Resolver module in terraphim_agent

New module `crates/terraphim_agent/src/context/` (~400 lines).

1. `context::resolve(event) -> ResolveOutcome` — builds a weighted
   `SearchQuery` from prompt automata + file signal + ast-grep + learnings,
   calls `TerraphimService::search`, runs the knapsack.
2. CLI: `terraphim-agent context {resolve,bootstrap,file,explain,index}`.
3. No I/O in the core function; inject `TerraphimService`, session store,
   learnings store, and signal providers.
4. Golden tests under `tests/fixtures/pcl/` — deterministic selection under
   fixed weights, budget respected.

Exit criteria: warm resolve <50ms on this repo's real `CLAUDE.md`.

### Phase 3 — Signal fan-out (fff + ast-grep + sessions + learnings)

1. `FileSignal` — calls `terraphim_file_search` with a `KgScorer` built from
   the `claude-md` role.
2. `SessionSignal` — calls `terraphim_sessions` for recent files and
   `terraphim-session-analyzer` for prior manifests.
3. `AstGrepSignal` — shells out to `ast-grep scan --json` against bundled rules
   under `crates/terraphim_agent/rules/astgrep/`; feature-detected.
4. `LearningSignal` — `terraphim_agent::learnings::query_semantic`.
5. Each signal is independently togglable (`--no-ast`, `--no-files`,
   `--no-sessions`, `--no-learnings`) so CI and offline runs stay
   deterministic.

Exit criteria: on a benchmark of 20 real prompts captured from past sessions
(via `terraphim_sessions` import), PCL selects the "obviously relevant"
fragment ≥90% of the time.

### Phase 4 — Hook wiring + MCP tools

1. Ship `pcl-session-start.sh`, `pcl-user-prompt.sh`, `pcl-pre-tool.sh` under
   `.claude/hooks/`.
2. Extend `scripts/install-terraphim-hooks.sh` with `--enable-pcl`, wired
   through `terraphim_hooks::discovery`.
3. Add `context.*` MCP tools in `terraphim_mcp_server` so the model can call
   resolve/expand/explain directly.
4. Integration test: mock Claude Code event stream via
   `terraphim_agent::learnings::hook` test harness, assert
   `additionalContext` contains the expected fragment ids.

Exit criteria: end-to-end test passes locally and in CI; MCP tools callable
from `rmcp-client` smoke test.

### Phase 5 — Observability + tuning

1. Trace via `terraphim_tracker` + `terraphim_usage` — one record per resolve
   (concepts, per-signal scores, selected fragments, token budget spent vs.
   actual tokens reported by `terraphim_ccusage`).
2. `terraphim-agent context explain <trace-id>` — pretty-prints the scoring
   table for a past resolve. Essential for iterating weights.
3. Role-scoped weight overrides via existing `Role::extra`.
4. Procedure capture: wire into `terraphim-agent learn procedure` so successful
   PCL manifests get recorded and replayed for similar future prompts.

### Phase 6 — Model-initiated expansion via RLM

Promote Phase 4's `context.expand` MCP tool into a full `terraphim_rlm` step:
when the model calls the tool, RLM charges its `BudgetTracker`, logs the
trajectory, and returns the additional fragments. RLM's existing loop gives us
retry, budget enforcement, and JSONL trace for free — no new recursive
infrastructure.

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
