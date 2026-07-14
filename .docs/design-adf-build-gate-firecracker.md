# Implementation Plan: ADF Build Gate Repair + Firecracker Runner Hardening

**Status**: Draft (awaiting human approval)
**Research Doc**: `.docs/research-adf-build-gate-firecracker.md`
**Author**: Claude Code session (for Alex Mikhalev)
**Date**: 2026-07-13
**Estimated Effort**: Track 1 ~0.5 day; Track 2 ~2-3 days
**Related**: Gitea #3097, #3085, #3086; ADR `adr-rch-build-queue-not-firecracker-ci.md`

## Overview

### Summary
Restore the terraphim-ai merge gate and remove the failure *class* behind it, in two
independently shippable tracks:
- **Track 1 (unwedge, hours):** land the clippy fixes so `native-ci` goes green, and
  stop `runner-health` failing on a shell `if`. No Firecracker involved.
- **Track 2 (harden, days):** run `native-ci` build steps inside ephemeral Firecracker
  microVMs via the *already-written* `VmCommandExecutor` → live `fcctl-web`, so builds
  are hermetic (detached from host drift) and the brittle first-token host allowlist
  ceases to be the security boundary.

### Approach
Reuse existing, tested code. Track 2 is **wiring, not new infrastructure**: fcctl-web
is live, `VmCommandExecutor` is real, `rust-ci.ext4` + `fcbr0` + SeaweedFS exist. The
only new code is a real VM session provider in the runner and a config switch, behind
a fail-open flag that defaults to today's host behaviour.

### Scope

