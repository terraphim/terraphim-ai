# Runbook: Fix Worktree Disk-Usage Alert (dedup storm + frozen title)

**Issue**: `terraphim/terraphim-ai#2220` (gtr tracker id `3083`)
**Theme-ID**: `adf-health-alert`
**Severity**: P2 — a monitor that emits 21 duplicate, stale-frozen open issues over 24
days is not a monitor; it is noise that crowds the backlog and misdirects every
cron-driven agent. It does not block production, but it degrades the entire triage
signal.
**Status**: Patch verified against live state on bigbox (2026-06-29); awaiting operator
apply (root required to edit the unversioned runtime config).

## Symptom

The Gitea issue tracker for `terraphim/terraphim-ai` contains a **storm of 21 open
issues** (verified via paginated API, 2026-06-29) all titled
`[ADF] Worktree disk usage: <N> MB`, created between 2026-06-02 and 2026-06-25 —
**20 duplicates plus the canonical tracker #2220**. Each carries a different frozen
snapshot of the disk size. The canonical tracker (#2220 / gtr id 3083) claims:

> Current usage: 22464 MB (~22.4 GB) in `.worktrees/` — Worktree count: 24

That snapshot was last refreshed **2026-06-05 06:00 CEST** (24 days ago). Its own first
comment (by `root`, 2026-06-05 06:02) already says *"unchanged from previous reading"*.

## Ground truth — measured this session (2026-06-29 23:40 +02:00)

Every measurement below was taken from the same host the cron runs on, using the **exact
formula** the cron uses (`du -sm /data/projects/terraphim/terraphim-ai/.worktrees/`):

| Signal | Issue #2220 claims | Reality (live) | Drift |
|---|---|---|---|
| Worktree disk usage | `22464 MB` | **~730 MB** (725–731 across the session) | ~30× over-reported |
| Worktree count | `24` | **`11`** | 2.2× over-reported |
| `/data` filesystem | (implied critical) | **68% used, 1.1 TB free** | not low |

> The exact `du` value churns minute-to-minute as worktrees are created/pruned by the
> orchestrator. Measurements taken 2026-06-29 ~23:40 +02:00 ranged 725–731 MB. The
> **point is the order of magnitude** (≈0.7 GB, not 22 GB) and that it is far below the
> 10 GB threshold — not a precise figure. The runbook's fail-loud patch (below) is what
> guarantees the title can never freeze on a stale value again.

Reproduction (non-sudo, single filesystem, errors suppressed — identical to cron):

```
$ du -sm /data/projects/terraphim/terraphim-ai/.worktrees/ 2>/dev/null | cut -f1
725        # observed 2026-06-29 23:42 +02:00 (was 731 at 23:40; churns with worktree lifecycle)
$ ls -1d /data/projects/terraphim/terraphim-ai/.worktrees/*/ | wc -l
11
$ df -h /data
/dev/md2  3.5T  2.2T  1.1T  68%  /data
```

The guard `[ "$WORKTREE_SIZE" -gt 10240 ]` evaluates `725 > 10240` → **false**, so the
alert is currently silent. The 21 open issues are **orphans of a past genuine condition**
(June 2–25, when un-pruned worktree `target/` dirs pushed usage to ~20 GB) that the check
kept re-firing on every 4h cycle because it has no dedup.

## Root cause — verified by reading the source

The alert is emitted by the meta-coordinator health-check task block in the **deployed,
unversioned** runtime config (NOT in this repo):

```
/opt/ai-dark-factory/conf.d/terraphim.toml   (root:alex 0640)
```

Block `### 3. Disk Usage Check (worktrees)`, lines 101–107:

```toml
### 3. Disk Usage Check (worktrees)
WORKTREE_SIZE=$(du -sm /data/projects/terraphim/terraphim-ai/.worktrees/ 2>/dev/null | cut -f1 || echo 0)
if [ "$WORKTREE_SIZE" -gt 10240 ]; then
  echo "WARNING: worktrees using $WORKTREE_SIZE MB (>10GB)"
  gtr create-issue --owner terraphim --repo terraphim-ai --title "[ADF] Worktree disk usage: $WORKTREE_SIZE MB" --body "Worktree directory exceeds 10GB threshold.
Theme-ID: adf-health-alert"
fi
```

**Two co-defects:**

1. **No dedup / no update-in-place.** Every 4h cycle that trips the threshold calls
   `gtr create-issue` unconditionally — a brand-new issue, even when an identical open
   issue already exists. Result: 21 open duplicates across June 2–25 (verified via the
   paginated Gitea API this session — `gtr list-issues` returns only the newest 50 of
   285 open issues and would miss them all).
   search API this session).
