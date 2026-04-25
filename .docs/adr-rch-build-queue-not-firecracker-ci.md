---
Status: Accepted
Date: 2026-04-25
Deciders: Alex Mikhalev
Related:
  - .docs/adr-rust-build-cache-seaweedfs.md
  - .docs/design-firecracker-ci-acceleration.md
  - infrastructure/firecracker-rust-ci/
  - infrastructure/rust-cache-stack/
Supersedes (in part):
  - .docs/design-firecracker-ci-acceleration.md (the "VMs as CI runtime" framing)
---

# ADR: RCH for Build Queueing; Firecracker VMs Reserved for Sandboxing

## Status

Accepted -- 2026-04-25.

## Context

Three populations issue cargo build/test on bigbox:

1. **GitHub Actions self-hosted runner** -- one workspace per runner, one job at a time, isolated by GitHub Actions itself.
2. **ADF orchestrator agents (21+)** -- all share `~/projects/terraphim/terraphim-ai/`. When 5-15 fire concurrently, they thunder-herd: 15 cargo processes contend for CPU, the same `target/` directory, and `Cargo.lock`.
3. **Dev REPL on bigbox** -- single-user, occasional.

We previously framed Firecracker microVMs as the "CI acceleration" answer (see `design-firecracker-ci-acceleration.md`). Implementation produced:

- `infrastructure/firecracker-rust-ci/` -- working build pipeline for a Rust-capable VM rootfs (`rust-ci.ext4`).
- `infrastructure/rust-cache-stack/` -- SeaweedFS S3 build cache on `fcbr0` (172.26.0.1:8333).
- `vm-cargo-probe` CI job -- synthetic single-dep cargo build inside a VM.
- The cache stack delivered a real, measurable CI speedup.
- VM dispatch never delivered a real CI speedup; vm-cargo-probe is a smoke test only.

After standing up the pieces, the architectural review (this ADR) concluded the VM-as-CI-runtime framing is wrong for the current workload.

## Decision

1. **Adopt RCH (Remote Compilation Helper)** as the queue/dispatch layer for all cargo invocations on bigbox. RCH runs as the `rchd` daemon, intercepts cargo via a Claude Code PreToolUse hook, and serialises jobs to a worker pool capped at `nproc/4` slots. This solves the ADF thundering-herd problem at the source.

2. **Reserve Firecracker microVMs for sandboxing**, not CI builds. The existing `rust-ci.ext4` image build pipeline stays as on-shelf infrastructure, ready when we need it for untrusted-code execution, multi-tenant builds, or LLM tool-call sandboxing.

3. **Drop `vm-cargo-probe` from CI.** It tests dispatch plumbing for a workload we have decided not to dispatch. The 24-second `Firecracker VM lifecycle proof` job remains as a cheap healthcheck for `fcctl-web`.

4. **CI dispatches via `rch exec --`** so the same queue + slot accounting applies to CI invocations as to ADF agents. RCH is fail-open: if `rchd` is down, the call falls through to local cargo with no behaviour change.

## Why VMs do NOT help for our build/CI workload

| Problem | Real solution | Does Firecracker help? |
|---|---|---|
| Concurrent ADF agents stomp on each other | RCH queue | No -- RCH is the answer |
| Test pollution from shared `/tmp/terraphim_sqlite` | hermetic settings.toml per test | No -- already fixed at source |
| Slow CI tests (cargo run x 8) | pre-built binary cache (50x speedup) | No -- already fixed at source |
| Cold-start compile times | sccache + SeaweedFS S3 | No -- already in place |
| Test workspace `target/` contention across runners | per-runner workspace dir (Actions does this) | No -- already isolated |
| Zombie test servers | `cleanup_test_files` + better teardown | Could (kill VM = all gone), but tests should clean up regardless |

The problems we actually have are solved more cheaply at the source than by introducing a per-build VM layer.

## Where Firecracker DOES belong

VMs remain the right answer for use cases we are not yet exercising:

- **Untrusted code execution**: LLM-generated tool calls, agent-spawned sub-processes that run code we did not write.
- **Multi-tenant build farms**: per-tenant rootfs, no shared filesystem.
- **Reproducible release artefacts**: build inside a known-good rootfs to detach from host drift.
- **Per-test environment isolation** for tests that genuinely cannot share host state -- after we have exhausted hermetic-env fixes.

