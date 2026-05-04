# Research Document: ADF CI Pipeline Degraded -- 2026-05-04

**Status**: Updated (after full log analysis)
**Author**: opencode (Phase 1 research)
**Date**: 2026-05-04
**Reviewers**: Alex
**Updated**: 2026-05-04 -- orchestrator stopped/disabled, journalctl + Quickwit logs analysed

## Executive Summary

The ADF CI pipeline is degraded across three axes: (1) Gitea commit status API returns malformed JSON, blocking PR gate reconciliation for all 24 open PRs; (2) the `anthropic` provider probes fail continuously, causing circuit breaker flapping every 5 minutes and forcing all spawns to fallback; (3) PR review comments from the pr-reviewer agent lack the required `<h3>Inline Findings</h3>` heading, preventing auto-merge verdict parsing. Additionally, the `merge-coordinator` and `odilo-developer` agents reference CLI tools missing from PATH.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | CI is the backbone of the development process; without it, PRs cannot merge |
| Leverages strengths? | Yes | The orchestrator, webhook, and pr_gate modules are already well-structured |
| Meets real need? | Yes | 24 open PRs are blocked; manual merges are the only workaround |

**Proceed**: Yes (3/3)

## Problem Statement

### Description
The ADF CI pipeline on Gitea is non-functional. The orchestrator is running (PID 1737906, active for 10h), consuming 43.9G RAM, but three cascading failures prevent PRs from progressing through the status-check gate.

### Impact
- 24 open PRs cannot merge (branch protection requires `adf/build` + `adf/pr-reviewer`)
- All agent spawns fall back to kimi (suboptimal routing)
- PR gate reconciliation fails for every PR on every tick due to API decode errors
- Manual intervention required for every merge

### Success Criteria
1. `adf/build` and `adf/pr-reviewer` commit statuses post successfully on PRs
2. PR gate reconciliation runs without `error decoding response body`
3. Anthropic provider probes succeed (or fail gracefully without breaker flapping)
4. PR review comments parse correctly (0 "missing Inline Findings" errors)

## Current State Analysis

### Existing Implementation

The ADF CI pipeline has three layers:

1. **build-runner** (`scripts/adf-setup/agents/build-runner.toml`): Bash-only agent triggered by `handle_push` webhook on push to main. Runs `cargo fmt/clippy/test` via `rch exec`. Posts `adf/build` commit status. **Currently working** -- last status was `success` at 23:51 on 2026-05-03.

2. **pr-reviewer** (`scripts/adf-setup/agents/pr-reviewer.toml`): LLM agent dispatched on PR open/reopened. Posts `adf/pr-reviewer` commit status. **Currently failing** -- review comments lack `Inline Findings` heading, so `parse_verdict` rejects them.

3. **PR gate reconciliation** (`lib.rs:5634`): Runs every N ticks, reads commit statuses via Gitea API, classifies each PR. **Currently failing** -- `list_commit_statuses` API returns malformed JSON (`error decoding response body`) for 24 PRs.

### Code Locations

| Component | Location | Purpose |
|-----------|----------|---------|
| `build-runner.toml` | `scripts/adf-setup/agents/build-runner.toml` | Deterministic CI agent (bash) |
| `pr-reviewer.toml` | `scripts/adf-setup/agents/pr-reviewer.toml` | LLM PR review agent |
| `handle_push` | `crates/terraphim_orchestrator/src/lib.rs:2719` | Spawns build-runner on push webhook |
| `handle_push_event` | `crates/terraphim_orchestrator/src/webhook.rs:492` | Parses push webhook payload |
| `reconcile_pr_gates` | `crates/terraphim_orchestrator/src/lib.rs:5634` | PR gate status reconciliation |
| `parse_verdict` | `crates/terraphim_orchestrator/src/pr_review.rs:114` | Parses review comment HTML |
| `list_commit_statuses` | `crates/terraphim_tracker/src/gitea.rs` | Gitea API client for commit statuses |
| `provider_probe` | `crates/terraphim_orchestrator/src/provider_probe.rs` | Provider health probing |
| `pr_gate` | `crates/terraphim_orchestrator/src/pr_gate.rs` | PR gate classification logic |

### Data Flow

```
Push to main → webhook → handle_push → spawn build-runner → cargo gates → POST adf/build status
PR opened    → webhook → handle_review_pr → spawn pr-reviewer → LLM review → POST adf/pr-reviewer status
Every N ticks → reconcile_pr_gates → list_commit_statuses → classify PR → enqueue auto-merge or remediation
```

### Integration Points
- Gitea Commit Status API (`POST /api/v1/repos/{owner}/{repo}/statuses/{sha}`)
- Gitea Branch Protection API (`GET /api/v1/repos/{owner}/{repo}/branch_protections/main`)
- Gitea PR List API (`GET /api/v1/repos/{owner}/{repo}/pulls`)
- `rch exec` for dispatching cargo builds to bigbox via rchd

## Constraints

