# Research Document: ADF Build Gate Repair + Firecracker Runner Hardening

**Status**: Draft
**Author**: Claude Code session (for Alex Mikhalev)
**Date**: 2026-07-13
**Reviewers**: Alex Mikhalev
**Related**: Gitea issue #3097, ADR `adr-rch-build-queue-not-firecracker-ci.md` (2026-04-25),
`design-firecracker-ci-acceleration.md`, PR #2804 (KG-driven runner allowlist)

## Executive Summary

The terraphim-ai merge gate has been red since 2026-06-30. Investigation on bigbox
shows **two independent faults**, neither of which is what the fleet-health alert
implied. (1) The `native-ci` build gate fails at `cargo clippy -D warnings` — a
**genuine code failure** (#3085/#3086), not a runner fault. (2) The `runner-health`
heartbeat fails every 15 min because the runner's command allowlist rejects shell
`if` — a **policy-parsing brittleness**, cosmetic to builds but it blinds fleet
health. Separately, the custom Gitea runner allocates "VMs" in **0 ns** — it runs
builds directly on the host; Firecracker (fcctl-web, live and healthy) is **not on
the build path**. This document maps the system and frames the design choice the
user has requested: harden the runner by routing real builds through Firecracker
microVMs, reconciled against the ADR that previously deferred firecracker-for-CI.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | ADF stabilisation is Q3 WIG-2/North-Star priority; a wedged merge queue blocks the whole fleet. |
| Leverages strengths? | Yes | terraphim_github_runner + terraphim_firecracker + fcctl-web are our own code; zero-LLM-on-hot-path CI is the CI-Compiler thesis. |
| Meets real need? | Yes | No PR merged in 13 days; agents produce PRs that cannot land. |

**Proceed**: Yes (3/3).

## Problem Statement

### Description
ADF worker agents open PRs that pass `adf/pr-reviewer`, `adf/verification` and
`adf/validation`, but the required `native-ci / build (push)` status is **failure**
on every PR, so nothing auto-merges. The runner's own health signal
(`runner-health` heartbeat) is also failing, which is what tripped the
`[ADF] Fleet health alert 20260713: DEGRADED` (#3091).

### Impact
- terraphim-ai merge queue wedged since 2026-06-30 (4 open PRs: #3088/#3093/#3094/#3095).
- Fleet-health pipeline reports DEGRADED / "observability blind".
- ADF throughput ~0 net (PRs open, none land).

### Success Criteria
1. `native-ci / build` reports success on a PR that meets the quality bar, and at
   least one terraphim-ai PR merges after 2026-06-30.
2. `runner-health` heartbeat succeeds for 3 consecutive cycles.
3. (Firecracker objective) `native-ci` build steps execute inside an ephemeral,
   hermetic environment detached from host drift, with the command-allowlist
   brittleness removed as a class.

## Current State Analysis

### The two runner populations (critical distinction)

| Population | Unit | Label / target | Serves | Status |
|-----------|------|----------------|--------|--------|
| **Gitea Actions runner** (custom, `terraphim_gitea_runner`) | 3× systemd **user** units `terraphim-gitea-runner[-2/-3]` | `terraphim-native` | `.gitea/workflows/native-ci.yml`, `runner-health.yml` | **active, but gate red** |
| GitHub Actions self-hosted | systemd **system** units `actions.runner...terraphim-ai-runner-2..5` | `[self-hosted]` | `.github/workflows/ci-firecracker.yml` etc. | active |

The **Gitea** population is the one behind the wedged merge gate. The Firecracker
CI design doc + `ci-firecracker.yml` target the **GitHub** population. They are not
the same pipeline — a key correction to any "just use the firecracker workflow" idea.

### Fault A — heartbeat: allowlist rejects `if` (confirmed)

`crates/terraphim_gitea_runner/src/taxonomy_policy.rs` compiles a `CommandPolicy`
from `default_policy.md` (embedded via `include_str!`, overridable by
`RUNNER_TAXONOMY_DIR`). For every workflow step it computes `program(&step.command)`
= the **first whitespace token** (`policy.rs:125`), and rejects it unless in the
`allowed` set (`taxonomy_policy.rs:188`).

Embedded `default_policy.md`:
```
allow:: cargo, make, bun, bunx, npm, yarn, pnpm, rch, sccache
allow:: echo, mkdir, git, ls, cat, cd, cp, mv, rm, chmod
allow:: sh, bash, test, export, source, true, set, rustup
deny:: docker, curl, wget, nc, ncat, python, python3, perl, ruby
route_to:: rch, cargo, build check clippy doc
```

The `heartbeat` job has two steps: `runner-alive` (`echo …`, allowed) and
`ping-healthchecks`, a multi-line block beginning
`if [ -n "$RUNNER_HEALTH_PING_URL" ]; then curl ... fi`. First token = `if` →
`PolicyRejected("program `if` is not on the allowlist")`, every 15 min (journal
confirms, 12:45→22:15). `TaxonomyPlanner::compile` rejects the **entire plan** on
the first bad step (`taxonomy_policy.rs:176-192`), so the passing `echo` step never
runs and the job fails atomically. Note `curl` is also in `deny::`, so the ping is
doubly incompatible.

The `runner-registration` job survives only via a loophole: its step begins
`response=$(curl …)` then `online=$(… python3 …)`, and `strip_env_assignments`
(`policy.rs:104-130`) consumes both `VAR=$(…)` assignments (paren-matched, quotes
ignored), so `program()` resolves to the later `echo` — which is allowed. The
**denied** `curl` and `python3` inside the command substitutions then execute
anyway.

**Architectural weakness exposed**: the policy checks only the first effective
token of a shell `run:` block. The same workflow file demonstrates both failure
modes: too strict (the heartbeat dies on shell `if` control-flow) and too loose
(the registration job runs two denied programs unchecked behind an env-assignment
prefix). A first-token allowlist is the wrong boundary for arbitrary shell steps;
any future tightening of the stripping logic would silently break the registration
job as well.

### Fault B — merge gate: clippy fails (confirmed, genuine code)

Journal shows `native-ci` executes `step 1/4: cargo fmt --all -- --check` (passes,
advances) then `step 2/4: cargo clippy --workspace --all-targets -- -D warnings`
and **stops** — no step 3/4, no "completed successfully". So clippy fails under
`-D warnings`. This matches open issues #3085 (6 `collapsible_if`) and #3086
(clippy failures + flaky port collision). The clippy invocation is routed through
`rch exec -- cargo clippy` via `route_to::`. **This is a code failure on the tree,
not a runner or firecracker failure.**

### Firecracker reality vs. design

- `fcctl-web.service` (Firecracker control web) is **active, healthy** (up 4 weeks,
  `{"status":"healthy"}` on 127.0.0.1:8080), with drop-ins for vm-lifecycle,
  vm-network, seaweedfs-credentials, jwt-secret, provisioner-ssh.
- `rchd.service` (RCH build queue, the ADR's chosen solution) is **active** (up ~2
  months); cargo build/check/clippy/doc are routed through it via taxonomy.
- BUT the runner's `terraphim_github_runner::session::manager` logs
  "Allocated VM host-… in **0ns**" and runs cargo on the host
  (`.cargo-runner-2/bin`, sccache→SeaweedFS). **The VM session is a local
  passthrough; no Firecracker microVM is booted for builds.**
- `infrastructure/firecracker-rust-ci/` holds a `rust-ci.ext4` rootfs build
  pipeline; `terraphim_rlm/src/executor/firecracker.rs` runs untrusted code in real
  VMs; `crates/terraphim_github_runner/` has the runner + (per design doc) an
  intended `fcctl_client.rs`. [Code-completeness of the FC runner path is mapped in
  the Appendix: the reusable client is `VmCommandExecutor`, not an `fcctl_client.rs`.]

### Prior decision (ADR 2026-04-25) — must be reconciled, not ignored

The ADR **rejected Firecracker as the CI build runtime** for the then-current
workload (ADF thundering-herd, cache, test pollution — all solved more cheaply by
RCH queue + sccache + hermetic fixtures) and **reserved Firecracker for sandboxing**.
It explicitly left three doors open that the current situation walks through:
- "Reproducible release artefacts: build inside a known-good rootfs to **detach
  from host drift**."
- "Per-test / per-build environment isolation … after we have exhausted hermetic
  fixes."
- "Re-evaluate if a future class … genuinely needs per-build VM isolation."

The user's directive ("leverage firecracker integration") + the newly-observed
failure class (host-side allowlist brittleness; a runner that fails a health job on
a shell `if`; builds coupled to host state) is a legitimate trigger to revisit the
"VMs as CI runtime" framing the ADR set aside — **for the isolation/hermeticity
benefit, not the (disproven) speed benefit.**

### Code / config locations

| Component | Location | Purpose |
|-----------|----------|---------|
| Gitea runner crate | `crates/terraphim_gitea_runner/` | poller, task_worker, taxonomy policy |
| Command allowlist | `crates/terraphim_gitea_runner/default_policy.md` + `src/taxonomy_policy.rs` | first-token program policy + rch routing |
| Policy parse/enforce | `src/policy.rs:125` (`program`), `src/taxonomy_policy.rs:175-205` | reject/route |
| FC runner + session mgr | `crates/terraphim_github_runner/` (`session::manager`, `workflow::executor`) | VM alloc (currently 0ns passthrough), step exec |
| FC VM lifecycle | `terraphim_firecracker/src/vm/firecracker.rs` | boot VM / exec-in-VM |
| RLM FC sandbox | `crates/terraphim_rlm/src/executor/firecracker.rs` | untrusted-code exec in VM |
| FC rootfs pipeline | `infrastructure/firecracker-rust-ci/` | `rust-ci.ext4` image |
| Native gate workflow | `.gitea/workflows/native-ci.yml` | fmt/clippy/build/test, `runs-on: terraphim-native` |
| Health workflow | `.gitea/workflows/runner-health.yml` | heartbeat + registration |
| FC GH workflow | `.github/workflows/ci-firecracker.yml` | fcctl-web VM builds (GitHub runners) |
| Runner deploy | `~/.config/systemd/user/terraphim-gitea-runner*.service`, `~/.config/terraphim-gitea-runner/env` | bigbox |
| fcctl-web | `~/projects/terraphim/firecracker-rust/fcctl-web` (private), 127.0.0.1:8080 | VM control plane |

## Constraints

### Vital Few (max 3)
| Constraint | Why vital | Evidence |
|-----------|-----------|----------|
| Do not weaken the security boundary while fixing `if` | The allowlist exists to stop untrusted agent-authored workflow steps running arbitrary host programs | trust-boundary note in `taxonomy_policy.rs`; `deny:: curl,python…` |
| Fail-open / no-regression on deploy | rchd + runner already fail-open; the fleet must not lose the working parts | ADR fail-open principle; runner `Restart=always` |
| Never implement on bigbox directly (write prompt → agent → review) | North-Star non-negotiable | CLAUDE.md / North Star |

### Eliminated from scope (5/25)
| Eliminated | Why |
|-----------|-----|
| GitHub-Actions `ci-firecracker.yml` overhaul | Different runner population; not the wedged gate |
| CI speed optimisation via VMs | ADR disproved the speed case; not the current problem |
| Replacing native-ci with Docker | ADR rejected; out of scope |
| Fixing Odilo build-runner fmt/sqlx (issue #3097 Fault 2) | Tracked separately; not the terraphim-ai gate |

## Risks and Unknowns

### Known risks
| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Loosening allowlist to pass `if` opens a first-token bypass | Med | High (security) | Fix at the right boundary: run steps in a shell/VM, keep program allow-list for the *host* path only; or drop the external ping instead of allowing `if` |
| Firecracker build path is stub, not production-ready | Med | High (effort) | Verify FC runner code completeness (Appendix) before committing to it; phase behind a flag |
| Re-opening firecracker-for-CI re-litigates the ADR | Low | Med | Frame as isolation/hermeticity (ADR-sanctioned), supersede ADR section explicitly, keep RCH for queueing |
| clippy failures are a moving target as agents push | Med | Med | Land #3085/#3086 on main first; require green tree before rebasing PRs |
| Building inside VM needs workspace + cache transport | Med | Med | Reuse sccache→SeaweedFS on fcbr0 already mounted; virtio-fs/9p for workspace (design-doc open question) |

### Open questions
1. ~~Is the FC session path real or a passthrough stub?~~ **Answered** — passthrough
   stub in this deployment, but the real HTTP client exists (`VmCommandExecutor`);
   see Appendix.
2. ~~Does an `fcctl_client.rs` exist / cover exec + artifact copy-out?~~ **Answered**
   — the client is `VmCommandExecutor` (exec + snapshot, no artifact transport);
   see Appendix.
3. Should the fix keep host execution (fast path) and add FC only for isolation, or
   move all `native-ci` steps into VMs? — design decision for human.
4. ~~Is `RUNNER_TAXONOMY_DIR` the sanctioned hot-patch path?~~ **Answered from
   code**: yes — the embedded `default_policy.md` header documents runtime override
   via `RUNNER_TAXONOMY_DIR` pointing at a directory containing `command_policy.md`;
   no recompile needed. (Not that we recommend loosening the policy — see risks.)

### Assumptions
| Assumption | Basis | Risk if wrong | Verified |
|-----------|-------|---------------|----------|
| native-ci red is clippy `-D warnings`, not env | journal stops after step 2/4 clippy | would chase wrong fix | Yes (journal) |
| heartbeat red is solely the `if` rejection | journal error text every cycle | extra cause missed | Yes (journal) |
| fcctl-web can boot a rust-capable VM and exec cargo | design doc + rust-ci.ext4 exists | FC path non-viable | Partial — needs a boot+exec spike |
| RCH routing is live for cargo | `route_to::` present, rchd active | builds not queued | Partial — confirm `rch` on runner PATH |

### Multiple interpretations considered

| Interpretation of "leverage firecracker integration" | Implications | Chosen/Rejected |
|---|---|---|
| A. Route native-ci builds through real FC VMs (runner hardening) | Wires `VmCommandExecutor`→fcctl-web into the Gitea runner session path; removes host-drift + allowlist class | **Chosen** — matches the failure class observed and the ADR's open door |
| B. Switch the gate to the GitHub `ci-firecracker.yml` pipeline | Different runner population; abandons the Gitea gate + KG/rch integration | Rejected — doesn't fix the wedged gate, throws away the custom runner |
| C. Use FC only for the health/ping steps (sandbox the risky bits) | Tiny scope but leaves builds host-coupled; ping doesn't need a VM at all | Rejected — ping is better moved host-side; builds are the drift risk |

## Research Findings

### Key insights
1. The alert conflated two independent faults; neither is a Firecracker fault, and
   one (clippy) is not an infrastructure fault at all.
2. The security policy fails in both directions in the same workflow file
   (rejects `if`; lets denied `curl`/`python3` run behind `VAR=$(…)` stripping) —
   evidence the *boundary* is wrong, not the list contents.
3. All the expensive pieces for VM-isolated builds already exist and are live
   (fcctl-web, `VmCommandExecutor`, the `VmProvider` trait +
   `SessionManager::with_provider()` hook already used by `task_worker.rs:221`,
   `rust-ci.ext4`, fcbr0, sccache→SeaweedFS); the gap is one `VmProvider`
   implementation, the task_worker selection, a registered `vm_type`, and — the
   biggest unknown — a source-delivery contract into the VM (no precedent even in
   `ci-firecracker.yml`, which builds on the host and only probes the VM).
4. The ADR's rejection of firecracker-for-CI was on *speed* grounds; its own text
   sanctions adoption on *isolation/host-drift* grounds.

### Technical spikes needed
| Spike | Purpose | Estimate |
|-------|---------|----------|
| fcctl-web boot+exec: create `rust-ci` VM, run `cargo --version`, destroy | Confirm vm_type registration, auth, working_dir/source-visibility contract, `timeout_seconds` vs `timeout_ms` drift | 2-3h |
| Confirm `rch` on runner PATH + route live | Validate RCH queueing assumption before/inside VM decision | 15m |

## Dependencies

| Dependency | Kind | Impact | Risk |
|-----------|------|--------|------|
| fcctl-web (private firecracker-rust repo, live service) | Internal service | Track 2 entirely depends on its HTTP contract | Med — contract only documented via `ci-firecracker.yml` + `FIRECRACKER_FIX.md`; spike verifies |
| `reqwest`, `async-trait`, `serde` | Crates (already in-tree) | No new external deps | Low |
| bigbox runner env (`~/.config/terraphim-gitea-runner/env`) | Ops config | Flag/URL/token delivery | Low — existing mechanism |

## Recommendations

### Proceed
Yes. Split into an **immediate unwedge** (independent of firecracker) and a
**structural hardening** (the firecracker objective), so the merge queue is not held
hostage to the larger change.

### Scope recommendation (preview of design)
- **Track 1 (unwedge, hours):** land #3085/#3086 clippy fixes → native-ci green;
  fix `runner-health.yml`/allowlist so heartbeat stops failing (prefer removing the
  `if`/`curl` ping or moving it behind an allowlisted helper, over allowing `if`).
- **Track 2 (harden, days):** route `native-ci` step execution through real
  Firecracker microVMs via fcctl-web / `terraphim_github_runner` session manager, so
  each build runs in an ephemeral known-good rootfs — eliminating host drift and the
  first-token-allowlist class of failure. Supersede the relevant ADR section on the
  isolation rationale; keep RCH for queueing inside/around the VM.

### Risk mitigation
Gate Track 2 behind readiness of the FC runner code (Appendix survey) and a
boot+exec+artifact spike; keep host passthrough as fail-open fallback.

## Next Steps
1. ~~Fold in the code-survey Appendix~~ Done (see Appendix).
2. Run the fcctl-web boot+exec spike before committing to Track 2 sequencing.
3. Request quality evaluation, then proceed to `disciplined-design` for the fix plan
   (design doc: `.docs/design-adf-build-gate-firecracker.md`).

## Appendix — Firecracker code survey (completed)

**Real / production-ready:**
- `terraphim_github_runner/src/workflow/vm_executor.rs` — `VmCommandExecutor`
  implements `CommandExecutor` with real reqwest calls to fcctl-web:
  `POST {base}/api/llm/execute {agent_id, language:"bash", code, vm_id, timeout_seconds, working_dir}`
  → `{exit_code, stdout, stderr}`; plus `create_snapshot`, `rollback`. Reads
  `FIRECRACKER_AUTH_TOKEN`. Ships with `SimulatedVmExecutor` mock. **This is the path
  to reuse.**
- fcctl-web contract (from `ci-firecracker.yml` `vm-infrastructure` + `FIRECRACKER_FIX.md`):
  `GET /health`; `POST /api/vms {vm_type:"focal-ci"}`→`{id}`; `GET /api/vms/{id}`
  (poll `status=="running"`); `POST /api/llm/execute`; `DELETE /api/vms/{id}`. Bearer JWT.
- `infrastructure/firecracker-rust-ci/` builds `rust-ci.ext4` (Ubuntu 22.04 + Rust
  stable/clippy/rustfmt + sccache 0.8.2 + sshd), `fcbr0` bridge (172.26.0.1/24) +
  SeaweedFS cache. All on-shelf.

**Stub / not to be used:**
- `terraphim_firecracker::FirecrackerClient::send_api_request` returns a fake
  `{"status":"success"}` — the in-repo VM manager is a prototype. `Sub2SecondVmManager`
  metrics are placeholders. Do not build the fix on this crate.

**Complete but gated (not the runner path):**
- `terraphim_rlm/src/executor/firecracker.rs` — full VM+snapshot+SSH exec, but
  `#[cfg(feature="firecracker")]` and its `fcctl-core` dep is **commented out** in
  `crates/terraphim_rlm/Cargo.toml:43` (needs the private firecracker-rust repo).
  `ensure_pool()` is an unimplemented TODO. Enabling the feature as-is will not compile.

**Runner reality:** `session::manager` logs "Allocated VM … in 0ns" ⇒ it is using the
**simulated / passthrough** executor, running cargo on the host. Flipping to real
Firecracker means constructing `VmCommandExecutor` against fcctl-web + create/poll/
destroy a VM per job. The client is already written and tested; the gap is the
session-manager wiring + a registered rust-capable `vm_type` in fcctl-web.

**Open questions now answered:** Q1 — FC session path is a passthrough stub in this
deployment, but the real HTTP client exists (`VmCommandExecutor`). Q2 — no
`fcctl_client.rs`; the client is `VmCommandExecutor`; it covers exec-in-VM +
snapshot (no explicit artifact copy-out — builds would use sccache/SeaweedFS + a
snapshot, not artifact transport). Minor API drift: client sends `timeout_seconds`,
`vm-infrastructure` curl sends `timeout_ms` — reconcile.

**Reference:** ADR `adr-rch-build-queue-not-firecracker-ci.md`; `design-firecracker-ci-acceleration.md`;
`FIRECRACKER_FIX.md`; `design-2185-native-runner-reliability.md`; `design-agents-native-ci-failure.md`.