**In scope (the vital 5):**
1. Land clippy fixes (#3085 + #3086) to green `native-ci` on `main`.
2. Break the gate deadlock (the clippy-fix PR is itself blocked by the red gate).
3. Fix `runner-health.yml` heartbeat so it stops failing every 15 min.
4. Add a real fcctl-web VM session provider to `terraphim_github_runner` (flag-gated).
5. Switch `native-ci` execution to Firecracker VMs when the flag is on; keep host
   passthrough as fail-open default.

**Out of scope:**
- Odilo build-runner fmt/sqlx failures (#3097 Fault 2 — separate agent).
- GitHub-Actions `ci-firecracker.yml` changes (different runner population).
- CI *speed* optimisation via VMs (ADR disproved; not our problem).
- Building `rust-ci.ext4` from scratch (already on shelf).

**Avoid At All Cost (from 5/25):**
- Reviving `terraphim_firecracker::FirecrackerClient` (stub with fake responses).
- Enabling RLM's `--features firecracker` / uncommenting `fcctl-core` (needs private
  repo, `ensure_pool()` is TODO — will not compile; wrong path).
- A new provider trait or "pluggable sandbox backend" abstraction — the `VmProvider`
  trait already exists (`session/manager.rs:56`) with mock + host impls; add one
  implementation, nothing else.
- Loosening the allowlist by simply adding `if`/`curl` (creates a first-token bypass).
- Per-build artifact-transport machinery — sccache/SeaweedFS + snapshots already cover it.

## Architecture

### Current (broken) flow
```
Gitea push ──> terraphim-gitea-runner (poller)
                   │ task_worker: git checkout into ~/.local/share/.../work
                   │ TaxonomyPlanner.compile(): program(step)=first token
                   │   └─ heartbeat step "if [ -n ... ]" -> REJECTED (Fault A)
                   │ session::manager.allocate() -> "VM in 0ns" (SIMULATED)
                   └─ workflow::executor runs cargo ON HOST (sccache->SeaweedFS)
                        step2 cargo clippy -D warnings -> FAIL (Fault B, real code)
                   post commit status  adf?  -> native-ci/build = failure
```

### Target flow (Track 2 on)
```
Gitea push ──> terraphim-gitea-runner (poller)
                   │ task_worker: git checkout (host, to obtain sources)
                   │ session::manager.allocate():
                   │   FcctlWebProvider (impl VmProvider)
                   │     POST fcctl-web /api/vms {vm_type:"rust-ci"} -> id
                   │     GET  /api/vms/{id} until status=running (<2s)
                   │ workflow::executor uses VmCommandExecutor(vm_id):
                   │     POST /api/llm/execute {code:"cargo fmt|clippy|build|test",
                   │        language:"bash", working_dir, timeout_seconds}
                   │        (rust-ci.ext4: rust+clippy+sccache; cache via fcbr0)
                   │   on success: optional snapshot; DELETE /api/vms/{id}
                   └─ post native-ci/build status from aggregated exit codes
Allowlist: enforced at workflow-authoring (in-repo, reviewed); host no longer
           executes untrusted step tokens -> Fault A class removed.
```

### Key design decisions
| Decision | Rationale | Alternatives rejected |
|----------|-----------|----------------------|
| Reuse `VmCommandExecutor` (HTTP→fcctl-web) | Real, tested client already in the runner crate | `terraphim_firecracker` (stub); RLM `fcctl-core` path (won't compile) |
| Flag-gated, host default | Fail-open, zero-regression; matches rchd/runner ethos | Hard cutover to VMs (risky, no fallback) |
| Fix heartbeat by removing `if`/`curl` from the sandboxed step | Keeps allowlist strict; ping moves to a host-side timer | Adding `if`,`curl` to allowlist (first-token bypass hole) |
| Supersede ADR §"VMs as CI runtime" on *isolation* grounds only | ADR rejected VMs for *speed*; it explicitly allows them for host-drift detachment | Ignoring the ADR (re-litigation risk) |
| Break deadlock via combined clippy PR + authorised force-merge | Gate blocks its own fix; force-merge is the documented override | Leaving gate red; disabling clippy permanently |

### Eliminated options (Essentialism)
| Rejected | Why | Risk if included |
|----------|-----|------------------|
| New rootfs / VM image work | `rust-ci.ext4` exists | Days of infra yak-shaving |
| Pluggable multi-backend sandbox trait | One provider + mock suffices | Speculative abstraction, maintenance |
| Artifact copy-out endpoints | sccache + snapshots cover reuse | New fcctl-web API surface |
| Migrating off the custom Gitea runner to `act_runner` | Custom runner is the KG/rch integration point | Throws away working investment |

### Simplicity Check
**What if this could be easy?** Track 1 is two small PRs (clippy fixes + a
workflow-step deletion) plus one authorised force-merge. Track 2 is one new file
(`FcctlWebProvider`, implementing the **already-existing** `VmProvider` trait) + a
provider/executor selection in `task_worker::run()` + one env flag — the trait, the
HTTP client, the VM image, network, and cache all already exist. A senior engineer
would call this proportionate: no new services, no new protocol, no new traits,
mock retained for tests.

**Nothing-speculative checklist:** no unrequested features; no "backend registry";
no flexibility beyond the one flag; no handling for VM types we don't ship; no
premature perf tuning (boot target already met by fcctl-web).

## File Changes

### Track 1 — Unwedge

#### Modified
| File | Changes |
|------|---------|
| (code for #3085) `crates/terraphim_sessions/**` | fix 6 `collapsible_if` clippy errors |
| (code for #3086) per issue | fix remaining clippy failures + flaky port-collision test |
| `.gitea/workflows/runner-health.yml` | **delete** the `ping-healthchecks` step (the `runner-alive` `echo` step already exists and passes policy — the whole plan is rejected only because compile() fails atomically on the bad second step); move the external ping to a host-side systemd timer. Also move the `runner-registration` check host-side (same timer script): today it passes only via the `strip_env_assignments` loophole while executing **denied** `curl`+`python3`, and any policy tightening would silently break it |

#### New (ops, not committed to repo — documented in plan)
| File | Purpose |
|------|---------|
| `~/.config/systemd/user/runner-health-ping.{service,timer}` (bigbox) | host-side script every 15 min, outside the sandbox: (1) curl to `RUNNER_HEALTH_PING_URL`; (2) the registration check (Gitea API online-runner count) currently done in-workflow via denied `curl`/`python3` |

### Track 2 — Firecracker hardening

#### New
| File | Purpose |
|------|---------|
| `crates/terraphim_github_runner/src/session/fcctl_provider.rs` | `FcctlWebProvider` implementing the **existing** `VmProvider` trait (create/poll/destroy via fcctl-web) |
| `crates/terraphim_github_runner/tests/fcctl_provider_test.rs` | integration test against fcctl-web (gated by `FCCTL_URL`, `--ignored`) |

#### Modified
| File | Changes |
|------|---------|
| `crates/terraphim_github_runner/src/session/mod.rs` + `lib.rs` | export `fcctl_provider::FcctlWebProvider` alongside `HostVmProvider`/`MockVmProvider` |
| `crates/terraphim_github_runner/src/workflow/vm_executor.rs` | reconcile `timeout_seconds` vs fcctl-web `timeout_ms`; confirm `working_dir` maps to the checked-out repo path inside the VM |
| `crates/terraphim_gitea_runner/src/config.rs` | add `vm_mode: VmMode { Host, Firecracker }` from `RUNNER_VM_MODE` (default `Host`) + `fcctl_url`, `fcctl_vm_type` |
| `crates/terraphim_gitea_runner/src/task_worker.rs` | at `run()` (currently `task_worker.rs:221-235`): select provider (`HostVmProvider` vs `FcctlWebProvider`) and executor (`HostCommandExecutor::new(work_dir)` vs `VmCommandExecutor`) by `vm_mode`; set `default_vm_type` from config; ensure sources reach the VM working_dir |
| `~/.config/terraphim-gitea-runner/env` (bigbox, ops) | set `RUNNER_VM_MODE=firecracker`, `FCCTL_URL=http://127.0.0.1:8080`, `FIRECRACKER_AUTH_TOKEN=…`, `FCCTL_VM_TYPE=rust-ci` once verified |

#### ADR
| File | Changes |
|------|---------|
| `.docs/adr-rch-build-queue-not-firecracker-ci.md` | add "2026-07 revisit" note: RCH stays the queue; Firecracker adopted for `native-ci` **isolation/host-drift** per the door left open in "Where Firecracker DOES belong". |

## API Design (Track 2)

**No new trait.** The extension point already exists and is already in use:
`VmProvider` (`session/manager.rs:56`, `allocate(vm_type) -> (vm_id, Duration)` /
`release(vm_id)`) with `MockVmProvider` + `HostVmProvider` implementations, and
`task_worker.rs:221` already constructs
`SessionManager::with_provider(Arc::new(HostVmProvider), …)`. Track 2 adds **one
implementation** of the existing trait plus an executor switch:

```rust
// crates/terraphim_github_runner/src/session/fcctl_provider.rs (new file)

/// Real provider backed by the live fcctl-web HTTP service.
pub struct FcctlWebProvider {
    base_url: String,          // e.g. http://127.0.0.1:8080
    auth_token: Option<String>,// FIRECRACKER_AUTH_TOKEN
    client: reqwest::Client,
    ready_timeout: Duration,   // default 30s, warn > 2s per TARGET_BOOT_TIME_MS
}

#[async_trait::async_trait]
impl VmProvider for FcctlWebProvider {
    // allocate(vm_type): POST {base}/api/vms {"vm_type": vm_type} -> {id};
    //   poll GET {base}/api/vms/{id} until status=="running" or ready_timeout;
    //   returns (id, elapsed).
    // release(vm_id): DELETE {base}/api/vms/{vm_id} (ignore 404; idempotent).
    async fn allocate(&self, vm_type: &str) -> Result<(String, Duration)>;
    async fn release(&self, vm_id: &str) -> Result<()>;
}
```

The VM type flows through existing plumbing: `SessionManagerConfig.default_vm_type`
(set from `FCCTL_VM_TYPE`; the current hardcoded default is `"bionic-test"`) or
`SessionStartSpec.vm_type`. Command execution switches from
`HostCommandExecutor::new(work_dir)` to the existing `VmCommandExecutor` in
`task_worker.rs` when the flag is on — both already implement `CommandExecutor`.

```rust
// crates/terraphim_gitea_runner/src/config.rs (additions)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VmMode { #[default] Host, Firecracker }

impl RunnerConfig {
    pub fn vm_mode(&self) -> VmMode;      // from RUNNER_VM_MODE (default Host)
    pub fn fcctl_url(&self) -> &str;      // FCCTL_URL, default http://127.0.0.1:8080
    pub fn fcctl_vm_type(&self) -> &str;  // FCCTL_VM_TYPE, default "rust-ci"
}
```

Selection in `task_worker::run()` (replacing the hardcoded `HostVmProvider` +
`HostCommandExecutor` pair at `task_worker.rs:221-235`):
```rust
let (provider, executor): (Arc<dyn VmProvider>, Arc<dyn CommandExecutor>) =
    match self.config.vm_mode() {
        VmMode::Firecracker => (
            Arc::new(FcctlWebProvider::new(self.config.fcctl_url(),
                std::env::var("FIRECRACKER_AUTH_TOKEN").ok())),
            Arc::new(VmCommandExecutor::new(self.config.fcctl_url())),
        ),
        VmMode::Host => (
            Arc::new(HostVmProvider),
            Arc::new(HostCommandExecutor::new(work_dir)),
        ),
    };
let session_manager = Arc::new(SessionManager::with_provider(
    provider,
    SessionManagerConfig { default_vm_type: self.config.fcctl_vm_type().into(),
                           ..Default::default() },
));
let exec = WorkflowExecutor::with_executor(executor, session_manager.clone(), …);
```

## Test Strategy

### Unit
| Test | Location | Purpose |
|------|----------|---------|
| `vm_mode_defaults_to_host` | `config.rs` | env-absent → `Host` |
| `vm_mode_parses_firecracker` | `config.rs` | `RUNNER_VM_MODE=firecracker` |
| `fcctl_provider_builds_urls` | `fcctl_provider.rs` | correct create/poll/delete URLs |
| `release_ignores_404` | `fcctl_provider.rs` | idempotent teardown |
| `policy_rejects_multiline_first_token` (regression) | `taxonomy_policy.rs` | documents the first-token limitation the VM path supersedes |
| `env_assignment_stripping_hides_denied_programs` (regression) | `policy.rs` | documents that `VAR=$(curl …)` prefixes resolve to a later allowed token — the loophole the registration job relied on, superseded by the VM path |

### Integration (real deps, no mocks — per repo rule)
| Test | Location | Purpose |
|------|----------|---------|
| `allocate_and_release_real_vm` (`#[ignore]`, needs `FCCTL_URL`) | `tests/vm_session_provider_test.rs` | boots a VM via live fcctl-web, asserts `status==running` < 5s, then destroys |
| `run_native_ci_steps_in_vm` (`#[ignore]`) | `tests/vm_session_provider_test.rs` | executes `cargo fmt`/`clippy`/`build`/`test` in the VM, asserts exit codes |
| `runner_health_heartbeat_passes` | manual/live | trigger `runner-health` on bigbox, assert heartbeat green 3 cycles |

### Acceptance (mini-UAT)
```gherkin
Given #3085 and #3086 are fixed on main
When native-ci/build runs on an open PR rebased on main
Then it reports success and the PR is auto-mergeable
And at least one terraphim-ai PR merges after 2026-06-30

Given runner-health.yml contains only allowlist-clean steps (echo liveness)
When the 15-min schedule fires
Then the heartbeat job succeeds for 3 consecutive cycles
And the host-side runner-health-ping.timer delivers the external ping
And performs the online-runner-count check formerly in the runner-registration job

Given RUNNER_VM_MODE=firecracker on the bigbox runner
When native-ci runs
Then each build step executes inside a rust-ci Firecracker VM (fcctl-web /api/llm/execute)
And the VM is destroyed after the run (DELETE /api/vms/{id})
And with RUNNER_VM_MODE unset the runner behaves exactly as today (fail-open)
```

## Implementation Steps

### Track 1 (do first, unblocks merges)
1. **Fix clippy (#3085 + #3086) in one branch.** fmt+clippy clean `cargo clippy --workspace --all-targets -- -D warnings` locally. Tests + fmt. ~2h.
2. **Break the deadlock.** The gate blocks its own fix. Options, pick per human:
   (a) **Authorised force-merge** of the combined clippy PR (documented override
   pattern: force_merge=true with per-PR user authorisation + green local evidence in
   the commit message); then the gate is green for subsequent PRs. OR
   (b) temporarily make clippy **warn-not-fail** in `native-ci.yml` (`-D warnings` →
   plain), merge fixes, then re-arm `-D warnings` in the same session.
   **Decided 2026-07-13: (a) authorised force-merge** — keeps the gate honest. ~15m.
3. **Heartbeat fix.** Edit `.gitea/workflows/runner-health.yml`: delete the
   `ping-healthchecks` step (the existing `runner-alive` echo step is the in-runner
   liveness signal) and move the `runner-registration` API check host-side with it.
   Add bigbox `runner-health-ping.timer` doing both external checks. ~1h.
4. **Verify Track 1.** Rebase an open PR on green main; confirm native-ci success +
   one merge; confirm 3 green heartbeats. Update #3097. ~30m.

### Track 2 (harden; independent, flag-gated)
5. **Config**: add `VmMode`/`fcctl_*` to `terraphim_gitea_runner` config + unit tests. ~2h.
6. **Provider**: new `fcctl_provider.rs` (`FcctlWebProvider` impl of the existing
   `VmProvider` trait) + unit tests for URL/idempotency. ~3h.
7. **Wire task_worker::run()** to select provider + executor by `vm_mode` (replacing
   the hardcoded `HostVmProvider`/`HostCommandExecutor` pair); set `default_vm_type`
   from config; ensure the checked-out sources are the VM `working_dir` (host bind /
   shared path or push into VM). Reconcile `timeout_seconds`/`timeout_ms` (the
   client sends `timeout_seconds`; `ci-firecracker.yml`'s probe sends `timeout_ms`). ~4h.
8. **Live integration test** against fcctl-web (allocate→exec cargo→release), `#[ignore]`.
   Confirm `rust-ci` vm_type is registered in fcctl-web (or register it from
   `rust-ci.ext4`). ~3h.
9. **Canary on bigbox**: set `RUNNER_VM_MODE=firecracker` on **one** of the three
   runner units; watch a full native-ci run in-VM; compare wall-time + correctness vs
   host. Roll to all three or roll back. ~half day.
10. **ADR update** documenting the isolation-rationale adoption — **after canary
    evidence** (decided 2026-07-13), citing the in-VM native-ci run. ~30m.

## Rollback Plan
- Track 1: revert the `runner-health.yml` edit; clippy fixes are normal reverts.
- Track 2: unset `RUNNER_VM_MODE` (→ `Host` default) and restart the runner unit —
  instant return to today's behaviour. The flag is the kill-switch; no schema/state
  migration to undo.

## Dependencies
### New
| Crate | Version | Justification |
|-------|---------|---------------|
| (none new) | — | `reqwest`, `async-trait`, `serde` already in `terraphim_github_runner` |

## Risks
| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| `rust-ci` vm_type not registered in fcctl-web | **High** | High | `ci-firecracker.yml` offers only `focal-ci`; runner default is `bionic-test`. Step 8 verifies/registers before canary; else fall back to `focal-ci` + rustup in-VM |
| Sources not visible inside VM working_dir | **High** | High | **No precedent anywhere**: even `ci-firecracker.yml` builds on the host and only probes the VM with `echo vm-ok`. This is the single biggest Track 2 unknown — the spike must settle it (shared mount / virtio-fs vs `git clone` inside VM vs tarball push via /api/llm/execute) before steps 6-9 are sequenced |
| VM build slower than host (ADR's original finding) | Med | Med | Canary measures; isolation is the goal, not speed; keep host mode available |
| Force-merge misused | Low | Med | Per-PR human authorisation + green-evidence commit message (documented pattern) |
| fcctl-web JWT/auth drift | Low | Med | `FIRECRACKER_AUTH_TOKEN` via env; `FIRECRACKER_FIX.md` documents the Bearer flow |
| Moving registration check host-side loses the in-Gitea red job on zero runners | Low | Low | The external ping monitor (healthchecks.io) is the alerting path either way — a runner-executed check can't fire when no runner is online, so host-side is strictly more reliable |

## Open Items
| Item | Status | Owner |
|------|--------|-------|
| Confirm `rust-ci` vm_type in fcctl-web (`GET /api/vms` types) | Pending | design→spike |
| Confirm fcctl-web working_dir / source-sharing contract | Pending | design→spike |
| Choose deadlock breaker (force-merge vs temp warn) | **Decided 2026-07-13: authorised force-merge** | Alex |
| Track 2 rollout | **Decided 2026-07-13: canary one runner unit, then roll or roll back** | Alex |
| Whether to supersede ADR now or after canary | **Decided 2026-07-13: after canary evidence** | Alex |

## Approval
- [x] Technical review complete (KLS evaluation 2026-07-13:
      `.docs/quality-eval-adf-build-gate-firecracker.md` — research PASS, design
      CONDITIONAL PASS; Track 2 steps 6-9 blocked on the fcctl-web spike)
- [x] Deadlock-breaker choice made: authorised force-merge (2026-07-13)
- [x] Track 2 canary approach agreed: one runner unit first (2026-07-13)
- [ ] Human approval received (North Star: no bigbox implementation without written→agent→review)
