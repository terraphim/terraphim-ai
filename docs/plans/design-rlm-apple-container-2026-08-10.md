# Apple Container RLM Backend Design Gate

> **For ADF:** Implement issue #3192 task-by-task; preserve the real Apple-silicon merge gate.

**Goal:** Add an argv-safe, session-affine Apple Container CLI backend to `terraphim_rlm` for Apple-silicon macOS 26.

**Architecture:** A feature-gated Rust CLI adapter implements `ExecutionEnvironment`, mirrors Docker's per-session affinity, positively probes `container system status`, and destroys the session container on timeout. Portable tests use an injected fake process runner; real runtime evidence remains mandatory on Apple silicon.

**Tech Stack:** Rust, Tokio process APIs, Apple `container` CLI, OCI `python:3.11-slim`.

---

## Goal

Add an **Apple Container execution backend** to `crates/terraphim_rlm` so `terraphim-grep` RLM work can run in per-container lightweight Linux VMs on Apple-silicon Macs using the CLI installed by:

```bash
brew install container
container system start
```

The backend must preserve the existing `ExecutionEnvironment` contract, session affinity, timeout/result semantics, capability reporting, KG validation, deterministic fallback, and explicit cleanup. It must not weaken Linux behavior or silently fall back to unisolated local execution without the existing warning.

## Why this backend

Apple's `container` CLI runs **one lightweight VM per Linux container**, uses Apple `Virtualization.framework`, consumes/produces OCI images, and is designed for Apple silicon. This gives macOS an RLM isolation path closer to Firecracker's per-workload VM model than Docker Desktop's shared Linux VM.

Authoritative evidence checked 2026-08-10:

- Apple README: per-container lightweight Linux VMs, OCI images, Apple silicon, supported on macOS 26: https://github.com/apple/container/blob/main/README.md
- Technical overview: one VM per container; Virtualization.framework, vmnet, XPC, launchd, Keychain; per-container runtime helper: https://github.com/apple/container/blob/main/docs/technical-overview.md
- CLI reference: `run`, `exec`, CPU/memory, capability drop, read-only root, network, volume, `--rm`, stop/kill, and JSON status contracts: https://github.com/apple/container/blob/main/docs/command-reference.md
- Low-level Swift package: https://github.com/apple/containerization
- Homebrew formula: https://formulae.brew.sh/formula/container (live API showed `1.2.2`, bottle `arm64_tahoe` only)
- Current Apple release inspected: https://github.com/apple/container/releases/tag/1.2.2 (published 2026-08-08)

## Current Terraphim ground truth

The extension belongs in `terraphim-ai`, not the standalone `terraphim-grep` client crate. `terraphim-grep` delegates recursive execution to `terraphim_rlm`.

- `crates/terraphim_rlm/src/executor/trait.rs` defines `ExecutionEnvironment`: code/bash execution, validation, snapshots, capabilities, health, cleanup, and per-session teardown.
- `crates/terraphim_rlm/src/executor/docker.rs` is the closest lifecycle model: one long-running `python:3.11-slim` container per `SessionId`, `python3 -c`/`bash -c` through exec, timeout/result capture, and cleanup.
- `crates/terraphim_rlm/src/executor/mod.rs` probes and selects `Firecracker -> E2B -> Docker -> Local` by preference and availability.
- `crates/terraphim_rlm/src/config.rs` owns `BackendType`, default preference, and session affinity.
- `crates/terraphim_rlm/Cargo.toml` gates Docker/Firecracker/E2B backends by feature.
- `crates/terraphim_rlm/tests/backend_demo.rs` exercises local and Docker behavior.

Important adjacent constraint: #3188 records that the current Firecracker feature is broken/orphaned. This issue must not depend on repairing Firecracker and must not broaden into #3188.

## Design decision

Implement a **CLI adapter**, not a Rust/Swift FFI binding.

Reasons:

1. The user-facing installation contract is `brew install container`.
2. The CLI exposes the required stable lifecycle and execution surface.
3. FFI would require a Swift toolchain and macOS linking in the Rust build, expanding portability and release risk without adding required behavior.
4. A narrow command-runner abstraction permits deterministic Linux unit tests with a fake CLI while reserving real execution tests for an Apple-silicon macOS 26 runner.

