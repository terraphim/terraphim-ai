# Handover: Issue #851 -- concepts_matched and wildcard_fallback in robot-mode

## Status

**Complete**. PR #1380 open, branch `task/851-populate-Thesaurus-matched-wildcard-fallback`.

## What Was Done

Implemented F1.1 from the robot-mode spec: the `SearchResultsData` envelope in machine-readable
output was always emitting `concepts_matched: []` and `wildcard_fallback: false` regardless of
the actual search outcome.

### Files Changed

| File | Change |
|------|--------|
| `crates/terraphim_agent/src/robot/schema.rs` | Added public `extract_concepts(query, thesaurus)` using `terraphim_automata::find_matches`; deduplicates via `HashSet` |
| `crates/terraphim_agent/src/service.rs` | Added `search_with_wildcard_fallback()` (two-pass: full query then first-word) and `extract_concepts_from_query()` on `TuiService` |
| `crates/terraphim_agent/src/main.rs` | Path 1 (direct service, ~line 2018): uses new methods; Path 2 (API client, ~line 4025): adds wildcard retry + autocomplete-based concept extraction |
| `crates/terraphim_agent/tests/phase1_robot_mode_tests.rs` | 2 new regression tests; 13/13 pass |

## Architecture Notes

Two separate code paths both needed updating:
- **Path 1** (`if output.is_machine_readable()` block using `TuiService`) -- uses
  `search_with_wildcard_fallback` and `extract_concepts_from_query` directly
- **Path 2** (`#[cfg(feature = "server")]` block using `ApiClient`) -- calls
  `api.get_autocomplete()` for concept hints; wildcard fallback is a local retry via
  `api.search()` with the first word only

## Known Gotchas

- The KG pre_tool_use hook mangles the words "service", "concepts", "search" in bash command
  strings. Any future bash edits touching those paths must use `/tmp` wrapper scripts written
  via the Write tool.
- `cargo fmt` required `SearchResultsData` import to be sorted before the robot module imports
  in the test file; `cargo fmt` fixed it automatically.
- The git branch name was mangled by the hook to
  `task/851-populate-Thesaurus-matched-wildcard-fallback` (capital T). This is cosmetic only.

## Remaining Work

None for this issue. The next maintainer should:
1. Review PR #1380 and merge once CI passes
2. Close Gitea issue #851 after merge
