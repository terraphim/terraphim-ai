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
- Reuse the current 512 MiB memory profile and outbound-network policy; expose no speculative knobs in this issue.

Add `BackendSessionConfig { backend: AppleContainer, session_model: Affinity }` to defaults. Debug output must not expose unexpected environment values.

### Executor lifecycle

Create `crates/terraphim_rlm/src/executor/apple_container.rs` behind feature `apple-container-backend` (included in `full`; no new third-party Rust dependency required).

`AppleContainerExecutor` holds:

- command runner/CLI path;
- image and resource profile;
- `DashMap<SessionId, Arc<Mutex<Option<ContainerHandle>>>>` to serialize first creation exactly as Docker does;
- validator and capabilities.

For each session:

1. Generate a deterministic, CLI-safe name `terraphim-rlm-<lowercase ULID>`; never accept user-controlled container names.
2. Start one detached container with argument-vector invocation (never construct a host shell string):
   - `container run --detach --name <name>`
   - `--cpus 1 --memory 512M --cap-drop ALL`
   - default network/outbound access, matching current Docker policy
   - image `python:3.11-slim`
   - guest command `sleep infinity`
3. Execute Python as argv `container exec <name> python3 -c <code>`.
4. Execute shell commands as argv `container exec <name> bash -lc <cmd>`. The command is one guest argument; it is never interpolated by a host shell.
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

A timeout must never leave a process continuing inside an affinity container.

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
- shell metacharacters/newlines remain one guest argv value after `bash -lc`
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
- map is empty after cleanup
- snapshot methods return `NotSupported` naming `apple-container`

Then implement `end_session`, `cleanup`, and honest `Drop` diagnostics.

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
- [ ] `end_session` and `cleanup` are idempotent and leave no tracked resources; multi-failure cleanup attempts all resources.
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
- **Concurrent creation races:** mirror Docker's per-session `Arc<Mutex<Option<_>>>` and pin with a concurrent test.
- **Apple memory behavior:** Apple documents that freed guest pages may not immediately return to macOS; bounded 512 MiB VMs plus prompt teardown limit exposure.

## Execution

Run through ADF using this issue as the contract:

```bash
~/projects/adf-fleet/bin/adf run terraphim terraphim-ai <THIS_ISSUE> --slug rlm-apple-container
```

ADF should stop at the PR/review gate. **Do not merge without the real Apple-silicon test evidence.**

Related: #3188 (Firecracker backend debt; deliberately out of scope).