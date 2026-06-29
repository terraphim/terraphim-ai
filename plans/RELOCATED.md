# Plans — Relocated After Polyrepo Extraction (#1910)

All specs that previously lived in this directory have been archived here after the
polyrepo extraction (Gitea #1910). The features they describe are **fully implemented**
in the appropriate downstream repositories; the acceptance criteria pass against the
vendored registry crates (v1.20.x).

## Where Each Spec Now Lives

| Spec | Implemented in | Status |
|------|---------------|--------|
| `design-gitea84-trigger-based-retrieval.md` | `terraphim-core` — `terraphim_automata` + `terraphim_rolegraph` v1.20.4 | PASS (verified 2026-06-25) |
| `design-gitea82-correction-event.md` | `terraphim-agents` — `terraphim_agent::learnings::capture` v1.20.4 | PASS |
| `learning-correction-system-plan.md` | `terraphim-agents` — `terraphim_agent::learnings` v1.20.4 | PASS |
| `d3-session-auto-capture-plan.md` | `terraphim-agents` — `terraphim_agent::learnings::procedure` v1.20.4 | PASS |
| `design-single-agent-listener.md` | `terraphim-agents` — operational, no Rust code changes | N/A |
| `research-single-agent-listener.md` | Research document — implemented operationally | N/A |

## Archive

The original spec files are preserved in `plans/archive/polyrepo-extracted/` for
historical reference. The spec-validator for `terraphim-ai` should not attempt to
validate these specs against local source — the relevant crates no longer reside in
this repository.

## Spec Validation Scope

For this repository, spec validation applies only to the 13 active workspace crates:
`terraphim_dsm`, `terraphim_lsp`, `terraphim_merge_coordinator`, `terraphim_rlm`,
`terraphim_server`, `terraphim_spawner`, `terraphim_tinyclaw`, `terraphim_update`,
`terraphim_validation`, `terraphim_weather_report`, `terraphim_workspace`,
`terraphim-firecracker`, `terraphim_ai_nodejs`.

Future specs for these crates should live directly under `plans/` with ACs referencing
paths in the crates listed above.

**Refs: Gitea #2972, #1910**
