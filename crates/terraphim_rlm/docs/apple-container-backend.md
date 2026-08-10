# Apple Container RLM backend

`terraphim_rlm` can execute RLM code and shell commands inside Apple's
`container` runtime. Apple starts **one lightweight Linux VM per container** on
top of `Virtualization.framework`, so on Apple silicon this gives an isolation
model closer to Firecracker's per-workload VM than to Docker Desktop's single
shared Linux VM.

## Requirements

- Apple silicon (`aarch64`) Mac
- macOS 26
- Apple `container` CLI, installed by the operator:

```bash
brew install container
container system start   # one-time; starts the API service
```

`container system start` is **never** run by Terraphim. It is host
administration — it may install a kernel or prompt for approval — so lifecycle
ownership of the service stays with the operator.

The backend is compiled in by the `apple-container-backend` feature, which is
part of `full` (and therefore of the default feature set). It adds no
third-party Rust dependency, so Linux and Intel-Mac builds are unaffected.

## Configuration

| Field | Default | Meaning |
| --- | --- | --- |
| `apple_container_binary` | `None` → `container` from `PATH` | Absolute path to the CLI |
| `apple_container_image` | `python:3.11-slim` | OCI image for session containers |
| `apple_container_cpus` | `1` | `container run --cpus` per session container |
| `apple_container_memory` | `512M` | `container run --memory` per session container |

All four are optional in serialized config: a config written before this backend
existed still loads and gets exactly the defaults above. `RlmConfig::validate()`
rejects an empty image, `apple_container_cpus == 0`, and any
`apple_container_memory` that is not a positive integer with an optional
`K`/`M`/`G`/`T`/`P` suffix — the suffix set Apple's `--memory` documents. A typo
like `512MB` is caught there rather than surfacing as a `container run` failure
that names neither the field nor the cause. The value is forwarded to the CLI
**verbatim**, so surrounding whitespace is rejected rather than trimmed: `"
512M "` fails validation instead of passing it and then reaching `container` as
a different string than the one that was validated.

The backend appears in `backend_preference` between E2B and Docker:

```text
Firecracker → E2B → AppleContainer → Docker → Local
```

Its session model is `Affinity`: one container per RLM session, so state written
in one call is visible to the next.

```json
{
  "backend_preference": ["apple-container", "docker", "local"],
  "apple_container_binary": "/opt/homebrew/bin/container",
  "apple_container_image": "python:3.11-slim",
  "apple_container_cpus": 1,
  "apple_container_memory": "512M"
}
```

`applecontainer` is accepted as a deserialization alias; `apple-container` is the
canonical spelling that is written back out.

## Availability and fallback diagnostics

Selection requires **positive** evidence, all of:

1. the build targets macOS on `aarch64` — on any other platform the CLI is not
   even spawned;
2. the `container` binary runs and `container system version --format json`
   exits zero;
3. `container system status --format json` exits zero.

The status *payload* is advisory only. Apple documents that `--format json`
exists but not the document it produces, so the exit status is the sole
authoritative signal; anything notable in the JSON (an unrecognised shape, a
sub-entry that looks stopped, non-JSON output) is logged at `debug` and never
vetoes selection. Vetoing on a guessed schema would silently fall through to the
next backend — on a Mac without Docker, all the way to `LocalExecutor`, which
has **no isolation at all**. That is the wrong direction to fail.

Each failure records a distinct reason, and `select_executor` continues to the
next backend. Run with `RUST_LOG=debug` to see them, or read the
`NoBackendAvailable { tried }` error:

| Reason fragment | Fix |
| --- | --- |
| `unsupported platform` | Expected off Apple-silicon macOS; another backend is used |
| `not runnable: ... No such file` | `brew install container` |
| `container system version failed` | CLI is installed but broken — reinstall |
| `container system status failed` | `container system start` |

On Linux this backend is always skipped, so Linux's effective backend choice is
unchanged.

## Isolation and security profile

Each session container is created as:

```bash
container run --detach --name terraphim-rlm-<ulid> \
  --cpus 1 --memory 512M --cap-drop ALL \
  python:3.11-slim sleep infinity
```

(`--cpus`/`--memory` are `apple_container_cpus`/`apple_container_memory`; the
values shown are the defaults.)

