# Implementation Plan: ADF Agent Flow Improvements

**Status**: Draft
**Research Doc**: `.docs/research-adf-agent-improvements.md`
**Author**: Claude (design)
**Date**: 2026-05-23
**Estimated Effort**: 3-4 days

## Overview

### Summary

This plan addresses critical security, reliability, and observability issues in the ADF agent flows identified during the 2026-05-22/23 nightly run.

### Approach

Three-track parallel implementation:
1. **Track A**: Fix credential leakage (Debug redaction)
2. **Track B**: Rust rewrite of merge-coordinator
3. **Track C**: Structured logging + exit code semantics

### Scope

**In Scope:**
- Custom `fmt::Debug` implementations for credential-containing structs
- Rust rewrite of merge-coordinator per spec
- Structured JSON logging for merge-coordinator
- Proper exit code semantics (0/1/2)
- PID file locking for concurrency

**Out of Scope:**
- Meta-coordinator bash-to-Rust rewrite
- Runtime-guardian implementation
- Drift-detector implementation
- Full test suite overhaul

**Avoid At All Cost:**
- Modifying working agent skill chains
- Changing provider routing logic
- Adding new agents without approval

## Architecture

### Component Diagram

```
ADF Orchestrator (bigbox)
├── merge-coordinator (Rust rewrite)
│   ├── pid_lock.rs - File-based PID locking
│   ├── gitea_client.rs - API with retry logic
│   ├── verdict_engine.rs - PASS/FAIL determination
│   └── structured_logger.rs - JSON logging
├── Credential configs (Debug redaction)
│   ├── tinyclaw/config.rs
│   ├── tracker/gitea.rs
│   └── github_runner_server/config/mod.rs
└── skill_chain (unchanged)
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Rust for merge-coordinator | Spec requires atomicity Python can't provide | Backport to Python (spec predates Python) |
| Custom Debug trait | Redact sensitive fields without changing API | Use `#[serde(skip)]` (doesn't affect Debug) |
| JSON structured logs | Observability requirement from spec | Using tracing crate (adds dependency) |
| PID file in /tmp | Simple, portable, atomic via flock | Inotify filesystem watches (too complex) |

### Eliminated Options

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| Python with threading | GIL, no true parallelism | Unnecessary complexity |
| etcd/consul for locking | Over-engineering for single-machine | Operational overhead |
| env_logger for JSON | Not structured JSON per spec | Doesn't meet OBS-1 |

### Simplicity Check

**What if this could be easy?**

A single Rust binary with:
- `merge-coordinator` CLI command
- Structured JSON logs to stdout (captured by orchestrator)
- PID lock via standard library `fs::File` + `flock`
- Gitea API calls via `reqwest` with retry

**Senior Engineer Test**: Would a senior engineer call this overcomplicated? **No** - this is the minimum viable implementation meeting the spec.

## File Changes

### New Files

| File | Purpose |
|------|---------|
| `crates/terraphim_merge_coordinator/src/main.rs` | CLI entry point |
| `crates/terraphim_merge_coordinator/src/pid_lock.rs` | PID file locking |
| `crates/terraphim_merge_coordinator/src/gitea_client.rs` | Gitea API with retry |
| `crates/terraphim_merge_coordinator/src/verdict_engine.rs` | Verdict determination |
| `crates/terraphim_merge_coordinator/src/structured_logger.rs` | JSON logging |
| `crates/terraphim_merge_coordinator/src/error.rs` | Error types |
| `crates/terraphim_merge_coordinator/Cargo.toml` | Crate manifest |
| `crates/terraphim_merge_coordinator/README.md` | Usage documentation |

### Modified Files

| File | Changes |
|------|---------|
| `crates/terraphim_tinyclaw/src/config.rs` | Add custom Debug impl for credential structs |
| `crates/terraphim_tracker/src/gitea.rs` | Add custom Debug impl for GiteaConfig |
| `crates/terraphim_github_runner_server/src/config/mod.rs` | Add custom Debug impl for Settings |
| `scripts/merge-coordinator.py` | Deprecate (keep for backward compat initially) |
| `scripts/merge-coordinator-gate.sh` | Deprecate (keep for backward compat initially) |
| `Cargo.toml` (workspace) | Add `terraphim_merge_coordinator` crate |
| `orchestrator.toml` | Update cli_tool path for merge-coordinator |

### Deleted Files

| File | Reason |
|------|--------|
| None | Deprecating only, keeping for rollback |

## API Design

### Public Types

