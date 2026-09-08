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

## terraphim_tinyclaw

**Status**: Removed from this repo (Gitea #3362, 2026-09-07).

**True home**: `terraphim-tinyclaw` polyrepo —
<https://git.terraphim.cloud/terraphim/terraphim-tinyclaw>.

**Extraction coordinates** (pinned so this deletion stays auditable after the
destination repo moves on):

| | |
|---|---|
| Source removal commit (this repo) | `313b6213d` |
| Destination repo | <https://git.terraphim.cloud/terraphim/terraphim-tinyclaw> |
| Destination commit at extraction | `e52631f` (crate made standalone) |
| Destination root commit | `461361c4c` |
| Extraction command | `git filter-repo --path crates/terraphim_tinyclaw --path-rename crates/terraphim_tinyclaw/:` |

To verify the deleted source is recoverable:

```bash
git clone https://git.terraphim.cloud/terraphim/terraphim-tinyclaw
cd terraphim-tinyclaw
git checkout e52631f
cargo check --lib --bins
```

Note that the destination has since advanced beyond the extraction commit: its
`main` now carries `239bf3d`, which restores roughly 4,600 lines that a bad
merge (`244c38b47` in this repo) had reverted before the extraction. Check out
the pinned commit above to see the crate exactly as it left this repository,
and `main` to see it repaired.

**Why removed**: It was a member of this workspace only because
`members = ["crates/*"]` is a glob and it was absent from `exclude` — not
because anything here used it. Nothing in this workspace depended on it (the
sole reference was a comment in the root `Cargo.toml`). Meanwhile four of its
integration tests referenced five methods that do not exist in the crate
(`JsonlBackend::from_shared`, `ToolCallingLoop::with_backend`,
`AcpState::with_bus`, `ProxyState::with_agent_bus`,
`TinyClawMcpServer::with_commands`), so
`cargo check --workspace --all-targets` failed — and because the pre-commit
hook runs exactly that, **no commit anywhere in this repo could pass the hook**.
Verified that it was the sole blocker: `cargo check --workspace --all-targets
--exclude terraphim_tinyclaw` exited 0.

It also already behaved like a polyrepo crate: four dependencies via
`registry = "terraphim"`, and a path dep reaching outside the repository
(`../../../terraphim-service/crates/haystack_jmap`). That relative path
additionally broke `cargo metadata` in every git worktree, blocking all cargo
commands there for unrelated crates (#3365).

**Extraction method**: `git filter-repo --path crates/terraphim_tinyclaw
--path-rename crates/terraphim_tinyclaw/:` against a throwaway clone,
preserving all 193 commits that touched the crate. (`git subtree split` walks
all 4,238 repo commits and did not finish; filter-repo took under six seconds.)

**Published to support the extraction** (its path deps had to become registry
deps):

| Crate | Version | Note |
|---|---|---|
| `terraphim_engine_events` | 0.1.0 | newly published |
| `terraphim_rlm` | 1.21.3 | newly published |
| `terraphim-firecracker` | 1.21.3 | newly published; required because cargo resolves optional deps at package time |
| `terraphim_spawner` | 1.22.0 | already published |

**Known outstanding**: `haystack_jmap` is still a path dep in the extracted
repo (`../terraphim-service/crates/haystack_jmap`) because it is not published;
publishing it requires publishing `haystack_core` first, both of which live in
the `terraphim-service` repo. Building terraphim-tinyclaw therefore still
requires a sibling checkout. The five missing APIs above travel with the crate
and remain unfixed — extraction stopped them blocking this workspace, it did
not repair them.

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