- Every CLI call is an argument vector. No host shell is ever constructed, so
  guest code cannot escape into host-side word splitting. Python is passed as one
  argv element after `python3 -c`; shell commands as one argv element after
  `bash -c`, interpreted by bash **inside the guest**. `-c` and not `-lc`: a
  login shell sources the image's profile scripts, which could rewrite `PATH`
  and the environment passed in via `--env`, so the same command would behave
  differently per image.
- Container names are derived from a fresh ULID, never from user input.
- **No** host directory mounts, home directory, SSH agent forwarding, Docker
  socket, registry credentials, or inherited host environment.
- Outbound networking is left at the CLI default, matching the Docker backend
  (needed for the LLM bridge and `pip`). **DNS allowlist enforcement is not
  implemented** by this backend — the same gap the Docker backend has. Do not
  read `dns_allowlist` as an enforced control here.
- `ExecutionContext::working_dir` and `env_vars` are forwarded as `--workdir`
  and one `--env key=value` per variable (sorted by key), each a single argv
  element — the same fields `LocalExecutor` and `SshExecutor` honour. Nothing
  else from the host environment is inherited.
- `ExecutionContext::max_output_bytes` is **not enforced**: stdout and stderr
  are accumulated in host memory until the command ends or its deadline fires.
  A guest that prints gigabytes grows the host process accordingly. This is
  parity with the Docker backend, not a control; bound untrusted output with
  `timeout_ms` instead.
- Snapshot operations return `RlmError::NotSupported { backend: "apple-container" }`.
  `container export` can capture a filesystem, but the RLM trait promises broader
  state restoration, so support is not claimed.

## Timeout behaviour (fails closed)

When a call exceeds `ExecutionContext::timeout_ms`:

1. the host `container exec` child is killed and reaped;
2. the session's container is force-removed — a guest exec process cannot
   otherwise be proven dead;
3. the session→container mapping is cleared, so the next call gets a fresh VM;
4. `ExecutionResult::timeout(partial_stdout, partial_stderr)` is returned with
   the elapsed time **and the usual `backend`/`container` metadata**, so a
   timed-out call can still be correlated with the container it ran in and with
   the recovery that followed.

A timeout therefore never leaves work running inside a container that a later
call would reuse — at the cost of losing that session's in-container state.

The same applies to a `container exec` that fails with a host **I/O error**
rather than an exit code. Such an error can arise after the CLI child was
already spawned — while waiting on it, for instance — so it is no proof that the
guest process never started. It is treated exactly like a timeout: the container
is unbound and force-removed under the execution's permit, and
`RlmError::ExecutionFailed` names the container and says it was discarded. The
command is not re-run. Pinned by
`a_runner_io_error_after_exec_started_destroys_the_container`,
`a_failed_io_error_recovery_delete_blocks_reuse_and_stays_tracked` and
`cleanup_cannot_overtake_an_io_error_recovery`.

## Vanished containers

If a session's container disappears between calls (removed by hand, service
restarted, host slept), `container exec` fails with the CLI's own
"container not found". The backend then:

1. unbinds the container from the session (compare-and-clear, see below);
2. force-removes it; and
3. returns `RlmError::ExecutionFailed` naming the container, rather than a guest
   exit code the caller might mistake for the command's own result.

**The command is never re-run.** The backend issues exactly one `container exec`
per call, always. Replaying would mean executing the caller's command twice, and
the only available trigger for a replay is `container exec` stderr — which is
mixed with output the guest writes byte for byte, and which names a container
whose name the caller already knows, because every `ExecutionResult` returns it
in `metadata["container"]`. Any non-idempotent action the command performed
before that text appeared would happen a second time. Whether re-issuing a
particular command is safe is knowledge the caller has and this backend does
not, so the decision is left to the caller: the session is clean and unwedged,
and the next command it chooses to send starts in a fresh container.

The "container missing" match is therefore a **recovery heuristic, not
provenance**. Its whole effect is discarding the session's own container, so a
caller that echoes the disclosed name back through guest stderr achieves nothing
it could not achieve by killing its own container — no replay, no amplification.

- **The absence phrase must be accompanied by the container name.** This is what
  keeps ordinary guest failures (`bash: foo: command not found`, exit 127) from
  needlessly destroying a healthy session. An `Error:` prefix on its own is not
  accepted. A genuine CLI absence message that omits the name is passed through
  as an ordinary failure; that direction is safe.
- **The unbind is a compare-and-clear under the session mutex.** The mapping is
  cleared only if it still points at the container that failed, so a stale
  failure arriving after a concurrent call already installed a replacement
  cannot unbind that replacement.
