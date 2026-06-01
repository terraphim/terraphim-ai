# Aggregator decoupling sketch (Stage B, Gitea #1910)

Design sketch only -- no code changes in Stage B. Targets the two fan-out hubs the DSM flags as
modularity drags. Both feed the polyrepo split: today each hub depends directly on every crate it
dispatches to, so they cannot be cut into a separate repo without dragging all providers with them.

Measured intra-workspace dependencies (`cargo metadata`, post-Stage-A):

- `terraphim_middleware` (normal): `terraphim_types`, `terraphim_config`, `terraphim_automata`,
  `terraphim_rolegraph`, `terraphim_persistence`, `terraphim_file_search`, `terraphim-session-analyzer`,
  `haystack_jmap`.
- `terraphim_service` (normal): `terraphim_types`, `terraphim_config`, `terraphim_automata`,
  `terraphim_rolegraph`, `terraphim_persistence`, `terraphim_middleware`, `terraphim_router`.

## 1. `terraphim_middleware` -- haystack provider dispatch

### Current coupling

`crates/terraphim_middleware/src/indexer/mod.rs` defines `trait IndexMiddleware` (the right
abstraction) but the orchestrator dispatches with a hardcoded `match ServiceType { ... }` over 11
arms: `Ripgrep`, `Atomic`, `QueryRs`, `ClickUp`, `Mcp`, `Perplexity`, `GrepApp`, `AiAssistant`,
`Quickwit`, `Jmap`. Each arm constructs a concrete provider, so the match site (and `Cargo.toml`)
gains a hard dependency on every provider crate. Adding or removing a haystack means editing
middleware's `match`, its imports, and its manifest -- the textbook open/closed violation, and the
reason middleware fans in/out so widely.

### Target seam: provider trait in `haystack_core` + a registry

1. Move (or confirm) `trait IndexMiddleware` into `haystack_core` so providers depend on the trait,
   not on `terraphim_middleware`. The trait stays dyn-safe:
   `async fn index(&self, needle: &str, haystack: &Haystack) -> Result<Index>` behind `#[async_trait]`.
2. Each provider crate (`haystack_jmap`, the ripgrep/atomic/clickup/quickwit/etc. indexers) implements
   `IndexMiddleware` for its own type and exposes a constructor + a `ServiceType` tag.
3. Replace the `match` with a `HashMap<ServiceType, Arc<dyn IndexMiddleware>>` registry built once at
   startup. `search_haystacks` looks the provider up by the haystack's `service` field.
4. The **binary** (`terraphim_server` / clients) owns registration -- it depends on the provider crates
   and inserts them into the registry. `terraphim_middleware` then depends only on `haystack_core` +
   the domain crates (`types`, `config`, `rolegraph`, `automata`, `persistence`), not on any provider.

### Effect

- middleware's provider fan-out drops from ~11 to 0; providers become leaves under `haystack_core`.
- New haystacks register without touching middleware (open for extension, closed for modification).
- Feature-gating per provider moves to the composition root, shrinking default build graph.
- Enables cutting `terraphim_middleware` and the haystack providers into separate repos cleanly.

### Constraints

- `search_haystacks` is consumed by `terraphim_service` (lib.rs:1540) and `build_thesaurus_from_haystack`
  (lib.rs:11). The registry must be threaded through `ConfigState` (already cloned into the call) so the
  public `search_haystacks` signature is preserved or extended additively (changelog if changed).
- Keep `ServiceType` in `terraphim_config`/`terraphim_types` as the registry key to avoid a new enum.

## 2. `terraphim_service` -- capability aggregation

### Current coupling

`terraphim_service/src/lib.rs` is the god file (~3876 lines) and aggregates search + KG/thesaurus +
document + LLM-chat behind one type, pulling `middleware`, `router`, and the four domain crates. It is
the E3 extraction target and the largest single contributor to `complex_fn_count`.

### Target seam: capability traits, lib.rs becomes facade

1. Define capability traits (search / KG-thesaurus / document / LLM-chat) -- aligns with
   architecture-improvement-plan Phase 2 and ADR-002.
2. Move each capability's implementation into its own module/crate; `lib.rs` becomes a thin facade that
   wires capabilities and exposes the existing public API unchanged.
3. The LLM-chat capability depends on `terraphim_router`; isolating it lets the router dependency be
   feature-gated and keeps it out of the search hot path.

### Effect

- `god_file_count` 1 -> 0 (a Stage E0b / DoD `quality_signal` driver).
- Each capability becomes independently testable and the `multi_agent -> service -> config` edge stops
  being tangled through one monolith.
- Precondition for the clean `terraphim-service` repo extraction (E3).

### Constraints

- `terraphim_service` carries no frozen public API itself, but it re-exports/consumes
  `terraphim_types` (frozen). Decomposition must keep the facade's public surface stable; run
  `cargo public-api -p terraphim_service diff` before/after to confirm.

## Sequencing

Both sketches are **MUST-PRECEDE** their respective Stage E extractions (middleware/providers before
the haystack repos; service before E3) and should land while still in the single workspace, where the
trait moves and registry wiring are a single reviewable change rather than a cross-repo migration.
