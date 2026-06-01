# ADF Agent Failure Analysis — Phase 1 Disciplined Research

**Date:** 2026-06-01
**Scope:** Remaining agent failures after fixes for (1) Spawner `--allowedTools` parsing and (2) Build-runner stdin consumption
**Data Source:** `journalctl -u adf-orchestrator` on bigbox, agent configs in `/opt/ai-dark-factory/conf.d/terraphim.toml`, agent logs in `/opt/ai-dark-factory/logs/agents/`
**Period Analysed:** 2026-05-20 through 2026-06-01 10:00 CEST (orchestrator stopped at 12:00 CEST)

---

## 1. Executive Summary

The ADF orchestrator exhibits **three dominant failure modes** that explain the majority of agent exit_code=1 events:

1. **Orchestrator Tick-Stall Cascade** — The main reconcile loop occasionally blocks for 10–53 seconds (normal: 100–1500ms). During these stalls, all running agents that use the Claude CLI simultaneously exit with code 1. This accounts for the clustered "quick failures" across compliance-watchdog, odilo-developer, implementation-swarm, security-sentinel, and spec-validator.

2. **Disabled-Agent Still Spawned** — `merge-coordinator` has `enabled = false` in its config but the orchestrator continues to spawn it, causing failures on every scheduled run.

3. **Build Runner Early-Exit Pattern** — The `build-runner` bash script fails with short runtimes (20–90s) before reaching compilation, producing exit_code=1 with no matched error patterns (classified as `unknown`).

Additional isolated issues include:
- **product-owner** Gitea collaborator access severance (known, documented in roadmap reports)
- **test-guardian** timeouts on long-running test suites
- **product-development** hitting wall-time limits after 35+ minutes

---

## 2. Failure Breakdown by Agent

### 2.1 Quick Failures (< 60s, exit_class=unknown)

#### compliance-watchdog
| Metric | Value |
|--------|-------|
| Failure count | 3 |
| Avg runtime | 39.5s |
| CLI/model | `/home/alex/.local/bin/claude` + `sonnet` (fallback) |
| Schedule | `5 0-10 * * *` |

**Evidence:**
- May 30 09:06:16 UTC: failed at exact same second as product-owner, odilo-developer, implementation-swarm (cluster failure).
- May 31 06:01:01 UTC: failed at exact same second as security-sentinel, product-development, implementation-swarm, odilo-developer during a 53-second tick stall.
- Journal: `agent exit classified agent=compliance-watchdog exit_code=Some(1) exit_class=unknown confidence=0.0`

**Suspected root cause:** Orchestrator tick stall causes child process pipe/stdio disruption; Claude CLI exits with code 1 when its stdout/stderr is closed or when a shared resource lock is lost.

---

#### merge-coordinator
| Metric | Value |
|--------|-------|
| Failure count | 3 |
| Avg runtime | 75.0s |
| CLI/model | `/home/alex/.bun/bin/opencode` + `kimi-for-coding/k2p5` |
| Schedule | `0 */4 * * *` |

**Evidence:**
```toml
# /opt/ai-dark-factory/conf.d/terraphim.toml
name = "merge-coordinator"
enabled = false  # Replaced by Rust binary at /usr/local/bin/merge-coordinator
layer = "Growth"
```
- Journal shows it being spawned despite `enabled = false`:
  `spawning agent agent=merge-coordinator layer=Growth cli=/home/alex/.bun/bin/opencode model=Some("kimi-for-coding/k2p5")`
- May 30 17:19:10: first failure after 71.7s; immediately respawned and failed again after 26.9s.
- May 31 04:00:08: `git worktree add failed, using shared working_dir agent=merge-coordinator error=fatal: not a git repository`

**Suspected root cause:**
1. **Primary bug:** Orchestrator ignores `enabled = false` and spawns the agent anyway.
2. **Secondary:** The agent's working directory is not a git repo, causing worktree creation to fail.

---

### 2.2 Medium Failures (60–200s)

#### build-runner (unknown class only — NOT compilation_error)
| Metric | Value |
|--------|-------|
| Failure count | 50 (estimated from "unknown" class events; 76 total exit_code=1 events) |
| Avg runtime | ~90s |
| CLI | `/bin/bash` (deterministic, no LLM) |
| Trigger | `event_only = true` on push |

