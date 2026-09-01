# Architecture: terraphim_agent

**Status**: Living document
**Date**: 2026-09-01
**Owner**: terraphim-agent maintainers
**Source of truth**: `terraphim-clients/crates/terraphim_agent/`
**Related sweep**: [handover 2026-09-01](../handovers/2026-09-01-terraphim-agent-shared-learning-hybrid.md)
**Code references in this document** link back to the `terraphim-clients` repo at the commit current when this document was last refreshed (`8c245fd78`).

## 1. Purpose

`terraphim_agent` is the primary CLI binary for Terraphim. It is the entry point
for everything that the agent system does on a developer machine: semantic
search across configured haystacks, knowledge-graph validation, an interactive
REPL, the safety guard for destructive commands, session history import, and
the cross-agent learning pipeline that lets one Terraphim agent re-use
corrections discovered by another.

Two of those subsystems — `learnings` and `shared_learning` — together
implement the **operational learning** feature. This document explains how
they fit together, where the data lives on disk, how trust is propagated, and
which scoring path runs when an agent looks a learning up.

## 2. Crate Map and Feature Flags

`terraphim_agent` is a library plus a binary in the `terraphim-clients` Cargo
workspace. The library is the public surface; the binary (`terraphim-agent`)
links the same modules and adds an entrypoint in `src/main.rs`.

| Module | Always on? | Purpose |
|---|---|---|
| `onboarding`, `service`, `tui_backend` | yes | First-run wizard, server-backed TUI plumbing, backend glue |
| `client` | behind `server` | HTTP client for server-backed TUI mode |
| `repl` | behind `repl` / `repl-interactive` | Interactive REPL with rustyline |
| `commands` | behind `repl-custom` | Markdown-defined command set |
| `robot` | yes | Robot mode: structured JSON output for automation |
| `forgiving` | yes | Typo-tolerant CLI parser |
| `mcp_tool_index` | yes | MCP tool discovery and search |
| `shared_learning` | behind `shared-learning` | Cross-agent learning store, wiki sync, hybrid scoring |
| `learnings::injector` | behind `cross-agent-injection` | Inject shared learnings into other agent contexts |

`learnings` itself is always compiled. The capture pipeline, hooks, redact,
guard, export-KG, and procedure modules live there regardless of feature
gates. Only the cross-agent injection and shared-learning re-use of those
learnings is feature-gated.

## 3. Sub-system A: `learnings` (capture pipeline)

`learnings/` is the part of the agent that listens to what other agents do,
extracts reusable corrections, and persists them as durable artefacts.

The flow is:

```text
PostToolUse hook
       |
       v
redact_secrets  ---->  capture_failed_command  ---->  MarkdownLearningStore
                                                             |
                                                             v
                                              corrections / procedures / KG thesaurus
```

Concrete pieces:

- `hook.rs` — `AgentFormat::{Claude, Opencode, Codex, Auto}` parses the JSON
  payload each AI coding agent emits and normalises it to a `HookInput`. The
  default is `Auto`, which shape-sniffs the JSON.
- `capture.rs` — `capture_failed_command` filters commands against
  `LearningCaptureConfig::should_ignore` (test runners like `cargo test*` are
  ignored by default) and writes a `CapturedLearning` to the
  `LearningCaptureConfig::storage_location()`. It also bridges to the
  `SharedLearning` store when the `shared-learning` feature is on, via
  `shared_learning_from_entry`.
- `redaction.rs` — `redact_secrets` strips API keys, bearer tokens, and other
  obvious credentials from stderr before they are written to disk.
- `guard.rs` — `evaluate_command` and `evaluate_command_with_learning` decide
  whether a command is safe to run. The latter consults a learning before
  returning a `GuardDecision`.
- `procedure.rs` — `ProcedureStore` persists multi-step replays.
- `compile.rs` and `export_kg.rs` — build KG thesauri and export corrections
  as KG markdown for downstream consumers.
- `install.rs` — `install_hook` registers the capture hook with Claude Code,
  opencode, or Codex.

Configuration lives in `LearningCaptureConfig`:

| Field | Default | Override |
|---|---|---|
| `project_dir` | `<cwd>/.terraphim/learnings` | none — fixed relative to cwd |
| `global_dir` | `dirs::data_dir() + /terraphim/learnings` | `TERRAPHIM_DEFAULT_DATA_PATH` env var |
| `enabled` | `true` | per-binary config |
| `ignore_patterns` | `cargo test*`, `npm test*`, `pytest*`, `yarn test*` | per-binary config |

`storage_location()` honours `TERRAPHIM_DEFAULT_DATA_PATH` first (so hermetic
tests can pin a sandbox), then prefers the project directory if it or its
parent exists, otherwise the global directory. This is the same convention
used by `terraphim_settings::DeviceSettings`; the two were aligned under
issue #144.