2. **`|| echo 0` masks measurement failure.** If `du` ever fails (e.g. permission denied
   on a root-owned file under a worktree, or the directory is renamed), `WORKTREE_SIZE`
   silently becomes `0`. `0 > 10240` is false so no issue is created that cycle — but
   the **title of the canonical issue never updates**, because titles are write-once at
   creation. The frozen `22464 MB` in #2220's title is a direct artefact of this: the
   title captured whatever value was live at creation (2026-06-05) and has never moved,
   even though the body of newer duplicates carried different values.

> **Note on `sudo`**: unlike the tick-stall / agent-failure blocks in the same file
> (which incorrectly use `sudo journalctl` and are blocked by `no_new_privs` — see
> `docs/runbooks/adf-meta-coordinator-health-no-sudo.md`, PR #3035), this `du` block
> runs **without** `sudo`. That is correct as far as it goes, but it means `du`
> **under-counts** any subtree containing root-owned files (e.g. leftover from a
> `sudo cargo` run). The fail-loud precondition in the patch below catches this case.

## The patch (verified)

Three surgical changes to the `### 3. Disk Usage Check (worktrees)` block.

### Change 1 — fail loud instead of `|| echo 0`

Replace:
```bash
WORKTREE_SIZE=$(du -sm /data/projects/terraphim/terraphim-ai/.worktrees/ 2>/dev/null | cut -f1 || echo 0)
```
with:
```bash
WORKTREE_SIZE=$(du -sm /data/projects/terraphim/terraphim-ai/.worktrees/ 2>/dev/null | cut -f1)
if [ -z "$WORKTREE_SIZE" ]; then
  echo "FATAL: du -sm on .worktrees returned empty — measurement broken, aborting (would produce false signal)" >&2
  gtr create-issue --owner terraphim --repo terraphim-ai \
    --title "[ADF] Worktree disk measurement broken (du returned empty)" \
    --body "The disk-usage health check aborted because \`du -sm .worktrees/\` returned no value. Investigate permissions / path. The check refuses to emit a stale or zero value.
Theme-ID: adf-health-alert" 2>/dev/null || true
  exit 1
fi
```

### Change 2 — dedup: skip create if an open worktree-disk issue already exists

Two fidelity traps were discovered while verifying this guard (2026-06-29):

1. **Gitea `q=` search is full-text** (matches issue bodies, not just titles). A search
   for `"Worktree disk usage"` returns spurious body matches (#2941 spawner-tests,
   #2909 tick-stall, etc.). **Not safe for dedup.**
2. **`gtr list-issues --limit 100` silently caps at 50** (Gitea server max per page).
   With **285 open issues** on this repo, the newest-50 slice (#2919–3030) does not
   contain a single worktree-disk issue (they are #2009–2601). A naive
   `gtr list-issues | grep` guard would be a **no-op** and silently re-create dupes.

The faithful dedup therefore calls the **raw Gitea API with pagination** and filters
client-side by the canonical title prefix. Inline it as a shell function so the block
stays readable:

```bash
# --- dedup helper: print the number of any open worktree-disk issue, or empty ---
_adf_worktree_disk_open_issue() {
  python3 - <<'PY'
import os, json, urllib.request
T, U = os.environ["GITEA_TOKEN"], os.environ["GITEA_URL"]
hits = []
page = 1
while True:
    req = urllib.request.Request(
        f"{U}/api/v1/repos/terraphim/terraphim-ai/issues?state=open&type=issues&limit=50&page={page}",
        headers={"Authorization": f"token {T}"})
    batch = json.loads(urllib.request.urlopen(req, timeout=15).read())
    if not batch:
        break
    hits += [str(i["number"]) for i in batch
             if str(i.get("title", "")).startswith("[ADF] Worktree disk usage")]
    if len(batch) < 50:
        break
    page += 1
    if page > 20:   # hard cap: 20 * 50 = 1000 issues scanned
        break
print(hits[0] if hits else "")
PY
}
_EXISTING=$(_adf_worktree_disk_open_issue)
if [ -n "$_EXISTING" ]; then
  echo "SKIP: open worktree-disk issue #$_EXISTING already tracks this — not creating a duplicate."
fi
```

Then gate the `create-issue` on `$_EXISTING` being empty (full block shown in Change 3).

### Change 3 — full replacement block

```bash
### 3. Disk Usage Check (worktrees)
WORKTREE_SIZE=$(du -sm /data/projects/terraphim/terraphim-ai/.worktrees/ 2>/dev/null | cut -f1)
if [ -z "$WORKTREE_SIZE" ]; then
  echo "FATAL: du -sm on .worktrees returned empty — measurement broken, aborting" >&2
  gtr create-issue --owner terraphim --repo terraphim-ai \
    --title "[ADF] Worktree disk measurement broken (du returned empty)" \
    --body "du -sm on .worktrees returned no value. Investigate permissions / path.
Theme-ID: adf-health-alert" 2>/dev/null || true
  exit 1
fi
# --- dedup helper (paginates the API; gtr list-issues caps at 50 and misses older issues) ---
_adf_worktree_disk_open_issue() {
  python3 - <<'PY'
import os, json, urllib.request
T, U = os.environ["GITEA_TOKEN"], os.environ["GITEA_URL"]
hits = []; page = 1
while True:
    req = urllib.request.Request(
        f"{U}/api/v1/repos/terraphim/terraphim-ai/issues?state=open&type=issues&limit=50&page={page}",
        headers={"Authorization": f"token {T}"})
    batch = json.loads(urllib.request.urlopen(req, timeout=15).read())
    if not batch: break
    hits += [str(i["number"]) for i in batch
             if str(i.get("title", "")).startswith("[ADF] Worktree disk usage")]
    if len(batch) < 50: break
    page += 1
    if page > 20: break
print(hits[0] if hits else "")
PY
}
_EXISTING=$(_adf_worktree_disk_open_issue)
if [ "$WORKTREE_SIZE" -gt 10240 ] && [ -z "$_EXISTING" ]; then
  echo "WARNING: worktrees using $WORKTREE_SIZE MB (>10GB)"
  gtr create-issue --owner terraphim --repo terraphim-ai \
    --title "[ADF] Worktree disk usage: $WORKTREE_SIZE MB" \
    --body "Worktree directory exceeds 10GB threshold.

Current usage: $WORKTREE_SIZE MB in /data/projects/terraphim/terraphim-ai/.worktrees/
$(ls -1d /data/projects/terraphim/terraphim-ai/.worktrees/*/ 2>/dev/null | wc -l) worktrees present.
$(du -sm /data/projects/terraphim/terraphim-ai/.worktrees/*/ 2>/dev/null | sort -rn | head -5)

Action required: Review and prune stale worktrees to reclaim disk space.
Theme-ID: adf-health-alert"
elif [ "$WORKTREE_SIZE" -gt 10240 ]; then
  echo "SKIP: worktrees at $WORKTREE_SIZE MB but open issue #$_EXISTING already tracks this."
fi
```

The enriched body also lists the top-5 worktrees by size, so an operator seeing a fresh
alert knows **which** worktree to prune without re-running `du`.

## Apply procedure (operator with root)

```bash
# 1. Back up the deployed config
sudo cp /opt/ai-dark-factory/conf.d/terraphim.toml \
        /opt/ai-dark-factory/conf.d/terraphim.toml.bak-2220-$(date +%Y%m%d-%H%M%S)

# 2. Edit the "### 3. Disk Usage Check (worktrees)" block in place, replacing the
#    5-line original with the full block from Change 3 above. Use your editor of
#    choice, e.g.:
sudo $EDITOR /opt/ai-dark-factory/conf.d/terraphim.toml

# 3. Verify the patch took
sudo grep -n 'du -sm.*worktrees' /opt/ai-dark-factory/conf.d/terraphim.toml   # should show the new, un-guarded du
sudo grep -c '|| echo 0' /opt/ai-dark-factory/conf.d/terraphim.toml           # should drop by 1 from baseline
sudo grep -c '_EXISTING' /opt/ai-dark-factory/conf.d/terraphim.toml           # should be >= 2 (set + used)

# 4. Reload the orchestrator so it re-reads conf.d
sudo systemctl restart adf-orchestrator.service

# 5. Smoke-test the patched formula manually (no root needed for the measurement itself):
WORKTREE_SIZE=$(du -sm /data/projects/terraphim/terraphim-ai/.worktrees/ 2>/dev/null | cut -f1)
echo "WORKTREE_SIZE=$WORKTREE_SIZE  (observed ~725-731 MB on 2026-06-29; must be non-empty)"
```

## Verification (acceptance)

After the next 4h cron cycle, OR after a manual dry-run of the patched block:

- [ ] `stat -c '%y' /opt/ai-dark-factory/conf.d/terraphim.toml` shows a fresh mtime.
- [ ] `grep -c '|| echo 0'` on the config dropped by exactly 1 (the worktree block no
      longer masks failure; other blocks untouched).
- [ ] A manual run of `du -sm .worktrees/ | cut -f1` returns a **non-empty** value
      (observed ~725–731 MB on 2026-06-29; the point is non-emptiness, never `0`
      unless the dir is genuinely empty).
- [ ] No new `[ADF] Worktree disk usage` issue is created while usage is below 10 GB
      (current state). The cycle log should print neither WARNING nor SKIP.
- [ ] **Dedup guard**: temporarily force `WORKTREE_SIZE=99999` in a manual run; exactly
      ONE new issue is created. Run the block a second time; it must print
      `SKIP: ... already tracks this` and create nothing.
- [ ] The 20 duplicate stale issues (June 2–25; the 21st, #2220, is the canonical
      tracker and stays open) are batch-closed (see next section).

## Batch-close the duplicate stale issues

The duplicates carry no information beyond the canonical tracker (#2220) and actively
harm triage (every `gtr ready` cycle re-surfaces them). Close them in a single sweep
with a comment pointing here. **Do not close #2220 itself** — it remains the canonical
open tracker until the operator applies this runbook and confirms the alert is healthy.

> **Pagination is mandatory.** As of 2026-06-29 this repo has **285 open issues** and
> there are **21** open `[ADF] Worktree disk usage` duplicates spanning #2009–2601.
> Gitea caps `limit` at 50/page, so a single `?limit=100` call (which the server
> truncates to 50) would miss **all 21** — the loop below paginates correctly.

```bash
python3 <<'PY'
import os, json, urllib.request
T, U = os.environ["GITEA_TOKEN"], os.environ["GITEA_URL"]
OWNER, REPO = "terraphim", "terraphim-ai"
CANONICAL = 2220   # do not close the canonical tracker

# 1. Paginate to collect EVERY open worktree-disk duplicate (excluding canonical).
to_close = []
page = 1
while True:
    req = urllib.request.Request(
        f"{U}/api/v1/repos/{OWNER}/{REPO}/issues?state=open&type=issues&limit=50&page={page}",
        headers={"Authorization": f"token {T}"})
    batch = json.loads(urllib.request.urlopen(req, timeout=15).read())
    if not batch:
        break
    for i in batch:
        if (str(i.get("title", "")).startswith("[ADF] Worktree disk usage")
                and i.get("number") != CANONICAL):
            to_close.append(i["number"])
    if len(batch) < 50:
        break
    page += 1
    if page > 20:
        break

print(f"closing {len(to_close)} duplicate(s): {sorted(to_close)}")

# 2. Comment + close each one.
BODY = """Closing as a duplicate of the canonical worktree-disk tracker #2220.

This issue is one of 21 near-identical alerts (June 2-25) created by a health check
with no dedup guard. Live measurement on 2026-06-29 shows .worktrees at ~730 MB
(not the frozen value in this title) and /data at 68% - the underlying condition has
already cleared. The patched check (paginated dedup + fail-loud) is documented in
`docs/runbooks/adf-worktree-disk-alert-dedup.md`.

Theme-ID: adf-health-alert"""

for idx in sorted(to_close):
    # comment
    data = json.dumps({"body": BODY}).encode()
    req = urllib.request.Request(
        f"{U}/api/v1/repos/{OWNER}/{REPO}/issues/{idx}/comments",
        data=data,
        headers={"Authorization": f"token {T}", "Content-Type": "application/json"},
        method="POST")
    urllib.request.urlopen(req, timeout=15).read()
    # close
    data = json.dumps({"state": "closed"}).encode()
    req = urllib.request.Request(
        f"{U}/api/v1/repos/{OWNER}/{REPO}/issues/{idx}",
        data=data,
        headers={"Authorization": f"token {T}", "Content-Type": "application/json"},
        method="PATCH")
    urllib.request.urlopen(req, timeout=15).read()
    print(f"  closed #{idx}")
PY
```

## Deeper defect (same as #4300 — filed separately)

The deployed `/opt/ai-dark-factory/conf.d/*.toml` is **unversioned** — not in any git
repo (verified: `git -C /opt/ai-dark-factory rev-parse` fails, no `.git` exists). That
is the root cause that let 21 duplicates accumulate over 24 days and let the frozen
title go unchallenged. The version-controlled twins
(`terraphim-agents/.../tests/fixtures/conf.d/terraphim.toml`,
`scripts/adf-setup/tests/expected/terraphim.toml`) do **not** contain this task block
at all (verified this session: grep for `Worktree disk` / `du -sm` returns nothing in
either twin). Bringing `conf.d/` under version control — or generating it from a
tracked template at deploy time — is the structural fix that prevents recurrence.
Already noted as a follow-up to PR #3035 (the #4300 journalctl twin of this defect).

## Why this runbook lives in `terraphim-ai` (not `terraphim-agents`)

The health check creates issues in **this** repo (`terraphim/terraphim-ai`) and
references this repo's worktrees. Per AGENTS.md dual-remote protocol, this runbook is
pushed to both `origin` (GitHub) and `gitea` mirrors, giving operators a reviewable,
reproducible artefact that satisfies the spirit of Bigbox Rule #1 (git pull/push, not
ad-hoc `scp`) even though the target file itself is not yet version-controlled.

---

**Theme-ID**: `adf-health-alert`
**Twin of**: PR #3035 (`docs/runbooks/adf-meta-coordinator-health-no-sudo.md`, issue #4300)
**Refs**: `terraphim/terraphim-ai#2220` (Gitea index), gtr tracker id `3083`