### Technical Constraints
- Gitea commit status API is returning malformed JSON -- this is a Gitea server-side bug or version incompatibility
- The orchestrator uses `reqwest` for HTTP; the deserialization failure means the response shape has changed
- `pr_review.rs:114` uses literal string matching for `<h3>Inline Findings</h3>` -- any deviation fails
- Provider probes execute via `bash -c "<action_template>"` -- exit status 1 is treated as provider failure

### Business Constraints
- 24 open PRs are blocked -- this is urgent
- Manual merge workaround requires disabling status checks (risky, requires admin)
- Agent memory at 43.9G/64G -- no immediate resource pressure

### Non-Functional Requirements

| Requirement | Target | Current |
|-------------|--------|---------|
| Status check posting | < 30s after build | Working for adf/build |
| PR gate reconciliation | 0 decode errors | 24 decode errors per cycle |
| Provider probe accuracy | True positives only | Anthropic false positive failures |
| Review parse success rate | 100% | 0% (all fail) |

## Vital Few (Essentialism)

### Essential Constraints (Max 3)

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Gitea API `list_commit_statuses` must return valid JSON | Blocks ALL PR gate reconciliation | 24 PRs affected every tick cycle |
| `parse_verdict` must accept review output format | Blocks ALL auto-merge | Every review comment fails parsing |
| `adf/pr-reviewer` status must be posted | Required by branch protection | PRs cannot merge without it |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| `merge-coordinator` / `odilo-developer` spawn failures | Non-blocking; these agents are not in the critical CI path |
| Provider probe tuning (circuit breaker thresholds) | Already addressed in design-1233; separate fix |
| Memory optimisation (43.9G usage) | Not causing current failures |
| GitHub Actions CI workflows | ADF is the Gitea CI path; GitHub Actions is separate |

## Dependencies

### Internal Dependencies

| Dependency | Impact | Risk |
|------------|--------|------|
| `terraphim_tracker::gitea::list_commit_statuses` | PR gate reconciliation | API response shape mismatch |
| `pr_review::parse_verdict` | Auto-merge pipeline | String matching too rigid |
| `pr-reviewer.toml` POST_STATUS function | adf/pr-reviewer commit status | Depends on review output format |

### External Dependencies

| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| Gitea API | Current (git.terraphim.cloud) | Response format may have changed in recent update | Manual status posting via curl |
| Anthropic Claude CLI | Installed on bigbox | Exit status 1 on probe | Disable anthropic probes temporarily |

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Gitea API response shape changed silently | High | High | Inspect raw response, update struct |
| pr-reviewer skill template changed | High | High | Normalise HTML in parser |
| Anthropic CLI misconfigured on bigbox | High | Medium | Check `claude --version` on bigbox |
| Fix requires orchestrator restart | Medium | Medium | Deploy during low activity |

### Open Questions
1. What Gitea version is running? Did it update recently?
2. What is the raw JSON response from `list_commit_statuses` that fails to decode?
3. What does the pr-reviewer output actually look like now (sample review comment)?
4. Is the `claude` CLI installed and functional on bigbox?

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| Gitea `list_commit_statuses` response changed | Repeated `error decoding response body` across all PRs | If it's a transient network issue, the fix is different | No -- need raw response |
| pr-reviewer still uses LLM but output format drifted | Parse failures are consistent (100%) | If the agent itself is broken, the fix is in agent config | Partially -- logs show comments exist but lack heading |
| `adf/build` works because build-runner posted it | Status shows `success` at 23:51 | Correct -- build-runner is bash, not LLM | Yes |

### Multiple Interpretations Considered

| Interpretation | Implications | Why Chosen/Rejected |
|----------------|--------------|---------------------|
| Gitea API response changed in a version upgrade | Need to update Rust struct to match new fields | Most likely -- affects ALL PRs consistently |
| Gitea API is rate-limiting the orchestrator | Would see 429 status codes, not decode errors | Rejected -- errors are decode failures, not HTTP errors |
| Orchestrator HTTP client has a bug | Would affect all API calls, not just statuses | Rejected -- branch protection API works fine |

## Research Findings

### Key Insights

1. **Root cause 1 (Critical): `list_commit_statuses` API returns JSON that fails deserialization.** This blocks ALL PR gate reconciliation for terraphim-ai. The same call works for other repos (no branch protection), so the failure is specific to commit status responses on this repo. The error is `error decoding response body` which suggests a field mismatch in the `CommitStatus` Rust struct.

2. **Root cause 2 (Critical): pr-reviewer review comments lack `<h3>Inline Findings</h3>`.** The `parse_verdict` function in `pr_review.rs:114` requires this exact string. The pr-reviewer agent is posting comments, but the LLM output does not consistently include this heading. 44 parse failures in 2 hours across PRs 1156 and 1195.

3. **Root cause 3 (Medium): Anthropic provider probes fail with exit status 1.** The circuit breaker re-opens every 5 minutes (observed at :26, :31, :41, :51, :56, :01). This forces all spawns to kimi fallback, but does not block CI.

4. **Root cause 4 (Low): `merge-coordinator` and `odilo-developer` CLI tools missing.** These agents fail to spawn (`No such file or directory`) but are not in the CI critical path.

5. **`adf/build` works.** The build-runner successfully posted `success` at 23:51 on 2026-05-03 for commit `5daadeb`. This confirms the push webhook, `handle_push`, and build-runner pipeline are functional.