**Evidence:**
- Agent config task uses `set -euo pipefail` then:
  ```bash
  git fetch --depth=1 gitea $ADF_PUSH_REF 2>/dev/null || true
  git checkout -f $ADF_PUSH_SHA
  sed -i "s|cargo build --workspace|cargo build --workspace --profile ci|" $ADF_WORKING_DIR/BUILD.md
  bash $ADF_WORKING_DIR/scripts/build-runner-llm.sh
  ```
- Journal shows two distinct failure classes:
  - `exit_class=compilation_error` with `matched_patterns=["could not compile"]` (these are legitimate build failures)
  - `exit_class=unknown` with `confidence=0.0` and `matched_patterns=[]` (these are the target failures)
- Example unknown failures:
  - May 31 18:07:48: 33.5s (push to `main`)
  - May 31 18:24:18: 51.5s (push to `main`)
  - Jun 1 01:17:48: 23.6s (push to `task/1913-concurrent-test-in-process`)

**Suspected root cause:** The bash script exits early before compilation. Likely causes:
1. `git checkout -f $ADF_PUSH_SHA` fails because shallow fetch doesn't include the SHA (race with push timing)
2. `sed -i` on `BUILD.md` or `scripts/build-runner-llm.sh` fails because files don't exist or patterns don't match (with `set -e`, `sed` exits 0 even on no-match, but file-not-found would fail)
3. `scripts/build-runner-llm.sh` itself fails early in `detect_commands()` because `BUILD.md` has unexpected format or `awk`/`grep` pipeline returns no commands

**Critical missing data:** Agent stderr is not captured in the log files (only headers are written). The orchestrator's `spawner` module does not redirect stderr to the agent log files for bash-based agents.

---

#### security-sentinel
| Metric | Value |
|--------|-------|
| Failure count | 2 |
| Avg runtime | 121.8s |
| CLI/model | `/home/alex/.local/bin/claude` + `sonnet` (fallback from opencode/kimi) |
| Schedule | `0 */6 * * *` |

**Evidence:**
- May 30 17:19:08: failed at exact same second as spec-validator and merge-coordinator (cluster failure).
- May 31 06:01:01: failed at exact same second as product-development, implementation-swarm, odilo-developer during 53-second tick stall.
- Configured primary: `cli_tool = "/home/alex/.bun/bin/opencode"`, `model = "kimi-for-coding/k2p6"`
- Journal shows fallback routing: `KG routed to fallback (primary unhealthy) agent=security-sentinel ... skipped_unhealthy=["openai-codex", "minimax-coding-plan", "zai-coding-plan"]`

**Suspected root cause:** Same as compliance-watchdog — orchestrator tick stall disrupts Claude CLI processes.

---

#### odilo-developer
| Metric | Value |
|--------|-------|
| Failure count | 3 |
| Avg runtime | 147.6s |
| CLI/model | `/home/alex/.bun/bin/opencode` + `kimi-for-coding/k2p5` (primary), fallback to claude/sonnet |
| Schedule | Cron-fired |

**Evidence:**
- All 3 failures occurred in clusters:
  - May 30 09:06:16 (with compliance-watchdog, implementation-swarm, product-owner)
  - May 31 06:01:01 (with security-sentinel, product-development, implementation-swarm)
  - May 31 06:01:01 again (different run)
- Journal shows `created isolated git worktree agent=odilo-developer path=/home/alex/projects/zestic-ai/odilo/.worktrees/...`

**Suspected root cause:** Same tick-stall cascade. The odilo project uses a different repo (`zestic-ai/odilo`) but the agent still falls back to Claude CLI, making it vulnerable to the same stall.

---

#### documentation-generator
| Metric | Value |
|--------|-------|
| Failure count | 1 |
| Avg runtime | 51.7s |
| CLI/model | `/home/alex/.local/bin/claude` + `sonnet` |

**Evidence:**
- May 30 11:41:38: single failure, 51.7s runtime.
- No cluster pattern for this failure.

**Suspected root cause:** Likely Claude CLI startup failure or early exit due to missing context. Needs more data (stderr not captured).

---

#### spec-validator
| Metric | Value |
|--------|-------|
| Failure count | 1 |
| Avg runtime | 189.6s |
| CLI/model | `/home/alex/.local/bin/claude` + `sonnet` (fallback) |

**Evidence:**
- May 30 17:19:09: failed in cluster with security-sentinel and merge-coordinator.

**Suspected root cause:** Same tick-stall cascade.

---

### 2.3 Long Failures (> 200s)

