---
Status: Accepted
Date: 2026-04-24
Deciders: Alex Mikhalev
Related: .docs/design-firecracker-ci-acceleration.md, plans/adf-flywheel-outstanding-actions.md
---

# ADR: Dedicated SeaweedFS Instance for Rust Build Cache on fcbr0

## Status

Accepted -- 2026-04-24.

## Context

Three independent workloads on bigbox need a shared Rust compilation cache:

1. **GitHub Actions self-hosted runner** -- currently takes 38 min for `cargo build` + `cargo test --workspace` (run 24882193163, cold cache).
2. **21 ADF agents** -- fire concurrent `cargo build` / `cargo clippy` / `cargo test` against the shared terraphim-ai workspace. RCH plan (plans/adf-flywheel-rch-bigbox-deployment.md) serialises invocations but does not avoid recomputation.
3. **Firecracker CI VMs** -- once the Earthly `rust-ci-rootfs` target is materialised and registered in `fcctl-images.yaml`, CI will dispatch `cargo build` / `cargo test` into isolated VMs via fcctl-web `/api/llm/execute`. VMs start cold, no local `target/` or registry.

All three populations compile overlapping workspaces. Without a shared cache, each is independently cold; together they recompute the same artefacts continuously.

`sccache` keys object files by `(compiler version, source hash, flags)` and supports S3-compatible backends. A single shared bucket allows the warm-build artefacts produced by any one of the three populations to satisfy `rustc` calls in the other two.

## Decision

Stand up a **dedicated SeaweedFS instance** on bigbox, separate from the existing Gitea compose stack, bound to the `fcbr0` bridge IP (172.26.0.1) so that Firecracker VMs and host processes share a single S3 endpoint for build cache.

### Layout

- Compose stack path: `~/rust-cache-stack/docker-compose.yml`
- S3 endpoint: `http://172.26.0.1:8333`
- Bucket: `rust-cache`
- Volume: dedicated docker volume `rust-cache-data`
- Independent lifecycle from `~/gitea-stack`
- Systemd user unit so it restarts before `fcctl-web.service` and `adf-orchestrator.service`

### Consumers

All three populations set:
```
RUSTC_WRAPPER=sccache
SCCACHE_BUCKET=rust-cache
SCCACHE_ENDPOINT=http://172.26.0.1:8333
SCCACHE_S3_USE_SSL=false
SCCACHE_REGION=us-east-1
```

Plus per-consumer integration:
- Self-hosted runner: env vars in `.github/workflows/ci-firecracker.yml` rust-build job
- ADF agents: env vars in `adf-orchestrator.service` environment
- Firecracker VMs: baked into Earthly `rust-ci-docker-image` target

## Alternatives Considered

### A. Local on-disk `SCCACHE_DIR` only

Simplest. Zero new infra. Covers host runner + ADF agents (both on bigbox filesystem).

**Rejected**: does not cross the Firecracker VM boundary. Once CI moves into VMs, host disk cache becomes unreachable without virtio-fs mounting, which defeats the isolation purpose of the VMs.

### B. Reuse Gitea's existing SeaweedFS bucket in `~/gitea-stack`

Works. SeaweedFS is already running. Add a bucket, point sccache at it.

**Rejected**:
- **Blast radius**: Gitea (`git.terraphim.cloud`) is authoritative for task tracking (memory: "Gitea is the single source of truth"). A runaway rust cache fill threatens production infra.
- **Backup coupling**: Gitea's volume has a backup policy; rust objects are regeneratable noise that would bloat backups.
- **Lifecycle coupling**: `cd ~/gitea-stack && docker compose down` for Gitea maintenance takes the build cache with it.
- **Network reachability**: Gitea's SeaweedFS is on `gitea-stack_gitea` docker network; exposing it to Firecracker VMs on `fcbr0` requires additional port binding or network bridging.
- **Precedent**: memory note "always use `cd ~/gitea-stack && docker compose up -d server`, never standalone `docker run`" exists because mixing concerns across this stack has bitten us before.

### C. External S3 (AWS, Cloudflare R2, etc.)

Provider-operated, no local ops burden.

**Rejected for now**: egress latency from Firecracker VMs and bigbox adds minutes over LAN S3. Cost is nonzero. All consumers are on one box; external S3 is overkill until we add off-box workers.

## Consequences

### Positive

- **Cache sharing across three consumer populations** with one deployment.
- **Independent lifecycle**: can `docker compose down` the cache stack without touching Gitea or vice versa.
- **Reachable from VMs**: binding to `fcbr0` means Firecracker guests reach it via 172.26.0.1 with no NAT / port-forwarding gymnastics.
- **Scales to remote workers**: if RCH grows off-box workers, expose S3 via Tailscale or similar; no architectural change.
- **Observable**: dedicated bucket means `du`, S3 metrics, and eviction policies are straight rust-cache metrics, not mixed with Gitea data.

### Negative

- **One more service to run**: a second SeaweedFS instance adds ~200 MB RAM and one more systemd unit to monitor.
- **First-build cost unchanged**: sccache is a cache; the first build of any new dep still compiles from source. Warm-cache wins only materialise after a few runs.
- **Potential cache misses on proc-macros / non-deterministic build.rs**: `fff-search`'s build.rs (which panics without `--features zlob` in CI) is a known risk. Fix the feature-flag handling before wiring sccache, or the panic itself gets cached.

### Neutral

- Cache size will grow to tens of GB. Dedicated volume means `SCCACHE_CACHE_SIZE=50G` bounds this cleanly without threatening other services.

## Implementation Notes

1. Create `~/rust-cache-stack/docker-compose.yml` on bigbox with seaweedfs master + volume + s3 services, bound to `172.26.0.1:8333`.
2. Create systemd user unit `rust-cache-stack.service` that runs `docker compose up -d` / `down` in that directory; ordered `Before=fcctl-web.service adf-orchestrator.service`.
3. Install `sccache` binary on bigbox host and bake into Earthly `rust-ci-docker-image` target.
4. Pre-wire env vars in CI workflow before measuring baseline, so one change to `ci-firecracker.yml` captures both `SCCACHE_*` and timing.
5. First CI run writes cache; second CI run measures warm-cache speedup. Expected: 38 min cold -> ~10-15 min warm.
6. Add `/health` probe target for the cache stack to `rch doctor` once RCH is deployed.

## Risks and Mitigations

| Risk | Mitigation |
|---|---|
| `fff-search` build.rs panic gets cached | Fix feature-flag handling in `fff-search` before enabling sccache on that crate, or set `CARGO_INCREMENTAL=0` and `SCCACHE_IGNORE=fff-search` |
| Cache poisoning by corrupted build | sccache validates compiler version; Rust version bumps naturally bust the cache. `SCCACHE_CACHE_SIZE=50G` with LRU eviction bounds worst-case. |
| fcbr0 goes down during maintenance | SeaweedFS binds to fcbr0 only. When bridge is down, sccache falls back to local compilation (fail-open). Document in runbook. |
| Concurrent writes from 21 agents | SeaweedFS handles concurrent S3 writes via its own locking. sccache writes are keyed by content hash; duplicate writes are idempotent. |

## Review

- 2026-05-15: measure cold vs warm CI runtime, cache hit rate. If hit rate < 50 % or no measurable speedup, revisit before proceeding to Firecracker VM integration.
- 2026-06-01: revisit whether Gitea's SeaweedFS can be consolidated with rust-cache (if both are stable and resource-light). Do not force this; separation is cheap.
