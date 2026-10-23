# Example: terraphim_agent shared learning end-to-end

This example shows how an AI coding agent captures a failed command, promotes
it to a `SharedLearning`, and lets a different agent retrieve it through the
hybrid scorer. The same code paths are used by the production `terraphim-agent`
binary and the `terraphim_agent` library.

> **Audience**: developers integrating the `terraphim_agent` library into a
> Rust binary, and operators debugging why a particular learning surfaces
> (or does not) in a `terraphim-agent suggest` response.
>
> **Source of truth**: `terraphim-clients/crates/terraphim_agent/`. The
> snippets below compile against the crate at commit `8c245fd78`.
>
> **Background reading**:
> [`docs/architecture/terraphim-agent.md`](../architecture/terraphim-agent.md)
> explains the data flow and hybrid scorer; this page is the working
> example to that architecture.

## 0. Build the crate

```bash
# Default build (no shared learning)
cargo add terraphim_agent

# Build with the cross-agent learning store and the hybrid scorer
cargo add terraphim_agent --features shared-learning,cross-agent-injection
```

The `shared-learning` feature pulls in `terraphim_rolegraph` and
`terraphim_types`; the `cross-agent-injection` feature adds the
`LearningInjector`.

## 1. Capture a failed command

The capture pipeline expects a `PostToolUse` event from an AI coding agent.
The simplest way to feed it from your own code is to construct a
`HookInput` and call `capture_failed_command` directly.

```rust
use terraphim_agent::learnings::{
    capture_failed_command, LearningCaptureConfig, AgentFormat, HookInput,
};

let config = LearningCaptureConfig::default();

let hook = HookInput::from_json_with_format(
    r#"{
        "tool_name": "Bash",
        "tool_input": { "command": "git push -f" },
        "tool_result": { "exit_code": 1, "stderr": "remote: rejected" }
    }"#,
    AgentFormat::Auto,
)?;

if hook.should_capture() {
    let path = capture_failed_command(
        &hook.command,
        &hook.stderr,
        hook.exit_code,
        &config,
    )?;
    eprintln!("captured to {}", path.display());
}
```

The capture pipeline reda secrets, filters test-runner commands, and writes
a markdown file to the resolved `storage_location()` (project dir if it
exists, otherwise global dir, both honouring `TERRAPHIM_DEFAULT_DATA_PATH`).

## 2. Bridge a capture to the cross-agent store

The per-agent `CapturedLearning` and the cross-agent `SharedLearning` are
deliberately different types. `shared_learning_from_entry` performs the
bridge when the `shared-learning` feature is on.

```rust
#[cfg(feature = "shared-learning")]
use terraphim_agent::learnings::shared_learning_from_entry;
#[cfg(feature = "shared-learning")]
use terraphim_agent::shared_learning::{
    MarkdownStoreConfig, SharedLearningStore, StoreConfig,
};

#[cfg(feature = "shared-learning")]
async fn promote_capture_to_shared(entry: CapturedLearning) -> Result<(), Box<dyn std::error::Error>> {
    let mut store_config = StoreConfig::default();
    store_config = store_config.with_similarity_threshold(0.85);
    // Pin storage to a hermetic directory in tests; leave default in production.
    let markdown_config = MarkdownStoreConfig {
        learnings_dir: std::env::var("TERRAPHIM_LEARNINGS_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::path::PathBuf::from("./.terraphim/learnings")),
        shared_dir_name: "shared".to_string(),
    };
    store_config = store_config.with_markdown_config(markdown_config);

    let store = SharedLearningStore::open(store_config).await?;
    let learning = shared_learning_from_entry(entry)?;
    store.store_with_dedup(learning).await?;
    Ok(())
}
```

`store_with_dedup` is the entry point that decides whether the new learning
is a fresh entry (`StoreResult::Created`) or merges into an existing
near-duplicate above the configured similarity threshold
(`StoreResult::Merged`).

## 3. Wire a Terraphim `RoleGraph` for hybrid scoring

Hybrid scoring is opt-in. Without a graph the store falls back to pure
BM25. The graph is supplied as a `terraphim_rolegraph::RoleGraph`; the
store wraps it in a `std::sync::RwLock` internally.

```rust
#[cfg(feature = "shared-learning")]
use terraphim_rolegraph::RoleGraph;

#[cfg(feature = "shared-learning")]
async fn attach_graph(mut store: SharedLearningStore) -> SharedLearningStore {
    // Build a thesaurus; in production this comes from the role config
    // and a thesaurus JSON the agent loads at startup.
    let thesaurus_json = std::fs::read_to_string("./terraphim_engineer.json")?;
    let graph = RoleGraph::new_from_thesaurus(&thesaurus_json, None)?;
    store.set_role_graph(graph);
    store
}
```

