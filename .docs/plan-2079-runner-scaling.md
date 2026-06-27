# Plan: scale the native Gitea Actions runner (terraphim/terraphim-ai#2079)

**Status:** plan + artefacts only. No live changes. A human executes the live
registration and systemd steps. The registration token MUST be supplied by the
human from `op` (or the existing mechanism); this document never fetches or
embeds it.

**Scope:** the native runner crate `crates/terraphim_gitea_runner`, deployed on
bigbox as the systemd `--user` service `terraphim-gitea-runner.service`. All
seven polyrepos plus the proof repo gate `native-ci` on this single runner, so
it serialises work and is the throughput bottleneck.

---

## 1. Findings from the code (grounding)

All claims below are read directly from the crate at the cited lines.

### 1.1 The runner executes exactly one task at a time (M1)

`src/poller.rs` confirms the single-task loop:

- `poll_once` (`poller.rs:53-82`) calls `fetch_task` once
  (`poller.rs:54`), and if a task is present, constructs a single `TaskWorker`
  (`poller.rs:68-73`) and `await`s `worker.run(...)` to completion
  (`poller.rs:77`) before returning.
- `run_forever` (`poller.rs:85-94`) is a strictly sequential loop: it `await`s
  `poll_once`, then `tokio::time::sleep(poll_interval)` (`poller.rs:88-92`).
  There is no `tokio::spawn`, no `JoinSet`, no concurrency. One task fully
  finishes (compile -> checkout -> execute -> stream logs -> report result)
  before the next `fetch_task` is issued.

`TaskWorker::run` (`task_worker.rs:145-272`) is the long pole: it checks out the
repo (`task_worker.rs:154`), builds a fresh host execution stack
(`task_worker.rs:158-172`), executes the workflow with a default per-step
timeout of 1800s and a hard cap of 7200s (`task_worker.rs:169-170`), and streams
logs per step (`task_worker.rs:210-224`). While this runs, the poll loop is
blocked.

### 1.2 State file and checkout dir ARE per-instance configurable via env

`src/bin/terraphim-gitea-runner.rs` reads every operationally relevant value
from the environment, so a second instance is purely an ops change:

- `RUNNER_STATE_FILE` -> `config.state_file` (`bin:67`), default `.runner`.
- `RUNNER_CHECKOUT_DIR` -> `checkout_dir` (`bin:73`), default `.`.
- `GITEA_URL` / `GITEA_ORG` (`bin:64-65`).
- `RUNNER_TOKEN` -> `registration_token`, used only on first registration
  (`bin:66`, consumed at `bin:84-95`).
- `RUNNER_LABELS` (`bin:68`), default `terraphim-native`.
- `RUNNER_ACTIVE_REPOS` (`bin:43`) with the coexistence guard at `bin:46-52`:
  an empty allowlist is rejected unless `RUNNER_ACCEPT_ALL=1`.

`RunnerState::load`/`save` (`state.rs:35-57`) read and write the path the daemon
passes from `RUNNER_STATE_FILE`; the file is written `0600` (`state.rs:51-55`).
Each instance therefore persists its own identity to its own path.

The checkout root flows into `TaskWorker.checkout_dir` (`poller.rs:72`,
`task_worker.rs:43-49`) and is laid out as `<root>/<owner>/<repo>` by
`checkout::ensure_checkout` (`checkout.rs:108`, target =
`checkout_root.join(owner).join(repo)`).

