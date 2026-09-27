# Design: PageRank Bug Fixes for Gitea Robot API

**Date:** 2026-03-14
**Phase:** 2 -- Disciplined Design + Phase 3 Implementation
**Status:** Implemented
**Research Document:** `docs/src/research/pagerank-bug-investigation.md`

---

## Summary of Changes

Four bugs were fixed across two files in the Gitea fork, addressing the
root cause of uniform PageRank scores (0.15) returned by all Robot API
endpoints.

---

## Bug Fix Details

### Bug 3 / Issue #6: xorm struct tags with table-qualified names (ROOT CAUSE)

**File:** `models/issues/graph_cache.go`
**Lines changed:** 59-64 (struct), 74-97 (query)

**Problem:** The `DependencyWithRepo` struct used xorm tags like
`xorm:"issue_dependency.issue_id"` with dotted table-qualified names.
xorm's `Find()` does not support this syntax for result mapping -- it
treats the entire string as a literal column name. Since the SQL result
set returns columns as `issue_id` (without table prefix), xorm cannot
match them to the struct fields. All fields silently remain at their zero
values (0 for int64), producing a degenerate graph with a single phantom
node at ID 0.

**Fix:** Replaced the xorm `Table().Join().Find()` approach entirely with
raw SQL using `db.GetEngine(ctx).SQL(...).Find(&deps)`. The struct tags
were simplified to `xorm:"'issue_id'"` and `xorm:"'dependency_id'"`. The
`IsClosed` field was removed from the struct since closed-issue filtering
is now handled directly in the SQL WHERE clause.

**Raw SQL query:**
```sql
SELECT DISTINCT d.issue_id, d.dependency_id
FROM issue_dependency d
INNER JOIN issue i1 ON i1.id = d.issue_id
INNER JOIN issue i2 ON i2.id = d.dependency_id
WHERE i1.repo_id = ? AND i2.repo_id = ?
  AND i1.is_closed = ? AND i2.is_closed = ?
```

This query:
- Selects columns with unambiguous names matching the struct fields
- Joins both sides of the dependency to filter by repo
- Excludes closed issues on both sides
- Uses DISTINCT to eliminate duplicates (see Bug 2)
- Uses `?` placeholders for cross-database compatibility (SQLite + PostgreSQL)

### Bug 2 / Issue #8: Duplicate dependency edges

**File:** `models/issues/graph_cache.go`
**Lines changed:** 74-97 (query consolidation)

**Problem:** Two separate queries retrieved dependencies by joining on
`issue_id` and `dependency_id` respectively, then merged them with
`append(deps, deps2...)`. For same-repo dependencies (the common case),
both queries matched the same `issue_dependency` row, producing duplicate
edges. While the duplicates happened to cancel out mathematically in the
power iteration (2 * rank/(2*outDegree) = rank/outDegree), they wasted
memory and computation time, and could cause subtle issues if the
algorithm were modified.

**Fix:** Replaced the two queries with a single `SELECT DISTINCT` query
(see Bug 3 fix above). The DISTINCT keyword ensures each edge appears
exactly once, even if the query's join conditions could theoretically
match a row through multiple paths.

### Bug 1 / Issue #7: Adjacency matrix direction inverted

**File:** `models/issues/graph_cache.go`
**Lines changed:** 103-152 (adjacency + power iteration)

**Problem:** The adjacency list was built as `adj[dep.DependencyID] =
append(adj[dep.DependencyID], dep.IssueID)`, mapping blockers to the
issues they block. The power iteration then transferred rank FROM
blockers TO blocked issues. This made leaf issues (which are blocked by
many things) rank highest, which is the opposite of what task
prioritisation requires.

**Design decision:** Root/blocker issues should rank HIGHEST because they
unblock the most downstream work. In PageRank terms, each blocked issue
"votes for" (links to) its blockers.

**Fix:**
1. Adjacency now maps `adj[blockedID] -> [blockerIDs]` (outgoing votes)
2. An `incoming` map was added: `incoming[blockerID] -> [voterIDs]`
3. The power iteration was rewritten as an O(edges) distribution loop:
   - Start all nodes at the teleportation baseline
   - For each voter, distribute its rank equally among its targets
   - This replaces the previous O(issues * deps) nested scan

