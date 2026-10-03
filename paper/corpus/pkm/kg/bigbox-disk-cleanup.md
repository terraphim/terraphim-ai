# Bigbox Disk Cleanup Incident

synonyms:: bigbox disk cleanup, disk space incident, docker ci prune, odilo ci cache, buildx cache volumes, dlt-rust-ci-cache, github actions runner cleanup, self-hosted runner disk leak

## Incident Summary

On 15 July 2026, bigbox (self-hosted GitHub Actions runner and Rust build host) root filesystem `/dev/md2` (3.5T) reached 78% with only 734G free. The volume was growing at roughly **550GB/day** while Odilo CI was active.

## Root Cause

The Odilo GitHub Actions workflow setup created fresh Docker artefacts on every run and never removed them:

- One **7.65GB `dlt-rust-ci-cache` image per commit SHA** — 69 images totalled ~528GB.
- One **17.4GB `buildx_buildkit_odilo-gha-builder-<runID>-rust-build-cache0_state` volume per CI run** — 35 such volumes, ~575GB, created in ~28 hours.
- **209 per-job buildx builder containers** that stayed `Up` for hours or days after runs finished, pinning their cache volumes.

No automated Docker cleanup existed. The only related cron, `cleanup-runners.sh`, ran weekly and only cleared GitHub Actions runner `_work` directories.

## Investigation Commands

```bash
df -h
sudo du -xh -d1 /
lsof +L1
docker system df -v
sqlite3 "file:$HOME/.local/share/opencode/opencode.db?mode=ro" ".tables"
```

## Actions Taken

1. Killed **204 zombie buildkit containers** (sparing active runs under six hours old).
2. Removed **35 per-run rust-build-cache volumes**.
3. Removed **67 old `dlt-rust-ci-cache` images**, keeping the two newest.
4. Pruned dangling images.
5. Installed nightly `/etc/cron.daily/docker-ci-prune` on bigbox (runs at 06:25).
6. Cleaned **41 Rust `target/` folders** on bigbox (~246G) and **five** on the local Mac (~76GiB).
7. Wrote `~/.local/bin/opencode-prune` to prune and VACUUM `~/.local/share/opencode`.
8. Filed `zestic-ai/odilo#554` for CI-side root-cause fixes.

## Results

| Host | Before | After |
|---|---|---|
| bigbox `/dev/md2` | 78% (734G free) | 41% (2.0T free) |
| Mac `/dev/disk3s1` | 91% (91G free) | 77% (214G free) |

## Scripts

### `/etc/cron.daily/docker-ci-prune` (bigbox)

Kills leaked builders older than 12 hours, removes orphaned `rust-build-cache0_state` volumes, keeps the two newest `dlt-rust-ci-cache` images, and prunes dangling images.

Log: `/var/log/docker-ci-prune.log`

### `~/.local/bin/opencode-prune` (Mac)

Prunes opencode sessions older than 90 days, deletes matching files under `storage/` and `tool-output/`, optionally removes idle `snapshot/` bare repos, and VACUUMs `opencode.db`.

## Related

- `zestic-ai/odilo#554`: CI root cause fix — stop leaking per-run buildx builders, reconsider per-commit ci-cache images, bound standing caches, and add registry tag retention.