- **The observed container is force-removed either way.** Names are ULIDs and
  are never reused, and `delete --force` treats an absent container as success —
  so this is a no-op when the container really vanished, and closes the leak if
  the report was wrong.

## Cleanup and recovery

`end_session` stops the session's container(s) with a 5s grace period and then
deletes them; an already-absent container counts as success. It is **terminal
for that session id**, and a deletion failure is **returned** to the caller
rather than logged and swallowed. `cleanup()` attempts **every** tracked session
even if one removal fails, and reports an aggregate error afterwards.

The public `TerraphimRlm::destroy_session` propagates that failure too: it
destroys the logical session and then returns the executor's error, so a caller
is never told a session was cleanly destroyed while its VM may still be running.
Retry ownership stays with the executor — the container remains tracked and
`cleanup()` retries it — so the recovery is `cleanup()`, not calling
`destroy_session` again (which is terminal for the session id).

### Teardown cannot be raced into leaking a container

Each session's map entry holds an explicit lifecycle state — `Active { bound,
pending }` or `Closing { pending }` — rather than a bare optional name. `bound`
is the container the session executes in; `pending` holds every name whose
deletion has been started but not confirmed. That distinction is what keeps
teardown safe against a concurrent execution:

- `end_session` marks the session `Closing` **while holding the same per-session
  mutex** and while the entry is still reachable from the map, then stops and
  deletes the container. The `Closing` entry is **kept in the map as a
  tombstone**: a caller that resolved the entry before teardown began, *and* one
  that looks the session up afterwards, both find `Closing` and **refuse to
  create**. A session id that has been ended is never resurrected. The refusal
  is a `BackendInitFailed`, not a guest exit code: no command ran.
- `end_session` holds an owned **read** permit on the executor-wide lifecycle
  gate for its whole stop-and-delete, and `cleanup()` takes the **write** side.
  So cleanup waits for an in-flight session deletion instead of clearing the map
  out from under it and reporting terminal success while a delete is still
  outstanding.
- **Failures stay tracked.** `pending` retains a container until the runtime
  confirms it gone, so a failed `end_session` delete is retried and aggregated
  by `cleanup()`, and a container `cleanup()` itself could not remove is
  re-inserted so a repeated `cleanup()` (and `Drop`) still sees it. A container
  this backend created is never dropped from tracking while it may still exist.
- **A name is tracked before it can name anything.** The generated name is put
  in `pending` *before* `container run` is spawned, so a `container run` that
  fails or times out — which may still have created a container — leaves a
  tracked name rather than an anonymous VM. The failed creation is force-deleted
  immediately; if that delete also fails, the name stays pending.
- **No replacement while a name is unconfirmed.** A session whose slot has any
  pending name refuses to create a container (`BackendInitFailed`, no command
  ran). One unconfirmed container per session is already one too many, and
  allowing a replacement is what previously forced a failed deletion to choose
  between orphaning a live replacement and losing the old container. Recovery is
  `cleanup()`, or the manual prefix sweep below.
- **One permit, acquired before the work and held through its recovery.** This
  is the whole lifecycle rule, and it replaces the recovery registry and bounded
  join rounds an earlier revision used. An owned read permit on the lifecycle
  gate is acquired *before* an operation can create, use or reclaim a container
  — before `container run`, before `container exec` — and it is held, by the
  owner of the work, until that operation **and every recovery it triggers**
  has finished. Timeout, vanished-container, runner-I/O-error,
  panicked-exec and cancellation recovery all run the same unbind →
  force-delete → retrack path under the permit their execution already held, so
  none of them can start after cleanup has drained. The cell is (re-)inserted into the session map, so
  nothing is restored into a cell terminal cleanup has detached.
- `cleanup()` is executor-wide and terminal. It raises a closing flag, then
  takes the **write** side. Acquiring it is by itself the proof: a write cannot
  be taken while a single read permit is outstanding, so by the time cleanup
  holds it, every execution, creation, `end_session` and recovery that started
  earlier has run to completion. There is nothing to register, nothing to join
  and no round limit — and equally, once cleanup returns, no permit-holding
  operation from before it exists, so **the drained map cannot be repopulated**.
  Any operation that starts later blocks on the gate and then sees the closing
  flag and refuses, before it could insert a map entry. `end_session` is part of
  that rule rather than an exception to it: it reads the closing flag *after*
  acquiring its permit, so one that arrives after cleanup — or that was queued
  behind cleanup's write gate — inserts no tombstone into the drained map. It
  still tears down whatever is *already* tracked, which is how a survivor
  cleanup re-inserted after a failed delete keeps being reclaimable. There is no
  interleaving
  in which a created container ends up untracked, including the concurrent
  insertion case where the session was not in the map when cleanup looked.
  The cost of this simplicity is honest and intended: `cleanup()` waits for
  in-flight executions, so it can take up to `ExecutionContext::timeout_ms` plus
  the recovery's lifecycle timeout to return.
- Lock order is always **permit before slot**; nothing takes the gate while
  holding a slot mutex, and no task holding a permit ever asks for a second one
  (recoveries inherit their execution's), so neither deadlock is possible.
- After `cleanup()` the executor is deliberately not reusable: it creates no
  further session containers. Build a new executor instead.

Each schedule above is pinned by a *deterministic* test — a gated fake runner
parks the CLI call at the exact lifecycle point, and the racing operation is
driven to completion (or polled and asserted pending) from there. None of these
tests relies on sleeping or on repetition:
`end_session_in_flight_refuses_a_racing_ensure_and_stays_terminal`,
`cleanup_waits_for_an_in_flight_creation_and_then_deletes_it`,
`cleanup_waits_for_an_in_flight_end_session_deletion`,
`cleanup_forced_between_abandonment_unbind_and_delete_reclaims_the_container`,
`cleanup_stays_pending_until_a_cancellation_recovery_completes`,
`a_failed_end_session_delete_is_returned_and_retried_by_cleanup`,
`a_failed_abandonment_delete_blocks_a_replacement_and_stays_tracked`,
`a_partially_created_container_that_cannot_be_deleted_stays_tracked`,
`cleanup_retains_a_container_it_could_not_delete`, and
`ensure_holding_a_stale_slot_refuses_to_create_after_end_session`.

The schedules that specifically exercise "cleanup versus an operation that has
not yet asked for the gate" — the ones a registry-based design could not close —
are pinned separately:

- `cleanup_started_in_the_same_tick_as_the_cancellation_drop_still_waits`
  starts `cleanup()` in the same tick as `ExecCancelGuard::drop`, before the
  owner has taken a single recovery step, and asserts it is pending.
- `cleanup_queued_during_exec_waits_for_timeout_recovery`,
  `..._for_vanished_container_recovery` and `..._for_panicked_exec_recovery`
  queue `cleanup()` for the write side **while `container exec` is still
  running**, then let the exec complete into each recovery path and assert
  cleanup is *still* pending once the recovery has started its force-delete.
- `cleanup_waits_for_far_more_cancellations_than_the_old_bounded_rounds`
  cancels twelve simultaneous executions — comfortably past the eight rounds the
  removed bounded-join mechanism allowed — and asserts cleanup waits for all of
  them and deletes every container.
- `nothing_can_reinsert_a_cell_after_cleanup_returns` asserts the postcondition
  directly: after `cleanup()` returns, the map stays empty across repeated
  scheduler yields, and a caller arriving afterwards is refused before it could
  insert a session cell.
- `end_session_after_cleanup_does_not_repopulate_the_map` and
  `end_session_queued_behind_cleanup_leaves_the_map_empty` assert the same
  postcondition for the teardown path — the second by parking `cleanup()` inside
  its delete so `end_session` is genuinely queued behind the write gate and
  resumes only after cleanup returned — while
  `end_session_after_cleanup_still_retries_a_survivor_cleanup_could_not_delete`
  pins that refusing to *insert* did not cost the ability to reclaim a container
  that is already tracked.

### Execution ownership is runtime-independent from the start

An execution is not owned by the future the caller awaits, and not by the
runtime polling it. Once the lifecycle permit is held and the container is
resolved — and **before the guest command is launched** — the permit and the
entire `container exec` operation are moved into a dedicated **execution owner**
running on its own OS thread with its own current-thread Tokio runtime. The
owner owns:

- the `ProcessRunner` call, and therefore the CLI child and its stdout/stderr
  drain tasks;
- the outcome (success, non-zero exit, timeout, vanished container, host I/O
  error, panicked exec task); and
- every recovery those paths need, plus cancellation recovery.

The caller keeps only a result channel. Establishing ownership *first* is what
makes its failure trivial: if the current-thread runtime cannot be built or the
OS thread cannot be spawned, the owner — permit included — is dropped, no
`container exec` was ever issued, and the caller gets a `BackendInitFailed`
naming the container. There is nothing to fail closed over, because no guest
command exists yet. Container *resolution* happens before this point, so a
fresh session may already have had its container created by `container run`;
that container stays tracked, bound and reusable exactly like any other session
container. That is the whole reason ownership is established up front
rather than manufactured inside `Drop`: a `Drop` that must create an owner has
branches on which it cannot, and each of those branches would have to choose
between releasing the permit early and leaking it.

### Cancelling an execution fails closed

`kill_on_drop(true)` kills the host `container exec` child, but killing the CLI
does not prove the guest process died. So dropping or aborting an
`execute_code`/`execute_command` future must run the same recovery the
elapsed-time timeout does — and, because the owner already exists, dropping the
future does not have to *arrange* any of it. Synchronously, before the drop
returns, the guard:

1. claims the execution's outcome, so a completion that already settled is not
   disturbed and a cancellation cannot be settled twice;
2. **quarantines** the session slot, so no later call can reuse that container;
   and
3. raises a cancellation signal on the in-flight CLI call.

It spawns nothing, moves nothing and registers nothing. The permit was acquired
before `container exec` was launched and has never left the owner, so there is
no window — however small — in which `cleanup()` could acquire the write side.

This holds **independently of every Tokio runtime**, not just of the thread the
future is dropped on. An execution future is `Send`, so it can be moved out of
the runtime and dropped on a plain OS thread where `Handle::try_current()` would
fail — and the runtime that started the execution may itself already have been
shut down by then. Neither event touches the owner: it was never a task on that
runtime, so its child is not dropped, its drain tasks are not dropped, and its
permit is not released. The earlier design, which transferred a `JoinHandle`
after the fact, could only observe that the CLI *future* had been destroyed by
runtime shutdown; it could not show the child had been reaped or the drains
joined. Now there is nothing to transfer.

The owner then finishes the operation, and its completion is observable:

4. it awaits the CLI call — unconditionally, with no timeout, no abort and no
   drop of the future that holds the child and the drain handles. The process
   runner returns only after it has killed **and reaped** its child and stopped
   its stdout/stderr readers, so termination is *observed* rather than assumed.
   There is no bounded backstop, deliberately: aborting the call would destroy
   the handles that carry the proof and would leave the owner deleting the
   container and releasing the permit on an assumption. `kill_on_drop` requests
   termination; it does not promise a reap.

   Prompt termination is therefore a hard requirement on `ProcessRunner`, not a
   best effort. A runner (or an OS) that never returns keeps the execution's
   lifecycle permit held indefinitely, so `cleanup()` blocks and the backend
   **fails closed** rather than reporting a terminal state it has not observed.
   A hung teardown is the intended, diagnosable consequence of a violated runner
   contract; a false "all clean" is not.
5. it force-deletes the container under the 60s lifecycle timeout, still holding
   the permit, with the name tracked as pending first — so `cleanup()` waits for
   it and a failed delete stays reclaimable;
6. only once the runtime confirms the container gone does it drop the name from
   tracking and lift the quarantine, letting the session start a *fresh*
   container. If the delete fails, the session stays refused and the container
   stays tracked for `cleanup()` to retry.

Nothing on this path is detached, and the permit is released only when the owner
returns — so terminal cleanup, **on any runtime**, cannot return while a
cancelled execution's child termination or container deletion is outstanding,
whichever thread the cancellation happened on and whether or not the runtime
that started the execution still exists.

Pinned by `aborting_an_execution_future_fails_the_session_closed`,
`a_cancelled_execution_marks_its_slot_unusable_before_the_drop_returns`,
`cleanup_started_in_the_same_tick_as_the_cancellation_drop_still_waits`,
`cleanup_waits_for_far_more_cancellations_than_the_old_bounded_rounds`,
`cleanup_stays_pending_until_a_cancellation_recovery_completes` and
`a_future_dropped_off_the_runtime_still_blocks_cleanup_until_recovery_ends` —
which polls the execution inside the runtime, moves it to an OS thread that
asserts it has no current runtime, drops it there, and then shows `cleanup()`
pending until the CLI call has ended (the runner reports it returned on the
cancellation signal) and the container deleted — and by
`recovery_outlives_the_shutdown_of_the_runtime_that_started_the_execution`,
which uses two explicit runtimes: it polls the execution to pending on runtime
A, shuts A down entirely, drops the outer future on a non-runtime thread, and
then runs `cleanup()` on a separate runtime B, showing it pending until the
owner's own thread has deleted the container, with no re-inserted cell and no
untracked container afterwards. It also asserts that runtime A's shutdown
abandoned **no** CLI call, which is the property the owner-first design adds.

Those two run on the fake runner. The claim they cannot make — that a real host
child was reaped and real drains stopped across a runtime shutdown — is made by
two real-process tests:

- `cancelling_a_real_child_kills_and_reaps_it_and_stops_the_drains` cancels a
  live `/bin/sh`, then asserts the child pid is gone from the process table
  (`kill -0` fails — it would still succeed for a zombie, so this is reap
  evidence) and that the call returned promptly even though a surviving
  grandchild still holds the stdout pipe open, which is what the readers being
  terminated buys.
- `a_real_child_is_reaped_before_cleanup_on_another_runtime_can_finish` runs the
  full two-runtime schedule against a real child: the guest command is an actual
  `/bin/sh` process started while runtime A exists, runtime A is shut down (and
  the child is asserted **still alive**, proving A never owned it), the outer
  future is dropped on a non-runtime thread, and `cleanup()` runs on runtime B.
  When the owner's gated `container delete` starts, the test asserts the child
  pid is absent from the process table — zombies included — and that the
  grandchild still holds the pipe, so both reaping and drain termination
  happened before the container was deleted; `cleanup()` is asserted pending
  until the delete is released.

The complementary claim — what happens when a runner *violates* the contract —
is pinned by `a_runner_that_withholds_its_return_blocks_recovery_and_cleanup_forever`.
Its runner ignores the cancellation signal entirely and returns only on an
explicit test notification. With virtual time advanced far past the five-second
grace the old abort backstop used, the test asserts that no `container delete`
has started, that `cleanup()` is still pending, and that the runner's future was
never dropped or aborted. Only when the runner finally returns do the recovery,
the delete and `cleanup()` complete. Blocking is the specified behaviour here,
not a bug to be timed out of.

`Drop` is best effort and does not claim success. It logs a warning
**unconditionally, before** attempting anything — every path through it is best
effort, including the common one where names are resolved and handed to a
detached cleanup task whose outcome is never observed — and outside a Tokio
runtime it also logs the leaked names with a ready-to-run recovery command.
Because every name is prefixed, manual recovery is:

```bash
container list --all --format json | grep terraphim-rlm-
container delete --force terraphim-rlm-<ulid>
```

## Testing

Portable tests (Linux included) use an injected fake process runner and pin the
argv contract, exactly-once creation, timeout teardown, cleanup, and every
lifecycle race listed above. A few tests deliberately use **real local
processes** instead, because host-child kill/reap and reader termination cannot
be demonstrated by a fake runner:
`tokio_runner_timeout_kills_and_reaps_child_preserving_partial_output`,
`tokio_runner_captures_exit_code_and_streams`,
`cancelling_a_real_child_kills_and_reaps_it_and_stops_the_drains`, and
`a_real_child_is_reaped_before_cleanup_on_another_runtime_can_finish`.

```bash
cargo test -p terraphim_rlm --features apple-container-backend
```

**A fake-runner pass is not evidence that the Apple runtime works.** Real
evidence requires an Apple-silicon macOS 26 host with the service started and the
image present:

```bash
container system version --format json
container system status --format json
container image pull python:3.11-slim   # see below

# Direct executor: argv contract, affinity, timeout teardown, no leaks.
cargo test -p terraphim_rlm --features apple-container-backend \
    --test backend_demo demo_apple_container_executor -- --ignored --nocapture

# Through select_executor(), the path production takes.
cargo test -p terraphim_rlm --features apple-container-backend \
    --test backend_demo apple_container_via_select -- --ignored --nocapture

container list --all --format json    # expect no terraphim-rlm-* resources
```

Both real-host tests must be run: the direct one proves the executor works, and
`demo_apple_container_via_select_executor` proves `select_executor` actually
*chooses* this backend and drives it end to end (it offers Apple Container as
the only preference, so a silent fall-through fails rather than passes).

Pull the image first. The demos do not pull it themselves, but `container run`
**does** pull implicitly when the image is not cached — and that pull runs
inside this backend's 60s lifecycle timeout, so an uncached image usually shows
up as a `container run` failure rather than as a slow first call. Pre-pulling
keeps the run measuring the backend instead of the network.
