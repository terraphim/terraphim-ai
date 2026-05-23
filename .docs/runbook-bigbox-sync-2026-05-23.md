# Runbook: Bigbox Sync + Deploy

**Created**: 2026-05-23
**Refs**: #1817 step (c)
**Script**: `scripts/bigbox-sync.sh`

## What it does

1. Validates working tree is clean (refuses if dirty -- memory rule: no `--hard` on bigbox)
2. `git fetch origin --quiet`
3. `git checkout main && git merge --ff-only origin/main`
4. `cargo build --release -p terraphim_orchestrator`
5. `sudo install -m 755 target/release/adf /usr/local/bin/adf`
6. `sudo systemctl restart adf-orchestrator`
7. Verifies `is-active`

No `--hard`. No `-X theirs`. No destructive operations. If the working tree is dirty, the script exits non-zero with the dirty file list and instructions to inspect before re-running.

## Default repo

`/data/projects/terraphim/terraphim-ai-fresh` -- the fresh clone created on 2026-05-23 after the original `/data/projects/terraphim/terraphim-ai` `.git` was found corrupted (see #1818).

Override via `BIGBOX_REPO` env var:

```bash
BIGBOX_REPO=/some/other/path bash scripts/bigbox-sync.sh
```

## Use

### Normal deploy

```bash
ssh bigbox
cd /data/projects/terraphim/terraphim-ai-fresh
bash scripts/bigbox-sync.sh
```

### Sync without restart (for testing the binary first)

```bash
bash scripts/bigbox-sync.sh --no-restart
# Then manually:
sudo install -m 755 target/release/adf /tmp/adf-test
/tmp/adf-test --version
# When happy:
sudo install -m 755 target/release/adf /usr/local/bin/adf
sudo systemctl restart adf-orchestrator
```

## Failure modes

| Symptom | Cause | Recovery |
|---|---|---|
| `working tree is dirty` exit 3 | uncommitted changes | `git status` and either commit or stash before re-running |
| `cannot fast-forward` | local branch diverged | usually means someone pushed directly; `git log --oneline -10 main origin/main` to inspect; resolve manually |
| `cargo build` fails | upstream regression | check `crates/*/Cargo.toml` for breaking changes; consider `git revert` of the offending commit |
| `systemctl restart` non-zero | bad config | `journalctl -u adf-orchestrator -n 50` to see the error |

## When to use the OLD /data/projects/terraphim/terraphim-ai

Never (it's corrupted, see #1818). The orchestrator at `/usr/local/bin/adf` reads `working_dir` and `taxonomy_path` from `/opt/ai-dark-factory/orchestrator.toml` which currently still points at the corrupted dir; tracked by #1821.

If you must read from there for forensic purposes:

```bash
cd /data/projects/terraphim/terraphim-ai
git --git-dir=.git fsck --no-dangling | head
```

## Related

- `.docs/runbook-adf-memory-watchdog-2026-05-23.md` -- install BEFORE first deploy after restart
- #1817 -- tracking issue
- #1818 -- the corrupted repo this script avoids
- #1821 -- orchestrator.toml still points at the corrupted dir (next deploy should fix)