## 4. Sub-system B: `shared_learning` (cross-agent store)

`shared_learning/` is the part of the agent that turns the per-agent
`CapturedLearning`s into a corpus that other agents can query. The storage is
deliberately boring — markdown files with YAML frontmatter — so a human can
read or edit them with `cat` and `vim`.

```text
CapturedLearning  ---->  SharedLearning  ---->  Markdown file
                                                  |
                                                  v
                                          SharedLearningStore
                                                  |
                                                  v
                            suggest / find_similar / query_relevant
                                          (hybrid scorer)
```

Concrete pieces:

- `types.rs` — re-exports `SharedLearning`, `TrustLevel`, `LearningSource`,
  `SuggestionStatus`, `QualityMetrics` from `terraphim_types::shared_learning`.
  This is the canonical type; the `terraphim_agent` crate adds no fields.
- `markdown_store.rs` — `MarkdownLearningStore` writes files at
  `{learnings_dir}/{source_agent}/{id}.md` (per-agent) or
  `{learnings_dir}/shared/{source_agent}-{id}.md` (cross-agent). The default
  `learnings_dir` is the platform data dir
  (`~/Library/Application Support/com.aks.terraphim/learnings` on macOS),
  overridable with `TERRAPHIM_LEARNINGS_DIR`.
- `store.rs` — `SharedLearningStore` is the in-memory cache over the markdown
  backend, plus the rolegraph integration described in section 6.
- `wiki_sync.rs` — `GiteaWikiClient` publishes L2/L3 learnings to a Gitea
  wiki so a human reviewer can audit them.
- `injector.rs` — `LearningInjector` (behind `cross-agent-injection`) reads
  shared learnings and injects them into the system prompt of other agents.

`SharedLearning` carries:

| Field | Type | Source |
|---|---|---|
| `id` | `String` | `learning-{uuid}-{millis}` |
| `title`, `content` | `String` | human-readable + markdown body |
| `trust_level` | `TrustLevel` | L0 / L1 / L2 / L3 |
| `quality` | `QualityMetrics` | applied / effective counts, agent roster, success rate |
| `source` | `LearningSource` | BashHook, AutoExtract, ToolHealth, GiteaComment, CjeVerdict, Manual |
| `source_agent` | `String` | the agent that originally captured the learning |
| `applicable_agents` | `Vec<String>` | empty = global, otherwise per-agent scope |
| `keywords` | `Vec<String>` | search-time boost terms |
| `verify_pattern` | `Option<String>` | regex that re-confirms the learning still applies |
| `original_command`, `error_context`, `correction` | `Option<String>` | full provenance for the correction |
| `wiki_page_name` | `Option<String>` | set when synced to Gitea wiki |
| `suggestion_status` | `SuggestionStatus` | Pending / Approved / Rejected |
| `bm25_confidence` | `Option<f64>` | last BM25 score from the suggestion engine |

## 5. End-to-end Data Flow

```text
                  +----------------------------+
                  |  AI coding agent           |
                  |  (Claude / opencode/Codex) |
                  +-------------+--------------+
                                |
                                | PostToolUse event
                                v
                  +-------------+--------------+
                  |  learnings::hook           |
                  |  (AgentFormat::Auto)       |
                  +-------------+--------------+
                                |
                                | redaction + filter
                                v
                  +-------------+--------------+
                  |  capture_failed_command    |
                  |  / capture_correction      |
                  +-------------+--------------+
                                |
                                v
                  +-------------+--------------+
                  |  MarkdownLearningStore     |
                  |  per-agent + shared dirs   |
                  +-------------+--------------+
                                |
                                | rebuild index
                                v
                  +-------------+--------------+
                  |  SharedLearningStore       |
                  |  + RoleGraph (optional)    |
                  +-------------+--------------+
                                |
                                v
            +-------------------+-------------------+
            | suggest(context, agent, limit)        |
            | find_similar(query, limit)            |
            | query_relevant(agent, ctx, min, lim)  |
            +---------------------------------------+
                                |
                                v
                  ranked list of SharedLearnings
```

## 6. Hybrid Scoring: the 2026-09 refactor

Before 2026-09, `SharedLearningStore::suggest` and `find_similar` ranked
learnings with a hand-rolled BM25 scorer (`Bm25Scorer`, K1=1.2, B=0.75,
tanh-normalised). That worked, but Terraphim already ships a hybrid scorer
inside `terraphim_rolegraph::RoleGraph::query_graph` — a weighted mean of
node rank, edge rank, and document rank with thesaurus term expansion — and
the agent was not using it for its own internal corpus.

