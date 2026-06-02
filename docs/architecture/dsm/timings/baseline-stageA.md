# Cold-build timing baseline -- Stage A (Gitea #1910)

D7 gate denominator. The full `cargo --timings` HTML (`baseline-stageA.html`, ~850 KB) is a generated,
regenerable artefact ignored by the repo's `*.html` rule; the headline figures are recorded here so the
gate has a tracked reference.

## Run

- **Command**: `cargo clean && cargo build --workspace --timings --release`
- **State**: post-Stage-A (manifest cycle broken, `terraphim_lsp` revived) on branch
  `task/1910-stage-a-cycle-break-lsp`.
- **Host**: developer macOS (Apple Silicon). Single run.
- **Wall time**: **3m 32s** (`Finished release profile in 3m 32s`).
- **Compilation units**: 1048.

> Note: this is a single run on a dev Mac with a warm `~/.cargo` registry cache (only `target/` was
> cleaned). The authoritative D7 denominator is a **median-of-3 cold build on the bigbox self-hosted
> runner**, to be captured in CI (`sentrux-quality-gate.yml` / `performance-benchmarking.yml`). Use this
> figure only as a local sanity reference, not the gate baseline.

## Top 15 slowest compilation units

| Duration | Unit | Kind |
|---------:|------|------|
| 87.7s | aws-lc-sys | build-script |
| 57.7s | terraphim_orchestrator | codegen |
| 52.5s | libgit2-sys | build-script |
| 50.0s | terraphim_tinyclaw | codegen |
| 41.3s | terraphim_tinyclaw | codegen |
| 38.3s | terraphim_server | codegen |
| 37.5s | image | codegen |
| 37.5s | terraphim_agent | codegen |
| 33.8s | terraphim_mcp_server | codegen |
| 33.6s | terraphim_server | codegen |
| 31.2s | terraphim_orchestrator | codegen |
| 29.9s | terraphim_agent_evolution | codegen |
| 29.6s | libsqlite3-sys | build-script |
| 29.2s | zstd-sys | build-script |
| 27.0s | fff-search | codegen |

## Observations for the split

- Native build-scripts dominate cold cost (`aws-lc-sys`, `libgit2-sys`, `libsqlite3-sys`, `zstd-sys` ~=
  200s combined). Per-repo builds that do not pull these (e.g. `terraphim-core`) should be markedly
  faster -- a key expected win from the split, and the reason the D7 cold-build gate is the tightest.
- The heaviest first-party units (`terraphim_orchestrator`, `terraphim_tinyclaw`, `terraphim_server`,
  `terraphim_agent`, `terraphim_mcp_server`) sit in the app/adapter and agent layers -- consistent with
  the extraction order placing `terraphim-core` first (cheap, foundational) and the composition roots
  last.