**Critical contention point:** two instances sharing one checkout root would
share `<root>/<owner>/<repo>` working trees. Because `ensure_checkout` runs
`git fetch` then `git checkout --force --detach <sha>` (`checkout.rs:148-187`)
and reuses the on-disk clone across tasks (`checkout.rs:117` "reuse the on-disk
clone"), two concurrent tasks for the same repo at different shas would race on
the same working tree -- one would force-checkout out from under the other.
**Therefore the two instances MUST use different `RUNNER_CHECKOUT_DIR` roots.**
Likewise their `.runner` state files MUST differ (each is a distinct runner
identity registered separately with Gitea).

### 1.3 Registration is org-scoped and first-run only

`register` (`bin:84-108`) is taken only when `RunnerState::load` returns `None`
(`bin:79`). It POSTs `Register` with a runner name
`terraphim-native-<uuid>` (`bin:91`) and the declared labels (`bin:93`), then
persists the returned identity to `RUNNER_STATE_FILE` (`bin:108`). On every
subsequent start it loads the existing state and skips registration
(`bin:80-83`). The registration token is required only when no state file
exists (`bin:85-87`). The client targets the org runner endpoints via
`x-runner-uuid` + `x-runner-token` headers (`client.rs:80-84`).

### 1.4 sccache sharing is safe

The build env uses `RUSTC_WRAPPER=sccache` with `SCCACHE_*` settings. sccache is
designed for concurrent compiler invocations against a shared cache; two runner
instances sharing the same sccache cache is fine and is in fact a benefit (the
second instance gets warm-cache hits). No code in this crate touches sccache; it
is purely a child-process build env, so there is nothing to isolate per
instance.

### 1.5 FetchTask cursor semantics (relevant to Option B)

`fetch_task(state, tasks_version)` (`client.rs:117-128`) POSTs
`FetchTaskRequest { tasks_version }`. The poll loop seeds `tasks_version = 0`
(`poller.rs:86`) and advances it to the value the server returns on every
iteration (`poller.rs:88-89`, `resp.tasks_version` at `poller.rs:56` and
`:81`). This is a single monotonic cursor per runner identity. It is the
server's mechanism to tell the runner "your view of the task queue is stale,
re-poll"; it is **not** a per-task lease. A single in-process runner that wanted
to hold N tasks concurrently would have to manage N in-flight tasks against one
cursor, which the current loop is not structured to do (see Option B risks).

---

## 2. Recommendation: Option A (second runner instance)

**Recommended: Option A.** The code confirms that state file, checkout dir,
labels, org, and active-repos are all env-configurable per process
(section 1.2), and that registration is org-scoped and first-run only
(section 1.3). A second systemd `--user` service running the same binary with
its own `.runner` and its own checkout root, registered with a fresh org
registration token under the same `terraphim-native` label, gives Gitea two
same-label runners to distribute queued tasks across -- roughly 2x throughput --
with **zero code changes** and zero new failure surface in the task execution
path.

Option B (in-process concurrency) is the right long-term lever for scaling a
single host beyond what two processes give cheaply, but it is a non-trivial code
change with real isolation and protocol risks (section 4). Do A now; keep B as a
documented design for later.

### Why A is the fast win, concretely

- No recompile, no new tests, no protocol change. Same audited binary.
- Gitea already load-balances queued jobs across multiple runners that share a
  label; adding a second `terraphim-native` runner is the supported, idiomatic
  scaling path.
- The only true contention points are eliminated by configuration: distinct
  `RUNNER_STATE_FILE` and distinct `RUNNER_CHECKOUT_DIR` (section 1.2); shared
  sccache is beneficial, not a hazard (section 1.4).
- Trivially reversible: stop and disable the second unit; the first runner is
  untouched.

### Capacity caveat (size the host first)

bigbox already runs the orchestrator, agents, Gitea in Docker, and the existing
runner. Two concurrent native builds means up to two simultaneous
`cargo build`/`cargo test` workspaces (each capped at 7200s, `task_worker.rs:170`).
Confirm there is headroom in CPU, RAM, and disk before enabling instance 2 -- a
full workspace checkout plus `target/` can be tens of GB per repo, and two
instances keep two independent checkout trees. If the host cannot take two full
builds at once, A still helps for mixed workloads (one heavy build + one light
proof/docs job) but will not deliver a clean 2x on two simultaneous heavy
builds.

---

## 3. Option A: complete artefacts and commands

Naming convention for the second instance: suffix `-2`. The first instance is
unchanged throughout.

### 3.1 Second env file

Path: `/home/alex/.config/terraphim-gitea-runner/env-2`

This mirrors the existing `env` file with **two values changed** (state file and
checkout dir) and the registration token replaced by a fresh one. Everything
else (URL, org, label, active repos, build env) is identical so the second
runner is interchangeable with the first.

```sh
# /home/alex/.config/terraphim-gitea-runner/env-2
# Second native runner instance. IDENTICAL to `env` except:
#   - RUNNER_STATE_FILE   (own identity)
#   - RUNNER_CHECKOUT_DIR (own working trees -- MUST differ; see plan section 1.2)
#   - RUNNER_TOKEN        (fresh org registration token, first run only)

GITEA_URL=https://git.terraphim.cloud
GITEA_ORG=terraphim

# Fresh org registration token. A human supplies this from `op` (or the existing
# mechanism). Used ONLY on first start to register; ignored once .runner-2 exists.
# Leave it set for the first start, then it is harmless to keep (load short-circuits
# registration once state exists -- bin:79-83).
RUNNER_TOKEN=__SUPPLIED_BY_HUMAN_FROM_OP__

# Same label as instance 1 so Gitea distributes terraphim-native jobs across both.
RUNNER_LABELS=terraphim-native

# Same allowlist as instance 1 (copy verbatim from the existing `env`).
RUNNER_ACTIVE_REPOS=<csv of the same 8 repos as instance 1>

# DISTINCT identity file (MUST differ from instance 1's .runner).
RUNNER_STATE_FILE=/home/alex/.config/terraphim-gitea-runner/.runner-2

# DISTINCT checkout root (MUST differ from instance 1's work dir to avoid
# force-checkout races on shared <root>/<owner>/<repo> trees -- plan section 1.2).
RUNNER_CHECKOUT_DIR=/home/alex/.local/share/terraphim-gitea-runner/work-2

# --- Build env: copy VERBATIM from instance 1's `env` ---
# PATH (incl. ~/.cargo-runner-2/bin), RUSTC_WRAPPER=sccache, SCCACHE_* (shared
# cache is fine and beneficial -- section 1.4), AWS creds, etc.
PATH=/home/alex/.cargo-runner-2/bin:/usr/local/bin:/usr/bin:/bin
RUSTC_WRAPPER=sccache
SCCACHE_DIR=__SAME_AS_INSTANCE_1__
SCCACHE_CACHE_SIZE=__SAME_AS_INSTANCE_1__
AWS_ACCESS_KEY_ID=__SAME_AS_INSTANCE_1__
AWS_SECRET_ACCESS_KEY=__SAME_AS_INSTANCE_1__
# ... any other build vars present in instance 1's `env`, copied verbatim ...
```

Note on the `~/.cargo-runner-2/bin` PATH entry: the existing `env` already uses
`~/.cargo-runner-2`. That directory name is incidental to instance 1 and is fine
to share read-only across both instances (it is a toolchain/bin location, not a
mutable per-task state dir). The state and checkout dirs are the only paths that
MUST be unique.

**Action for the human:** copy the real `env` to `env-2`, then change only
`RUNNER_STATE_FILE`, `RUNNER_CHECKOUT_DIR`, and `RUNNER_TOKEN`. Do not hand-retype
the build env -- copy it to avoid drift.

```sh
# Suggested (human runs on bigbox):
cp /home/alex/.config/terraphim-gitea-runner/env \
   /home/alex/.config/terraphim-gitea-runner/env-2
# then edit env-2: set RUNNER_STATE_FILE=.../.runner-2,
#                  RUNNER_CHECKOUT_DIR=.../work-2,
#                  RUNNER_TOKEN=<fresh token from op>
chmod 600 /home/alex/.config/terraphim-gitea-runner/env-2
mkdir -p /home/alex/.local/share/terraphim-gitea-runner/work-2
```

### 3.2 Second systemd `--user` unit (full file)

Path: `~/.config/systemd/user/terraphim-gitea-runner-2.service`

This is the existing unit with the unit description and `EnvironmentFile`
pointed at `env-2`. ExecStart is the same binary. Adjust the boilerplate
(`After=`, `WantedBy=`, restart policy) to match the existing unit if it
differs; the load-bearing changes are `Description`, `EnvironmentFile`, and the
unit filename.

```ini
[Unit]
Description=Terraphim native Gitea Actions runner (instance 2)
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
EnvironmentFile=/home/alex/.config/terraphim-gitea-runner/env-2
ExecStart=/home/alex/.local/bin/terraphim-gitea-runner
Restart=on-failure
RestartSec=5
# Match any resource/sandbox directives present in instance 1's unit.

[Install]
WantedBy=default.target
```

**Diff from instance 1's unit (the only intended differences):**

```diff
-Description=Terraphim native Gitea Actions runner
+Description=Terraphim native Gitea Actions runner (instance 2)
-EnvironmentFile=/home/alex/.config/terraphim-gitea-runner/env
+EnvironmentFile=/home/alex/.config/terraphim-gitea-runner/env-2
```

ExecStart, restart policy, and all other directives stay identical. Confirm
against the real `terraphim-gitea-runner.service` on bigbox
(`systemctl --user cat terraphim-gitea-runner.service`) and carry over anything
this template omits.

### 3.3 Registration token (human supplies; do NOT fetch here)

The daemon registers itself on first start using `RUNNER_TOKEN` (section 1.3),
so the only manual API step is obtaining a fresh **org** registration token. The
human runs this with their own Gitea credentials/token from `op`; this document
deliberately does not fetch it.

```sh
# Human runs on bigbox (token from op / existing mechanism, NOT embedded here):
#   GITEA_TOKEN must be a token with org-admin rights for `terraphim`.
curl -fsS -X POST \
  -H "Authorization: token ${GITEA_TOKEN}" \
  "https://git.terraphim.cloud/api/v1/orgs/terraphim/actions/runners/registration-token"
# Response JSON contains the registration token; put its value into
# RUNNER_TOKEN in env-2.
```

The daemon then calls `Register` itself on first start (`bin:88-95`); there is no
separate manual register step. Each registration token is single-use for one
runner identity, so instance 2 needs its own fresh token (do not reuse instance
1's).

### 3.4 Enable and start

```sh
# Human runs on bigbox, after env-2 + token + unit file are in place:
systemctl --user daemon-reload
systemctl --user enable --now terraphim-gitea-runner-2.service

# Watch the first start register and declare:
journalctl --user -u terraphim-gitea-runner-2.service -f
# Expect log lines:
#   "registered new runner: RunnerState { ... name: terraphim-native-<uuid> ... }"
#   "declared; polling for tasks (labels=[\"terraphim-native\"])"
```

On the very first start the log should show registration (`bin:109`); on every
restart afterwards it should show "loaded existing runner state" (`bin:81`)
because `.runner-2` now exists.

### 3.5 Verification (both runners online, tasks distribute)

```sh
# Both runners should appear online under the same label.
curl -fsS \
  -H "Authorization: token ${GITEA_TOKEN}" \
  "https://git.terraphim.cloud/api/v1/orgs/terraphim/actions/runners"
# Expect two entries, both label terraphim-native, both status "online"/"idle".
```

Then confirm distribution: trigger CI on two repos (or two PRs) at the same time
and watch both `journalctl --user -u terraphim-gitea-runner.service` and
`-u terraphim-gitea-runner-2.service`. Each should pick up one job
("task complete: success=..." at `poller.rs:78`), proving Gitea assigned the two
queued jobs to the two runners rather than serialising them on one.

### 3.6 Rollback

```sh
systemctl --user disable --now terraphim-gitea-runner-2.service
# Optionally deregister instance 2 from Gitea (org runners admin UI/API) and
# remove .runner-2, env-2, work-2. Instance 1 is untouched throughout.
```

---

## 4. Option B: in-process concurrency (design sketch for later)

Goal: one runner process fetches and executes up to N tasks concurrently
(bounded), so a single registered runner saturates the host without operating
two systemd units.

### 4.1 Change surface

- **`src/poller.rs` (`run_forever`, `poll_once`).** Replace the
  "fetch one, await to completion, sleep" loop (`poller.rs:85-94`) with a
  bounded concurrency driver:
  - a `tokio::task::JoinSet<()>` (or a `Semaphore` of N permits) tracking
    in-flight tasks;
  - the loop fetches a task only while in-flight count < N, spawns
    `worker.run(...)` onto the set, and reaps finished handles;
  - the sleep becomes a "no task available OR at capacity" backoff rather than
    an unconditional per-iteration sleep.
  `poll_once` would no longer own the whole task lifecycle; it would fetch and
  hand off.
- **`src/task_worker.rs`.** `TaskWorker` is already constructed per task
  (`poller.rs:68`) and owns its own `SessionManager`, `WorkflowExecutor`, and
  host executor rooted at the resolved work dir (`task_worker.rs:158-172`), so
  it is mostly fine to run several concurrently. The unsafe shared resource is
  the **checkout tree**: two concurrent tasks for the same `owner/repo` at
  different shas would race in `ensure_checkout`
  (`checkout.rs:148-187`, force-detach). B must add per-`owner/repo`
  serialisation (e.g. a keyed async mutex / `DashMap<(owner,repo), Mutex>`) so
  same-repo checkouts never overlap, or give each in-flight slot its own
  checkout sub-root.
- **Cursor handling (`tasks_version`).** Today the cursor is a single monotonic
  value advanced once per iteration (`poller.rs:86-89`). With N concurrent
  fetches the loop must serialise `fetch_task` calls and advance the cursor on
  each, treating concurrency as "how many tasks may run", not "how many
  `fetch_task` calls are in flight". Keep one fetch in flight at a time; only the
  execution fans out.
- **Log streaming (`src/logs.rs`, used at `task_worker.rs:210-235`).** Each
  `LogStreamer` is per task (`LogStreamer::new(task.id)`, `task_worker.rs:204`),
  so log rows are already keyed by task id and will not interleave incorrectly
  on the wire. No change needed beyond confirming the shared `client`
  (`Arc<C>`) tolerates concurrent `update_log`/`update_task` calls -- it does,
  as `ReqwestRunnerClient` is stateless per call (`client.rs:70-104`).
- **Session isolation.** Each task builds its own `SessionManager` +
  `SessionId::new()` (`task_worker.rs:158-178`), so sessions are already
  isolated. Verify `HostCommandExecutor` and `HostVmProvider` have no shared
  mutable global that two concurrent host executions would corrupt (e.g. cwd,
  env mutation, temp paths).

### 4.2 Risks

- **Worktree races** (highest): same-repo concurrent checkout corrupts the
  working tree. Mandatory mitigation: per-`owner/repo` lock or per-slot checkout
  root.
- **Cursor semantics**: advancing `tasks_version` incorrectly under concurrency
  could cause missed or repeatedly-fetched tasks. Keep fetch single-flight.
- **Resource exhaustion**: N concurrent `cargo` builds (each up to 7200s,
  `task_worker.rs:170`) can OOM or thrash disk. N must be small and host-sized.
- **Log/result ordering**: keyed by task id today, so low risk, but must be
  re-verified once `run` is spawned rather than awaited inline.
- **Graceful shutdown**: in-flight tasks must drain on SIGTERM; today shutdown
  is trivial because only one task runs.

### 4.3 Test impact

- New unit tests for the bounded driver: a fake `GiteaRunnerClient`
  (the trait is already mockable -- it backs the existing tests) that serves M
  tasks and asserts at most N run concurrently and all M complete.
- A test asserting same-repo tasks serialise (no overlapping `ensure_checkout`).
- Reuse the existing `checkout.rs` test harness (`checkout.rs:192-345`) to add a
  concurrent-checkout race test against a local file remote.
- Integration: confirm `tasks_version` advances correctly when several tasks are
  in flight.
- This is a meaningfully larger change than A and should go through the normal
  TDD + review path before deployment.

---

## 5. How to verify 2x throughput

The aim is to show that two same-label runners (Option A) roughly halve
wall-clock time for a batch of queued jobs versus one runner.

1. **Baseline (one runner).** With only instance 1 running, queue a fixed batch
   of K independent CI jobs (e.g. trigger `native-ci` on K repos/PRs at once).
   Record wall-clock from first job queued to last job's commit status going
   green. Because the loop is strictly sequential (`poller.rs:85-94`), expect
   total time approximately the sum of the K job durations.
2. **Two runners.** Enable instance 2 (section 3.4). Queue the identical batch.
   Record the same wall-clock.
3. **Compare.** With two runners and K >= 2 independent jobs of comparable
   duration, expect total wall-clock to drop toward ~50% of baseline (ideal),
   bounded by host CPU/RAM/disk contention (section 2 caveat). A clean 2x needs
   the host to absorb two concurrent builds; otherwise expect a partial speed-up.
4. **Confirm distribution, not luck.** During the two-runner batch, tail both
   journals; each runner should log interleaved "task complete: success=..."
   lines (`poller.rs:78`), and the Gitea Actions UI should show jobs running on
   two distinct runner names (`terraphim-native-<uuid-1>` and
   `terraphim-native-<uuid-2>`). If only one runner ever picks up work, the
   label/allowlist on instance 2 is misconfigured (re-check
   `RUNNER_LABELS=terraphim-native` and `RUNNER_ACTIVE_REPOS`).
5. **Sustained check.** Leave both running for a normal day of CI and compare
   mean queue wait time (time from job queued to job started) against the
   single-runner baseline; it should fall materially.

---

## 6. Summary of human action items (Option A)

1. Obtain a fresh org registration token from `op` / existing mechanism
   (section 3.3). Do not reuse instance 1's token.
2. `cp env -> env-2`; change only `RUNNER_STATE_FILE` (`.runner-2`),
   `RUNNER_CHECKOUT_DIR` (`work-2`), `RUNNER_TOKEN` (fresh). `chmod 600`.
   `mkdir -p` the new work dir (section 3.1).
3. Create `terraphim-gitea-runner-2.service` from the template; verify it
   differs from instance 1 only in `Description` + `EnvironmentFile`
   (section 3.2).
4. `daemon-reload`; `enable --now`; watch the journal for register + declare
   (section 3.4).
5. Verify both runners online via the org runners API and confirm two
   simultaneous jobs distribute across them (section 3.5).
6. Measure throughput per section 5; keep Option B (section 4) on file for when
   two processes are no longer enough.