```rust
/// Merge coordinator configuration
#[derive(Debug, Clone)]
pub struct Config {
    /// Gitea base URL
    pub gitea_url: String,
    /// Repository owner
    pub owner: String,
    /// Repository name
    pub repo: String,
    /// Lock file path
    pub lock_path: PathBuf,
    /// Lock timeout
    pub lock_timeout_secs: u64,
    /// API retry count
    pub retry_count: u32,
    /// Retry base delay
    pub retry_base_delay_ms: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            gitea_url: std::env::var("GITEA_URL")
                .unwrap_or_else(|_| "https://git.terraphim.cloud".into()),
            owner: "terraphim".into(),
            repo: "terraphim-ai".into(),
            lock_path: PathBuf::from("/tmp/merge-coordinator.lock"),
            lock_timeout_secs: 30,
            retry_count: 3,
            retry_base_delay_ms: 1000,
        }
    }
}

/// Verdict for a PR review
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Verdict {
    pub pr_number: u64,
    pub result: VerdictResult,
    pub reason: String,
    pub comment_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum VerdictResult {
    Pass,
    Fail,
    Missing,
}

/// Structured log entry (one JSON object per line)
#[derive(Debug, Serialize)]
struct LogEntry {
    timestamp: String,
    level: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pr_number: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}
```

### Public Functions

```rust
/// Run the merge coordinator
///
/// # Arguments
/// * `config` - Configuration
/// * `dry_run` - If true, log mutations without executing
///
/// # Returns
/// Exit code: 0 = success, 1 = failures present, 2 = critical error
pub async fn run(config: &Config, dry_run: bool) -> Result<ExitCode>;

/// Acquire PID lock with timeout
///
/// # Arguments
/// * `lock_path` - Path to lock file
/// * `timeout` - Maximum wait time
///
/// # Returns
/// Lock handle or error
pub fn acquire_lock(lock_path: &Path, timeout: Duration) -> Result<PidLock, LockError>;

/// Evaluate PR reviews and determine verdict
///
/// # Arguments
/// * `client` - Gitea client
/// * `pr_number` - PR number
///
/// # Returns
/// Verdict or error
pub async fn evaluate_pr(client: &GiteaClient, pr_number: u64) -> Result<Verdict, GiteaError>;
```

### Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum MergeCoordinatorError {
    #[error("lock timeout after {0:?}")]
    LockTimeout(Duration),

    #[error("lock error: {0}")]
    LockFailed(String),

    #[error("API error: {0}")]
    Api(#[from] GiteaError),

    #[error("partial failure: merge succeeded but {0}")]
    PartialFailure(String),

    #[error("critical: {0}")]
    Critical(String),
}

#[derive(Debug, thiserror::Error)]
pub enum GiteaError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("rate limited, retry after {0}s")]
    RateLimited(u64),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("token missing")]
    TokenMissing,
}
```

## Test Strategy

### Unit Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_pid_lock_acquire_release` | pid_lock.rs | Lock lifecycle |
| `test_pid_lock_timeout` | pid_lock.rs | Timeout behavior |
| `test_verdict_engine_pass` | verdict_engine.rs | All reviewers PASS |
| `test_verdict_engine_fail` | verdict_engine.rs | Any reviewer FAIL |
| `test_verdict_engine_missing` | verdict_engine.rs | All reviewers MISSING |
| `test_exponential_backoff` | gitea_client.rs | Retry delays |
| `test_config_defaults` | main.rs | Default values |

### Integration Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_full_evaluation_dry_run` | main.rs | Dry run mode |
| `test_full_evaluation_live` | main.rs | Real API calls |
| `test_concurrent_execution` | main.rs | Two instances, one succeeds |

### Property Tests

```rust
proptest! {
    #[test]
    fn verdict_result_serialization(variant in "pass|fail|missing") {
        let result: VerdictResult = serde_json::from_str(&format!("\"{}\"", variant)).unwrap();
        prop_assert!(matches!(result, VerdictResult::Pass | VerdictResult::Fail | VerdictResult::Missing));
    }
}
```

## Implementation Steps

### Step 1: Create crate and error types

**Files:** `crates/terraphim_merge_coordinator/Cargo.toml`, `crates/terraphim_merge_coordinator/src/error.rs`
**Description:** Set up crate structure and error types
**Tests:** Error type construction tests
**Estimated:** 2 hours

```toml
[package]
name = "terraphim_merge_coordinator"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
tracing = "0.1"
```

### Step 2: Implement PID lock

**Files:** `crates/terraphim_merge_coordinator/src/pid_lock.rs`
**Description:** File-based PID locking with flock
**Tests:** Unit tests for lock/unlock/timeout
**Dependencies:** Step 1
**Estimated:** 3 hours

```rust
pub struct PidLock {
    file: File,
    path: PathBuf,
}

impl Drop for PidLock {
    fn drop(&mut self) {
        // Release lock on drop
        drop(&self.file);
        let _ = std::fs::remove_file(&self.path);
    }
}
```