**New power iteration:**
```go
for voterID, targets := range adj {
    outDegree := len(targets)
    contribution := dampingFactor * pageRanks[voterID] / float64(outDegree)
    for _, targetID := range targets {
        newRanks[targetID] += contribution
    }
}
```

### Bug 4 / Issue #9: Ready/Graph never trigger PageRank computation

**File:** `routers/api/v1/robot/ready_graph.go`
**Lines changed:** +7 lines in `getReadyIssues()`, +7 lines in `getDependencyGraph()`

**Problem:** Both the Ready and Graph API handlers called
`issues.GetPageRanksForRepo()` which only reads from the `graph_cache`
table. Neither handler ever called `CalculatePageRank()` or
`EnsureRepoPageRankComputed()`. If the Triage endpoint had never been
called for a repository, the cache was empty and all scores fell back to
the baseline 0.15.

**Fix:** Added `issues.EnsureRepoPageRankComputed()` calls in both
`getReadyIssues()` and `getDependencyGraph()`, immediately before the
`GetPageRanksForRepo()` call. This lazily computes PageRank on first
access and uses cached values thereafter. Errors are logged as warnings
but do not block the response (graceful degradation).

---

## Files Changed

| File | Changes | Issues Fixed |
|------|---------|-------------|
| `models/issues/graph_cache.go` | Struct tags, query, adjacency, power iteration | #6, #8, #7 |
| `routers/api/v1/robot/ready_graph.go` | Add EnsureRepoPageRankComputed calls | #9 |

---

## Design Decisions

1. **Raw SQL over xorm builder:** Using `SQL()` with explicit column
   selection avoids all xorm struct-tag mapping ambiguity. The `?`
   placeholder syntax is compatible with both SQLite and PostgreSQL
   through xorm's driver layer.

2. **Both sides filtered by repo:** The query requires both `i1.repo_id`
   and `i2.repo_id` to match, which means cross-repo dependencies are
   excluded. This is correct for per-repo PageRank computation.

3. **Both sides filtered by is_closed:** Closed issues are excluded from
   both sides of each dependency edge. If either the blocker or the
   blocked issue is closed, the edge is dropped from the graph entirely.

4. **O(edges) power iteration:** The new distribution-based loop is more
   efficient than the previous O(issues * deps) scan approach. For each
   iteration, it visits each edge exactly once.

5. **No cache invalidation on CRUD:** Adding `InvalidateCache()` calls
   to `CreateIssueDependency` / `RemoveIssueDependency` is out of scope.
   The lazy computation via `EnsureRepoPageRankComputed` handles the cold
   cache case. Stale cache entries will be refreshed when the TTL-based
   check is implemented (future work).

6. **Graceful degradation:** The `EnsureRepoPageRankComputed` calls in
   Ready/Graph use `log.Warn` on failure rather than returning an error,
   so users still get results (with baseline scores) even if PageRank
   computation fails.

---

## Verification Plan

After deploying to `git.terraphim.cloud`:

1. **Clear stale cache:** Call the Triage endpoint to force recomputation
2. **Check Triage:** `GET /api/v1/robot/triage?owner=alex&repo=tlaplus-ts`
   - PageRank scores should vary (not all 0.15)
   - Root issues (#1, #2) should have highest scores
   - Sum of all scores should approximate 1.0
3. **Check Ready:** `GET /api/v1/robot/ready?owner=alex&repo=tlaplus-ts`
   - PageRank scores should match Triage
4. **Check Graph:** `GET /api/v1/robot/graph?owner=alex&repo=tlaplus-ts`
   - Node PageRank scores should match Triage
   - Edges should show correct dependency structure

---

## Risk Mitigation

- **Database compatibility:** Raw SQL uses only standard SQL syntax with
  `?` placeholders. Tested concept against both SQLite and PostgreSQL
  query parsing.
- **No schema changes:** The `graph_cache` table schema is unchanged.
  Only the data written to it changes (correct scores vs zeroes).
- **Backward compatible:** The API response format is unchanged. Only
  the numeric values of PageRank scores change.
- **Minimal diff:** 61 insertions, 60 deletions in graph_cache.go;
  14 insertions in ready_graph.go. No other files touched.
