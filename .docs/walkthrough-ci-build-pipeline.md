---
date: 2026-04-25
type: walkthrough+howto
audience: maintainers + contributors
related_adrs:
  - .docs/adr-rch-build-queue-not-firecracker-ci.md
  - .docs/adr-rust-build-cache-seaweedfs.md
---

# CI Build Pipeline Walkthrough

This document is the operator-facing reference for terraphim-ai's CI/CD on
bigbox. It explains what each workflow does, what the optimal build
configuration is, and how to diagnose the common failure modes.

## TL;DR

- **Canonical workflow**: `.github/workflows/ci-firecracker.yml`. It uses the
  optimal stack: `rch exec --` dispatch + sccache + SeaweedFS S3 cache,
  pinned to the bigbox runner. Use it as the template for any new pipeline.
- **Legacy workflows**: `ci-main.yml`, `ci-pr.yml`, `ci-native.yml` predate
  the ADRs and use the GitHub-Actions cache backend on the generic
  `[self-hosted, Linux, X64]` runner pool. They mostly work but flake on
  `sccache: caused by: Connection reset by peer` because GHA cache is
  rate-limited and adds a network hop. Migration is tracked as follow-up.
- **VM lifecycle proof flake**: fcctl-web reports `status=running` as soon
  as the firecracker VMM is up, but the in-guest sshd needs a few more
  seconds before it accepts connections. The Execute step now retries 6×
  with 5s backoff (commit `129364a5`). No more single-shot failures.

## Architecture

```
                   GitHub Actions trigger (push/PR/dispatch)
                                  │
                                  ▼
           ┌──────────────────────────────────────────┐
           │         self-hosted bigbox runner         │
           │  (4 actions-runner-N processes, 1 free)   │
           └──────────────────────┬───────────────────┘
                                  │
                       /home/alex/.local/bin/rch exec --
                                  │
                                  ▼
            ┌───────────────────────────────────────┐
            │   rchd v1.0.16 (systemd user unit)    │
            │   FIFO queue + 6 slots = nproc/4      │
            │   Socket: ~/.cache/rch/rch.sock       │
            └───────────────────┬───────────────────┘
                                │
                                ▼
                   bigbox-local worker (127.0.0.1)
                                │
                                ▼
          ┌─────────────────────────────────────────┐
          │  sccache → SeaweedFS S3 (172.26.0.1)    │
          │  Bucket: rust-cache                      │
          │  Prefix: terraphim-ai                    │
          │  CARGO_INCREMENTAL=0 (sccache compat)    │
          └─────────────────────────────────────────┘

                  ┌────────────────────────────────┐
                  │ Firecracker (sandbox shelf)    │
                  │ fcctl-web 127.0.0.1:8080       │
                  │ Used for VM-execution probe    │
                  │ NOT on the build path          │
                  └────────────────────────────────┘
```

## Workflow inventory

