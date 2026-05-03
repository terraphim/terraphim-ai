# Design and Implementation Plan: Fix ADF Fleet DEGRADED (Issue #1233)

## 1. Summary of Target Behavior

After implementation, the ADF fleet will exhibit the following operational characteristics:

1. **Provider Health Accuracy:** Provider probes distinguish between "CLI tool missing" and "provider API unhealthy". Circuit breakers tolerate transient failures without marking providers permanently unhealthy. Primary providers (openai, anthropic) are used when genuinely available.

2. **Review Format Resilience:** The PR review parser accepts review comments that emit `<h3>Inline Findings</h3>` with normalised whitespace and optional HTML attributes, eliminating 100% of "missing Inline Findings" false negatives.

3. **Auto-Merge Idempotency:** The auto-merge failure handler creates at most one tracking issue per `(PR number, head SHA)` pair within a 24-hour window, eliminating duplicate noise.

---

## 2. Key Invariants and Acceptance Criteria

### Invariants
- **I1:** A provider is marked unhealthy only if its API endpoint is genuinely unreachable, NOT if its CLI tool is misconfigured or missing from PATH.
- **I2:** The review parser never rejects a comment that contains a readable "Inline Findings" section, regardless of minor HTML/markup variations.
- **I3:** No duplicate `[ADF] Auto-merge failed for PR #N` issues exist for the same PR with the same head SHA within 24 hours.

### Acceptance Criteria (Testable)

| ID | Criterion | Test Type |
|----|-----------|-----------|
| AC1 | When a provider's action template references a missing CLI tool, the probe logs "CLI tool not found" and does NOT open the circuit breaker | Unit |
| AC2 | When a provider probe fails 5 times consecutively, the circuit breaker opens; after 300s it transitions to HalfOpen | Unit |
| AC3 | When a review comment contains `<h3 class="x">Inline Findings</h3>`, the parser succeeds | Unit |
| AC4 | When a review comment contains `### Inline Findings` with extra whitespace, the parser succeeds | Unit |
| AC5 | When auto-merge fails for PR #123 with SHA `abc123`, only one issue is created; subsequent failures within 24h add a comment | Integration |
| AC6 | After 24h, a new failure for the same PR/SHA MAY create a new issue | Unit |

---

## 3. High-Level Design and Boundaries

### Architecture Overview

```
┌──────────────────────────────────────────────────────────────────────────┐
│                        AgentOrchestrator                           │
│                                                                    │
│   ┌─────────────┐   ┌─────────────┐   ┌────────────────┐  │
│   │ ProviderHealthMap │   │   pr_review       │   │  AutoMergeDedupe  │  │
│   │   (modified)      │   │   (modified)    │   │     (new)          │  │
│   └─────────────┘   └─────────────┘   └────────────────┘  │
│                                                                    │
└──────────────────────────────────────────────────────────────────────────┘
```

### Component Boundaries

**ProviderHealthMap** (`provider_probe.rs`)
- **Inside:** Probe execution logic, circuit breaker state, health queries
- **Changes:** Add CLI pre-check, tune circuit breaker config, improve error classification
- **Does NOT touch:** KG router rules, spawn dispatch logic, provider API endpoints

**pr_review** (`pr_review.rs`)
- **Inside:** String parsing, verdict extraction, evaluation logic
- **Changes:** Normalise whitespace before matching, add regex-based heading detection
- **Does NOT touch:** Gitea API, skill templates, auto-merge criteria thresholds

**AutoMergeDedupe** (new module or inline in `pr_poller.rs`)
- **Inside:** In-memory cache of recent failure issues
- **Changes:** New cache structure, check-before-create logic
- **Does NOT touch:** Merge API call itself, review parsing, provider routing

---

## 4. File/Module-Level Change Plan

