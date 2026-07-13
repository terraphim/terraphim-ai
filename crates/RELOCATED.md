# Crates — Relocated After Polyrepo Extraction (#1910)

This index records source crates that previously lived in this repository
(`terraphim-ai`) but were extracted to standalone polyrepos during the #1910
decomposition. They are consumed from the `terraphim` registry; **no in-repo
copy remains**. Spec-validation and search tooling in this repo must not
attempt to resolve references to these paths against local source.

## terraphim_orchestrator

**Status**: Removed from this repo (Gitea #3090, 2026-07-13). The 62k-LOC
source restored by commit `2f276886c` was a divergent, unbuildable stale
fork and has been deleted.

**True home**: `terraphim-agents` polyrepo —
`/home/alex/projects/terraphim/terraphim-agents/crates/terraphim_orchestrator`

**Why removed**: The restored copy declared path-deps on six crates
(`terraphim_router`, `terraphim_types`, `terraphim_tracker`,
`terraphim_persistence`, `terraphim_automata`, `terraphim_agent_evolution`)
that were themselves extracted to polyrepos in #1910 and no longer exist in
this repo. Consequently it could not build (`cargo metadata` failed: "current
package believes it's in a workspace when it's not"), and the workspace
`Cargo.toml` had to exclude it — producing the zombie documented in #3090.

**Deployed binary provenance**: The `/usr/local/bin/adf` binary is built from
the `terraphim-agents` repo, not this one (AGENTS.md rule #3). The installed
binary hash matches the `terraphim-agents` build artefact
(`target/release/deps/adf-b7d747a1c218d613`), confirming the polyrepo is the
proven deploy source.

**Consumers**: The only in-repo consumer, `terraphim_weather_report`, depends
on `terraphim_orchestrator` via the `terraphim` registry
(`registry = "terraphim"`), not a local path-dep. No build or test path in
this repo references the deleted source.

**Fidelity check (pre-deletion)**: Every source file in the deleted copy
exists in the polyrepo except `agent_allowlist_kg.rs`, which is tracked
separately via `terraphim-agents#70` (Refs #3024). The polyrepo is strictly
newer — it contains `meta_coordinator`, `run_synthetic_with_findings`,
`pr_review/extractor.rs`, and `pr_review/poster.rs` that the in-repo copy
lacked.

## Earlier extractions (already absent from this repo)

These crates were extracted in earlier waves of #1910 and are listed only for
completeness. See `plans/RELOCATED.md` for spec-status details.

| Crate family | Polyrepo home | Registry |
|---|---|---|
| terraphim_types, terraphim_automata, terraphim_rolegraph, terraphim_test_utils, terraphim-markdown-parser | `terraphim-core` | `terraphim` |
| terraphim_config, terraphim_config_persistence | `terraphim-config-persistence` | `terraphim` |
| terraphim_service (+ sub-crates) | `terraphim-service` | `terraphim` |
| terraphim_agent (+ sub-crates) | `terraphim-agents` | `terraphim` |
| terraphim_kg_agents (+ sub-crates) | `terraphim-kg-agents` | `terraphim` |

**Refs: Gitea #3090, #3024, #3030, #1910, #2972**