### Backend identity and selection

Add `BackendType::AppleContainer` with canonical serialized/display value `apple-container` and a backward-compatible serde alias `applecontainer` if derive behavior previously emitted that spelling.

Default preference:

```text
Firecracker -> E2B -> AppleContainer -> Docker -> Local
```

Availability is **positive evidence**, all of:

1. compile/runtime platform is `macos/aarch64`;
2. `container` resolves on `PATH` and `container system version --format json` succeeds;
3. `container system status --format json` exits successfully and reports a responsive API service.

Do **not** run `container system start` automatically. It is host administration, may install a kernel or prompt, and lifecycle ownership belongs to the operator. On failure, record a precise tried-reason and continue to Docker/Local.

### Configuration

Add minimal backend config to `RlmConfig`:

- `apple_container_binary: Option<PathBuf>` — optional absolute CLI override; default resolves `container` from `PATH`.
- `apple_container_image: String` — default `python:3.11-slim`.
- `apple_container_cpus: u32` — default `1`; forwarded verbatim as `--cpus`.
- `apple_container_memory: String` — default `512M`; validated to be a positive integer with an optional case-insensitive `K`/`M`/`G`/`T`/`P` suffix and no surrounding whitespace, because it is forwarded verbatim as `--memory`.
- Keep the outbound-network policy as-is. CPU and memory are configurable because operators must be able to size a per-session VM for the workload; nothing else is exposed in this issue.

Add `BackendSessionConfig { backend: AppleContainer, session_model: Affinity }` to defaults. Debug output must not expose unexpected environment values.

### Executor lifecycle

Create `crates/terraphim_rlm/src/executor/apple_container.rs` behind feature `apple-container-backend` (included in `full`; no new third-party Rust dependency required).

`AppleContainerExecutor` holds:

