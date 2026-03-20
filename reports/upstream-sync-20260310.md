# Upstream Sync Report - 2026-03-10

Generated: 2026-03-10

## Scope
- `/home/alex/terraphim-ai`
- `/home/alex/terraphim-skills` (exists)

## Output Path
- Requested path: `/opt/ai-dark-factory/reports/upstream-sync-20260310.md`
- Result: write blocked by sandbox policy in this session
- Saved report instead to: `/home/alex/terraphim-ai/reports/upstream-sync-20260310.md`

## Command Status
1. `cd /home/alex/terraphim-ai && git fetch origin && git log HEAD..origin/main --oneline`
- `git fetch origin` failed with network resolution error:
  `fatal: unable to access 'https://github.com/terraphim/terraphim-ai.git/': Could not resolve host: github.com`
- Analysis below uses cached `origin/main`.

2. `cd /home/alex/terraphim-skills && git fetch origin && git log HEAD..origin/main --oneline`
- `git fetch origin` failed inside the sandbox:
  `error: cannot open '.git/FETCH_HEAD': Permission denied`
- Analysis below uses cached `origin/main`.

## Cached Remote Ref Freshness
- `terraphim-ai`: cached `origin/main` last updated locally on `2026-03-06 19:57:03 +0100`
- `terraphim-skills`: cached `origin/main` last updated locally on `2026-03-06 11:27:34 +0100`
- `terraphim-skills` note: that update was recorded as `fetch origin: forced-update`

## Repository Snapshot
| Repo | Local `HEAD` | Cached `origin/main` | Remote-only commits | Local-only commits | Merge base | Assessment |
|---|---|---|---:|---:|---|---|
| `terraphim-ai` | `f770aae0` | `f770aae0` | 0 | 0 | same commit | Low risk in cached view |
| `terraphim-skills` | `44594d2` | `6a7ae16` | 86 | 29 | none | Very high risk; unrelated histories |

## Findings

### terraphim-ai
- No upstream commits are visible relative to cached `origin/main`.
- Cached local and remote refs match exactly.
- Confidence is limited because live fetch failed on 2026-03-10.

### terraphim-skills
- Cached upstream contains 86 commits not present locally.
- Local `main` also contains 29 commits not present in cached upstream.
- `git merge-base HEAD origin/main` returned no common ancestor.
- Combined with the `forced-update` reflog entry, this strongly suggests upstream history was rewritten or the local branch tracks a different lineage.
- This is not a routine fast-forward or small rebase. It needs manual reconciliation.

## Breaking Changes and Major Refactors

### High-Risk Manual Review
1. `f21d66f` `chore: rename repository to terraphim-skills`
- Changes repository identity, marketplace metadata, URLs, and install commands.
- High breaking-change risk for any automation still pinned to the old repo name.

2. `d6eeedf` `feat(hooks): Add PreToolUse hooks with knowledge graph replacement for all commands`
- Expands command rewriting from narrow cases to all Bash commands.
- This can alter commit messages, PR bodies, issue text, and package-manager commands.

3. `3e256a0` `fix(config): add hooks to project-level settings`
- Makes the hook stack project-active rather than purely opt-in documentation.
- Workflow impact is high because contributors can start seeing changed command behavior immediately.

4. `4c26610` `feat(judge): add multi-iteration runner script and extend verdict schema (#20)`
- Introduces a new runner and extends the verdict schema.
- Downstream tooling that reads verdict JSONL may break if it assumes the older shape.

5. `98b1237` `feat(judge): add pre-push hook and terraphim-agent config template (#22)`
- Adds a new push-time gate.
- This is workflow-breaking by design if a repo adopts the hook.

6. `ef6399d` `feat(judge): v2 rewrite with terraphim-cli KG integration and file-based prompts (#23)`
- Major refactor of `automation/judge/run-judge.sh`.
- Changes prompt delivery, parsing strategy, optional enrichment, and supporting docs/knowledge-graph files.

7. `1038f9f` `feat(judge): add disagreement handler and human fallback (#21)`
- Adds side effects beyond local evaluation: GitHub issue creation and an outbound HTTP POST to `http://100.106.66.7:8765/api/`.
- Needs manual review for network policy, credentials, and failure modes.

8. `6a7ae16` `feat: add OpenCode safety guard plugins`
- Adds runtime command-blocking plugins plus installation automation.
- The plugin shells raw command text through single-quoted snippets, which is fragile when commands contain quotes.

### Medium-Risk Compatibility Changes
- `372bed4` `fix(skills): align skill names with directory names and remove unknown field`
  Potential consumer breakage if tooling references older skill identifiers.
- `851d0a5` `feat: add terraphim_settings crate and cross-platform skills documentation`
  Structural addition with likely downstream config assumptions, but lower immediate break risk than hook/judge changes.

## Security-Relevant and Hardening Commits
- `e5c3679` adds the `git-safety-guard` skill for destructive-command blocking.
- `90ede88` extends guard guidance to block `--no-verify` bypass flags.
- `0aa7d2a` adds UBS-driven static-analysis hooks.
- `6a7ae16` adds OpenCode safety/advisory guard plugins.
- `1038f9f` adds automated escalation behavior with outbound notifications.

## Judge Stack Churn
The judge subsystem changed rapidly across these cached upstream commits:
- `14eae06` initial judge skill and schema
- `0fcbe45` provider config and model-reference corrections
- `4c26610` multi-iteration runner and schema expansion
- `1038f9f` disagreement handler and human fallback
- `98b1237` pre-push integration
- `ef6399d` v2 rewrite
- `cf21c47` PATH fix for non-interactive shells
- `547aee2` macOS `mktemp` compatibility fix
- `dd09d96` deep-model correction
- `0f8edb2` pre-push runner-path correction

Interpretation:
- The feature area is active and valuable, but it was still being stabilized immediately after introduction.
- If you sync this stack, test it as a system, not commit-by-commit.

## Risk Summary
- `terraphim-ai`: low risk in cached view; no visible upstream delta.
- `terraphim-skills`: very high risk.

Primary reasons for the `terraphim-skills` rating:
- 86 cached upstream-only commits
- 29 local-only commits
- no merge base between local `main` and cached `origin/main`
- cached `origin/main` was updated by a forced update
- multiple workflow-altering hook and judge changes

## Recommended Next Actions
1. Re-run both fetch commands from an environment with working GitHub network access and write access to both repos' `.git` directories.
2. Treat `terraphim-skills` as a manual integration exercise, not `git pull`.
3. Review these commits before syncing hook/judge behavior into active use:
   `f21d66f`, `d6eeedf`, `3e256a0`, `4c26610`, `98b1237`, `ef6399d`, `1038f9f`, `6a7ae16`
4. Decide on an explicit recovery path for `terraphim-skills`:
   fresh clone of upstream, selective cherry-pick of local-only work, or unrelated-history merge in a throwaway branch
5. After any sync, run smoke tests for:
   pre-tool hooks, pre-push hook behavior, `run-judge.sh`, `handle-disagreement.sh`, and OpenCode guard handling of quoted shell commands
