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
  "apple_container_image": "python:3.11-slim"
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

- Every CLI call is an argument vector. No host shell is ever constructed, so
  guest code cannot escape into host-side word splitting. Python is passed as one
  argv element after `python3 -c`; shell commands as one argv element after
  `bash -lc`, interpreted by bash **inside the guest**.
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
   the elapsed time.

A timeout therefore never leaves work running inside a container that a later
call would reuse — at the cost of losing that session's in-container state.

## Vanished containers

If a session's container disappears between calls (removed by hand, service
restarted, host slept), `container exec` fails with the CLI's own
"container not found". The backend recognises *CLI-originated* absence errors
only — a guest's `bash: foo: command not found` is a guest failure and is passed
through untouched — clears the affinity mapping, and retries the call **once**
against a fresh container. Session state is lost, but the session is not wedged.
A second consecutive disappearance is reported as `RlmError::ExecutionFailed`
rather than as a guest exit code, so callers do not retry it as if the guest had
misbehaved.

## Cleanup and recovery

`end_session` stops the container with a 5s grace period and then deletes it;
an already-absent container counts as success. `cleanup()` attempts **every**
tracked session even if one removal fails, and reports an aggregate error
afterwards.

`cleanup()` empties the session map *before* attempting removal, so a container
whose deletion fails becomes untracked: `Drop` will not retry it, and it is
named only in the returned error and the warning log. Recover those with the
prefix sweep below.

`Drop` is best effort and does not claim success: outside a Tokio runtime it logs
the leaked names with a ready-to-run recovery command. Because every name is
prefixed, manual recovery is:

```bash
container list --all --format json | grep terraphim-rlm-
container delete --force terraphim-rlm-<ulid>
```

## Testing

Portable tests (Linux included) use an injected fake process runner and pin the
argv contract, exactly-once creation, timeout teardown, and cleanup:

```bash
cargo test -p terraphim_rlm --features apple-container-backend
```

**A fake-runner pass is not evidence that the Apple runtime works.** Real
evidence requires an Apple-silicon macOS 26 host with the service started and the
image present:

```bash
container system version --format json
container system status --format json
cargo test -p terraphim_rlm --features apple-container-backend \
    --test backend_demo apple_container -- --ignored --nocapture
container list --all --format json    # expect no terraphim-rlm-* resources
```

Tests never pull images implicitly; pull `python:3.11-slim` explicitly first if
it is not cached.