### Step 3: Implement Gitea client with retry

**Files:** `crates/terraphim_merge_coordinator/src/gitea_client.rs`
**Description:** API client with exponential backoff retry
**Tests:** Mock tests for retry logic
**Dependencies:** Step 1
**Estimated:** 4 hours

```rust
pub struct GiteaClient {
    client: reqwest::Client,
    base_url: Url,
    token: String,
}

impl GiteaClient {
    pub async fn get_reviews(&self, pr_number: u64) -> Result<Vec<Review>, GiteaError> {
        let mut delay_ms = self.base_delay_ms;
        for attempt in 0..self.retry_count {
            match self.fetch_reviews(pr_number).await {
                Ok(reviews) => return Ok(reviews),
                Err(GiteaError::RateLimited(after)) => {
                    tokio::time::sleep(Duration::from_secs(after)).await;
                    continue;
                }
                Err(e) if attempt < self.retry_count - 1 => {
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                    delay_ms *= 2; // exponential backoff
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
        unreachable!()
    }
}
```

### Step 4: Implement verdict engine

**Files:** `crates/terraphim_merge_coordinator/src/verdict_engine.rs`
**Description:** PASS/FAIL/MISSING determination logic
**Tests:** Table-driven tests for verdict logic
**Dependencies:** Step 3
**Estimated:** 3 hours

### Step 5: Implement structured logger

**Files:** `crates/terraphim_merge_coordinator/src/structured_logger.rs`
**Description:** JSON log output to stdout
**Tests:** JSON serialization tests
**Dependencies:** Step 1
**Estimated:** 1 hour

### Step 6: Implement main CLI

**Files:** `crates/terraphim_merge_coordinator/src/main.rs`
**Description:** CLI entry point with dry-run support
**Tests:** Integration tests
**Dependencies:** Steps 2-5
**Estimated:** 2 hours

### Step 7: Debug redaction for tinyclaw

**Files:** `crates/terraphim_tinyclaw/src/config.rs`
**Description:** Custom Debug impl for TelegramConfig, DiscordConfig, etc.
**Tests:** Verify secrets not in debug output
**Estimated:** 2 hours

### Step 8: Debug redaction for tracker

**Files:** `crates/terraphim_tracker/src/gitea.rs`
**Description:** Custom Debug impl for GiteaConfig
**Tests:** Verify token not in debug output
**Estimated:** 1 hour

### Step 9: Debug redaction for github-runner

**Files:** `crates/terraphim_github_runner_server/src/config/mod.rs`
**Description:** Custom Debug impl for Settings
**Tests:** Verify secrets not in debug output
**Estimated:** 1 hour

### Step 10: Workspace integration

**Files:** `Cargo.toml`, `orchestrator.toml`
**Description:** Add crate to workspace, update agent config
**Tests:** Build verification
**Dependencies:** Steps 1-9
**Estimated:** 1 hour

## Rollback Plan

1. Revert `orchestrator.toml` to use shell scripts
2. Revert `Cargo.toml` workspace changes
3. Keep deprecated Python/shell scripts for 1 release cycle

Feature flag: Deploy shell version alongside Rust, switch via config.

## Dependencies

### New Dependencies

| Crate | Version | Justification |
|-------|---------|---------------|
| reqwest | 0.12 | Gitea API calls |
| thiserror | 2 | Error handling |
| tokio | 1 | Async runtime |

### Dependency Updates

| Crate | From | To | Reason |
|-------|------|-----|--------|
| None | - | - | No changes required |

## Performance Considerations

### Expected Performance

| Metric | Target | Measurement |
|--------|--------|-------------|
| Lock acquisition | < 100ms | Unit benchmark |
| Single PR evaluation | < 2s | Integration benchmark |
| Memory usage | < 10MB | Profiling |

### Benchmarks to Add

```rust
#[tokio::test]
async fn bench_evaluate_pr(b: &mut Bencher) {
    let client = GiteaClient::new().unwrap();
    b.iter(|| evaluate_pr(&client, 1234));
}
```

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Verify shell scripts are truly deprecated | Pending | Review |
| Confirm PID lock directory (/tmp) is acceptable | Pending | Ops |
| Validate retry delays (1s, 2s, 4s) with Gitea rate limits | Pending | Research |

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Performance targets agreed
- [ ] Human approval received

---

## Appendix: Credential Debug Redaction Pattern

```rust
// Before (INSECURE)
#[derive(Debug)]
struct TelegramConfig {
    token: String,
}

// After (SECURE)
struct TelegramConfig {
    token: String,
}

impl Debug for TelegramConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_struct("TelegramConfig")
            .field("token", &"***REDACTED***")
            .finish()
    }
}

// Reference: LinearConfig in same file has correct pattern
```
