# Design & Implementation Plan: Fix ADF CI Pipeline Degraded (2026-05-04)

**Updated:** 2026-05-04 after full log analysis (journalctl + Quickwit), orchestrator stopped and disabled.

## 1. Summary of Target Behaviour

After implementation, the ADF CI pipeline will:
- Successfully decode Gitea commit status API responses, unblocking PR gate reconciliation
- Parse push webhook payloads where commit file lists are `null` instead of `[]`
- Correctly parse review comments that use `<h3>Findings</h3>` instead of `<h3>Inline Findings</h3>`
- Skip non-reviewer comments (security-checklist, security-audit, etc.) before attempting verdict parsing
- Post `adf/pr-reviewer` commit status on PRs where the pr-reviewer agent has commented
- Resume Quickwit log ingestion (currently stalled since 2026-04-21)
- Allow PRs to progress through the status-check gate and auto-merge

## 2. Key Invariants and Acceptance Criteria

| ID | Invariant | Acceptance Criteria |
|----|-----------|---------------------|
| AC-1 | `list_commit_statuses` decodes without error | 0 `error decoding response body` in logs per tick cycle |
| AC-2 | PR gate reconciliation runs cleanly | All 24 open PRs classified without API errors |
| AC-3 | Review parser accepts both `Inline Findings` and `Findings` headings | 0 `missing Inline Findings` parse failures for valid review comments |
| AC-4 | Non-reviewer comments are skipped before parsing | 0 parse attempts on security-checklist/audit/traceability comments |
| AC-5 | Push webhook accepts `null` commit file arrays | 0 `failed to parse push webhook payload` for valid push events |
| AC-6 | Quickwit sink receives logs | `adf-logs` index shows entries within 60s of orchestrator start |
| AC-7 | `adf/pr-reviewer` status is posted when review is complete | Status appears in commit statuses within 1 tick after review |
| AC-8 | Existing tests pass | `cargo test -p terraphim_orchestrator` and `cargo test -p terraphim_tracker` pass |

## 3. High-Level Design and Boundaries

### Architecture

Six independent fixes across five modules:

```
Fix A: CommitStatusEntry serde      Fix B: Push webhook null arrays    Fix C: Review parser tolerance
(terraphim_tracker/gitea.rs)        (terraphim_orchestrator/webhook.rs) (terraphim_orchestrator/pr_review.rs)
         |                                    |                                    |
         v                                    v                                    v
   list_commit_statuses()              handle_push_event()                parse_verdict()

Fix D: Comment pre-filter            Fix E: Quickwit sink re-enable     Fix F: Merge-coordinator CLI path
(terraphim_orchestrator/lib.rs)       (conf.d/terraphim.toml)            (conf.d/terraphim.toml)
         |                                    |                                    |
         v                                    v                                    v
   poll_pending_reviews()             QuickwitFleetSink                  agent spawn paths
```

### Severity Classification (from log analysis)