#### implementation-swarm
| Metric | Value |
|--------|-------|
| Failure count | 5 |
| Avg runtime | 213.6s |
| CLI/model | `/home/alex/.local/bin/claude` + `sonnet` (fallback) or `/home/alex/.bun/bin/opencode` + kimi |

**Evidence:**
- Failures at:
  - May 30 09:06:16 (cluster)
  - May 30 21:10:08 (599.5s — much longer)
  - May 31 06:01:01 (cluster, 51.4s)
  - May 31 10:58:18 (179.5s)
  - May 31 12:58:18 (28.9s)
- The 599.5s run on May 30 21:10 suggests it was making progress before failing, possibly a timeout or resource exhaustion.

**Suspected root cause:**
- Short failures: tick-stall cascade (same as other quick failures).
- Long failure (599.5s): Possibly hit `max_cpu_seconds` or `max_ticks` limit, or the opencode CLI timed out on a long operation.

---

#### test-guardian
| Metric | Value |
|--------|-------|
| Failure count | 4 |
| Avg runtime | 557.9s |
| CLI/model | `/home/alex/.local/bin/claude` + `sonnet` (fallback) |
| Schedule | `35 0-10 * * *` |
| max_cpu_seconds | 7200 |

**Evidence:**
- Runtimes: 720.8s, 1256.6s, 1451.6s, 29.5s (one very short outlier)
- The long runtimes (10–24 minutes) suggest the agent is running tests but eventually failing.
- Configured primary: `cli_tool = "/home/alex/.bun/bin/opencode"`, `model = "kimi-for-coding/k2p6"`
- Journal shows fallback to Claude: `KG routed to fallback (primary unhealthy) agent=test-guardian ... provider=anthropic model=sonnet`

**Suspected root cause:**
1. **Primary provider unhealthy:** opencode/kimi is consistently marked unhealthy, forcing fallback to Claude.
2. **Claude CLI test timeouts:** When running via Claude CLI, the test suite may exceed implicit timeouts or the CLI may disconnect during long operations.
3. **One outlier (29.5s):** This matches the tick-stall cascade pattern (May 31 06:04:38).

---

#### product-owner
| Metric | Value |
|--------|-------|
| Failure count | 3 |
| Avg runtime | 389.3s |
| CLI/model | `/home/alex/.local/bin/claude` + `opus` |

**Evidence:**
- Roadmap report (2026-06-01 10:55) explicitly documents this:
  > "The product-owner token (id=39) is valid and authenticating — /user succeeds. The failure is 404 on the repo, not 403. ... product-owner has been removed as a Write collaborator on terraphim/terraphim-ai."
- Failures at:
  - May 30 09:06:16 (659.1s — cluster failure, likely tick-stall)
  - May 31 06:01:01 (179.5s — cluster failure during 53s tick stall)
  - May 31 10:58:18 (329.2s)

**Suspected root cause:**
- **Two distinct failures:**
  1. Gitea collaborator access severance causes 404 on repo operations (documented, human-action-required).
  2. Tick-stall cascade on May 30 and May 31 caused early exits despite the agent having been running for 5–11 minutes.

---

#### product-development
| Metric | Value |
|--------|-------|
| Failure count | 1 |
| Avg runtime | 2152.5s (35.9 minutes) |
| CLI/model | `/home/alex/.local/bin/claude` + `sonnet` |

**Evidence:**
- May 31 06:01:01: Failed after 2152.5s during the 53-second tick stall.
- Journal shows: `agent exceeded wall-clock timeout, killing for fallback respawn agent=product-development elapsed_secs=7227 max_wall_secs=7200` (Jun 1 05:25:49 UTC, a different run).

**Suspected root cause:**
- The 2152.5s failure was a tick-stall cascade (agent killed/failed after 35 minutes of work).
- The Jun 1 timeout shows the agent regularly exceeds 2-hour wall-time limits.

---

## 3. Categorised Summary of Failure Modes

### Category A: Infrastructure/Config Issues (Highest Impact)

| Issue | Affected Agents | Evidence | Fix Priority |
|-------|----------------|----------|--------------|
| **merge-coordinator `enabled = false` ignored** | merge-coordinator | Config has `enabled = false` comment; orchestrator spawns anyway | P0 |
| **product-owner Gitea collaborator severance** | product-owner | Roadmap report documents 8 consecutive lockouts; token works but repo returns 404 | P0 |
| **Primary providers (kimi/opencode) marked unhealthy** | All agents using opencode CLI | Journal: `skipped_unhealthy=["openai-codex", "minimax-coding-plan", "zai-coding-plan"]` for nearly every spawn | P1 |
| **Provider probe failures for anthropic/claude** | All agents falling back to Claude | Journal: `probe failed provider="anthropic" model="sonnet" error=exit exit status: 1` | P1 |
| **Agent stderr not captured** | All agents | Log files only contain headers; no stderr content for debugging | P1 |

