# Archived Plans

These plans were valid at the time of authoring but became stale after the **polyrepo extraction (Gitea #1910, closed)** relocated the referenced crates to standalone repositories.

## Relocation Map

| Original path (terraphim-ai) | New home |
|------------------------------|----------|
| `crates/terraphim_agent/src/learnings/` | `terraphim-agents/crates/terraphim_agent/src/learnings/` |
| `crates/terraphim_automata/src/` | `terraphim-core/crates/terraphim_automata/src/` |
| `crates/terraphim_rolegraph/src/` | `terraphim-core/crates/terraphim_rolegraph/src/` |
| `crates/terraphim_types/src/` | `terraphim-core/crates/terraphim_types/src/` |

## Verification Status (as of 2026-06-21)

All four prescriptive specs covered by these plans were verified IMPLEMENTED in the polyrepo homes:

- **gitea82** CorrectionEvent: `terraphim-agents/crates/terraphim_agent/src/learnings/capture.rs` (CorrectionEvent@502)
- **gitea84** trigger/pinned/TF-IDF: `terraphim-core/crates/terraphim_types/src/lib.rs:625` + `terraphim-core/crates/terraphim_rolegraph/src/lib.rs` (find_matching_node_ids_with_fallback)
- **d3** FromSession: `terraphim-clients/crates/terraphim_agent/src/main.rs:1213,3590`
- **listener**: operational-only, no code verification required

## Why Archived

The parent issues (#82, #84, #693, #1910) are all closed. The plans in this directory caused repeated false positives in spec-validation cycles because the path references no longer exist in this repository.

See issue #2855 for full context.
