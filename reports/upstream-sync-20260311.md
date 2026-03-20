# Upstream Sync Report - 2026-03-11

## Scope

Checked upstream status for:

- `/home/alex/terraphim-ai`
- `/home/alex/terraphim-skills`

## Limitation

Attempted `git fetch origin` in both repositories, but the environment could not complete a live upstream refresh:

- `terraphim-ai`: fetch failed because the sandbox could not resolve `github.com`
- `terraphim-skills`: fetch could not update `.git/FETCH_HEAD` from this sandbox

This report therefore analyzes the locally cached `origin/main` refs already present on disk.

Cached remote-tracking refs used:

- `terraphim-ai` `origin/main` last updated: `2026-03-06 19:57:03 +0100`
- `terraphim-skills` `origin/main` last updated: `2026-03-06 11:27:34 +0100`

## Summary

| Repository | Local HEAD | Cached `origin/main` | Upstream-only commits | Risk |
|---|---|---|---:|---|
| `terraphim-ai` | `f770aae` | `f770aae` | 0 | Low |
| `terraphim-skills` | `44594d2` | `6a7ae16` | 86 | High |

## Repository Analysis

### `terraphim-ai`

- No upstream-only commits in the cached `origin/main` range.
- No breaking changes, security fixes, or refactors detected from the local remote-tracking ref.
- Risk: low.

### `terraphim-skills`

Cached upstream range contains 86 commits and a large content change set:

- `88 files changed`
- `12,943 insertions`
- `218 deletions`

The bulk of the risk is not from content additions alone, but from developer-workflow enforcement changes:

1. Hook behavior was expanded from narrow use cases to active command mediation.
2. A new `judge` automation stack now participates in push-time workflow.
3. Repo/plugin naming and marketplace metadata changed multiple times.
4. OpenCode now has blocking safety plugins and install automation.

## Breaking Changes / Operational Risk

### 1. Hook stack now modifies all Bash commands

High-risk commits:

- `d6eeedf` `feat(hooks): Add PreToolUse hooks with knowledge graph replacement for all commands`
- `3e256a0` `fix(config): add hooks to project-level settings`

Impact:

- `examples/hooks/pre_tool_use.sh` now applies Terraphim replacement logic to all Bash commands, not only commit text.
- `.claude/settings.local.json` enables repo-level hook wiring to `~/.claude/hooks/pre_tool_use.sh` and `~/.claude/hooks/post_tool_use.sh`.
- This can change command behavior across contributors and CI depending on what is installed in each home directory.

Manual review needed for:

- command mutation risk
- environment-specific hook behavior
- whether repo-local config should depend on home-directory scripts

### 2. Judge system is now a push-path workflow gate

High-risk commits:

- `98b1237` `feat(judge): add pre-push hook and terraphim-agent config template (#22)`
- `ef6399d` `feat(judge): v2 rewrite with terraphim-cli KG integration and file-based prompts (#23)`
- `1038f9f` `feat(judge): add disagreement handler and human fallback (#21)`
- follow-up fixes: `cf21c47`, `547aee2`, `0f8edb2`, `dd09d96`

Impact:

- `automation/judge/pre-push-judge.sh` can block or alter push flow.
- `automation/judge/run-judge.sh` is now a multi-round runner with `opencode`, `python3`, JSON extraction, temp files, and optional `terraphim-cli` enrichment.
- `automation/judge/handle-disagreement.sh` creates GitHub issues and attempts a POST to `http://100.106.66.7:8765/api/` for Agent Mail notification.

This is a major refactor with rollout risk. The follow-up fixes show portability issues were found after introduction:

- non-interactive `PATH` fix
- macOS `mktemp` fix
- symlinked hook path fix
- deep-model correction

### 3. Judge config/schema drift in final `origin/main`

High-risk state in the final cached upstream tip:

- `automation/judge/run-judge.sh` uses deep model `opencode/glm-5-free`
- `automation/judge/verdict-schema.json` still enumerates `opencode/kimi-k2.5-free`
- `automation/judge/terraphim-agent-hook.toml` still sets `deep_model = "opencode/kimi-k2.5-free"`
- `skills/judge/SKILL.md` still documents `opencode/kimi-k2.5-free`