### Category B: Code Bugs (Orchestrator)

| Issue | Affected Agents | Evidence | Fix Priority |
|-------|----------------|----------|--------------|
| **Tick-stall cascade kills agents** | compliance-watchdog, security-sentinel, odilo-developer, implementation-swarm, spec-validator, product-owner, product-development | Multiple agents exit at exact same second; tick times spike to 25–53s | P0 |
| **Git worktree add/remove failures** | merge-coordinator, multiple agents | `git worktree add failed`, `git worktree remove failed ... is not a working tree` | P1 |
| **Compound review blocks tick loop** | All agents running during 06:00 UTC | Compound review starts at 06:00; tick at 08:01 took 53s | P1 |

### Category C: Resource Limits

| Issue | Affected Agents | Evidence | Fix Priority |
|-------|----------------|----------|--------------|
| **test-guardian test suite timeouts** | test-guardian | Avg 557s, runs up to 1451s; primary provider unhealthy forces slower fallback | P1 |
| **product-development wall-time exceeded** | product-development | Journal: `agent exceeded wall-clock timeout ... elapsed_secs=7227 max_wall_secs=7200` | P2 |
| **Global concurrency limit reached** | spec-validator, compliance-watchdog | Journal: `skipping spawn: global concurrency limit reached agent=spec-validator active=5` | P2 |

### Category D: External Service Failures

| Issue | Affected Agents | Evidence | Fix Priority |
|-------|----------------|----------|--------------|
| **Gitea branch protection 404/403** | PR gate reconciliation (all PRs) | Journal: `failed to get branch protection ... 404 Not Found` on atomic-server, better-auth-rust, digital-twins, gitea-robot, gitea; `403 Forbidden` on odilo | P2 |
| **Gitea issue/comment 404** | merge-coordinator, compound review | `failed to fetch assignees ... 404 Not Found on issue 1887`, `failed to post comment ... issue 514 ... 500 Internal Server Error` | P2 |

---

## 4. Root Cause Analysis: The Tick-Stall Cascade

### Hypothesis

The orchestrator's `reconcile_tick` is single-threaded and occasionally blocks for 10–53 seconds. During this block:
1. Child agent processes (Claude CLI) detect that their parent (orchestrator) is not responding on stdout/stderr pipes.
2. The Claude CLI exits with code 1 (non-zero) because it cannot write output or because a shared lock/resource is held by the stalled orchestrator.
3. All agents that happen to be running at that moment simultaneously fail with `exit_class=unknown`.

### Supporting Evidence

| Timestamp | Tick Duration | Agents Failed Simultaneously |
|-----------|---------------|------------------------------|
| May 30 09:06:16 | Tick=61 took 4.4s just before | product-owner, compliance-watchdog, odilo-developer, implementation-swarm |
| May 30 11:08:42 | Tick=66 took 25.8s | (anthropic probes failed after this) |
| May 31 06:01:01 | Tick=2131 took 53.1s | security-sentinel, product-development, implementation-swarm, odilo-developer |

All affected agents in these clusters were using `cli=/home/alex/.local/bin/claude`.

### Likely Trigger for Stalls

The May 31 06:01 stall occurred shortly after compound review started at 06:00:10. The compound review creates git worktrees and may hold a lock or perform blocking I/O on the main thread instead of a background task.

Additionally, git worktree operations are synchronous and can be slow:
```
git worktree add failed, using shared working_dir agent=merge-coordinator error=fatal: not a git repository
git worktree remove failed agent=product-development ... is not a working tree
```

### Why Claude CLI Specifically?

Agents using opencode/bun (kimi) that do NOT fall back to Claude seem less affected by the stall. The Claude CLI may:
- Use a persistent server process that detects parent unresponsiveness
- Hold a file lock or socket that conflicts with the stalled orchestrator
- Have stricter pipe handling that causes SIGPIPE on parent stall

---

## 5. Recommendations (Highest Impact First)

### P0: Fix Immediately

1. **Respect `enabled = false` in agent configs**
   - File: `crates/terraphim_orchestrator/src/config.rs` or agent dispatch logic
   - `merge-coordinator` is explicitly disabled but spawns every 4 hours, wasting resources and producing false failures.

