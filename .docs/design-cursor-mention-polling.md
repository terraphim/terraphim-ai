# Implementation Plan: Cursor-Based Mention Polling

**Research doc:** `.docs/research-mention-replay-storm.md`  
**Gitea issue:** #186  
**Closes:** #249 (superseded)

---

## 5/25 Elimination

**IN (top 5):**
1. Cursor-based repo-wide polling (single API call)
2. Persistent cursor via terraphim_persistence
3. max_dispatches_per_tick rate limit
4. Startup guard (cursor=now on first run)
5. Backward compat deprecation of watch_issues

**AVOID:**
- Per-issue depth tracking (remove, cursor makes it unnecessary)
- Complex deduplication logic
- Webhook-driven mentions (Issue #149, deferred — requires HTTP server in ADF)
- **Gitea Notifications API** (`/notifications`) — evaluated and rejected: doesn't include comment bodies, aggregates by issue not comment, requires N extra API calls to parse `@adf:` patterns. See comparison below.
- Processed HashSet persistence (cursor replaces it)
- Custom SQLite table for cursor (use existing KV store)

### Approach Comparison (evaluated Apr 3 2026)

| | Repo comments API | Notifications API | Webhooks |
|---|---|---|---|
| **API calls/tick** | 1 | 1 + N (fetch bodies) | 0 (push) |
| **Comment body** | Included ✅ | Not included ❌ | Included ✅ |
| **Cursor** | `since` param ✅ | mark-as-read ✅ | N/A |
| **Replay safe** | Yes (cursor) | Yes (read state) | Misses offline events |
| **Setup** | None | User subscription | HTTP endpoint in ADF |
| **Winner** | ✅ **This one** | ❌ Too many calls | ❌ Too complex |

The repo-wide comments endpoint (`GET /repos/{owner}/{repo}/issues/comments?since=&limit=50`) gives full comment bodies with `@adf:` patterns in a single call. Confirmed working on Gitea 1.26.0.

---

## File Changes

### Step 1: New `MentionCursor` type + persistence

**File:** `crates/terraphim_orchestrator/src/mention.rs`

```rust
/// Persistent cursor for mention polling.
/// Stored via terraphim_persistence as JSON at key "adf/mention_cursor".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MentionCursor {
    /// ISO 8601 timestamp of the last processed comment.
    pub last_seen_at: String,
    /// Counter of dispatches in current tick (reset each poll cycle).
    #[serde(skip)]
    pub dispatches_this_tick: u32,
}

impl MentionCursor {
    /// Create a cursor set to "now" (skip all historical mentions).
    pub fn now() -> Self {
        Self {
            last_seen_at: chrono::Utc::now().to_rfc3339(),
            dispatches_this_tick: 0,
        }
    }

    /// Load from persistence or create "now" cursor.
    pub async fn load_or_now() -> Self { ... }

    /// Save to persistence.
    pub async fn save(&self) { ... }
}
```

**Remove:** `MentionTracker.processed` HashSet (no longer needed).  
**Keep:** `MentionTracker.depth_counters` renamed to `dispatches_per_issue: HashMap<u64, u32>` (flood protection).

### Step 2: New `fetch_repo_comments` in tracker

**File:** `crates/terraphim_tracker/src/gitea.rs`

```rust
/// Fetch comments across all issues in a repo since a timestamp.
pub async fn fetch_repo_comments(
    &self,
    since: &str,
    limit: u32,
) -> Result<Vec<IssueComment>> {
    // GET /api/v1/repos/{owner}/{repo}/issues/comments?since={since}&limit={limit}
}
```

The `IssueComment` struct needs an `issue_number` field extracted from `issue_url`.

### Step 3: Update `MentionConfig`

**File:** `crates/terraphim_orchestrator/src/config.rs`

```rust
pub struct MentionConfig {
    /// DEPRECATED: Ignored when cursor polling is active.
    #[serde(default)]
    pub watch_issues: Vec<u64>,

    /// Max mentions to dispatch per poll tick.
    #[serde(default = "default_max_dispatches_per_tick")]
    pub max_dispatches_per_tick: u32,  // default: 3

    /// Max concurrent mention-spawned agents.
    #[serde(default = "default_max_concurrent_mention_agents")]
    pub max_concurrent_mention_agents: u32,  // default: 5

    /// Poll every N reconciliation ticks.
    #[serde(default = "default_poll_modulo")]
    pub poll_modulo: u64,

    /// Persistence key prefix for cursor.
    #[serde(default)]
    pub cursor_key: String,  // default: "adf/mention_cursor"
}
```

Remove `max_mention_depth` (replaced by `max_dispatches_per_tick`).

### Step 4: Rewrite `poll_mentions()`

**File:** `crates/terraphim_orchestrator/src/lib.rs` (lines 1055-1170)

```rust
async fn poll_mentions(&mut self) {
    let cfg = match self.config.mentions.clone() { ... };
    let gitea_cfg = match self.config.gitea.clone() { ... };

    if self.tick_count % cfg.poll_modulo != 0 { return; }

    // Count currently active mention-spawned agents
    let active_mention_agents = self.active_agents.values()
        .filter(|a| a.spawned_by_mention)
        .count() as u32;
    if active_mention_agents >= cfg.max_concurrent_mention_agents { return; }

    // Load cursor
    let mut cursor = MentionCursor::load_or_now().await;
    cursor.dispatches_this_tick = 0;

    // Single API call: all comments since cursor
    let comments = tracker.fetch_repo_comments(&cursor.last_seen_at, 50).await?;

    for comment in &comments {
        if cursor.dispatches_this_tick >= cfg.max_dispatches_per_tick { break; }

        let mentions = parse_mentions(comment, comment.issue_number, &agents, &personas);
        for m in mentions {
            if cursor.dispatches_this_tick >= cfg.max_dispatches_per_tick { break; }

            // Spawn
            self.spawn_agent(&mention_def).await?;
            cursor.dispatches_this_tick += 1;
        }

        // Advance cursor past this comment
        cursor.last_seen_at = comment.created_at.clone();
    }

    cursor.save().await;
}
```

### Step 5: Track mention-spawned agents

**File:** `crates/terraphim_orchestrator/src/lib.rs`

Add `spawned_by_mention: bool` to `ManagedAgent` struct. Set to `true` when spawned from `poll_mentions()`.

---

## Test Strategy

| Test | Type | What it verifies |
|---|---|---|
| `test_cursor_load_or_now` | Unit | Fresh cursor uses current time |
| `test_cursor_persistence_roundtrip` | Unit | Save + load preserves last_seen_at |
| `test_poll_mentions_respects_max_dispatches` | Unit | Stops after N dispatches per tick |
| `test_poll_mentions_advances_cursor` | Unit | Cursor advances to latest comment |
| `test_poll_mentions_concurrent_limit` | Unit | Skips poll when max concurrent reached |
| `test_no_replay_on_restart` | Integration | Restart with persisted cursor doesn't re-dispatch |

---

## Step Sequence

| Step | Description | Files | Depends on |
|---|---|---|---|
| 1 | MentionCursor + persistence | mention.rs | — |
| 2 | fetch_repo_comments API | terraphim_tracker/gitea.rs | — |
| 3 | Update MentionConfig | config.rs | — |
| 4 | Rewrite poll_mentions() | lib.rs | 1, 2, 3 |
| 5 | spawned_by_mention tracking | lib.rs | 4 |
| 6 | Tests | mention.rs, lib.rs tests | 1-5 |
| 7 | Deploy + verify on bigbox | orchestrator.toml | 6 |

Steps 1-3 are independent and can be done in parallel.

---

## Rollback

If cursor polling fails, re-enable `[mentions]` with `watch_issues` — the old code path still works. The cursor is additive.

---

## Config migration

```toml
# OLD (remove)
[mentions]
watch_issues = [107, 108, ...]
max_mention_depth = 10
poll_modulo = 2

# NEW
[mentions]
max_dispatches_per_tick = 3
max_concurrent_mention_agents = 5
poll_modulo = 2
```
