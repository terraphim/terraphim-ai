# Structural Review Handoff: PR #2628

## PR Under Review

- **Repo**: `terraphim/terraphim-ai` on `git.terraphim.cloud`
- **PR**: [#2628](https://git.terraphim.cloud/terraphim/terraphim-ai/pulls/2628)
- **Title**: Fix #1715: use per-project tracker for PR commit statuses
- **Head**: `46900c28ea3a662f951f865cbc34b94dc745a853`
- **Base**: `7ab758dab1963c2fdfc6d19ad4877ac35de8ffed`
- **Author**: root
- **Stats**: +4,603 / -444, 48 changed files (Gitea reports; local diff has 6,323 lines)

## Scope Mismatch Already Noted

The PR title/body describe a narrow bug fix for `post_pending_status` / `post_terminal_commit_status` posting commit statuses to the wrong repo. The actual diff is a **large feature batch** that appears to introduce or heavily modify the `terraphim_orchestrator` crate, add `project_sources` loading, telemetry/weather-report design docs, TLA+ specs, spawner changes, and many new tests. This discrepancy should be called out in the final review and may affect confidence scoring.

## What Has Been Done

1. Loaded the `structural-pr-review` skill.
2. Fetched PR metadata and the full diff from Gitea into:
   - `.tmp_review/pr2628.diff`
   - `.tmp_review/pr2628_files.json`
3. Downloaded every changed file at **head** into `.tmp_review/head/` (full tree).
4. Attempted to download **base** versions; most `crates/terraphim_orchestrator/*` files do not exist at base (`7ab758d`), confirming the crate is effectively new in this PR. Base versions that exist are in `.tmp_review/base/`.

## What Remains

1. Read every substantive code file in `.tmp_review/head/` (especially the orchestrator crate, `terraphim_config/src/project.rs`, `terraphim_spawner`, and test files).
2. Compare against base versions in `.tmp_review/base/` where available; for new files, the diff is the full file.
3. Trace data flow for:
   - `project_sources` → `OrchestratorConfig` → agent registry
   - webhook commit-status posting (`post_pending_status`, `post_terminal_commit_status`)
   - telemetry/weather reporting
   - provider probe / gate logic
4. Apply the structural review dimensions (security/data exposure, API contracts/error handling, runtime/platform awareness, performance/concurrency, type safety, maintainability, cross-file consistency).
5. Produce the final review comment using the skill template: Summary, Confidence Score (1-5), Important Files Changed, Mermaid diagram, P0/P1/P2 findings, optional Comments Outside Diff, footer with reviewed commit hash.
6. Optionally post the review via `tea comment` / `gtr comment` on Gitea.

## Next-Agent Starting Position

- All source material is in `.tmp_review/`.
- Start by reading `.tmp_review/pr2628_files.json` to see status and paths, then read the head files.
- Key logic files to prioritise:
  - `crates/terraphim_orchestrator/src/config.rs`
  - `crates/terraphim_orchestrator/src/webhook.rs`
  - `crates/terraphim_orchestrator/src/project_adf.rs`
  - `crates/terraphim_orchestrator/src/agent_registry.rs`
  - `crates/terraphim_orchestrator/src/lib.rs`
  - `crates/terraphim_orchestrator/src/bin/adf.rs`
  - `crates/terraphim_orchestrator/src/control_plane/telemetry.rs`
  - `crates/terraphim_orchestrator/src/dual_mode.rs`
  - `crates/terraphim_orchestrator/src/provider_probe.rs`
  - `crates/terraphim_config/src/project.rs`
  - `crates/terraphim_spawner/src/{config,lib,output}.rs`
  - new/updated test files under `crates/terraphim_orchestrator/tests/`
- Run `cargo build --workspace` and relevant tests before finalising the review.

## Local Notes

- The local repository has unrelated uncommitted changes (`BUILD.md`, `.docs/design-pr-1951-doc-gaps-merge.md`, `.docs/research-pr-1951-doc-gaps-merge.md`, `.docs/research-release-blockers.md`, `.terraphim/flow-state/`, `.terraphim/learnings/`, `session-ses_156a.md`, `session-ses_167f.md`). These were **not** staged or committed as part of this handoff.
- Several local refs point to missing objects (`refs/heads/css_grid_modern`, etc.), which can cause `git fetch` failures. Use Gitea API or raw URLs for file access if needed.