| File | Action | Before | After | Dependencies |
|------|--------|--------|-------|--------------|
| `provider_probe.rs` | Modify | `probe_single` executes action template directly via bash; no CLI validation | Add `validate_cli_tool` function that checks `which <cli>` before probe; skip probe with distinct error if CLI missing | `std::process::Command` for `which` check |
| `provider_probe.rs` | Modify | `CircuitBreakerConfig { failure_threshold: 2, cooldown: 60s }` | `CircuitBreakerConfig { failure_threshold: 5, cooldown: 300s }` | None (const change) |
| `provider_probe.rs` | Modify | `ProbeResult.error` contains generic strings | `ProbeResult.error` uses structured error categories: `CliNotFound`, `TemplateMissing`, `ProviderError`, `Timeout` | Enum addition |
| `pr_review.rs` | Modify | `parse_verdict` uses `body.contains("<h3>Inline Findings</h3>")` | `parse_verdict` normalises body (strip attrs, collapse whitespace) before matching; also accepts regex `<h3[^\u003e]*>\s*Inline Findings\s*</h3>` | `regex` crate (lightweight) or manual normalisation |
| `pr_review.rs` | Modify | `parse_confidence` uses literal string find | Same -- no change needed (already tolerant) | None |
| `pr_poller.rs` | Create (new type) | No deduplication | Add `AutoMergeDedupeCache` struct with `HashMap<(u64, String), Instant>` | `std::collections::HashMap`, `std::time::{Duration, Instant}` |
| `pr_poller.rs` | Modify | `open_failure_issue` always creates issue | `open_failure_issue` checks cache first; if entry exists and is fresh, adds comment instead | `AutoMergeDedupeCache` |
| `lib.rs` | Modify | `AgentOrchestrator` holds `provider_health: ProviderHealthMap` | Same -- no structural change; initialises dedupe cache if needed | `pr_poller::AutoMergeDedupeCache` |

---

## 5. Step-by-Step Implementation Sequence

### Step 1: Tune Circuit Breaker Config (Deployable)
**Purpose:** Reduce false-positive provider unhealthiness.
**Changes:**
- In `provider_probe.rs`, change `failure_threshold` from 2 to 5.
- Change `cooldown` from `Duration::from_secs(60)` to `Duration::from_secs(300)`.
**Deployable:** Yes -- pure const change, no API breakage.
**Risk:** Low.

### Step 2: Add CLI Pre-Check to Probe (Deployable)
**Purpose:** Distinguish "tool missing" from "provider down".
**Changes:**
- Add `fn cli_tool_on_path(tool: &str) -> bool` that runs `which {tool}`.
- In `probe_single`, extract CLI tool name from action template.
- If CLI not on PATH, return `ProbeResult` with status `Error` and error category `CliNotFound`.
- In `ProviderHealthMap::probe_all`, skip updating circuit breaker for `CliNotFound` errors.
**Deployable:** Yes -- additive logic, backwards compatible.
**Risk:** Low. May reveal previously-hidden tool misconfigurations.

### Step 3: Improve Review Parser Tolerance (Deployable)
**Purpose:** Eliminate false negatives from minor HTML variations.
**Changes:**
- Add `fn normalise_html_headings(body: &str) -> String` that:
  - Removes attributes from `<h3>` tags (regex: `<h3[^\u003e]*>` -> `<h3>`)
  - Collapses whitespace inside tags
- In `parse_verdict`, normalise body before checking for `Inline Findings`.
**Deployable:** Yes -- pure parsing change.
**Risk:** Low. Could theoretically accept malformed reviews, but the rest of the parser (confidence, footer) still enforces structure.

### Step 4: Add Auto-Merge Failure Deduplication (Deployable)
**Purpose:** Prevent duplicate issue creation.
**Changes:**
- Add `AutoMergeDedupeCache` struct to `pr_poller.rs`.
- Modify `AutoMergeExecutor::open_failure_issue` trait method signature to accept cache reference (or add cache to executor impl).
- In `GiteaPrTracker` (the concrete impl), check cache before creating issue.
- On cache hit, call Gitea API to add comment to existing issue instead.
- On cache miss, create issue and insert into cache with TTL.
- Add background cleanup of expired cache entries (on each check, remove entries older than 24h).
**Deployable:** Yes -- additive, no breaking changes.
**Risk:** Medium. Requires Gitea API call to add comment; if API is down, behaviour degrades gracefully (silently skips duplicate creation).

### Step 5: Integration Test for Full Flow (Not Deployable Alone)
**Purpose:** Verify all three fixes work together.
**Changes:**
- Add integration test in `orchestrator_tests.rs` that:
  - Mocks a provider with missing CLI tool -> asserts circuit breaker stays closed.
  - Provides review comment with `<h3 class="x">Inline Findings</h3>` -> asserts parse success.
  - Simulates two auto-merge failures for same PR -> asserts only one issue created.