Additional schema incompatibility:

- `automation/judge/handle-disagreement.sh` writes human override records with:
  - `model: "human"`
  - `mode: "override"`
  - `scores: 0`
  - `round: 0`
- those values do not satisfy the published `verdict-schema.json`

Risk:

- downstream validators can reject actual judge output
- docs/templates can configure a non-matching deep model
- operational debugging will be harder because source-of-truth files disagree

This needs manual review before adopting the upstream judge stack.

### 4. Marketplace/repository identity churn

Relevant commits:

- `77fd112` move `marketplace.json` to repo root
- `e1691c4` revert and move it back
- `537efd8` update marketplace name and URLs for repo rename
- `f21d66f` rename repository to `terraphim-skills`

Impact:

- install docs and automation that pinned older repo names may break
- plugin marketplace discovery expectations may differ across versions and scripts

Net state looks coherent, but the migration path was noisy enough to justify manual verification of all install commands.

### 5. OpenCode plugin enforcement changes

High-risk commit:

- `6a7ae16` `feat: add OpenCode safety guard plugins`

Impact:

- adds advisory and blocking plugins under `examples/opencode/plugins/`
- `examples/opencode/install.sh` mutates OpenCode config to enable plugins
- includes custom forbidden patterns such as `pkill tmux`

Risk:

- existing terminal/session workflows can be disrupted
- behavior depends on `terraphim-agent` and `dcg` availability
- plugin install changes local user config

Manual review recommended before rollout.

## Security Fixes / Hardening

No explicit CVE-style vulnerability patch was identified in the cached range, but several security-hardening commits were added:

- `90ede88` blocks hook-bypass flags like `git commit --no-verify` and `git push --no-verify`
- `d6eeedf` adds destructive-command blocking ahead of KG replacement
- `0aa7d2a` adds UBS hook examples for critical bug detection
- `6a7ae16` adds blocking safety plugins for OpenCode

These are meaningful safeguards, but they also increase operational coupling and need rollout review.

## Major Refactors

Major refactors in the cached upstream range:

- `ef6399d` judge v2 rewrite with file-based prompts and KG integration
- `4c26610` multi-iteration judge protocol and schema expansion
- `d6eeedf` hook architecture shift from passive docs to active command interception
- `dc96659` repository archive/migration rewrite of README positioning

## High-Risk Commits Requiring Manual Review

Priority 1:

- `ef6399d` `feat(judge): v2 rewrite with terraphim-cli KG integration and file-based prompts (#23)`
- `98b1237` `feat(judge): add pre-push hook and terraphim-agent config template (#22)`
- `1038f9f` `feat(judge): add disagreement handler and human fallback (#21)`
- `d6eeedf` `feat(hooks): Add PreToolUse hooks with knowledge graph replacement for all commands`

Priority 2:

- `3e256a0` `fix(config): add hooks to project-level settings`
- `6a7ae16` `feat: add OpenCode safety guard plugins`
- `dd09d96` `fix(judge): use free model for deep judge`
- `77fd112`, `e1691c4`, `537efd8`, `f21d66f` marketplace/repo rename churn

## Recommendation

Do not fast-forward `terraphim-skills` blindly.

Recommended sequence:

1. Re-run this check from an environment with live network access so `git fetch origin` can complete.
2. Review the final `judge` contract across:
   - `automation/judge/run-judge.sh`
   - `automation/judge/verdict-schema.json`
   - `automation/judge/terraphim-agent-hook.toml`
   - `skills/judge/SKILL.md`
3. Validate whether repo-level Claude hook config should remain enabled by default.
4. Verify OpenCode plugin rollout in a non-critical environment before broad adoption.

## Commands Attempted

```bash
cd /home/alex/terraphim-ai && git fetch origin && git log HEAD..origin/main --oneline
cd /home/alex/terraphim-skills && git fetch origin && git log HEAD..origin/main --oneline
```
