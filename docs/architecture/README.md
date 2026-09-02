# Architecture

Crate-level architecture, polyrepo topology, and design notes for the
Terraphim stack.

## Crate / sub-system explainers

- [`terraphim-agent`](terraphim-agent.md) — the `terraphim_agent`
  CLI / library, covering the `learnings` capture pipeline, the
  `shared_learning` cross-agent store, the 2026-09 hybrid-scoring
  refactor, the markdown storage layout, and the trust-level promotion
  rules.
- [`polyrepo-topology.md`](polyrepo-topology.md) — the layer map and
  cross-repo dependency direction after the polyrepo split (Gitea
  #1910).

## ADRs

- [`adr/0002-polyrepo-github-publish-pipeline.md`](adr/0002-polyrepo-github-publish-pipeline.md)

## Design / dependency / metric data

- [`dsm/`](dsm/) — workspace dependency graphs, reverse-dep reports,
  test index, and metric snapshots used during the dependency
  topology work.