The 2026-09 refactor routes both `suggest` and `find_similar` through the
hybrid scorer when a `RoleGraph` is configured, and falls back to pure BM25
otherwise. The graph is an accelerator, not a source of truth: the markdown
backend is still authoritative for persistence, and every public method
returns a `Result<...>` that does not change shape across the two paths.

### 6.1 The two paths

**Hybrid path** (graph configured):

1. The in-memory `index: RwLock<HashMap<String, SharedLearning>>` is filtered
   to the candidates the caller cares about (per-agent scope for `suggest`,
   full corpus for `find_similar`).
2. `hybrid_rank(query, candidates, limit)` acquires a read lock on
   `self.role_graph` and calls `RoleGraph::query_graph(query, None, cap)`
   where `cap = limit.saturating_mul(2).max(8)`. The cap prevents the graph
   from over-fetching when the corpus grows.
3. Each candidate that appears in the graph result, **or** whose
   `extract_searchable_text` contains the lowercased query as a substring,
   is kept. The graph result is not the only source of candidates because
   the thesaurus may not yet cover the query term.
4. For each survivor, the `IndexedDocument.rank` is normalised against the
   maximum rank in the result set, then multiplied by
   `TrustLevel::weight()` (L0=0, L1=1, L2=2, L3=3) so the final score sits
   in `[0, 3]` and stays comparable with the BM25 path.
5. Survivors are sorted by score descending and truncated to `limit`.

**Pure BM25 path** (graph not configured, locked, or empty):

1. The same candidate set is used.
2. A `Bm25Scorer` is built from the candidate set: average document length,
   per-term document frequencies, then per-document term frequencies.
3. Each candidate is scored with the same `extract_searchable_text` body
   used in the hybrid path, normalised by `tanh(score / query_len)`, and
   multiplied by the trust-level weight.
4. Survivors are sorted and truncated identically.

### 6.2 Document indexing strategy

The rolegraph keys its internal hashmap on the `document_id` parameter
passed to `RoleGraph::insert_document`, not on `Document.id`. The
`build_document_for_graph` helper in `store.rs` therefore:

- Leaves `Document.id` empty (the parameter is the real key; populating the
  field would cost one extra clone per insert for no observable benefit).
- Leaves `Document.tags` empty, because `Document::fmt` — which the
  rolegraph uses to derive the indexing string — does not include tags.
  Keyword coverage is already in the body via
  `SharedLearning::extract_searchable_text`.
- Sets `Document.body` to `learning.extract_searchable_text()` so the
  hybrid scorer indexes the same surface form the BM25 fallback uses.
  `extract_searchable_text` lowercases and concatenates `title + content +
  keywords + original_command + error_context`.
- Sets `Document.source_haystack = Some("shared_learning_store")` so
  downstream queries can filter on origin if they need to.

### 6.3 Failure modes and the BM25 fallback

`hybrid_rank` returns `Option<Vec<(f64, SharedLearning)>>`. It returns
`None` — which makes the caller fall back to pure BM25 — in any of these
cases:

- No graph is configured (`set_role_graph` was never called).
- The graph read lock is poisoned (a writer panicked holding the lock).
- The graph query itself fails.
- The graph returns an empty result set (no thesaurus node matched the
  query, so `query_graph` early-returns).

The poisoned-lock case is logged with `tracing::warn!` and the learning-id,
so operators can detect degraded mode in the logs without having to
reproduce it. The empty-result case is silent because it is the common path
on a cold cache.

`SharedLearningStore::set_role_graph` performs a one-time initial sync of
the in-memory index into the graph. If either the index read lock or the
graph write lock is contended at that moment, the graph is left empty
(with a warning) and the next `insert` re-syncs that single learning via
`sync_to_graph`. There is no background resync job; the graph is always
eventually consistent with the index, never the other way around.

## 7. Trust Levels and Promotion

Trust levels are an integer enum with a `weight()` used by the scorer and a
`display_name()` used by the UI:

| Level | Weight | Display | Source of promotion |
|---|---|---|---|
| L0 | 0 | Extracted | raw extraction, never exposed via suggestion |
| L1 | 1 | Unverified | default for new learnings |
| L2 | 2 | Peer-Validated | auto-promote when `QualityMetrics::meets_l2_criteria()` (3+ applications across 2+ agents with positive outcome) and `StoreConfig::auto_promote_l2` is on |
| L3 | 3 | Human-Approved | `SharedLearningStore::promote_to_l3` after `/evolve` review or Gitea issue approval |

`allows_wiki_sync()` returns true only for L2 and L3, so the wiki never
sees unverified learnings.

