## spec-validator verdict: CONDITIONAL PASS

**Date**: 2026-06-04 00:29 CEST
**Agent**: Carthos (Domain Architect / spec-validator)
**Issue**: #1992 — `test_server_mode_search_with_selected_role` HTTP 400 → exit 6

---

### Findings Summary

| Severity | Finding |
|----------|---------|
| P1 BLOCKER | PR #2011 targets Gitea `main`, but `crates/terraphim_agent` does not exist on Gitea — it was extracted to GitHub (origin) in the E1-E5 polyrepo cycle. `mergeable: false` is structural, not a rebase conflict. |
| P2 GAP | No unit or integration test covers `classify_error` with a real `reqwest::Error` that has `is_status() == true`. The `is_status()` branch added by PR #2011 is unverified by any test. |

---

### Spec Alignment Analysis

**Contract** (`crates/terraphim_agent/src/robot/exit_codes.rs`):

| Code | Name | Semantic |
|------|------|----------|
| 1 | `ErrorGeneral` | General/unspecified error |
| 4 | `ErrorNotFound` | No results found |
| 6 | `ErrorNetwork` | **Network or connectivity issue** |

**Violation on `main`**: `classify_error` (line 1340) returns `ErrorNetwork` (6) for ANY `reqwest::Error`, including HTTP status responses (400, 404, etc.). HTTP 400 is a semantic protocol response — not a network/connectivity failure. This violates the documented meaning of `ErrorNetwork`.

**Test assertion** (`server_mode_tests.rs:222`): `code == 0 || code == 1` is correct. A search failure due to unknown role configuration is a general error (1), not a network error (6).

**Fix in PR #2011** (`crates/terraphim_agent/src/main.rs` diff):
```rust
if re.is_status() {
    if let Some(status) = re.status() {
        if status == reqwest::StatusCode::NOT_FOUND {
            return ExitCode::ErrorNotFound;
        }
    }
    return ExitCode::ErrorGeneral;
}
// Connection refused, DNS failure, transport errors → network error.
return ExitCode::ErrorNetwork;
```

This is **semantically correct** and consistent with the documented exit code contract. Existing test `unreachable_server_exits_6` (connection refused → code 0 or 6) is unaffected because `is_status()` returns `false` for transport-level failures.

---

### Actions Required

**P1 (blocks merge)**: Re-route the fix to GitHub. The PR must target `origin/main` (GitHub) where `crates/terraphim_agent` exists. Either:
- Open a new GitHub PR from the same branch `task/1992-fix-search-http-status-exit-code`, OR
- Close PR #2011 on Gitea and apply the change directly to GitHub

**P2 (test coverage)**: Add a test that invokes a live server returning HTTP 400 and asserts `code == 1`. The integration test suite at `crates/terraphim_agent/tests/exit_codes.rs` already has `unreachable_server_exits_6` (lines 128–147) as the pattern. A companion test `server_http_error_exits_1` using a responding server with a bad query would close this gap. No mocks — use a real server or test with the existing `start_test_server()` fixture.

---

### Existing Test Coverage

Six `classify_error` unit tests in `mod classify_error_tests` (lines 1393–1507) use plain `anyhow::anyhow!("string")` errors. None exercise the `#[cfg(feature = "server")]` reqwest path. The `is_status()` branch added by the fix has **zero test coverage** in both unit and integration suites.

---

### Traceability

| Req | Requirement | Spec Ref | Impl Ref | Test | Status |
|-----|-------------|----------|----------|------|--------|
| REQ-01 | HTTP status errors exit 1, not 6 | `exit_codes.rs` `ErrorNetwork` doc | `main.rs:1340` classify_error | `server_mode_tests.rs:222` | ⚠️ fix exists (PR#2011), not merged |
| REQ-02 | `ErrorNetwork` reserved for transport failures | `exit_codes.rs:18` | `main.rs:1348` | `exit_codes.rs:130` unreachable_server | ✅ |
| REQ-03 | `is_status()` path tested | — | PR#2011 `main.rs:1340–1355` | **MISSING** | ❌ |

---

If PR #2011 is re-submitted targeting GitHub with a companion integration test for the `is_status()` path, verdict becomes **PASS**.
