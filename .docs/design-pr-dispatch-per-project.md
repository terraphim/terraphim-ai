---
date: 2026-04-26
type: design
status: approved
owner: alex
parent_plan: ".docs/plan-adf-agents-replace-gitea-actions.md"
gitea_issue: terraphim/terraphim-ai#962
tags: [adf, orchestrator, pr-dispatch, multi-project]
---

# Design: Per-project `pr_dispatch` (move from `OrchestratorConfig` to `IncludeFragment`)

## Problem

Phase 2 (commits `8880bda6`, `dc27434f`) placed the `[pr_dispatch]` block on
`OrchestratorConfig` directly (`crates/terraphim_orchestrator/src/config.rs:198`).
The bigbox fleet now runs three projects -- `terraphim`, `odilo`, and
`digital-twins` -- each defined in its own `/opt/ai-dark-factory/conf.d/<project>.toml`
file pulled in via the top-level `include = [...]` glob. A single global
fan-out list cannot express different agents per project, and the operator's
attempt to put `[pr_dispatch]` in `conf.d/terraphim.toml` was rejected at
load: `IncludeFragment` uses `#[serde(deny_unknown_fields)]` and only accepts
`projects`, `agents`, `flows` (`config.rs:893-902`). Multi-project ADF
deployment is therefore blocked until the dispatch table can travel with
its project.

## Approach

Move the `pr_dispatch: Option<PrDispatchConfig>` field onto `IncludeFragment`.
Each `conf.d/<project>.toml` already declares the `[[projects]]` block(s)
it owns, so the natural rule is: **a `[pr_dispatch]` block in an include file
applies to every project declared in that same file.** After
`OrchestratorConfig::from_file` expands the include glob, walk every parsed
fragment and -- for each `(project, pr_dispatch)` pair -- insert into a new
`pr_dispatch_per_project: HashMap<String, PrDispatchConfig>` field on
`OrchestratorConfig`.

Edge cases:

- **Include declares `[pr_dispatch]` but no `[[projects]]`**: log a warn
  (`"pr_dispatch block in include without projects; ignored"`) and skip.
  No silent failure, no panic.
- **Include declares N `[[projects]]` plus one `[pr_dispatch]`**: same
  block applies to all N (single file, single intent).
- **Two include files declare the same project id**: load-time validation
  already rejects duplicate ids (`validate()`). Map insertion therefore
  cannot collide for valid configs.

## API change

Replace `OrchestratorConfig::agents_on_pr_open()` (config.rs:1039) with:

```rust
pub fn agents_on_pr_open_for_project(&self, project: &str) -> Vec<PrDispatchEntry>
```

Return type stays `Vec<PrDispatchEntry>` (cloned, like today). Keeps the
single call site in `handle_review_pr` (lib.rs:1826) a one-line change
(`self.config.agents_on_pr_open_for_project(&project)`) and avoids forcing
callers into `&PrDispatchConfig` -> `.agents_on_pr_open.iter()` reshaping.

**Lookup order** (deliberate cascade):

1. `pr_dispatch_per_project.get(project)` -- per-project block, first preference.
2. `pr_dispatch.as_ref()` -- top-level block, kept as **fallback for backward
   compat**. Existing `orchestrator.toml` files that declared `[pr_dispatch]`
   at the top level continue to work; that block now means "default for any
   project without its own block" instead of "global single-project setting".
3. `PrDispatchConfig::legacy_default()` -- the hard-coded `pr-reviewer`-only
   list, unchanged.

## Backward compatibility

- Configs with `[pr_dispatch]` at the top level: still parsed, still applied
  to every project that lacks its own block. Zero churn for existing
  `scripts/adf-setup/orchestrator.toml`.
- Configs with no `[pr_dispatch]` anywhere: fall through to `legacy_default()`.
  Zero churn.
- Configs that *only* set per-project blocks (the new bigbox shape):
  per-project block returned for projects in the map; `legacy_default()` for
  any project not declared (defensive).

## Test strategy (TDD per step)

Each implementation step ships its own test added before the production code.

**Step 1 -- IncludeFragment parses the new field**:
- `include_fragment_parses_pr_dispatch_block`: parse a small TOML containing
  `[[projects]]`, `[[agents]]`, `[pr_dispatch]` with two entries; assert
  `pr_dispatch.is_some()` and entries match.
- Doubles as a regression for the bigbox failure -- `deny_unknown_fields`
  must no longer reject this shape.

**Step 2 -- aggregate into the map**:
- `from_file_aggregates_pr_dispatch_from_includes`: minimal
  `orchestrator.toml` plus two include files (one per project) each with its
  own `[pr_dispatch]`; assert the resulting map has both project entries
  with the right contents.
- `from_file_warns_when_pr_dispatch_in_include_has_no_projects`: include
  file with `[pr_dispatch]` but no `[[projects]]`; assert the load succeeds
  and the map is empty (warn is logged, not asserted on).

**Step 3 -- lookup precedence**:
- `agents_on_pr_open_for_project_returns_per_project_block`
- `agents_on_pr_open_for_project_falls_back_to_top_level_block`
- `agents_on_pr_open_for_project_falls_back_to_legacy_default`

**Step 4 -- migrate one Phase 2 helper**:
- Rebuild `review_pr_config_with_fanout` to populate
  `pr_dispatch_per_project` keyed by `"alpha"` (the project id used by all
  Phase 2 fixtures -- not `"terraphim"` as the brief assumed; verified
  lib.rs:7660, :7708, :7941). The other Phase 2 tests
  (`handle_review_pr_skips_missing_agents`,
  `handle_review_pr_pending_status_posted_per_agent`,
  `handle_review_pr_skipped_agent_does_not_post_pending`) keep using
  `config.pr_dispatch = Some(...)` -- this exercises the top-level fallback
  path, providing backward-compat coverage for free.
- All existing Phase 2 fan-out assertions must still pass.

## Phase 2c/d/e PR follow-up

Phase 2c (#956), Phase 2d (#958), and Phase 2e (replacement of #957) live
on PR branches that build their test helpers against
`OrchestratorConfig::agents_on_pr_open()`. Migrating them inside *this*
change would create a three-way merge conflict. **Decision**: don't
touch them. The new `agents_on_pr_open_for_project(project)` API plus
the top-level fallback means those PRs continue to compile and pass --
their tests just route through the fallback. The follow-up rebase is a
trivial rename in their fixtures once this lands.

## Plan deltas

`.docs/plan-adf-agents-replace-gitea-actions.md`:

- Add `D5` to the locked-decisions table:
  *"`pr_dispatch` is per-project (declared inside `IncludeFragment`).
  Top-level block kept as fallback."*
- §5 Phase 2: clarify that `[pr_dispatch]` lives in
  `conf.d/<project>.toml`, not `orchestrator.toml`.
- §13: remove the multi-project line from "out of scope"; add a
  forward-pointer to this design.

## Out of scope

- Webhook payload extension to carry the project id (already present;
  `DispatchTask::ReviewPr.project` flows from the webhook handler).
- Renaming any agent template or status-check context -- pure config
  + dispatch wiring change.
- Touching Phase 1 (`set_commit_status`), Phase 3 (`webhook.rs`,
  `handle_push`), or any agent template `.toml`.