Promotion is one-way at the API level (`promote_to_l1`, `promote_to_l2`,
`promote_to_l3`, `approve`, `reject`). Demotion happens by writing a new
trust level directly to a learning's frontmatter; the store has no
`demote_to_l1` method.

## 8. Storage Layout

`MarkdownStoreConfig::default()` resolves `learnings_dir` to:

| Platform | Path |
|---|---|
| macOS | `~/Library/Application Support/com.aks.terraphim/learnings` |
| Linux | `$XDG_DATA_HOME/com.aks.terraphim/learnings` (fallback `~/.local/share/...`) |
| Windows | `%LOCALAPPDATA%\com.aks.terraphim\learnings` |

Override at runtime with `TERRAPHIM_LEARNINGS_DIR`.

Filesystem layout:

```
<learnings_dir>/
  <source_agent>/
    <learning-id>.md
  shared/
    <source_agent>-<learning-id>.md
```

Each `.md` file is a YAML frontmatter block followed by a markdown body.
The body is `learning.content`; the frontmatter carries every other
field. Round-trip is lossless: the
`test_save_and_load_roundtrip_preserves_full_state` test in
`markdown_store.rs` exercises every field including the sparse case
(`test_sparse_old_frontmatter_still_loads`).

```markdown
---
id: learning-5f3e-1717432000123
title: Use --force-with-lease instead of --force
agent_id: security-sentinel
captured_at: 2026-08-15T12:00:00Z
updated_at: 2026-08-15T12:00:00Z
trust_level: L1
source: bash_hook
applicable_agents:
  - security-audit
  - code-review
keywords:
  - git
  - force-push
verify_pattern: git push --force-with-lease
quality:
  applied_count: 0
  effective_count: 0
  agent_count: 0
  agent_names: []
  last_applied_at: null
  success_rate: null
original_command: git push -f
error_context: "remote: rejected"
correction: use --force-with-lease
---

`git push -f` is rejected on protected branches. Use
`git push --force-with-lease` instead — it checks that the upstream has
not moved before overwriting.
```

## 9. Configuration Knobs

| Knob | Default | Effect |
|---|---|---|
| `TERRAPHIM_LEARNINGS_DIR` | platform data dir | override markdown store root |
| `TERRAPHIM_DEFAULT_DATA_PATH` | unset | when set, forces the capture pipeline's global dir under `<path>/terraphim/learnings` |
| `StoreConfig::similarity_threshold` | `0.8` | threshold for `store_with_dedup` to merge instead of insert |
| `StoreConfig::auto_promote_l2` | `true` | when on, `record_application` auto-promotes L1 -> L2 once L2 criteria are met |
| `LearningCaptureConfig::ignore_patterns` | four common test runners | glob patterns excluded from capture |

## 10. Build, Lint, Test

```bash
# Default build (no shared learning)
cargo build -p terraphim_agent

# With hybrid scoring and cross-agent injection
cargo build -p terraphim_agent --features shared-learning,cross-agent-injection

# Lint and format
cargo fmt -p terraphim_agent
cargo clippy -p terraphim_agent --all-targets --features shared-learning,cross-agent-injection -- -D warnings

# Tests
cargo test -p terraphim_agent --lib
cargo test -p terraphim_agent --lib --features shared-learning
```

Integration tests in `tests/kg_ranking_integration_test.rs` exercise the
hybrid path against a real `RoleGraph` and assert that hybrid ranking
out-performs pure BM25 on a thesaurus-enriched corpus. They require the
`shared-learning` feature.

## 11. Cross-References

- **Working example**:
  [`docs/examples/terraphim-agent-shared-learning.md`](../examples/terraphim-agent-shared-learning.md)
  walks through capture -> store -> suggest -> find_similar with code and
  CLI invocations.
- **Session handover**:
  [`docs/handovers/2026-09-01-terraphim-agent-shared-learning-hybrid.md`](../handovers/2026-09-01-terraphim-agent-shared-learning-hybrid.md)
  records the 2026-09 hybrid-scoring refactor, the four P2 findings
  addressed in `85b1b8b`, and the verification, validation, and PR review
  artefacts produced during that cycle.
- **Source artefacts** (in `terraphim-clients`):
  `.docs/verification/verification-report-hybrid-scoring.md`,
  `.docs/validation/validation-report-hybrid-scoring.md`,
  `.docs/pr-review/pr-review-ba2e292.md`.
- **Source-of-truth types**:
  `terraphim-core/crates/terraphim_types/src/shared_learning.rs`
  (`SharedLearning`, `TrustLevel`, `LearningSource`, `QualityMetrics`,
  `SuggestionStatus`).
- **Polyrepo placement**: `terraphim-clients` is the clients layer of the
  polyrepo split; see [`docs/architecture/polyrepo-topology.md`](polyrepo-topology.md).