**Deployable:** N/A -- test code.
**Risk:** None.

---

## 6. Testing and Verification Strategy

| Acceptance Criteria | Test Type | Test Location | How Verified |
|---------------------|-----------|---------------|--------------|
| AC1: CLI missing does not open breaker | Unit | `provider_probe.rs` tests | Mock `probe_single` with missing CLI; assert breaker state remains Closed |
| AC2: 5 failures open breaker, 300s cooldown | Unit | `provider_probe.rs` tests | Record 5 failures; assert Open. Wait 300s (or mock time); assert HalfOpen |
| AC3: HTML attributes in h3 tag accepted | Unit | `pr_review.rs` tests | Call `parse_verdict` with `<h3 class="x">Inline Findings</h3>`; assert Ok |
| AC4: Markdown heading with whitespace accepted | Unit | `pr_review.rs` tests | Call `parse_verdict` with `###  Inline Findings  `; assert Ok |
| AC5: Duplicate auto-merge issues suppressed | Integration | `orchestrator_tests.rs` | Mock `AutoMergeExecutor` that records calls; trigger 2 failures; assert 1 `open_failure_issue` call |
| AC6: Cache expiry after 24h | Unit | `pr_poller.rs` tests | Insert entry with timestamp T; at T+23h assert blocked; at T+25h assert allowed |

### Regression Tests
- Run existing `provider_probe` tests to ensure circuit breaker behaviour change doesn't break existing assertions.
- Run existing `pr_review` tests with the new parser to ensure all existing valid reviews still parse.
- Run existing `pr_poller` tests to ensure auto-merge flow still works when dedupe cache is empty.

---

## 7. Risk and Complexity Review

| Risk | Mitigation | Residual Risk |
|------|------------|---------------|
| Tuning circuit breaker too loosely masks genuine provider outages | Keep `success_threshold=1` so recovery is fast; monitor probe results JSON for trends | Low -- provider genuinely down for >5 consecutive probes is rare |
| CLI pre-check adds spawn overhead (extra `which` process per probe) | Use `which` only once per unique CLI tool per probe cycle; cache results in `probe_all` | Low -- `which` is sub-millisecond |
| Review parser normalisation is too permissive and accepts incomplete reviews | The confidence score and footer checks still enforce structure; add test corpus of known-good reviews | Low -- blast radius limited to auto-merge gating |
| Auto-merge dedupe cache is in-memory only; orchestrator restart loses state | Duplicate issues after restart are acceptable (infrequent); persistent cache is overkill | Medium -- acceptable per requirements |
| Gitea API comment-add fails silently | Log warning on comment-add failure; fallback to creating duplicate issue is acceptable | Low -- better than current behaviour |

### Complexity Assessment
- **Cyclomatic complexity:** Low. All changes are additive or const-tuning.
- **Blast radius:** Confined to `provider_probe.rs`, `pr_review.rs`, `pr_poller.rs`.
- **Rollback:** Each step is independently rollbackable (const change, pure function, additive struct).

---

## 8. Open Questions / Decisions for Human Review

1. **Circuit breaker thresholds:** Is `failure_threshold=5` and `cooldown=300s` acceptable, or do you prefer different values? Current is 2/60s.

2. **CLI pre-check scope:** Should we check `which` for the CLI tool, or also validate the tool's `--version` flag? The latter catches broken installations but adds latency.

3. **Review parser scope:** Should we also normalise `<h2>` and `<h4>` headings, or strictly `<h3>`? The skill template specifies `<h3>`.

4. **Dedupe cache TTL:** Is 24 hours appropriate? Would 12h or 48h be better for your triage workflow?

5. **Deploy order:** Should we deploy Step 1 (circuit breaker tuning) immediately as a hotfix while implementing Steps 2-4?

---

## Appendix: Estimated Effort

| Step | Estimated Time | Files Changed |
|------|---------------|---------------|
| Step 1: Tune circuit breaker | 15 min | `provider_probe.rs` |
| Step 2: CLI pre-check | 1 hour | `provider_probe.rs` |
| Step 3: Review parser tolerance | 45 min | `pr_review.rs` |
| Step 4: Auto-merge dedupe | 1.5 hours | `pr_poller.rs` |
| Step 5: Integration tests | 1 hour | `orchestrator_tests.rs` |
| **Total** | **~4.5 hours** | 3-4 files |