| Error Pattern | Count/24h | Severity | Fix |
|---------------|-----------|----------|-----|
| `error decoding response body` (status API) | 315 | Critical | A |
| `missing Inline Findings` (review parser) | 540 | Critical | C + D |
| `Circuit breaker re-opened` (anthropic) | 152 | Medium | Separate (#1233) |
| `probe failed provider=anthropic` | 68 | Medium | Separate (#1233) |
| `failed to get branch protection` (other repos) | 65 | Low | Expected -- repos have no protection |
| `probe skipped` / `provider failure` | 358 | Low | Normal operations |
| `cron spawn failed agent=odilo-developer` | 11 | Low | F |
| `cron spawn failed agent=merge-coordinator` | 6 | Low | F |
| `failed to parse push webhook payload` | 3 | Medium | B |
| `failed to load config` (TOML parse) | 9 | Low | Transient -- file was being written |
| `output events lagged` | 26 | Low | Normal -- console subscriber |
| `webhook secret ... no signature header` | 3 | Low | Non-authenticated webhook test |
| `safety agent exceeded max restarts` | 2 | Low | Agent task error -- self-healing |

## 4. File/Module-Level Change Plan

### Fix A: Gitea API Field Mismatch (Root Cause 1 -- 315 decode errors/24h)

| File | Action | Before | After | Dependencies |
|------|--------|--------|-------|--------------|
| `crates/terraphim_tracker/src/gitea.rs:1402` | Modify | `pub state: String` | `#[serde(rename = "status")] pub state: String` | None |

**Rationale:** The Gitea `/commits/{sha}/statuses` endpoint returns `{"status": "failure", ...}` but the Rust struct expects `state`. Verified by fetching raw API response:

```json
{"id": 33, "status": "failure", "target_url": "/...", "context": "ci-native.yml / lint-and-format"}
```

The struct field `state` never matches any key in the JSON, so deserialisation produces an empty string (or fails if the struct has no `default`). The actual error is `error decoding response body` from `response.json().await`.

### Fix B: Push Webhook Null File Arrays (3 parse failures/24h)

| File | Action | Before | After | Dependencies |
|------|--------|--------|-------|--------------|
| `crates/terraphim_orchestrator/src/webhook.rs:181-187` | Modify | `added/removed/modified: Vec<String>` with `#[serde(default)]` | Add `deserialize_with = "deserialize_null_default_vec"` to each field | `deserialize_null_default_vec` already exists in same file |

**Rationale:** Error: `invalid type: null, expected a sequence at line 29 column 19`. Some push events (likely from Gitea tag pushes or force pushes) have `null` instead of `[]` for the `added`/`removed`/`modified` arrays in commit objects. `#[serde(default)]` only handles missing fields, not explicit `null`. The helper `deserialize_null_default_vec` already exists (line 144) and is used for the top-level `commits` field but not for the inner commit file arrays.

### Fix C: Review Parser Tolerance (Root Cause 2 -- 540 parse failures/24h)

| File | Action | Before | After | Dependencies |
|------|--------|--------|-------|--------------|
| `crates/terraphim_orchestrator/src/pr_review.rs:126` | Modify | Requires `<h3>Inline Findings</h3>` or `### Inline Findings` | Also accept `<h3>Findings</h3>` and `### Findings` | None |

**Rationale:** The review comments on PRs 1156 and 1195 contain `<h3>Findings</h3>` (without "Inline"). Verified by inspecting actual comments from Gitea API:

```html
<h3>security_checklist Summary</h3>
...
<h3>Findings</h3>
**No security_checklist-relevant findings.**
```

The agents' output uses `Findings`, not `Inline Findings`. The parser should accept both.

### Fix D: Comment Pre-Filter (reduces parse noise)

| File | Action | Before | After | Dependencies |
|------|--------|--------|-------|--------------|
| `crates/terraphim_orchestrator/src/lib.rs` (poll_pending_reviews) | Modify | Parses every PR comment as potential verdict | Skip comments from known non-reviewer agents before calling `parse_verdict` | None |

**Rationale:** The `poll_pending_reviews` function iterates all PR comments and tries `parse_verdict` on each one. PR 1156 has 6 comments, none from pr-reviewer -- all from security-checklist, security-audit, and requirements-traceability agents. These all contain `<h3>` headings and `Verdict` sections but not in the structural-pr-review format.

Skip patterns (in the comment body opening):
- `<h3>security_checklist Summary</h3>`
- `<h3>Security Audit Summary</h3>`
- `<h3>Requirements Traceability Summary</h3>`

Implementation: check if body starts with `<h3>` followed by a known non-reviewer heading keyword before calling `parse_verdict`.

### Fix E: Quickwit Sink Re-enable (logs stalled since 2026-04-21)

| File | Action | Before | After | Dependencies |
|------|--------|--------|-------|--------------|
| `/opt/ai-dark-factory/conf.d/terraphim.toml` (on bigbox) | Modify (deploy only) | Quickwit sink may be misconfigured or disabled | Verify Quickwit `[projects.quickwit]` section is correct; restart orchestrator | Quickwit service must be running |

**Rationale:** The `adf-logs` Quickwit index has 17,859 entries but the latest is from `2026-04-21T10:00:12Z` -- 13 days stale. The orchestrator's `[projects.quickwit]` config looks correct (`endpoint = "http://127.0.0.1:7280"`, `index_id = "adf-logs"`), and Quickwit is responding. The sink may have silently failed after a transient error. Restarting the orchestrator with the fixed binary should re-establish the connection.

No code change needed -- this is a deployment/runtime fix.

### Fix F: Missing CLI Tools for merge-coordinator and odilo-developer (17 spawn failures/24h)

| File | Action | Before | After | Dependencies |
|------|--------|--------|-------|--------------|
| `/opt/ai-dark-factory/conf.d/terraphim.toml` (on bigbox) | Modify (deploy only) | `cli_tool` points to non-existent path | Either install the tool or remove/disable the agent | None |

**Rationale:** `merge-coordinator` and `odilo-developer` agents fail to spawn with `No such file or directory`. These are non-critical agents. Either install the referenced CLI tools or disable the agents by commenting them out. No code change needed.

## 5. Step-by-Step Implementation Sequence

### Step 1: Fix `CommitStatusEntry` serde rename (Deployable)
**Purpose:** Unblocks PR gate reconciliation (315 errors/24h).
**Changes:**
- In `gitea.rs:1403`, add `#[serde(rename = "status")]` before `pub state: String`
- Add a unit test that deserialises a sample Gitea response with `"status"` field
**Deployable:** Yes.
**Risk:** Low. `CommitStatusEntry` is only created by `list_commit_statuses`. No other code path constructs this struct.

### Step 2: Fix push webhook null array deserialisation (Deployable)
**Purpose:** Prevents 3 push webhook parse failures (potentially more with future push types).
**Changes:**
- In `webhook.rs:183-187`, replace `#[serde(default)]` with `#[serde(default, deserialize_with = "deserialize_null_default_vec")]` on `added`, `removed`, and `modified` fields
- Add a test with a payload where commit file arrays are `null`
**Deployable:** Yes.
**Risk:** Low. The helper function already exists and is tested.

### Step 3: Widen review parser heading acceptance (Deployable)
**Purpose:** Accept review comments with `<h3>Findings</h3>` (540 errors/24h).
**Changes:**
- In `pr_review.rs:126`, change the condition to also accept `<h3>Findings</h3>` and `### Findings`
- Add unit tests for both variants
**Deployable:** Yes.
**Risk:** Low. Confidence score and footer checks still enforce structure.

### Step 4: Add comment pre-filter (Deployable)
**Purpose:** Skip non-reviewer comments before parsing, reducing noise.
**Changes:**
- In `poll_pending_reviews` loop, add a guard that skips comments whose body starts with known non-reviewer heading patterns
- Log skipped comments at DEBUG level
**Deployable:** Yes.
**Risk:** Low. Use a conservative denylist; log skips for audit.

### Step 5: Fix merge-coordinator / odilo-developer agent config (Deployable, deploy-only)
**Purpose:** Eliminate 17 cron spawn failures per 24h.
**Changes:**
- On bigbox, either install the referenced CLI tools or disable the agents in `conf.d/terraphim.toml`
**Deployable:** Yes.
**Risk:** Low. These agents are non-critical.

### Step 6: Build, test, and deploy
**Purpose:** Verify all fixes work end-to-end.
**Commands:**
```bash
cargo test -p terraphim_tracker
cargo test -p terraphim_orchestrator
cargo clippy --workspace --all-targets -- -D warnings
```
**Deploy:**
```bash
cargo build --release -p terraphim_orchestrator
scp target/release/adf bigbox:/usr/local/bin/adf
ssh bigbox 'sudo systemctl enable adf-orchestrator && sudo systemctl start adf-orchestrator'
```
**Post-deploy verification:**
```bash
ssh bigbox 'journalctl -u adf-orchestrator -f'  # Watch for 5 minutes
# Verify: 0 "error decoding response body"
# Verify: 0 "missing Inline Findings" for reviewer comments
# Verify: Quickwit adf-logs receives new entries
```

## 6. Testing & Verification Strategy

| AC | Test Type | Test Location | How Verified |
|----|-----------|---------------|--------------|
| AC-1 | Unit | `terraphim_tracker/src/gitea.rs` tests | Deserialise sample Gitea JSON with `"status"` field; assert `state` is populated |
| AC-2 | Integration | `terraphim_orchestrator/tests/` | Run `reconcile_pr_gates` against mock Gitea; assert 0 decode errors |
| AC-3 | Unit | `terraphim_orchestrator/src/pr_review.rs` tests | Call `parse_verdict` with body containing `<h3>Findings</h3>`; assert Ok |
| AC-3 | Unit | `terraphim_orchestrator/src/pr_review.rs` tests | Call `parse_verdict` with body containing `### Findings`; assert Ok |
| AC-4 | Unit | `terraphim_orchestrator/src/pr_review.rs` tests | Call `parse_verdict` with security-checklist body; assert error is sensible |
| AC-5 | Unit | `terraphim_orchestrator/src/webhook.rs` tests | Parse push payload with `null` file arrays; assert Ok |
| AC-6 | Observation | Quickwit `adf-logs` index | After deploy, query index for recent entries |
| AC-7 | Observation | Gitea commit statuses | After deploy, check PR for `adf/pr-reviewer` status |
| AC-8 | CI gate | `cargo test -p terraphim_orchestrator -p terraphim_tracker` | All existing tests pass |

### Regression Tests
- Run existing `pr_review` tests to ensure heading change doesn't break existing valid reviews
- Run existing `gitea.rs` tracker tests to ensure struct change doesn't break other API calls
- Run existing `pr_gate` tests to ensure classification still works with new field name
- Run existing `webhook` tests to ensure push parsing still works with `[]` arrays

## 7. Risk & Complexity Review

| Risk | Mitigation | Residual Risk |
|------|------------|---------------|
| `#[serde(rename)]` breaks another API call that returns `state` | Audit all uses of `CommitStatusEntry`; only `list_commit_statuses` creates it | Low |
| Parser accepts incomplete reviews | Confidence score and footer checks still enforce structure | Low |
| Comment filter is too aggressive | Use conservative denylist; log skipped comments at DEBUG | Low |
| Push webhook fix changes serialisation semantics | Helper already exists and is used for top-level `commits` field | Low |
| Quickwit sink does not reconnect on restart | Restart flushes state; sink re-initialises from config | Low |
| Config TOML parse error recurs | Was transient (file being written); service has restart limits | Low |

### Complexity Assessment
- **Cyclomatic complexity:** Very low. All changes are attribute additions, condition widening, or guard additions.
- **Blast radius:** 4 code files, ~15 lines of actual change + 2 config changes on bigbox.
- **Rollback:** Each step is independently revertable. Re-enable orchestrator with old binary if needed.

## 8. Open Questions / Decisions for Human Review

1. **`CommitStatusEntry.state` rename:** Use `#[serde(rename = "status")]` (keeps Rust name `state`), or rename to `status`? Recommendation: keep `state` with serde rename.

2. **`Findings` vs `Inline Findings`:** Treat as equivalent, or prefer one? Recommendation: equivalent.

3. **Comment filtering approach:** Denylist of non-reviewer headings, or allowlist of reviewer headings? Recommendation: denylist initially.

4. **merge-coordinator / odilo-developer:** Install CLI tools or disable agents? Recommendation: disable agents for now; re-enable when tools are available.

5. **Deploy timing:** Orchestrator is already stopped and disabled. Deploy as soon as tests pass.

6. **Anthropic provider probe failure (152 breaker events/24h):** Address in this fix or defer to #1233? Recommendation: defer. The circuit breaker tuning in design-1233 is the correct fix.

## Appendix A: Full Log Analysis Summary

### Source: journalctl (24h window, 9367 lines)

| Category | Count | Pattern |
|----------|-------|---------|
| API decode errors | 315 | `error decoding response body` (24 PRs x 13 ticks) |
| Review parse failures | 540 | `missing Inline Findings` (2 PRs x 270 ticks at 5min) |
| Circuit breaker re-open | 152 | `Circuit breaker re-opened after failed probe` |
| Provider probe failures | 69 | `probe failed provider=anthropic` (68) + openai (1) |
| Branch protection 404 | 65 | `failed to get branch protection` (5 other repos, no protection) |
| Probe skipped/provider failure | 358 | Normal probe operations |
| Config load failure | 9 | `TOML parse error at line 41` (transient, file write race) |
| Cron spawn failures | 17 | `merge-coordinator` (6) + `odilo-developer` (11) `No such file or directory` |
| Push webhook parse failure | 3 | `null, expected a sequence at line 29` |
| Agent max restarts exceeded | 2 | `pr-spec-validator-retry-1` + `pr-security-sentinel-retry-1` |
| Webhook secret missing | 3 | Non-authenticated webhook test |
| Reconcile tick timeout | 2 | Exceeded timeout |

### Source: Quickwit

| Index | Entries | Latest Entry | Status |
|-------|---------|--------------|--------|
| `adf-logs` | 17,859 | 2026-04-21T10:00:12Z | **Stale -- 13 days behind** |
| `adf-digital-twins-logs` | 155 | Unknown | Low activity |
| `adf-odilo-logs` | 58 | Unknown | Low activity |
| `otel-logs-v0_7` | 0 | N/A | Empty |
| `otel-traces-v0_7` | Unknown | N/A | Not checked |
| `workers-logs` | 0 | N/A | Empty |

### Key Observations
1. The orchestrator has been running since `2026-05-03 23:26:33` (PID 1737906) with 43.9G memory
2. Only 2 PRs (1195 and 1156) generate review parse failures -- these are the only PRs with review comments
3. The parse failures repeat every 5 minutes (tick interval = 300s per config)
4. All 24 open PRs generate API decode errors every reconciliation cycle (every 20 ticks = 100 minutes)
5. The Anthropic probe fails consistently (68/69 probe failures) -- `claude` CLI exits with status 1
6. Config TOML parse errors were transient (9 failures over 4 minutes, then succeeded)
7. The orchestrator was restarted 3 times in 24h (PIDs 1367890 -> 3013090 -> 3238683 -> 1737906)

## Appendix B: Root Cause Analysis

### Root Cause 1: `CommitStatusEntry` field name mismatch
- **API:** `GET /commits/{sha}/statuses` returns `{"status": "failure"}`
- **Rust struct:** expects `{"state": "failure"}` (field name `state`)
- **Result:** serde_json fails -- `state` field is missing, `status` is unknown
- **Impact:** 315 errors/24h, blocks ALL PR gate reconciliation

### Root Cause 2: Review parser heading mismatch
- **Agent output:** `<h3>Findings</h3>` (without "Inline")
- **Parser expects:** `<h3>Inline Findings</h3>` or `### Inline Findings`
- **Result:** 540 errors/24h on 2 PRs, preventing auto-merge

### Root Cause 3: Push webhook null arrays
- **Some push events:** `{"added": null, "removed": null, "modified": null}`
- **Rust struct:** `Vec<String>` with `#[serde(default)]` -- only handles missing, not null
- **Result:** 3 parse failures/24h, some push events silently dropped

### Root Cause 4: Quickwit sink stalled
- **adf-logs last entry:** 2026-04-21 -- 13 days behind
- **Likely cause:** Transient Quickwit connection failure caused sink to stop; no reconnect logic
- **Impact:** No structured log analytics for 13 days

### Root Cause 5: Missing CLI tools
- **merge-coordinator:** References CLI tool not on PATH
- **odilo-developer:** References CLI tool not on PATH
- **Impact:** 17 spawn failures/24h, non-critical agents
