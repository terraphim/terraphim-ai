# Research Document: CLI Bugs #920, #921, #922

**Status**: Approved
**Date**: 2026-04-26

## Executive Summary

Three CLI bugs share a common theme: commands fail when expected resources are unavailable. Bugs #920 and #922 both stem from the knowledge graph / thesaurus not being configured or loadable. Bug #921 is an independent GitHub API rate limiting issue in the update mechanism.

## Bug #920: suggest, validate, extract fail with "Knowledge graph not configured"

### Root Cause
`ensure_thesaurus_loaded()` in `crates/terraphim_service/src/lib.rs:498-501` returns a hard error when the role has no `kg` field. The call chain is:
- `main.rs` Command handler -> `service.get_thesaurus(role_name)` -> `service.ensure_thesaurus_loaded(role_name)` -> checks `role.kg` is `Some` -> returns `Err("Knowledge graph not configured")` when `None`.

### Code Locations
| Component | Location | Purpose |
|-----------|----------|---------|
| Command handlers | `main.rs:1783-2050` | Extract, Replace, Validate, Suggest offline handlers |
| TuiService | `service.rs:340-342` | `get_thesaurus()` delegates to `ensure_thesaurus_loaded` |
| Thesaurus loading | `terraphim_service/src/lib.rs:139-503` | `ensure_thesaurus_loaded` - loads from automata_path, local KG, or fails |
| Error origin | `terraphim_service/src/lib.rs:498-501` | `"Knowledge graph not configured"` when `role.kg` is `None` |

### Impact
- Users with roles that lack KG configuration cannot use suggest, validate, extract commands
- No graceful degradation - hard error with no helpful message

### Assumption
Roles may legitimately have no KG (e.g., newly created roles). Commands should degrade gracefully: return empty results rather than hard errors.

## Bug #922: replace command fails to load thesaurus with NotFound error

### Root Cause
Same as #920 - `get_thesaurus()` fails. The `--fail-open` flag IS handled correctly (lines 1836-1848), so the command does pass through unchanged text. However, the underlying error message about `thesaurus_imported.json` is confusing and undocumented.

### Code Locations
| Component | Location | Purpose |
|-----------|----------|---------|
| Replace handler (offline) | `main.rs:1806-1926` | Handles fail-open correctly |
| Thesaurus persistence | `terraphim_types` Thesaurus `save()`/`load()` | Uses `thesaurus_imported.json` filename |
| Server-mode Replace | `main.rs:3837-3870` | Only works with fail-open in server mode |

### Impact
- Replace works with `--fail-open` but shows confusing NotFound errors
- Without `--fail-open`, hard error for roles without KG

## Bug #921: check-update and update commands fail with GitHub API 403

### Root Cause
`check_for_updates_auto()` in `crates/terraphim_update/src/lib.rs:943-991` uses `self_update::backends::github::Update::configure()` without setting a GitHub API token. Unauthenticated GitHub API requests are rate-limited to 60/hour.

Additionally, the startup update check at `main.rs:1211-1215` prints errors to stderr on every invocation, which is noisy.

### Code Locations
| Component | Location | Purpose |
|-----------|----------|---------|
| Update check | `terraphim_update/src/lib.rs:943-991` | `check_for_updates_auto` - no auth token |
| Startup check | `main.rs:1211-1215` | `check_for_updates_startup` - prints error on failure |
| Offline CheckUpdate | `main.rs:1478-1490` | `check_for_updates` - returns hard error |
| Offline Update | `main.rs:1493-1505` | `update_binary` - returns hard error |
| Server CheckUpdate | `main.rs:3811-3822` | Same pattern in server mode |
| Server Update | `main.rs:3824-3835` | Same pattern in server mode |

### Impact
- Users behind shared IPs hit rate limit quickly
- Startup check prints error on every invocation (noisy)
- No guidance on how to fix (set GITHUB_TOKEN)

## Common Constraints

### Technical
- No new external dependencies
- Must not break existing KG-configured roles
- Must handle both offline and server mode command paths

### Vital Few
1. Graceful degradation for roles without KG (#920, #922)
2. GitHub token support for update checks (#921)
3. Better error messages for all three bugs

## Recommendations

### #920 + #922 Fix
Return an empty thesaurus when role has no KG configured, instead of a hard error. This lets suggest/validate/extract/replace return "no results" naturally rather than crashing. For `--fail-open`, suppress the confusing NotFound error entirely.

### #921 Fix
1. Support `GITHUB_TOKEN` env var in the update check for authenticated API access
2. Downgrade the startup check error from stderr to debug logging
3. Improve the 403 error message to explain rate limiting and suggest setting GITHUB_TOKEN