`set_role_graph` performs an initial sync of the in-memory index into the
graph, so subsequent `suggest` and `find_similar` calls can use the hybrid
scorer without an explicit warm-up call. If the index or graph is locked
when `set_role_graph` runs, the graph is left empty (with a `tracing::warn!`)
and the next `insert` re-syncs that single learning.

## 4. Suggest learnings to a running agent

`suggest` is the per-agent lookup. It applies the `applicable_agents`
filter first, then scores with hybrid or BM25.

```rust
#[cfg(feature = "shared-learning")]
async fn suggest_for_agent(
    store: &SharedLearningStore,
    context: &str,
    agent_name: &str,
) -> Result<Vec<SharedLearning>, Box<dyn std::error::Error>> {
    let mut suggestions = store.suggest(context, agent_name, 5).await?;
    for s in &suggestions {
        println!("  [{}] {} — {}", s.trust_level, s.title, s.id);
    }
    Ok(suggestions)
}
```

Empty `applicable_agents` on a learning means "global" — every agent sees
it. Populated lists scope the learning to those agent names only.

## 5. Find similar learnings (corpus-wide)

`find_similar` does not filter by `applicable_agents`. Use it when the
caller already knows the corpus is relevant (e.g. the binary is running
its own internal audit and only cares about its own learnings).

```rust
#[cfg(feature = "shared-learning")]
async fn audit_similar(
    store: &SharedLearningStore,
    query: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let hits = store.find_similar(query, 10).await?;
    for (score, learning) in hits {
        println!("{:.3}  [{}]  {}", score, learning.trust_level, learning.title);
    }
    Ok(())
}
```

The returned `Vec<(f64, SharedLearning)>` is sorted by score descending
and the score is normalised against the trust-level weight, so a hit from
a L3 learning can outrank a more textually similar L1 hit.

## 6. Apply a learning and let it promote

When a learning is actually used and works, record the application. The
store increments the quality counters and, if `auto_promote_l2` is on
and the L2 criteria (3+ applications across 2+ agents with positive
outcome) are met, automatically promotes the learning to L2.

```rust
#[cfg(feature = "shared-learning")]
async fn record_success(
    store: &SharedLearningStore,
    learning_id: &str,
    agent_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    store
        .record_application(learning_id, agent_name, /* effective = */ true)
        .await?;
    Ok(())
}
```

## 7. Inspect the on-disk file

After the previous steps, the on-disk file looks like the one in the
architecture document. You can confirm the frontmatter with `head -20`:

```bash
head -20 "$TERRAPHIM_LEARNINGS_DIR/$(whoami)/learning-*.md"
```

Or list every learning the store currently knows about:

```bash
find "${TERRAPHIM_LEARNINGS_DIR:-$HOME/Library/Application Support/com.aks.terraphim/learnings}" \
  -name "*.md" \
  -path "*/shared/*" \
  -exec grep -l '^trust_level: L[23]' {} +
```

L2 and L3 learnings are the ones the wiki sync client will publish to
Gitea. Run `terraphim-agent --help` for the current subcommand list if you
want to drive the store from the CLI instead of from Rust.

## 8. Putting it together

The full happy path, in one sequence:

1. Install the capture hook:
   `terraphim-agent install-hook --agent claude`
2. A coding session fails a `git push -f`; the hook captures it.
3. `shared_learning_from_entry` promotes the capture to a `SharedLearning`.
4. `store_with_dedup` writes it to the markdown backend.
5. A second agent starts up, calls `set_role_graph`, and asks
   `suggest("git push rejected", "code-review-agent", 5)`.
6. The hybrid scorer returns the new learning at L1 with a positive score.
7. The second agent applies the correction and the store auto-promotes to
   L2 once criteria are met.
8. The wiki sync client publishes the L2 learning to the Gitea wiki.

Each step is independently testable: see the unit tests in
`markdown_store.rs` and `store.rs`, and the integration test in
`tests/kg_ranking_integration_test.rs`.

## 9. Troubleshooting

- **Learning not surfaced** — first check that `applicable_agents` is
  empty or includes the calling agent's name. Then check whether the
  thesaurus loaded into the rolegraph contains the query terms. With
  no thesaurus node matching the query, the hybrid path returns `None`
  and the store falls back to BM25.
- **`store_with_dedup` always merges into the same learning** — lower
  `StoreConfig::similarity_threshold` or check the
  `extract_searchable_text` body — the merge is driven by BM25 on that
  lowercased concatenation, not on the title alone.
- **Hybrid path silently disabled** — set `RUST_LOG=terraphim_agent=debug`
  and look for `"rolegraph write lock poisoned"` or
  `"shared_learning_store index contended"` warnings. The store does not
  fail the call; it falls back to BM25 and logs the reason.
- **Frontmatter will not parse** — older learnings may use PascalCase
  `source:` values (`BashHook` instead of `bash_hook`). The
  `parse_learning_source` helper accepts both, but a hand-edited file
  using an unknown source string will be silently downgraded to
  `AutoExtract`.