- command runner/CLI path;
- image and resource profile;
- `DashMap<SessionId, Arc<SessionCell>>` to serialize first creation. A `SessionCell` is a `Mutex<SessionSlot>` plus an `AtomicBool` "unusable" flag: `SessionSlot` is `Active { bound: Option<name>, pending: Vec<name> }` or `Closing { pending: Vec<name> }`, and the atomic exists so a cancellation `Drop` — which cannot await the mutex — can quarantine the slot synchronously. `pending` holds every name whose deletion is unconfirmed, including a name generated for a `container run` that failed. A bare `Option` cell (Docker's shape) is **not** sufficient: it cannot distinguish "no container yet" from "this session is being torn down", nor hold a survivor alongside a binding, which are the leaks this design must avoid;
- an executor-wide lifecycle `RwLock` gate plus a terminal `closing` flag, governed by **one rule**: an *owned* read permit is acquired **before** an operation can create, use or reclaim a container — before `container run`, before `container exec` — and it is held, by the owner of the work, until that operation *and every recovery it triggers* (timeout, vanished container, runner I/O error, panicked exec, cancellation) has finished. `cleanup` takes the write side, and acquiring it is by itself the proof that no earlier operation is outstanding. No recovery registry, no join list and no bounded retry rounds: the permit is the registration, and it exists from before the work starts rather than being installed after it. Lock order is always **permit before slot**, and no permit holder ever acquires a second permit, so neither deadlock is possible;
- validator and capabilities.

Lifecycle semantics:

- `end_session` is **terminal for that session id**. The `Closing` tombstone stays in the map, so no concurrent or later `ensure_container` can resurrect the session.
- Teardown failures **propagate** to the caller and stay tracked (in `pending`), so `cleanup` retries and aggregates them. A container is never dropped from tracking while it may still exist; a container `cleanup` cannot remove is re-inserted for a repeated `cleanup`/`Drop`. `TerraphimRlm::destroy_session` propagates the executor's error too, while retry ownership stays with the executor's `cleanup`.
- A generated name is tracked as `pending` **before** `container run` is spawned, and a session with any pending name refuses to create a replacement. So a failed or timed-out creation cannot leave an anonymous container, and a failed deletion is never displaced by a newer one.
- `cleanup` waits — through the write gate alone — for in-flight creation, in-flight execution, in-flight `end_session` deletion *and* every in-flight recovery, including one that has not started yet but whose execution's permit is still held. Once it returns, no cell can be re-inserted into the drained map — `end_session` included: it reads the closing flag *after* acquiring its permit, so one arriving after cleanup, or queued behind cleanup's write gate, creates no tombstone while still owning any container that is already tracked. Then it refuses all further creation. The honest cost: `cleanup` blocks for as long as an in-flight execution plus its recovery takes.
- Execution ownership is **runtime-independent from the start**: once the lifecycle permit is held and the container is resolved, and *before the guest command is launched*, the permit and the whole `container exec` operation are moved into a dedicated execution owner — an OS thread driving its **own** current-thread runtime. That owner holds the `ProcessRunner` call (and therefore the CLI child and its stdout/stderr drain tasks), the outcome, and every timeout / vanished-container / runner-I/O / panicked-exec / cancellation recovery, and it releases the permit only when all of that has finished; the caller awaits a result channel. Establishing ownership before anything can run in the guest is what makes its failure trivial: a runtime that cannot be built or a thread that cannot be spawned returns `BackendInitFailed` before any `container exec`, so no guest command was launched and there is nothing to recover — and there is no `Drop`-time branch that has to choose between releasing the permit early and leaking it. Container *resolution* runs earlier (`ensure_container` precedes owner creation), so on a fresh session a `container run` may already have succeeded and bound a container by then; that container stays tracked, bound and reusable exactly as any other resolved session container, and is reclaimed by the ordinary `end_session`/`cleanup` paths.
- Cancelling an execution future therefore fails **closed** by construction: synchronously the outcome is claimed, the slot is quarantined and a cancellation signal is raised on the CLI call — nothing is spawned, moved or registered, because the owner already exists. The owner then awaits the `ProcessRunner` call unconditionally — never aborting it and never dropping the future that holds the child and drain handles, because only the runner's return proves the child was killed **and reaped** and the readers stopped — then force-deletes the container under the lifecycle timeout, and lifts the quarantine only on confirmed deletion. There is no bounded backstop: prompt cooperative termination is a hard `ProcessRunner` contract, and a runner that violates it holds the lifecycle permit indefinitely, blocking `cleanup` **fail-closed** rather than letting the backend report a terminal state it has not observed. This holds identically for a `Send` execution future moved out of the runtime and dropped on a plain OS thread, **and** for one whose originating runtime has already been shut down: that runtime never owned the child, the drains or the permit, so its shutdown cannot destroy them. Nothing is detached, and there is no drop site on which the permit is released without an owner.

For each session:

1. Generate a deterministic, CLI-safe name `terraphim-rlm-<lowercase ULID>`; never accept user-controlled container names.
2. Start one detached container with argument-vector invocation (never construct a host shell string):
   - `container run --detach --name <name>`
   - `--cpus 1 --memory 512M --cap-drop ALL`
   - default network/outbound access, matching current Docker policy
   - image `python:3.11-slim`
   - guest command `sleep infinity`
3. Execute Python as argv `container exec <name> python3 -c <code>`.
4. Execute shell commands as argv `container exec <name> bash -c <cmd>`. The command is one guest argument; it is never interpolated by a host shell. `-c`, not `-lc`: a login shell sources the guest image's profile scripts, which can rewrite `PATH` and the environment this backend passes in via `--env`, making the same command behave differently per image.
5. Capture stdout/stderr/exit code and elapsed time into `ExecutionResult`.
6. On `end_session`, stop with a bounded grace period, then delete/remove the named container. Treat already-gone as idempotent success; surface other cleanup failures with the backend and container name.
7. `cleanup()` drains all tracked sessions and attempts all removals, returning an aggregate error only after every cleanup was attempted.
8. `Drop` must not claim cleanup succeeded. If no Tokio runtime is available, log tracked leaked names with an operator recovery command; normal correctness relies on explicit `end_session`/`cleanup`.

### Timeout and cancellation contract

Wrap each `container exec` child in `tokio::time::timeout(Duration::from_millis(ctx.timeout_ms), ...)` with piped stdout/stderr.

On timeout:

1. kill/reap the host CLI child;
2. stop/kill and remove that session's container, because the guest exec process cannot otherwise be proven dead;
3. remove the session mapping so the next call creates a fresh VM;
4. return `ExecutionResult::timeout(partial_stdout, partial_stderr)` with elapsed time and no background process leak.

A timeout must never leave a process continuing inside an affinity container. A `container exec` that fails with a host I/O error rather than an exit code takes the same path: the error may be raised after the child was spawned, so it is not proof that no guest process ran.

### Snapshots and capabilities

Initial backend capabilities:

- `VmIsolation`
- `ContainerIsolation`
- `PythonExecution`
- `BashExecution`
- `FileOperations`

Snapshot operations return typed `RlmError::NotSupported { backend: "apple-container", op }`. Do not claim snapshot support merely because `container export` can snapshot a filesystem; the RLM trait promises broader state restoration.

### Image and network policy

- Runtime may allow the Apple CLI to pull the configured OCI image, but tests must not implicitly pull.
- Real integration tests first inspect image availability or are explicitly opt-in.
- Do not mount the repository, home directory, SSH agent, Docker socket, or arbitrary host paths by default.
- Do not pass `--ssh`, registry credentials, inherited secrets, or host env wholesale.
- Keep default outbound networking for parity with Docker/LLM bridge/pip. DNS allowlist enforcement is not implemented by the existing Docker executor, so it is out of scope rather than falsely claimed.

## Detailed implementation plan (TDD, small commits)

### 1. Pin backend identity and config

Files:

- Modify `crates/terraphim_rlm/src/config.rs`
- Modify `crates/terraphim_rlm/Cargo.toml`

Tests first:

- `apple_container_backend_serializes_as_kebab_case`
- `apple_container_alias_deserializes`
- `apple_container_default_session_model_is_affinity`
- default preference places Apple Container before Docker and Local
- config JSON round-trip preserves binary/image settings

Then add the enum/config/defaults and `apple-container-backend` feature.

### 2. Build a command-runner seam and availability probe

Files:

- Create `crates/terraphim_rlm/src/executor/apple_container.rs`
- Modify `crates/terraphim_rlm/src/executor/mod.rs`

Tests first with a fake runner:

- non-macOS/non-arm64 is unavailable without spawning the CLI
- missing binary, failed `system version`, and failed `system status` each produce distinct tried-reasons
- healthy JSON status selects Apple Container
- selector falls through to Local when Apple Container is explicitly preferred but unavailable
- selector never auto-starts the system service

Keep probing injectable/testable; do not rely on this Linux host having Apple software.

### 3. Implement exactly-once per-session creation

Tests first:

- generated names contain only the allowed prefix/ULID characters
- command argv includes detached/name/cpu/memory/cap-drop/image/sleep, with no host shell
- eight concurrent first commands for one session issue exactly one `container run`
- two sessions get distinct containers
- failed create does not poison the map; retry can create a fresh container

Then implement `ensure_container` and lifecycle state.

### 4. Implement Python/bash execution and result mapping

Tests first:

- Python code is passed as one argv value after `python3 -c`
- shell metacharacters/newlines remain one guest argv value after `bash -c` (not `-lc`: no guest profile is sourced, so the environment passed via `--env` is what the command sees)
- stdout, stderr, non-zero exit, elapsed time, and metadata map correctly
- validator behavior matches Local/Docker
- command output is not mistaken for container ID; creation reads the authoritative name/ID contract

Then implement `execute_code`, `execute_command`, `validate`, backend identity and health.

### 5. Make timeout fail closed

Tests first with a controllable fake process:

- timeout kills and reaps CLI child
- timeout force-stops/removes the container
- session mapping is cleared
- next execution creates a new container
- partial stdout/stderr is preserved
- no child remains after the test

Then implement timeout/cancellation. This is a release-blocking invariant.

### 6. Implement idempotent teardown and cleanup

Tests first:

- unknown session teardown is a no-op
- end-session stop/remove order is correct
- already-absent container is success
- cleanup attempts every tracked session even when one removal fails
- map holds nothing reclaimable after a fully successful cleanup, and retains exactly the container a failed removal left behind
- a failed `end_session` delete is returned to the caller and retried by `cleanup`
- deterministic lifecycle-race schedules (end_session vs ensure, cleanup vs ensure, cleanup vs in-flight end_session, aborted execution future, cleanup started in the same tick as the cancellation drop, cleanup queued during `container exec` against timeout/vanished/I-O-error/panic recovery, twelve simultaneous cancellations, an execution future dropped on a non-runtime thread, `end_session` after and queued behind cleanup, and no map re-insertion after cleanup returns), forced with gates rather than sleeps or repetition
- a process runner that withholds its return after cancellation, far past the old five-second abort grace (virtual time, explicit notifications, no sleeps): no delete starts, `cleanup` stays pending, the runner future is never aborted or dropped, and recovery/delete/`cleanup` complete only once the runner itself returns
- snapshot methods return `NotSupported` naming `apple-container`

Then implement `end_session` (terminal, tombstoned, error-propagating), `cleanup` (gated, retrying, aggregating), the cancellation guard, and honest `Drop` diagnostics.

### 7. Integrate docs/demo/status

Files likely to change:

- Modify `crates/terraphim_rlm/src/lib.rs` architecture docs/re-exports
- Modify `crates/terraphim_rlm/src/executor/trait.rs` backend docs
- Modify `crates/terraphim_rlm/tests/backend_demo.rs`
- Add/update focused RLM README/operator docs under `crates/terraphim_rlm/` or repository docs
- Update `CHANGELOG.md` if required by repo convention

Document:

```bash
brew install container
container system start
cargo test -p terraphim_rlm --features apple-container-backend
```

Add an ignored macOS integration test that verifies Python, bash, non-zero exit, same-session state, timeout recreation, teardown, and no leaked `terraphim-rlm-*` container.

## Verification gates

### Portable/Linux gate (must pass in ADF)

```bash
cargo fmt --check
cargo clippy -p terraphim_rlm --all-targets --features apple-container-backend -- -D warnings
cargo test -p terraphim_rlm --features apple-container-backend
cargo test -p terraphim_rlm --test backend_demo
```

**Not a gate:** `cargo test -p terraphim_rlm --no-default-features --features
apple-container-backend` does not compile, and did not compile before this work.
`crates/terraphim_rlm/src/config.rs` and `src/rlm.rs` name `terraphim_types`
unconditionally while the dependency is optional, and `local.rs`, `executor/mod.rs`
and `rlm.rs` name `crate::validator` unconditionally while the module is behind
`#[cfg(feature = "kg-validation")]`. Verify with `git stash && cargo check -p
terraphim_rlm --no-default-features --features apple-container-backend`. Fixing
that feature matrix is a separate change; listing it here as a passing gate would
be dishonest, so it is excluded.

Also run the repository's existing required gate or `.adf-gates.sh`. If the workspace has pre-existing failures, prove them against clean `main`; do not expand this issue.

### Real Apple-silicon gate (required before merge; unavailable on this Linux orchestrator)

Host: Apple silicon, macOS 26, `container` installed with Homebrew, system service started, default image present or explicitly pulled.

```bash
container system version --format json
container system status --format json
cargo test -p terraphim_rlm --features apple-container-backend --test backend_demo apple_container -- --ignored --nocapture
container list --all --format json   # no leaked terraphim-rlm-* resources
```

Capture CLI/server versions and test output in the PR. A Linux fake-runner pass is not evidence that the Apple runtime works.

## Acceptance criteria

- [ ] `BackendType::AppleContainer` has stable `apple-container` config/display identity and affinity defaults.
- [ ] `full` includes `apple-container-backend`; Linux/non-Apple builds still compile and test.
- [ ] Availability requires macOS/aarch64 + healthy CLI and service; the backend never auto-starts host services.
- [ ] Default preference tries Apple Container before Docker/Local without changing Linux's effective choice.
- [ ] One VM-backed container is created exactly once per RLM session under concurrency.
- [ ] Python and bash execute through argv-safe CLI calls with correct result/exit mapping.
- [ ] Timeout kills/reaps the CLI process, destroys the session container, clears affinity, and preserves partial output.
- [ ] `end_session` is terminal for a session id, propagates deletion failures (as does the public `destroy_session`), and leaves nothing reclaimable on success; `cleanup` is idempotent, coordinates with in-flight execution, in-flight `end_session` *and* in-flight recovery (including a recovery that has not begun yet), cannot have its drained map repopulated after it returns — by a recovery, a new caller *or* an `end_session` — attempts all resources on multi-failure, and retains what it could not remove. Every possibly-created name — including one from a failed `container run` — stays tracked until confirmed deleted, and blocks a replacement meanwhile.
- [ ] Cancelling an execution future fails closed: the slot is unusable before the drop returns, and the execution owner — established before the guest command was launched, on its own thread and runtime, carrying the execution's lifecycle permit — kills and reaps the CLI child, terminates the stream readers and force-deletes the container before that permit is released, so terminal cleanup necessarily waits for it. Independent of the thread the execution future is dropped on **and of the runtime that started the execution, including after that runtime has been shut down**. Proven on a real local process across two runtimes (real PID absent from the process table, zombies included, before cleanup can finish), not only on the fake runner.
- [ ] No host directories, SSH sockets, secret environment, or privileged capabilities are exposed by default.
- [ ] Snapshot calls return typed `NotSupported`; capability claims are truthful.
- [ ] Focused portable tests and repo gates pass.
- [ ] Real Apple-silicon/macOS 26 integration evidence is attached before merge, including zero leaked `terraphim-rlm-*` containers.
- [ ] Docs include `brew install container`, one-time `container system start`, requirements, fallback diagnostics, and cleanup recovery.
- [ ] Independent structural PR review reaches 5/5 with P0=0, P1=0, P2=0 and checks green.

## Non-goals

- Swift FFI or embedding `Containerization` directly.
- Repairing Firecracker/#3188 or implementing E2B.
- Supporting Intel Macs or claiming support below macOS 26.
- Automatic Homebrew installation, `sudo`, service start/stop, kernel installation, or DNS resolver changes.
- Snapshot/restore, host volume mounts, SSH forwarding, registry-login automation, Kubernetes, GPU, nested virtualization, or container-machine support.
- Fixing the existing Docker output-limit/DNS-allowlist gaps unless a shared extraction is strictly necessary and separately tested.

## Risks and mitigations

- **CLI drift:** pin argv contract tests and record tested CLI/server versions in real integration evidence.
- **No Apple runner in current ADF host:** portable fake-runner tests are mandatory but cannot satisfy the merge gate; route the final ignored test to an Apple-silicon macOS 26 host.
- **Leaked VMs after timeout/crash:** fail-closed timeout destroys the affinity container; explicit teardown is primary; names are prefix-scoped for recovery.
- **Prompting/system mutation:** health probing only; no automatic `system start` or kernel install.
- **False security claims:** document default outbound network and unsupported snapshots; do not imply DNS allowlist enforcement.
- **Concurrent creation and teardown races:** per-session `Arc<SessionCell>` with an explicit `SessionSlot` lifecycle state (not Docker's bare `Option` cell, which reintroduces the detached-slot leak), an executor-wide gate whose owned read permit is taken before the work and held through its recovery (permit-before-slot lock ordering), and a retained `Closing` tombstone. Pinned by *deterministic* tests: a gated fake runner parks the CLI call at each lifecycle point and the racing operation is driven from there — no sleeps, no repetition.
- **Apple memory behavior:** Apple documents that freed guest pages may not immediately return to macOS; bounded 512 MiB VMs plus prompt teardown limit exposure.

## Execution

Run through ADF using this issue as the contract:

```bash
~/projects/adf-fleet/bin/adf run terraphim terraphim-ai <THIS_ISSUE> --slug rlm-apple-container
```

ADF should stop at the PR/review gate. **Do not merge without the real Apple-silicon test evidence.**

Related: #3188 (Firecracker backend debt; deliberately out of scope).