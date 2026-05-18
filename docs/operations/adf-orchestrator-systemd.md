# ADF Orchestrator systemd Pre-Start Sweep

## Purpose

The ADF orchestrator runs as a long-lived service and accumulates
stale worktrees when sub-process agents (notably the compound-review
agent and container-build agents) crash or are terminated before
their cleanup hooks run. Some of these residues are owned by `root`
because they include build artefacts written by container processes
that escalated privileges, so the orchestrator running as a service
user cannot reclaim them.

This document describes the root-privileged `ExecStartPre` hook that
sweeps the residue before the orchestrator starts. It is the Layer 3
defence from the four-layer worktree lifecycle plan (epic
`terraphim/terraphim-ai#1567`, issue `#1571`); Layers 1 and 2 prevent
new residue from accumulating at the source, and Layer 4 adds a
periodic safety net. Layer 3 is the only layer that can run as root,
so it is the only layer that can reliably reclaim root-owned trees.

The sweep script lives in-tree at
`scripts/adf-setup/adf-cleanup.sh`. It is POSIX `sh`, idempotent, and
emits a single summary line to stdout in the form
`adf-cleanup: swept=N failed=M repo=PATH`.

## Drop-in snippet

Add the following systemd drop-in at
`/etc/systemd/system/adf-orchestrator.service.d/cleanup.conf`:

```ini
[Service]
Environment=ADF_REPO_PATH=/data/projects/terraphim/terraphim-ai
Environment=ADF_WORKTREE_ROOT=/data/projects/terraphim/terraphim-ai/.worktrees
ExecStartPre=/opt/ai-dark-factory/bin/adf-cleanup.sh
```

The three environment variables `ADF_REPO_PATH`, `ADF_WORKTREE_ROOT`,
and `ADF_AGENT_TMP_ROOT` are all overridable. `ADF_AGENT_TMP_ROOT`
defaults to `/tmp/adf-worktrees` and rarely needs an override on the
bigbox host.

`ExecStartPre` runs synchronously before `ExecStart`, inherits the
service's environment, and runs with the unit's privileges. Because
the `adf-orchestrator.service` unit on bigbox runs as `root` for the
duration of the pre-start hook before dropping privileges for the
main process, the sweep can reclaim root-owned residue.

## Manual install

There is currently no in-tree installer. Until one lands, deploy the
script manually from a checkout of `main`:

```bash
sudo install -m 750 -o root -g root \
    scripts/adf-setup/adf-cleanup.sh \
    /opt/ai-dark-factory/bin/adf-cleanup.sh
sudo systemctl daemon-reload
sudo systemctl restart adf-orchestrator
```

The `install` invocation sets ownership to `root:root` and mode
`0750` so the script is readable and executable by root and the
`root` group only; this matches the principle of least privilege for
a script that runs as root.

## Verification

The full verification recipe is in §8.4 of
`docs/design/adf-worktree-lifecycle-design.md`. The short version:

```bash
# 1. Pre-seed a root-owned residue.
ssh bigbox 'sudo mkdir -p \
    /data/projects/terraphim/terraphim-ai/.worktrees/review-rootowned/target && \
    sudo chown -R root:root \
    /data/projects/terraphim/terraphim-ai/.worktrees/review-rootowned'

# 2. Restart the service.
ssh bigbox 'sudo systemctl restart adf-orchestrator'

# 3. Confirm the sweep line in the journal.
ssh bigbox 'journalctl -u adf-orchestrator -n 50 | grep adf-cleanup'
# expected: "adf-cleanup: swept=1 failed=0 repo=/data/..."

# 4. Confirm the residue is gone.
ssh bigbox 'ls /data/projects/terraphim/terraphim-ai/.worktrees/review-rootowned 2>/dev/null'
# expected: empty
```

The in-tree shell test at
`scripts/adf-setup/tests/test_adf_cleanup.sh` exercises the same
control flow against a disposable git repo under `mktemp -d` and is
safe to run in CI without privileged residue. Run it directly:

```bash
./scripts/adf-setup/tests/test_adf_cleanup.sh
```

It exits 0 on PASS and 1 on FAIL.

## Rollback

To disable the pre-start sweep:

```bash
sudo rm /etc/systemd/system/adf-orchestrator.service.d/cleanup.conf
sudo rm /opt/ai-dark-factory/bin/adf-cleanup.sh
sudo systemctl daemon-reload
sudo systemctl restart adf-orchestrator
```

The orchestrator will restart without the sweep step; stale residue
will then accumulate again until either Layers 1 and 2 prevent it
upstream or the sweep is re-enabled.
