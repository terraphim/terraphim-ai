# Terraphim AI v1.21.3

Release date: 2026-08-16

## Highlights

This patch restores the extracted orchestrator integration on canonical `main` and hardens native agent execution.

### Fixed

- Re-included `terraphim_orchestrator` in the workspace and repointed its extracted dependencies to the canonical registry packages.
- Ensured an agent's configured working directory exists before spawning the child process.
- Removed native-CI Clippy blockers and closed the spawner stdin handoff race.
- Corrected the stale `author_is_agent` review test after its signature changed.

### Validation

- Dynamic-routing golden tests now assert that provider selection is data-driven rather than hard-coded.
- The release candidate is version-bound to `1.21.3`; the comprehensive tag workflow must verify `terraphim_server --version` metadata against `v1.21.3` before building artifacts.

## Source

Canonical source: `terraphim/terraphim-ai` on Gitea. The exact annotated tag is mirrored to GitHub; both release objects must resolve to the same commit.
