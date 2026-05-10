# Phase 1 §1 — Tooling Inventory

Captured: 2026-05-10. Workspace root: `/home/alex/projects/terraphim/terraphim-ai`.

## Installed and verified

| Tool | Version | Role in this analysis |
|------|---------|----------------------|
| `sentrux` | 0.5.7 | Primary architectural sensor — 5 root-cause metrics + `gate` baseline |
| `cargo` | 1.91.0 (ea2d97820 2025-10-10) | Workspace metadata, tree, build, test |
| `rustc` | 1.91.0 (f8297e351 2025-10-28) | Compiler (workspace MSRV pinned to 1.80 in `clippy.toml`) |
| `cargo-depgraph` | 1.6.0 | Workspace dependency DOT graph |
| `cargo-hakari` | 0.9.29 | (Pending Stage A adoption) workspace-hack for unified feature resolution |
| `cargo-machete` | 0.6.0 | Unused-dependency detection |
| `cargo-udeps` | (no `--version` flag) | Unused-deps in nightly mode |
| `terraphim_dsm` | path-member of workspace | Sentrux companion: semantic cluster labelling via Terraphim KG |
| `jq` | available | JSON post-processing of `cargo metadata` output |
| `rg` | available | Code grepping for cross-boundary inventories |

## Required, install in progress

| Tool | Reason | Install command (used) |
|------|--------|------------------------|
| `cargo-public-api` | API surface freeze for already-published crates | `cargo install cargo-public-api --locked` (running) |
| `cargo-modules` | Per-crate API map for aggregator decoupling | `cargo install cargo-modules --locked` (running) |
| `tokei` | LOC counts per crate | `cargo install tokei --locked` (running) |

## Not installed; Phase 1 cannot proceed without these

(none — all Phase-1 MUST tooling is on hand or installing)

## Notes

- `dot` (Graphviz) is not on this machine. SVG rendering of `depgraph-workspace.dot` is deferred; DOT files are sufficient for textual analysis. Add `apt install graphviz` to the team's dev-environment bootstrap if visual graphs are required.
- `cargo-public-api` and `cargo-modules` failed to install without `--locked` because their dependencies require Rust 1.93 while the toolchain here is 1.91. With `--locked` they pull versions compatible with 1.91.
- `terraphim_dsm` is a workspace member (`crates/terraphim_dsm/`) and has no separate install — invoked via `cargo run -p terraphim_dsm`.

## Sentrux baseline (Phase 1 §2)

Saved at `.sentrux/baseline.json`. Numbers (2026-05-10):

| Metric | Value | Notes |
|--------|-------|-------|
| `quality_signal` | 0.5249 (5249 / 10000) | Geometric mean of 5 root-cause metrics |
| `coupling_score` | 0.0663 | Lower = better |
| `cycle_count` | **2** | Brief expected the 3-clique to surface as 1-3 SCCs; sentrux sees 2 file-level SCCs. The empirically-confirmed 3-way crate-level cycle (`config↔persistence↔multi_agent`) is real (see §`reverse-deps/`) — sentrux's 2 may reflect file-grain SCC partitioning rather than a contradiction. |
| `god_file_count` | 1 | Single file flagged as a "god file" — investigated in aggregator decoupling sketch |
| `hotspot_count` | 0 | Sentrux's hotspot definition is per-file; per-crate fan-in (35-39 for `terraphim_types`) is invisible to this metric |
| `complex_fn_count` | 388 | Functions over the cyclomatic-complexity threshold |
| `max_depth` | 7 | Longest dependency chain in the DAG |
| `total_import_edges` | 1041 | After dedup |
| `cross_module_edges` | 774 | 74% of edges cross module boundaries |

The full sentrux 5-metric breakdown (modularity_q, complexity_gini, redundancy_ratio individually) is not in the gate output of 0.5.7; it is folded into `quality_signal`. Use `sentrux scan` GUI or `sentrux mcp` for per-metric drill-down if/when needed.