6. **`adf/pr-reviewer` is never posted.** No `adf/pr-reviewer` status exists in the commit statuses for the latest main commit. The pr-reviewer agent either does not run, runs but fails before posting, or posts but the review output is never parsed.

7. **Gitea Actions CI workflows are stuck in pending.** 15 CI jobs are queued but not running -- 0 workflows currently running. The Gitea runner appears to be offline or not picking up jobs. This is separate from the ADF pipeline.

## Full Log Analysis (2026-05-04, after stopping orchestrator)

### Source: journalctl (24h window, 9367 lines)

| Category | Count | Pattern |
|----------|-------|---------|
| API decode errors | 315 | `error decoding response body` (24 PRs x 13 reconciliation cycles) |
| Review parse failures | 540 | `missing Inline Findings` (2 PRs: 1156 + 1195, every 5min tick) |
| Circuit breaker re-open | 152 | anthropic probes fail -> breaker flaps every 5min |
| Provider probe failures | 69 | anthropic (68) + openai (1) |
| Branch protection 404 | 65 | 5 other repos have no branch protection (expected) |
| Probe skipped/provider failure | 358 | Normal probe lifecycle operations |
| Config load failure | 9 | TOML parse error at line 41 (transient -- file write race, resolved after 4min) |
| Cron spawn failures | 17 | merge-coordinator (6) + odilo-developer (11) -- CLI tool not on PATH |
| Push webhook parse failure | 3 | `null, expected a sequence at line 29` in commit file arrays |
| Agent max restarts exceeded | 2 | pr-spec-validator-retry-1 + pr-security-sentinel-retry-1 |
| Webhook secret missing | 3 | Non-authenticated webhook test calls |
| Reconcile tick timeout | 2 | Tick exceeded timeout |

### Source: Quickwit

| Index | Entries | Latest Entry | Status |
|-------|---------|--------------|--------|
| `adf-logs` | 17,859 | 2026-04-21T10:00:12Z | **13 days stale** |
| `adf-digital-twins-logs` | 155 | Unknown | Low activity |
| `adf-odilo-logs` | 58 | Unknown | Low activity |
| `otel-logs-v0_7` | 0 | N/A | Empty |
| `workers-logs` | 0 | N/A | Empty |

### New Findings from Full Analysis

1. **Push webhook null arrays (NEW):** 3 push events failed to parse because `GiteaPushCommit.added/removed/modified` are `null` instead of `[]`. The `#[serde(default)]` attribute only handles missing fields, not explicit `null`. The `deserialize_null_default_vec` helper already exists but is only used for the top-level `commits` field.

2. **Config TOML parse race (NEW):** 9 config load failures on 2026-05-03 between 22:54-22:58, then succeeded. Python `tomllib` parses the same file fine. This was a transient race -- the config file was being written while the orchestrator was restarting (systemd 30s restart loop).

3. **Quickwit sink stalled since 2026-04-21 (NEW):** The `adf-logs` index has had no new entries for 13 days. The orchestrator config has `[projects.quickwit]` enabled with correct endpoint. The sink likely encountered a transient error and never reconnected. Fix: restart orchestrator with corrected binary.

4. **Only 2 PRs generate review parse failures:** PRs 1156 and 1195 -- these are the only PRs with agent-generated review comments. Neither has a comment from `pr-reviewer`; all comments are from security-checklist, security-audit, and requirements-traceability agents.

5. **Orchestrator restarted 3 times in 24h:** PIDs 1367890 -> 3013090 -> 3238683 -> 1737906 (current). The config parse failure on 2026-05-03 22:54 caused a crash loop (9 failures in 4 minutes with 30s restart interval).

6. **Anthropic probe failure confirmed:** `claude` CLI exits with status 1 on probe. 68 of 69 provider probe failures are anthropic (opus: 12, sonnet: 44, haiku: 12). The single openai failure was `openai/gpt-5.4`.

## Recommendations

### Proceed/No-Proceed
Proceed. All root causes are fixable with targeted code changes.

### Scope Recommendations
Fix in priority order:
1. Fix `CommitStatusEntry` serde rename (unblocks PR gate reconciliation -- 315 errors/24h)
2. Fix push webhook null arrays (prevents dropped push events)
3. Fix `parse_verdict` to tolerate `Findings` heading (unblocks auto-merge -- 540 errors/24h)
4. Add comment pre-filter (reduces noise from non-reviewer agents)
5. Re-enable Quickwit sink (13 days of missing logs)
6. Fix missing CLI tools (17 spawn failures/24h)
7. Defer Anthropic probe fix to design-1233

### Risk Mitigation Recommendations
- Inspect raw Gitea API response before fixing the struct (DONE -- confirmed `status` field)
- Fetch sample review comment from PR 1156 (DONE -- confirmed `<h3>Findings</h3>` format)
- Test changes locally before deploying to bigbox
- Keep orchestrator disabled until deploy is ready

## Next Steps

Proceed to Phase 2 design (updated in `.docs/design-2026-05-04-adf-ci-degraded.md`).
