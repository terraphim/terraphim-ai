# Research: ADF Mention Replay Storm

## Phase 1 — Disciplined Research

### 1. Problem Understanding

**What:** On ADF restart, `poll_mentions()` replays ALL unprocessed `@adf:agent-name` mentions from watched Gitea issues in a single tick. Each mention spawns a full agent process (opencode/claude). After extended downtime this creates a process storm.

**Impact:** On Apr 3 2026, 43 agents spawned simultaneously → 170+ processes → load average 301 → bigbox unresponsive for ~5 minutes.

**Who:** ADF operator (Alex). Any restart after >6h downtime triggers this.

**Success:** Mentions are processed incrementally with bounded concurrency. Restarts never cause storms.

### 2. Existing System Analysis

#### Current code (two versions in play)

| Component | Local (`feature/warp-drive-theme`) | Bigbox (`task/173-automata-orchestrator-integration`) |
|---|---|---|
| `MentionTracker` | In-memory `HashSet`, no persistence | Has `load_or_new()` + `Persistable` impl |
| `poll_mentions()` | Same in both: per-issue loop, fetch ALL comments, spawn per unprocessed mention | Same |
| Concurrency limit | **None** | **None** |
| Cursor | **None** — refetches all comments every poll | **None** |

#### Data flow

```
reconcile_tick() [every 30s]
  → poll_mentions()
    → for each issue in watch_issues (16 issues):
        fetch_comments(issue, None)  ← ALL comments, no cursor
        for comment in comments:
          parse_mentions(comment)
          if not tracker.is_processed(mention):
            spawn_agent(mention_def)    ← NO concurrency limit
            mark_processed(mention)
```

#### Key files

| File | Lines | Role |
|---|---|---|
| `crates/terraphim_orchestrator/src/mention.rs` | ~400 | MentionTracker, parse_mentions, resolve_mention |
| `crates/terraphim_orchestrator/src/lib.rs:1055-1170` | ~115 | poll_mentions() implementation |
| `crates/terraphim_orchestrator/src/config.rs` | ~20 | MentionConfig struct |
| `crates/terraphim_tracker/src/gitea.rs` | ? | fetch_comments() API call |

#### Existing issue

**Issue #186** already has a complete design for the fix:
- Replace per-issue polling with repo-wide cursor-based polling
- Single API call: `GET /repos/{owner}/{repo}/issues/comments?since={cursor}`
- Persistent cursor → no replay on restart
- Eliminates `watch_issues` config entirely

#### Why #186 wasn't implemented

The persistence code for `MentionTracker` was added in commit `1983aa0a` on branch `task/173-automata-orchestrator-integration`, but:
1. It only persists the `processed` HashSet — not a cursor
2. It still fetches ALL comments per issue (no `since` parameter)
3. It has no concurrency limit on spawning
4. The branch was never merged to main

### 3. Constraint Identification

| Constraint | Detail |
|---|---|
| **Gitea API** | `/repos/{owner}/{repo}/issues/comments?since=&limit=50` — confirmed working |
| **terraphim_persistence** | Already available, SQLite profile works on bigbox |
| **Concurrency** | bigbox has 125GB RAM but only 8 cores — 170 node processes will thrash |
| **MentionTracker.processed** | Serialised as HashSet<(u64,u64,String)> — grows unbounded |
| **Backward compat** | Old `watch_issues` config must degrade gracefully |
| **Existing branches** | `task/173-automata-orchestrator-integration` has partial persistence, not merged |

### 4. Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Cursor desync (missed mentions) | Low | Medium | Also keep processed set as safety net |
| Gitea API rate limit | Low | Low | One call per tick (every 60s with poll_modulo=2) |
| Large cursor gap (many new mentions) | Medium | High | Batch-process with max_dispatches_per_tick |
| Orphaned agents from storm | Already happened | High | Add max_concurrent_mentions config |

### 5. Assumptions

1. Gitea API `since` parameter uses `created_at` timestamp (confirmed via curl)
2. Comments are returned in chronological order
3. A single repo-level call is cheaper than N per-issue calls
4. `terraphim_persistence` DeviceStorage is initialised before `poll_mentions` runs

### 6. Unknowns

1. What happens if two mentions for the same agent arrive in one batch? (Should deduplicate by agent, only spawn one)
2. Should Safety agents (security-sentinel, drift-detector) be exempt from mention concurrency limits?
3. Is the `processed` HashSet in the old persistence unbounded? (Yes — needs TTL or size cap)

---

## Recommendation

Implement Issue #186 design with these additions:

1. **Cursor-based polling** (from #186) — single API call with `since=` parameter
2. **Persistent cursor** via `terraphim_persistence` — survives restarts
3. **`max_dispatches_per_tick`** (default: 3) — bounds concurrency per poll cycle
4. **`max_concurrent_mention_agents`** (default: 5) — total concurrent mention-spawned agents
5. **Startup guard** — on first run (no cursor), set cursor to `now` to skip backlog

This replaces Issue #249 (which I created prematurely — should be closed in favour of #186).
