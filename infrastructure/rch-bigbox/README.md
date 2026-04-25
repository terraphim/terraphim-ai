# RCH on bigbox

Per [.docs/adr-rch-build-queue-not-firecracker-ci.md](../../.docs/adr-rch-build-queue-not-firecracker-ci.md),
all cargo invocations on bigbox flow through `rchd`'s queue + slot accounting
so the 21+ ADF agents and CI share a single dispatch surface.

This directory mirrors the live config so it is reproducible. Files here are
deployed under `/home/alex/.config/rch/` and `/home/alex/.config/systemd/user/`
on bigbox. System-wide files (`/etc/ssh/sshd_config.d/`, `/dp` symlink,
`/data/projects/terraphim-ai` bind-mount) are listed in the deployment script.

## Files

| File | Deployed at | Purpose |
|---|---|---|
| `workers.toml` | `~/.config/rch/workers.toml` | Worker pool definition; one bigbox-local worker, 6 slots (nproc/4) |
| `config.toml` | `~/.config/rch/config.toml` | RCH user config; canonical_root left at rch default `/data/projects` |
| `rchd.service` | `~/.config/systemd/user/rchd.service` | Daemon unit, ordered `Before=adf-orchestrator.service` |
| `10-rch-localhost.conf` | `/etc/ssh/sshd_config.d/10-rch-localhost.conf` | sshd Match block: pubkey-only auth from 127.0.0.1 for user alex (rch-wkr SSHes localhost; bigbox sshd otherwise requires 2FA) |
| `deploy.sh` | n/a -- run from this directory on bigbox | Idempotent installer; safe to re-run |

## Required system state on bigbox

These exist outside this directory but are required for RCH to dispatch
terraphim-ai builds:

- `/dp -> /data/projects` symlink (`sudo ln -sfn /data/projects /dp`).
  rch's `topology audit` enforces `/dp` as alias for the canonical root.
- `/data/projects/terraphim-ai` bind-mount of `/home/alex/projects/terraphim/terraphim-ai`.
  rch's hook normalizes project paths and rejects anything outside the
  canonical root `/data/projects`. Bind-mounting the workspace satisfies the
  topology check without moving the actual checkout. Make persistent via
  fstab:
  ```
  /home/alex/projects/terraphim/terraphim-ai  /data/projects/terraphim-ai  none  bind  0  0
  ```
- `~/.local/bin/rch{,d,-wkr}` binaries (install with the upstream script):
  ```bash
  curl -fsSL https://raw.githubusercontent.com/Dicklesworthstone/remote_compilation_helper/main/install.sh | bash
  ```

## Verifying

```bash
rch status                    # Daemon + workers + posture
rch workers probe --all       # SSH reach + capabilities
rch doctor                    # Comprehensive health check
rch diagnose -- cargo build   # Show interception decision
```

Expected steady state:
- `Posture : remote-ready (All workers healthy, remote compilation available)`
- `Workers : 1/1 healthy, 6/6 slots available`