`infrastructure/firecracker-rust-ci/` and the registered `rust-ci` image stay so we can pull them off the shelf when these use cases arrive.

## Architecture After This ADR

```
   Build sources                     Queue/dispatch              Workers
┌──────────────────────┐         ┌─────────────────────┐      ┌────────────────────────┐
│ CI rust-build job    │--rch -->│ rchd                │----->│ localhost slot 1..N    │
│ ADF agents (21+)     │--rch -->│ ~/.cache/rch.sock   │      │ N = nproc/4            │
│ dev REPL             │--rch -->│ FIFO queue, slot    │      │ same workspace         │
│                      │         │ accounting          │      │ shared sccache         │
└──────────────────────┘         └─────────────────────┘      └────────────────────────┘

Cache (orthogonal):  sccache --> SeaweedFS S3 on fcbr0 (172.26.0.1:8333)
Sandbox shelf:       infrastructure/firecracker-rust-ci/  (not on the build path)
```

## Alternatives Considered

### A. Bump VM resources (4 vCPU / 16 GB RAM / 32 GB rootfs) and route CI through VMs

Would have made `vm-cargo-probe` a viable real-build job. Rejected: still does not solve the ADF agent problem (which is not a CI problem), still requires expensive workspace transport (rsync or virtio-fs) per build, still gives nothing sccache + per-runner workspace does not already give. Pure complexity tax.

### B. Replace bigbox runner with a Firecracker VM as the GitHub Actions runner host

Would re-isolate CI from bigbox at the cost of running the GitHub runner inside a VM. Rejected: GitHub Actions already gives us `_work/<repo>/` isolation per job; nothing to gain.

### C. Keep Firecracker VMs as CI workers but only for slow/flaky tests

Plausible. Rejected for now: we fixed the slow/flaky tests at the source (binary cache + hermetic settings + service lock release). Re-evaluate if a future class of tests genuinely needs per-test VM isolation.

### D. Do nothing about ADF contention; accept the thundering herd

Rejected: the contention is observable (compile process count spikes when implementation-swarm fires). RCH is cheap, fail-open, and well-suited.

## Consequences

### Positive

- Clear separation of concerns: cache layer, queue layer, sandbox layer (when needed).
- Single mental model for build dispatch: every cargo invocation flows through `rch`.
- ADF contention solved at its root (queue), not symptomatically (more cores or VMs).
- CI does not pay a VM-spawn cost per build.
- Firecracker investment preserved for the use cases where it actually helps.
- Less CI noise: one fewer flaky job.

### Negative

- `rchd` becomes a new daemon to monitor. Mitigated by fail-open design and `rch doctor` healthcheck.
- One more abstraction in the build pipeline (`rch exec --` prefix). Mitigated: when `rchd` is down, the prefix is a no-op.

### Neutral

- The `rust-ci.ext4` image keeps being maintained at low frequency so it does not bit-rot.
- `Firecracker VM lifecycle proof` CI job stays at 24 s as a fcctl-web healthcheck.

## Implementation Plan

Driven by the existing RCH deployment plan (linked) plus minor CI cleanup:

1. **Drop `vm-cargo-probe` from `.github/workflows/ci-firecracker.yml`.** Keep `vm-infrastructure` healthcheck.
2. **Install `rch` + `rchd` on bigbox** (`Phase 1` of the RCH plan).
3. **Configure workers** (`Phase 2`): localhost worker only initially, slot count = `nproc/4`.
4. **Start `rchd` as a systemd user unit** (`Phase 5`) ordered before `adf-orchestrator.service`.
5. **Verify the existing PreToolUse hook in `~/.claude/settings.json`** points at the deployed `rch` binary (`Phase 4`).
6. **Wire CI's `rust-build` job to dispatch via `rch exec --`** for cargo build/test/clippy (with fail-open fallback).
7. **Smoke test under load** (`Phase 6`): trigger 4 concurrent `cargo check` calls, confirm `rch queue` distributes them and `rch status` shows expected slot utilisation.

Step 1 is independent and immediate. Steps 2-5 are sequential. Steps 6-7 follow.

## Review

- 2026-05-15: revisit after one week of `rchd` in production. Measure: queue depth, slot utilisation, end-to-end cargo wall-time deltas vs. pre-RCH baseline.
- 2026-06-01: revisit whether any new use case has emerged for Firecracker on the build path. If not, formally archive the `vm-cargo-probe` design notes.
