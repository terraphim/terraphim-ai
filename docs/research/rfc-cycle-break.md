# RFC: Cycle Break for `terraphim_config ↔ terraphim_persistence ↔ terraphim_multi_agent`

Status: Draft (Phase 1 §7 stub). Decision: **extract `terraphim_persistence_traits`**. Alternative considered: `terraphim_agent_contracts` (rejected — see §Decision rationale).

## 1. Empirical edges (file-level)

Captured 2026-05-10 from `crates/*/Cargo.toml`:

| Edge | Source crate | Target crate | Manifest line |
|------|--------------|--------------|----------------|
| E1 | `terraphim_config` | `terraphim_persistence` | `crates/terraphim_config/Cargo.toml` (`terraphim_persistence = { path = "../terraphim_persistence", version = "1.0.0" }`) |
| E2 | `terraphim_config` | `terraphim_multi_agent` | `crates/terraphim_config/Cargo.toml` (`terraphim_multi_agent = { path = "../terraphim_multi_agent" }`) |
| E3 | `terraphim_persistence` | `terraphim_config` | `crates/terraphim_persistence/Cargo.toml` (`terraphim_config = { path = "../terraphim_config" }`) |
| E4 | `terraphim_persistence` | `terraphim_multi_agent` | `crates/terraphim_persistence/Cargo.toml` (`terraphim_multi_agent = { path = "../terraphim_multi_agent" }`) |
| E5 | `terraphim_multi_agent` | `terraphim_config` | `crates/terraphim_multi_agent/Cargo.toml` (`terraphim_config = { path = "../terraphim_config", features = ["openrouter"] }`) |
| E6 | `terraphim_multi_agent` | `terraphim_persistence` | `crates/terraphim_multi_agent/Cargo.toml` (`terraphim_persistence = { path = "../terraphim_persistence" }`) |

Six edges; all six pairs of the 3-clique are present. This is a fully-connected directed cycle, not a chain. Sentrux gate reports `cycle_count = 2` at file-level — the file-level decomposition is finer than the crate-level 3-clique we see in manifests; both views are valid.

## 2. Concrete usage (file:line) — to be filled in Phase 2 §15

(Phase 1 stub does not enumerate every `use` site. Phase 2 specification interview produces the full list of cross-clique imports per file before the trait surface is finalised.)

Inputs needed for Phase 2:
- `rg "use terraphim_persistence" crates/terraphim_config/src crates/terraphim_multi_agent/src`
- `rg "use terraphim_config" crates/terraphim_persistence/src crates/terraphim_multi_agent/src`
- `rg "use terraphim_multi_agent" crates/terraphim_config/src crates/terraphim_persistence/src`

## 3. Decision: extract `terraphim_persistence_traits`

A new crate `crates/terraphim_persistence_traits/` holds:

- `trait PersistenceProvider` — the dyn-safe interface persistence-consumers depend on
- `trait KeyValueStore` — narrower KV interface for config-only callers
- `trait ConfigSource` — the read side of config that multi_agent and persistence need without taking a `terraphim_config` dep
- Associated error type(s) — `thiserror`-derived
- All async methods declared `#[async_trait]`

Acyclic post-state:

```
terraphim_persistence_traits        (new, leaf — only depends on terraphim_types, async-trait, thiserror)
  ↑           ↑          ↑
terraphim_config   terraphim_multi_agent   terraphim_persistence
                                           (impl PersistenceProvider, KeyValueStore)
```

`terraphim_config` no longer depends on `terraphim_persistence` or `terraphim_multi_agent`. `terraphim_multi_agent` no longer depends on `terraphim_config` or `terraphim_persistence`. Both depend only on the traits crate. `terraphim_persistence` keeps its real implementation but drops its `terraphim_config` and `terraphim_multi_agent` deps in favour of the traits.

## 4. Alternative considered: `terraphim_agent_contracts` (rejected)

A second abstraction crate exclusively for agent–config contracts. Rejected because:
- The empirical cycle is centered on **persistence access**, not agent-contract registration. Edges E1, E3, E4, E6 are all persistence-related; only E2 and E5 involve multi_agent.
- A `terraphim_agent_contracts` crate would only break E2 + E5, leaving E1+E3 still cyclic via persistence ↔ config.
- Adding two new abstraction crates instead of one increases surface area without proportional benefit.

If post-extraction the residual config↔multi_agent edges (which are NOT cycles by themselves once persistence is symmetric to traits) prove problematic, agent_contracts can be added later as Stage A.2 — the persistence_traits cut does not preclude it.

## 5. Verification

After the extract-and-rewire PR:

```
sentrux gate .                            # cycle_count must drop to 0 (or 1 if a residual file-level loop remains; investigate any non-zero)
cargo build --workspace                   # green
cargo public-api diff terraphim_config    # only intentional removals (terraphim_persistence symbols)
cargo public-api diff terraphim_persistence  # only intentional removals
cargo public-api diff terraphim_multi_agent  # only intentional removals
```

Public-API diffs SHOULD show removals of the moved trait-bound types from the consumer crates' surfaces. Anything other than the planned removals/additions is a regression.

## 6. Open questions for Phase 2.5 specification interview

- Are `PersistenceProvider`, `KeyValueStore`, `ConfigSource` the right granularity, or should there be more (e.g. `SecretsProvider`)?
- Do the existing trait method signatures need redesign for object-safety, or are they already `dyn`-friendly?
- Feature-flag matrix: `multi_agent` currently uses `terraphim_config` with `features = ["openrouter"]` — does the trait need an OpenRouter-shaped extension trait?
- Where does the traits crate live: `terraphim-core` repo (Phase 2 §13) or its own `terraphim-persistence-traits` repo? — D5.

## 7. Out of scope for cycle-break

- Splitting `terraphim_persistence` itself into multiple backends (memory/sqlite/redb). The cycle break is orthogonal.
- Renaming any of the three crates. Names stable.
- Changing the persistence file format or schema.
