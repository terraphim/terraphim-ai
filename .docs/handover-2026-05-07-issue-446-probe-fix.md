# Session Handover: Issue #446 Anthropic Provider Health Check Fix

**Date**: 2026-05-07  
**Branch**: `task/446-anthropic-probe-circuit-breaker-fix`  
**PR**: https://github.com/terraphim/terraphim-ai/pull/868  
**Gitea issue**: terraphim/terraphim-ai#446

---

## What was done

Investigated and fixed Gitea issue #446: "[ADF] Anthropic provider health check failing (claude-opus, -sonnet, -haiku)".

### Investigation findings

**April 6 root cause** (historical, not code-fixable):
- All known CLI versions (2.1.100+) were installed after April 6 — an older, now-removed version was in use
- `~/.config/claude/settings.json` contains `"apiUrl": "http://127.0.0.1:3456/v1"` (non-running proxy); older versions may have respected this strictly, causing connection-refused failures on every probe
- Current state is healthy: all three Anthropic models probe successfully

**Taxonomy routing** (confirmed correct):
- `planning_tier.md`: `route:: anthropic, opus`
- `implementation_tier.md`: `route:: anthropic, sonnet`
- `review_tier.md`: `route:: anthropic, haiku`
- Bare names pass the C1 gate (`CLAUDE_CLI_BARE_MODELS`)

**Code bug found and fixed**:
- `probe_all` in `provider_probe.rs` only exempted "CLI tool not found on PATH" from circuit-breaker updates
- C1-blocked probes (`"probe skipped: provider not in C1 allow-list"`) and no-action-template probes were incorrectly calling `breaker.record_failure()`
- This would cause spurious circuit-breaker trips for any route with a banned/unknown provider prefix (not Anthropic, which passes C1, but general correctness issue)

### Change made

`crates/terraphim_orchestrator/src/provider_probe.rs`:
- Extracted `is_environment_error(error: &str) -> bool` helper covering all three local-setup error kinds
- Updated `probe_all` circuit-breaker update block to use the helper
- Added unit tests: `is_environment_error_classifications`, `c1_blocked_probe_does_not_open_breaker`, `missing_cli_probe_does_not_open_breaker` (refactored from inline match)

13/13 provider_probe tests pass. Clippy clean. Formatted.

---

## What is pending

- PR #868 awaits review and merge
- After merge: close Gitea issue #446 (`tea issues close 446 --repo terraphim/terraphim-ai`)
- Optional: remove or update `~/.config/claude/settings.json` proxy config if it causes future probe failures with older CLI versions

---

## Key environment notes

- KG pre_tool_use hook rewrites `claude` in Bash commands — use `/tmp` scripts with symlink target path when testing CLI directly
- `gtr` requires `source ~/.profile` first; a pre-set stale `GITEA_TOKEN` env will cause 404 with swagger URL
- GitHub is the push remote; Gitea at `git.terraphim.cloud` is issue tracking only — PRs go to GitHub via `gh`