2. **Fix orchestrator tick-stall**
   - Move git worktree operations (add/remove) off the main reconcile thread to an async task pool or blocking thread.
   - Compound review should run in a dedicated thread/task, not block `reconcile_tick`.
   - Add tick timeout telemetry: log which operation within the tick took >5s.

3. **Restore product-owner Gitea collaborator access**
   - Re-add `product-owner` (id=39) as a Write collaborator on `terraphim/terraphim-ai`.
   - Do NOT regenerate the token — the token is fine.

### P1: Fix This Week

4. **Capture agent stderr in log files**
   - The `terraphim_spawner` module currently does not redirect child stderr to the agent log files.
   - Without stderr, all `exit_class=unknown` failures are un-debuggable.
   - Add `Stdio::piped()` and write stderr to the agent log file after process exit.

5. **Fix build-runner early-exit (unknown class)**
   - Add explicit error handling and logging to `build-runner-llm.sh`:
     ```bash
     set -euo pipefail
     trap 'echo "FAILED at line $LINENO" >&2' ERR
     ```
   - Verify `git fetch --depth=1` actually retrieves the SHA before `git checkout`.
   - Make `sed` operations idempotent (check file existence first).
   - Or: run build-runner with `bash -x` temporarily to capture the exact failing command.

6. **Investigate and fix provider probe failures**
   - Anthropic (Claude) probes fail with `exit status: 1`. Run the probe manually:
     ```bash
     /home/alex/.local/bin/claude --version
     /home/alex/.local/bin/claude -p "test" --allowed-tools ...
     ```
   - The `--allowedTools` bug was allegedly fixed — verify the fix is deployed and the probe command uses the correct flag format.

7. **Fix primary provider health (opencode/kimi)**
   - Determine why opencode/kimi is consistently marked unhealthy.
   - Check if the opencode CLI has a similar probe failure or if the model string `kimi-for-coding/k2p6` is no longer valid.

### P2: Fix Next Sprint

8. **Increase or configure test-guardian wall-time limit**
   - Current `max_cpu_seconds = 7200` may be insufficient for full test suites.
   - Consider splitting test-guardian into per-crate test runners.

9. **Fix Gitea branch protection warnings**
   - The 404/403 errors on branch protection are noisy but not fatal.
   - Either configure branch protection on those repos or suppress the warnings.

10. **Add agent concurrency limit telemetry**
    - When `global concurrency limit reached` fires, log which agents are active and for how long.

---

## 6. Data Quality Notes

- **Agent stderr is NOT captured.** All log files in `/opt/ai-dark-factory/logs/agents/` contain only a 5-line header. The `.tmp-*` files contain brief descriptions, not stderr. This makes root-cause analysis of `exit_class=unknown` failures speculative.
- **Journal granularity:** The orchestrator logs agent spawns and exits but does not log intermediate state. A `DEBUG` or `TRACE` log level for the spawner module would help.
- **No systemd OOM events found.** The cluster failures are not explained by the kernel OOM killer.
- **Orchestrator restarts:** The adf-orchestrator.service restarted 4 times during the analysis period (May 30 08:37, May 30 12:25, May 31 10:52, May 31 12:52). Each restart kills all running agents without graceful shutdown, which may account for some cluster failures.

---

## 7. Appendix: Raw Failure Counts (May 20 – Jun 1 10:00 UTC)

| Agent | exit_code=1 Count | Avg Runtime | Max Runtime |
|-------|-------------------|-------------|-------------|
| build-runner (unknown) | ~50 | ~97s | ~126s |
| build-runner (compilation_error) | ~26 | ~35s | ~59s |
| pr-reviewer | 10 | ~17s | ~33s |
| implementation-swarm | 5 | ~214s | ~600s |
| test-guardian | 4 | ~558s | ~1452s |
| product-owner | 3 | ~389s | ~659s |
| odilo-developer | 3 | ~148s | ~360s |
| merge-coordinator | 3 | ~75s | ~127s |
| compliance-watchdog | 3 | ~40s | ~60s |
| security-sentinel | 2 | ~122s | ~191s |
| spec-validator | 1 | ~190s | ~190s |
| product-development | 1 | ~2153s | ~2153s |
| documentation-generator | 1 | ~52s | ~52s |

*Note: build-runner has two distinct failure classes. The user asked to focus on the "unknown" class (50 failures, avg ~90s), not the `compilation_error` class.*
