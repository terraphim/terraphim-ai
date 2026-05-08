<h3>Requirements Traceability Summary</h3>

PR #1356 carries three commits across two distinct concerns:

1. **issue #1340** (`fix(test)` — unique temp path): replaces the shared `std::env::temp_dir()/test-mcp-index.json` path with a timestamp-suffixed filename to reduce parallel test collision in `test_tool_index_save_and_load`.
2. **issue #1331** (`docs` — rustdoc warnings): fixes unresolved intra-doc link references in `crates/terraphim_persistence/src/lib.rs` (`ConversationPersistence` path, backtick-escaped `Arc<DeviceStorage>`) and `crates/terraphim_rolegraph/src/lib.rs` (bare `[new]`/`[from_serializable]` → qualified `[RoleGraph::new]`/`[RoleGraph::from_serializable]`).
3. **issue #1331** (`docs` — gap report): adds `reports/doc-gap-report-20260508.md` confirming zero remaining documentation warnings across 52 crates.

**Conflict**: PR #1347 addresses the same issue #1340 hunk with a stronger fix (`tempfile::TempDir` vs. timestamp). These PRs are mutually exclusive on the `mcp_tool_index.rs` hunk.

---

<h3>Verdict: concerns</h3>

---

<h3>Traceability Matrix</h3>

| Req ID | Requirement | Design Ref | Impl Ref | Tests | Status |
|-------:|-------------|------------|----------|-------|--------|
| REQ-001 | `test_tool_index_save_and_load` must not collide with concurrent test workers (issue #1340) | Issue #1340 | `mcp_tool_index.rs:283-286` — `subsec_nanos()` suffix appended to filename | Modified test IS the verification | ⚠️ |
| REQ-002 | Intra-doc links in `terraphim_persistence` must resolve without warnings (issue #1331) | Issue #1331 | `crates/terraphim_persistence/src/lib.rs:15` — qualified path `[conversation::ConversationPersistence]`; `lib.rs:97,113` — backtick-escaped `Arc<DeviceStorage>` | `cargo doc --no-deps -p terraphim_persistence` zero warnings (per gap report) | ✅ |
| REQ-003 | Intra-doc links in `terraphim_rolegraph` must resolve without warnings (issue #1331) | Issue #1331 | `crates/terraphim_rolegraph/src/lib.rs:361,436` — `[RoleGraph::new]` and `[RoleGraph::from_serializable]` | `cargo doc --no-deps -p terraphim_rolegraph` zero warnings (per gap report) | ✅ |
| REQ-004 | Doc gap report confirms zero remaining warnings across workspace | Issue #1331 AC | `reports/doc-gap-report-20260508.md` — 52 crates, 0 content warnings | Gap report is the evidence artefact | ✅ |

---

<h3>Gaps</h3>

**⚠️ REQ-001 — timestamp isolation is weaker than `tempfile::TempDir`**

`subsec_nanos()` returns nanoseconds within the current second (range 0–999,999,999). Under heavy parallelism, two test workers spawned within the same second at identical nanosecond offsets would still produce the same path. Additionally, the timestamp approach leaves the file behind on the filesystem — there is no cleanup on drop. PR #1347 uses `tempfile::TempDir` which guarantees uniqueness via OS-level random suffix and auto-removes the directory via `Drop`. Both approaches satisfy the requirement for typical CI conditions, but PR #1347's solution is structurally stronger. This is the reason for the `concerns` verdict rather than `fail`: the fix is sufficient in practice but not optimal in design.

**Note — competing PR #1347**

PR #1347 addresses issue #1340 on the identical hunk with a better strategy. Recommendation: merge PR #1347 for the #1340 fix, then consider a separate PR containing only the rustdoc commits from this PR (`53408a7be` and `0c4948474`) if issue #1331 is not yet fully closed on `main`.

**Note — CHANGELOG update included**

This PR updates `CHANGELOG.md`. The entry is accurate and matches the commits. No traceability gap; noted for completeness.

---

<sub>Last spec-validated commit: 0c49484</sub>
