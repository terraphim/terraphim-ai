<h3>Requirements Traceability Summary</h3>

PR #1347 addresses the parallel test-interference defect in `test_tool_index_save_and_load` (issue #1340). The test previously wrote to a shared path under `std::env::temp_dir()`, causing non-deterministic failures when two test workers ran concurrently. The fix replaces the shared path with a `tempfile::TempDir` guard — a unique directory created per test invocation, automatically removed on drop.

**Scope**: one test function in one file. No production code changed. `tempfile` was already a workspace dependency and already declared in `crates/terraphim_agent/Cargo.toml` — no new dependency introduced.

**Conflict note**: PR #1356 addresses the same invariant (issue #1340) on the identical hunk with a different strategy (timestamp-based filename). These two PRs are mutually exclusive; only one can merge.

---

<h3>Verdict: pass</h3>

---

<h3>Traceability Matrix</h3>

| Req ID | Requirement (issue #1340) | Design Ref | Impl Ref | Tests | Status |
|-------:|---------------------------|------------|----------|-------|--------|
| REQ-001 | `test_tool_index_save_and_load` must not collide with concurrent test workers | Issue #1340 root-cause | `mcp_tool_index.rs:282` — `tempfile::TempDir::new()` creates a unique directory per invocation | Modified test IS the verification; isolation enforced structurally by directory uniqueness | ✅ |
| REQ-002 | Temp file cleanup must be deterministic, not rely on manual `remove_file` | Issue #1340 / Rust idiom | `mcp_tool_index.rs:295` — `TempDir` guard held in scope; directory deleted automatically on drop | Auto-cleanup guaranteed by `Drop` impl; removes previous `remove_file` manual step | ✅ |
| REQ-003 | No new workspace dependencies introduced | Workspace hygiene | `tempfile = { workspace = true }` already present in `crates/terraphim_agent/Cargo.toml` and root `Cargo.toml` | Verified: `tempfile = "3.27"` already in workspace dep table on `main` | ✅ |

---

<h3>Gaps</h3>

**None blocking.**

**Note — competing PR #1356**

PR #1356 also fixes issue #1340 via a timestamp-based unique filename (`subsec_nanos()`) and additionally carries rustdoc fixes (issue #1331). The two PRs conflict on the same hunk. Recommendation: merge PR #1347 for the #1340 fix (stronger isolation guarantee, idiomatic `Drop`-based cleanup), then cherry-pick the rustdoc commits from PR #1356 onto `main` separately if #1331 is not yet closed.

**Note — single test, no workspace-level regression run**

AC coverage is narrow by design (the change is a one-function test fix). The broader `cargo test --workspace` run is not independently verified here; risk of regression is negligible given the scope.

---

<sub>Last spec-validated commit: cde6964</sub>
