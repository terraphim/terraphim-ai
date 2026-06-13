# Incomplete Handoff: Structural Review of PR #2664

**PR**: terraphim/terraphim-ai #2664 — "Fix #2431 #2430: implement list_snapshots and stop VMs in cleanup"  
**Head commit**: `0607a5732cd56d5d7bcd0bfb02ebed13b4fa4aa7` (first 7: `0607a57`)  
**Base commit**: `782cd08061fbf6b13c35d32d2d2698ab0706ed6a`  
**Review started**: 2026-06-13 22:00+02:00  
**Status**: WIP — findings drafted, score not finalised, requested output block not yet produced.

## What has been done

1. Fetched PR metadata and full diff from Gitea (`/api/v1/repos/terraphim/terraphim-ai/pulls/2664.diff`).
2. Stored diff at `.pr2664.diff` and head versions of all changed Rust/bash files under `.review_head/`.
3. Read the key changed files:
   - `crates/terraphim_rlm/src/executor/firecracker.rs`
   - `crates/terraphim_rlm/src/rlm.rs`
   - `crates/terraphim_rlm/src/validator.rs`
   - `crates/terraphim_rlm/src/config.rs`
   - `crates/terraphim_rlm/src/executor/mod.rs`
   - `crates/terraphim_rlm/src/executor/context.rs`
   - `scripts/rustup-with-perms.sh`
   - `scripts/install-rustup-perms-guard.sh`
4. Drafted the following candidate findings (severity not yet finalised):

### Candidate P1 — `list_snapshots` returns unusable `SnapshotId`s
- **File**: `crates/terraphim_rlm/src/executor/firecracker.rs`, line ~689
- `list_snapshots` maps each `fcctl-core` snapshot to `SnapshotId::new(s.name, *session_id)`. `SnapshotId::new` generates a fresh ULID for `id`, discarding the Firecracker snapshot ID. `restore_snapshot`/`delete_snapshot` later use `id.id.to_string()` as the FC snapshot ID, so snapshots returned by `list_snapshots` cannot be restored or deleted. This undermines the whole purpose of the new method.
- **Suggested fix**: either carry the FC snapshot ID through `SnapshotId` (e.g. parse it as ULID or add a backend-id field), or change `restore_snapshot`/`delete_snapshot` to resolve by name/session.

### Candidate P1 — `kg_max_retries` is ignored on the execution hot path
- **File**: `crates/terraphim_rlm/src/rlm.rs`, lines ~336 and ~407
- `execute_code` and `execute_command` call `self.validator.validate(code)?` directly. `RlmConfig::kg_max_retries` is passed into `KnowledgeGraphValidator::from_config` but never used because `validate_with_context` (which honours retries/escalation) is not called.
- **Suggested fix**: create a per-call `ValidationContext`, call `validate_with_context`, and handle retry/escalation semantics.

### Candidate P1/P2 — `list_snapshots` silently swallows SnapshotManager errors
- **File**: `crates/terraphim_rlm/src/executor/firecracker.rs`, line ~685
- `.unwrap_or_default()` on `snapshot_manager.list_snapshots(...).await` discards errors, returning an empty list on failure. This masks IO/config issues and contradicts the error-handling in `delete_snapshot`/`restore_snapshot`.
- **Suggested fix**: propagate the error or return a distinct error instead of `unwrap_or_default`.

### Candidate P2 — Feature-gate inconsistency for KG validation
- `rlm.rs` adds a `KnowledgeGraphValidator` and validates unconditionally (no `#[cfg(feature = "kg-validation")]`), while `executor/{docker,firecracker,local}.rs` gate their validators behind that feature. This creates duplication and confusing build semantics.
- **Suggested fix**: decide whether KG validation is always compiled or feature-gated, and apply the same rule consistently.

### Candidate P2 — PR diff includes unrelated/unmerged work
- The PR diff from base..head contains 20 files, including `.docs/research-release-blockers.md`, `.sessions/session-20260613-134146.md`, KG wiring (#2482/#2415), rustup wrapper fixes (#2605), version alignment, etc. Many of these commits already exist on `main`; the branch appears not rebased. This makes the review scope noisy.
- **Suggested action**: rebase the branch onto current `main` so the PR diff reflects only the intended #2431/#2430 changes.

### Candidate P2 — `cleanup` stops VMs sequentially and holds `vm_manager` lock
- **File**: `crates/terraphim_rlm/src/executor/firecracker.rs`, lines ~808-817
- `cleanup` holds the `tokio::sync::Mutex` on `vm_manager` across all sequential `stop_vm` calls. If stopping a VM is slow or hangs, other async operations needing the manager are blocked.
- **Suggested fix**: stop VMs concurrently with `try_join`/` FuturesUnordered` and drop the lock around each call.

## What remains

1. **Calibrate severities and confidence score**: decide whether the snapshot-ID mismatch and ignored retries are blocking P1s, and whether the feature-gate/noise issues are P2s.
2. **Inspect remaining changed files**:
   - `crates/terraphim_rlm/src/executor/docker.rs`
   - `crates/terraphim_rlm/src/executor/local.rs`
   - `crates/terraphim_rlm/src/query_loop.rs`
   - `crates/terraphim_tinyclaw/src/skills/executor.rs`
   - `crates/terraphim_tinyclaw/tests/skills_benchmarks.rs`
   - `crates/terraphim_update/src/signature.rs`
   - `.gitea/workflows/native-ci.yml`
   - Cargo.toml / Cargo.lock changes
3. **Verify PR claims**: confirm `cargo test -p terraphim_rlm` and clippy/format checks.
4. **Produce the final markdown block** exactly as requested by the user:
   - `### Summary`
   - `### Confidence Score: N/5`
   - `### Inline Findings`
   - `<sub>Last reviewed commit: 0607a57</sub>`
5. **Post the review** (orchestrator will capture output; no manual PR comment required by user).

## Next-agent starting position

- Resume from this handoff file.
- Use the diff in `.pr2664.diff` and head file copies in `.review_head/` for line-specific references.
- The most important correctness issue to resolve first is the `SnapshotId` mapping in `list_snapshots`; the second is the ignored `kg_max_retries` in `rlm.rs`.
- Re-run the skill's calibration steps before generating the final output block.