| Workflow | Trigger | Runner | Cache | rch exec? | Status |
|---|---|---|---|---|---|
| `ci-firecracker.yml` | PR (crates/**) + dispatch | `[self-hosted, bigbox]` | sccache + SeaweedFS S3 | yes | **canonical** |
| `ci-main.yml` | push to main | `[self-hosted, Linux, X64]` | sccache + GHA backend | no | flaky (GHA conn reset) |
| `ci-pr.yml` | pull_request | `[self-hosted, Linux, X64]` | sccache + GHA backend | no | flaky (same cause) |
| `ci-native.yml` | various | `[self-hosted, Linux, X64]` | sccache + GHA backend | no | usually green |
| `test-firecracker-runner.yml` | push | bigbox | n/a | n/a | green (healthcheck only) |
| `deploy-docs.yml` | push (docs/**) | hosted | n/a | n/a | green |
| `frontend-build.yml`, `tauri-build.yml` | manual | various | n/a | n/a | stale, needs review |
| `vm-execution-tests.yml`, `performance-benchmarking.yml` | manual | various | mixed | no | red, out of scope here |

**Recommendation**: migrate `ci-main.yml` and `ci-pr.yml` to the
canonical pattern (see "Migration recipe" below). After migration the
legacy workflows can be retired.

## ci-firecracker.yml: canonical reference

The file has two independent jobs.

### Job 1: `vm-infrastructure` (Firecracker VM lifecycle proof)

Smoke-tests the Firecracker sandbox path against fcctl-web running on
bigbox. Not on the build path; this proves the sandbox shelf is alive and
ready when an LLM tool call eventually needs it.

Steps:
1. `GET /health` on fcctl-web (3 retries × 2s).
2. `POST /api/vms` to create a focal-ci VM, parse `id`.
3. Poll `GET /api/vms/{id}` until `status=running` (up to 90s, 5s sleep).
4. **`POST /api/llm/execute` with retries** — runs `echo vm-ok && uname -r && id`
   inside the VM via SSH. Retries 6× with 5s backoff because sshd in the
   guest takes a few seconds longer than Firecracker's `running` state to
   accept connections.
5. `DELETE /api/vms/{id}` cleanup (always runs).

### Job 2: `rust-build`

The actual CI on the optimal stack.

```yaml
runs-on: [self-hosted, bigbox]
env:
  RUSTC_WRAPPER: /home/alex/.local/bin/sccache
  SCCACHE_BUCKET: rust-cache
  SCCACHE_ENDPOINT: http://172.26.0.1:8333
  SCCACHE_S3_USE_SSL: "false"
  SCCACHE_REGION: us-east-1
  SCCACHE_S3_KEY_PREFIX: terraphim-ai
  AWS_ACCESS_KEY_ID: any
  AWS_SECRET_ACCESS_KEY: any
  CARGO_INCREMENTAL: "0"

steps:
  - uses: actions/checkout@v4
  - name: sccache start and zero stats
    run: |
      /home/alex/.local/bin/sccache --start-server || true
      /home/alex/.local/bin/sccache --zero-stats
  - name: cargo fmt --check
    run: /home/alex/.local/bin/rch exec -- cargo fmt -- --check
  - name: cargo clippy
    run: /home/alex/.local/bin/rch exec -- cargo clippy -- -D warnings
  - name: cargo build --workspace
    run: /home/alex/.local/bin/rch exec -- cargo build --workspace
  - name: cargo test --workspace
    run: /home/alex/.local/bin/rch exec -- cargo test --workspace -- --skip test_chat_command
  - name: sccache stats
    if: always()
    run: /home/alex/.local/bin/sccache --show-stats
```

Three things make this optimal:

1. **Bigbox label pinning**: the runner pool has many machines but only
   bigbox has both `/home/alex/.local/bin/sccache` and routable access to
   the SeaweedFS bridge on `fcbr0`. Pinning prevents a job ending up on
   another runner with neither.
2. **rch dispatch**: `rch exec --` puts cargo into rchd's FIFO queue with
   slot accounting. CI competes for the same 6 slots as the 21+ ADF
   agents, so we don't oversubscribe the box. Fail-open: if rchd is down,
   rch transparently runs the command locally — CI never breaks.
3. **sccache + SeaweedFS**: shared, durable, fast. `CARGO_INCREMENTAL=0`
   matters — sccache cannot cache incremental compilations and the cold
   run originally lost 438 lookups to this. Always set it.

The `test_chat_command` skip is intentional — it requires LLM API
credentials not present in CI. Every other failure must be fixed at
source, not skipped.

## How-to: trigger and monitor

### Trigger manually

```bash
gh workflow run ci-firecracker.yml --ref main
gh workflow run ci-firecracker.yml --ref main -f vm_type=focal
```

### Watch a run

```bash
gh run list --workflow=ci-firecracker.yml --limit 5
gh run watch <RUN_ID>
gh run view <RUN_ID> --log-failed | tail -100
```

### Inspect bigbox infra health

```bash
ssh bigbox '
  systemctl --user status rchd | head -10
  /home/alex/.local/bin/rch status
  curl -sf http://172.26.0.1:8333 -o /dev/null -w "seaweed=%{http_code}\n"
  curl -sf http://127.0.0.1:8080/health
'
```

Expected steady state:

- rchd: `Active: active (running)`, `Posture: remote-ready`,
  `Workers: 1/1 healthy, 6/6 slots available`
- SeaweedFS: HTTP 200 on the master endpoint
- fcctl-web: `{"status":"healthy"}`

### Inspect rch queue under load

```bash
ssh bigbox '/home/alex/.local/bin/rch queue'
ssh bigbox '/home/alex/.local/bin/rch status'
```

If `Builds` doesn't increment when CI runs, the job is bypassing rch (or
running on the wrong runner; see troubleshooting).

## How-to: diagnose common failure modes

### "ssh: connect to host 172.26.0.2 port 22: Connection timed out"

Symptom: VM lifecycle proof fails at the Execute step.

Cause: in-guest sshd not yet listening when execute is called.

Fix: already handled — Execute step retries 6× with 5s backoff. If it
still fails after the retries, sshd in the VM image is broken. Check:

```bash
ssh bigbox 'sudo journalctl -u fcctl-web --since "30 min ago" | grep -E "VM_EVENT|TAP|execute"'
```

Investigate by running a manual VM and SSHing into it:

```bash
ssh bigbox '
  curl -sX POST http://127.0.0.1:8080/api/vms -H "Content-Type: application/json" -d "{\"vm_type\":\"focal-ci\"}" | python3 -m json.tool
  # then SSH to the IP shown
'
```

### "sccache: caused by: Connection reset by peer (os error 104)"

Symptom: ci-main, ci-pr, or any workflow using SCCACHE_GHA_ENABLED.

Cause: GitHub Actions cache backend is rate-limited and adds a network
hop. Self-hosted runners get throttled under concurrent jobs.

Fix: migrate the workflow to the canonical SeaweedFS config (see below).
Workaround until migrated: re-run the workflow.

### "Workers: 0/1 healthy" or "no slot available"

Symptom: rch falls open to local cargo, you see CI not increment
`rch status` Builds counter.

Cause: rchd is down, ssh to localhost from rch-wkr is blocked, or
worker config drifted.

Fix:

```bash
ssh bigbox '
  systemctl --user restart rchd
  sleep 2
  /home/alex/.local/bin/rch workers probe --all
  /home/alex/.local/bin/rch status
'
```

If probe shows `FAIL`, the sshd Match block at
`/etc/ssh/sshd_config.d/10-rch-localhost.conf` likely got removed. See
`infrastructure/rch-bigbox/deploy.sh`.

### "input resolves outside canonical root"

Symptom: rch refuses to dispatch a build.

Cause: the cwd is not under `/home/alex/projects/*` (the symlinked
canonical root is `/data/projects → /home/alex/projects`).

Fix: rch fails open in this case (just runs cargo locally), so CI is
unaffected. Local dev: move the project under `/home/alex/projects/`
or accept the local-cargo fall-open. See ADR for the topology rationale.

## Migration recipe (legacy workflows → canonical)

To move `ci-main.yml` or `ci-pr.yml` onto the optimal stack:

1. **Pin runner label**:
   ```diff
   - runs-on: [self-hosted, Linux, X64]
   + runs-on: [self-hosted, bigbox]
   ```

2. **Replace the GHA cache backend with SeaweedFS S3**:
   ```diff
   - - name: Configure sccache
   -   run: |
   -     echo "SCCACHE_GHA_ENABLED=true" >> $GITHUB_ENV
   -     echo "RUSTC_WRAPPER=sccache" >> $GITHUB_ENV
   + env:
   +   RUSTC_WRAPPER: /home/alex/.local/bin/sccache
   +   SCCACHE_BUCKET: rust-cache
   +   SCCACHE_ENDPOINT: http://172.26.0.1:8333
   +   SCCACHE_S3_USE_SSL: "false"
   +   SCCACHE_REGION: us-east-1
   +   SCCACHE_S3_KEY_PREFIX: terraphim-ai
   +   AWS_ACCESS_KEY_ID: any
   +   AWS_SECRET_ACCESS_KEY: any
   +   CARGO_INCREMENTAL: "0"
   ```

3. **Wrap cargo invocations through rch**:
   ```diff
   - - run: cargo build --workspace
   + - run: /home/alex/.local/bin/rch exec -- cargo build --workspace
   ```

4. **Drop the GHA setup-sccache action**; bigbox already has sccache at
   `/home/alex/.local/bin/sccache`. Replace the action with:
   ```yaml
   - name: sccache start and zero stats
     run: |
       /home/alex/.local/bin/sccache --start-server || true
       /home/alex/.local/bin/sccache --zero-stats
   ```

5. **Verify**: trigger the workflow, then on bigbox run
   `rch status` and check that `Builds` increments and that
   `sccache --show-stats` reports cache hits on a warm run.

## Configuration reference

### Live state on bigbox

| Path | Purpose |
|---|---|
| `/home/alex/.local/bin/rch{,d,-wkr}` v1.0.16 | RCH binaries |
| `/home/alex/.local/bin/sccache` | sccache binary |
| `/home/alex/.config/rch/{workers,config}.toml` | rch config |
| `/home/alex/.config/systemd/user/rchd.service` | rchd unit (enabled, ordered Before=adf-orchestrator) |
| `/etc/ssh/sshd_config.d/10-rch-localhost.conf` | pubkey-only auth from 127.0.0.1 for rch-wkr |
| `/data/projects` → `/home/alex/projects` | rch canonical root (symlink) |
| `/dp` → `/data/projects` | rch topology alias |
| `~/.profile` sources `~/.config/rust-cache.env` | sccache + SeaweedFS env for interactive shells |
| `/home/alex/actions-runner-{1..4}` | GitHub Actions runners |

### Mirrored under version control

| Path | Purpose |
|---|---|
| `infrastructure/rch-bigbox/` | rch config + deploy.sh + smoke_test.sh |
| `infrastructure/rust-cache-stack/` | SeaweedFS S3 stack |
| `infrastructure/firecracker-rust-ci/` | VM image build pipeline (sandbox shelf) |
| `.docs/adr-rch-build-queue-not-firecracker-ci.md` | Architectural decision: RCH not Firecracker for build queue |
| `.docs/adr-rust-build-cache-seaweedfs.md` | Architectural decision: SeaweedFS S3 for sccache backend |

## Re-deploy after a bigbox rebuild

```bash
ssh bigbox
cd ~/projects/terraphim/terraphim-ai/infrastructure/rch-bigbox
./deploy.sh
# verify
~/.local/bin/rch status
~/.local/bin/rch workers probe --all
```

The deploy script is idempotent. It handles the `/data/projects` symlink
topology, sshd Match drop-in, systemd user unit registration, and rch
config files. If the SeaweedFS stack is also down, redeploy that first
from `infrastructure/rust-cache-stack/`.

## Open follow-ups

- Migrate `ci-main.yml` and `ci-pr.yml` to the canonical pattern; retire
  the GHA cache backend altogether.
- Watch upstream rch issue #10
  (https://github.com/Dicklesworthstone/remote_compilation_helper/issues/10):
  if `RCH_CANONICAL_PROJECT_ROOT` becomes functional, drop the
  `/data/projects` symlink workaround.
- Decide on `/home/alex/terraphim-ai` (ADF terraphim working dir): leave
  as fall-open outlier or move under `/home/alex/projects/`.
- Add a periodic ADF + CI burst measurement to confirm rch is shaping
  load as expected (review date 2026-05-15 per ADR).
