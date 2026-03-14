# Research: PageRank Computation Bug in Gitea Robot API

**Date:** 2026-03-14
**Phase:** 1 -- Disciplined Research
**Status:** Complete, awaiting approval for Phase 2

---

## Problem Statement

The Gitea Robot API at `https://git.terraphim.cloud` returns **uniform PageRank scores of 0.15** (which equals `1 - damping_factor` where `damping_factor = 0.85`) for ALL issues, despite the `tlaplus-ts` repository having 8 issues and 12 dependency edges. All three endpoints exhibit this behaviour:

- `/api/v1/robot/ready` -- returns `page_rank: 0.15` for every issue
- `/api/v1/robot/triage` -- returns `pagerank: 0.15` for every recommendation
- `/api/v1/robot/graph` -- returns `page_rank: 0.15` for every node

**Expected behaviour:** Issues that are depended upon by many downstream issues (e.g. root issues #1, #2) should have higher PageRank scores than leaf issues (e.g. #8) that have no dependents.

**Success criteria:**
1. Root/hub issues receive higher PageRank scores than leaf issues
2. PageRank scores vary across issues proportionally to dependency structure
3. Sum of all PageRank scores approximates 1.0
4. The algorithm correctly converges within configured iterations

---

## Existing System Analysis

### Code Locations

| File | Role |
|------|------|
| `/Users/alex/projects/terraphim/gitea/models/issues/graph_cache.go` | **Core PageRank computation** (lines 66-173), cache CRUD, ranked issue queries |
| `/Users/alex/projects/terraphim/gitea/routers/api/v1/robot/ready_graph.go` | Ready and Graph API handlers, reads cached PageRank |
| `/Users/alex/projects/terraphim/gitea/routers/api/v1/robot/robot.go` | Triage API handler, input validation, permission checks |
| `/Users/alex/projects/terraphim/gitea/services/robot/robot.go` | Triage service -- the **only caller** of `CalculatePageRank()` |
| `/Users/alex/projects/terraphim/gitea/modules/setting/graph.go` | Configuration (damping=0.85, iterations=100) |
| `/Users/alex/projects/terraphim/gitea/models/issues/dependency.go` | `IssueDependency` model -- defines `issue_id` and `dependency_id` columns |
| `/Users/alex/projects/terraphim/gitea/models/migrations/v1_26/v326.go` | Migration creating `graph_cache` table |

### Architecture Flow

```
Request --> Router --> Service --> CalculatePageRank() --> graph_cache table --> Response
```

1. **Triage endpoint** (`robot.go:79`) calls `service.Triage()` which calls `issues_model.CalculatePageRank(ctx, repoID, 0.85, 100)` -- this is the **only place** that triggers computation.
2. **Ready endpoint** (`ready_graph.go:197`) calls `issues.GetPageRanksForRepo()` -- this **only reads** from cache; it never triggers computation.
3. **Graph endpoint** (`ready_graph.go:471`) calls `issues.GetPageRanksForRepo()` -- same as Ready, read-only.

### Dependency Semantics in Gitea

From `dependency.go` (line 107-114):
```go
type IssueDependency struct {
    ID           int64
    UserID       int64
    IssueID      int64              // The issue that IS BLOCKED
    DependencyID int64              // The issue that BLOCKS it
}
```

When creating: `CreateIssueDependency(ctx, user, issue, dep)` stores `IssueID = issue.ID, DependencyID = dep.ID`. This means: **"issue is blocked by dep"** or equivalently **"dep blocks issue"**.

---

## Root Cause Analysis

### Bug 1 (CRITICAL): Adjacency matrix direction is inverted

**Location:** `graph_cache.go`, lines 102-114

```go
// adj[depID] = list of issues that depend on it (blocked by it)
adj := make(map[int64][]int64)

for _, dep := range deps {
    validIssues[dep.IssueID] = true
    validIssues[dep.DependencyID] = true
    adj[dep.DependencyID] = append(adj[dep.DependencyID], dep.IssueID)
}
```

The adjacency list `adj[depID]` maps a dependency (blocker) to the list of issues it blocks. This means `adj` encodes **outgoing edges from blockers to blocked issues**.

Then in the power iteration (lines 129-152):
```go
for issueID := range validIssues {
    newRank := (1.0 - dampingFactor) / float64(issueCount)

    for _, dep := range deps {
        if dep.IssueID == issueID {
            blockerID := dep.DependencyID
            if currentRank, ok := pageRanks[blockerID]; ok {
                outDegree := len(adj[blockerID])
                if outDegree > 0 {
                    newRank += dampingFactor * currentRank / float64(outDegree)
                }
            }
        }
    }
    newRanks[issueID] = newRank
}
```

This says: for each issue, find its blockers, and transfer rank FROM blockers TO the blocked issue. The `outDegree` is counted as the number of issues that the blocker blocks (`adj[blockerID]`).

**The problem:** In standard PageRank applied to dependency graphs, the "importance" should flow in the **opposite direction**. An issue that many other issues depend on (a root/hub issue) should accumulate more rank. The current code gives rank to blocked issues (leaves), which is the opposite of what is desired for task prioritisation.

However, this inversion alone would still produce non-uniform scores. The reason all scores are uniform is Bug 2.

### Bug 2 (CRITICAL): Duplicate dependency edges cause incorrect graph construction

**Location:** `graph_cache.go`, lines 74-97

```go
// Get all dependencies for this repo, joined with issue info to filter by repoID
var deps []DependencyWithRepo
err := db.GetEngine(ctx).
    Table("issue_dependency").
    Join("INNER", "issue", "issue.id = issue_dependency.issue_id AND issue.repo_id = ?", repoID).
    Where("issue.is_closed = ?", false).
    Find(&deps)

// Also get dependencies where the issue is the dependency (blocked by)
var deps2 []DependencyWithRepo
err = db.GetEngine(ctx).
    Table("issue_dependency").
    Join("INNER", "issue", "issue.id = issue_dependency.dependency_id AND issue.repo_id = ?", repoID).
    Where("issue.is_closed = ?", false).
    Find(&deps2)

// Merge both dependency lists
deps = append(deps, deps2...)
```

The code runs **two separate queries** and merges them:
- Query 1: finds rows where `issue_id` belongs to this repo (and is open)
- Query 2: finds rows where `dependency_id` belongs to this repo (and is open)

For a **same-repo dependency** (the common case where both the issue and its dependency are in the same repository), both queries will match the **same row**, producing **duplicate edges** in the `deps` slice.

**Effect of duplicates on the adjacency list:**

```go
adj[dep.DependencyID] = append(adj[dep.DependencyID], dep.IssueID)
```

With duplicate edges, each `adj[depID]` entry will contain the same issue ID twice, doubling the `outDegree`. Since PageRank distributes rank proportionally to `1/outDegree`, the doubled out-degree halves each contribution.

**Why this causes uniform scores:** The duplicate entries also cause the power iteration inner loop to process each edge twice (once for each duplicate in `deps`), but the `outDegree` is also doubled. The net effect: `2 * (rank / (2 * outDegree))` = `rank / outDegree`, which is the same as without duplicates. So the duplicates **cancel out** in the iteration math.

This means the root cause for uniform scores must be elsewhere. Let me trace through the algorithm more carefully.

### Bug 3 (CRITICAL -- the actual root cause): The `DependencyWithRepo` struct field tags do not match xorm expectations

**Location:** `graph_cache.go`, lines 59-64

```go
type DependencyWithRepo struct {
    IssueID      int64 `xorm:"issue_dependency.issue_id"`
    DependencyID int64 `xorm:"issue_dependency.dependency_id"`
    IsClosed     bool  `xorm:"issue.is_closed"`
}
```

The xorm tags here use `table.column` syntax (`issue_dependency.issue_id`). However, **xorm does not support table-qualified column names in struct tags for `Find()` operations**. The xorm tag is interpreted as the column name in the result set. When using `Table("issue_dependency").Join(...)`, xorm builds a query like:

```sql
SELECT issue_dependency.issue_id, issue_dependency.dependency_id, issue.is_closed
FROM issue_dependency
INNER JOIN issue ON issue.id = issue_dependency.issue_id AND issue.repo_id = ?
WHERE issue.is_closed = ?
```

But the result columns from this query are typically returned as just `issue_id`, `dependency_id`, and `is_closed` (without table prefixes). The xorm struct tags containing dots (`issue_dependency.issue_id`) will **not match** the result columns, and xorm will fail to populate the struct fields. The fields will remain at their zero values (`0` for int64, `false` for bool).

**This is the smoking gun.** When all `IssueID` and `DependencyID` fields are 0:
- `validIssues` will contain only `{0: true}` -- a single phantom issue
- `adj` will contain `{0: [0, 0, ...]}` -- self-loops on issue 0
- The PageRank iteration will compute scores for issue ID 0 only
- All real issues will not appear in `pageRanks`
- When `UpdatePageRank` is called, it will upsert a single row for issue_id=0

Then when the API handlers call `GetPageRanksForRepo()`:
- No cached scores exist for any real issue IDs
- They fall back to the baseline: `1.0 - 0.85 = 0.15`

**This explains the uniform 0.15 scores perfectly.**

### Bug 4 (SECONDARY): Ready and Graph endpoints never trigger PageRank computation

**Location:** `ready_graph.go`, lines 197-201 and 471-475

Both the `Ready` and `Graph` endpoints call `issues.GetPageRanksForRepo()` which only reads from the cache. They never call `CalculatePageRank()` or `EnsureRepoPageRankComputed()`.

If the Triage endpoint has never been called for a repository (or if the computation produced zero values due to Bug 3), the Ready and Graph endpoints will always return baseline values.

### Bug 5 (MINOR): Testing plan tests are all marked TODO

The `testing-plan.md` lists 12 PageRank-specific tests, all marked as TODO. No unit tests exist for `CalculatePageRank()`. Had `TestCalculatePageRank_SimpleChain` been implemented, it would have caught the xorm struct tag issue immediately.

---

## Verification of Root Cause Hypothesis

To confirm Bug 3, we can check whether xorm correctly populates `DependencyWithRepo` by:

1. Adding debug logging in `CalculatePageRank` to print the length of `deps` and the actual values of `deps[0].IssueID` and `deps[0].DependencyID`
2. Or by running the raw SQL query directly and comparing

The hypothesis predicts:
- `len(deps)` will be > 0 (the JOIN works, rows are returned)
- `deps[i].IssueID` and `deps[i].DependencyID` will both be 0 for all entries
- The log message on line 169 will report something like "1 issues, N dependencies" where 1 is the single phantom issue ID 0

---

## Proposed Fixes

### Fix 1: Correct the xorm struct tags (fixes Bug 3)

Replace the table-qualified tags with simple column names and add explicit column selection:

```go
type DependencyWithRepo struct {
    IssueID      int64 `xorm:"'issue_id'"`
    DependencyID int64 `xorm:"'dependency_id'"`
    IsClosed     bool  `xorm:"'is_closed'"`
}
```

Or better, use `Cols()` in the query to select specific columns with aliases:

```go
var deps []DependencyWithRepo
err := db.GetEngine(ctx).
    SQL(`SELECT d.issue_id, d.dependency_id, i.is_closed
         FROM issue_dependency d
         INNER JOIN issue i ON i.id = d.issue_id AND i.repo_id = ?
         WHERE i.is_closed = ?`, repoID, false).
    Find(&deps)
```

### Fix 2: Eliminate duplicate edges (fixes Bug 2)

Use a single query with OR condition, or use `UNION` to combine both directions, or query all dependencies and filter in Go:

```go
var deps []DependencyWithRepo
err := db.GetEngine(ctx).
    SQL(`SELECT DISTINCT d.issue_id, d.dependency_id
         FROM issue_dependency d
         INNER JOIN issue i1 ON i1.id = d.issue_id
         INNER JOIN issue i2 ON i2.id = d.dependency_id
         WHERE (i1.repo_id = ? OR i2.repo_id = ?)
           AND i1.is_closed = ?
           AND i2.is_closed = ?`,
        repoID, repoID, false, false).
    Find(&deps)
```

### Fix 3: Reverse PageRank direction (fixes Bug 1)

For task prioritisation, rank should flow from blocked issues TO their blockers (upstream). An issue that blocks many others should accumulate more rank. Change the adjacency direction:

```go
// adj[issueID] = list of dependencies (blockers) of this issue
// In PageRank terms: issue "links to" its dependencies
adj := make(map[int64][]int64)

for _, dep := range deps {
    validIssues[dep.IssueID] = true
    validIssues[dep.DependencyID] = true
    // Issue depends on dep, so issue "votes for" dep
    adj[dep.IssueID] = append(adj[dep.IssueID], dep.DependencyID)
}
```

Then in the power iteration:
```go
for issueID := range validIssues {
    newRank := (1.0 - dampingFactor) / float64(issueCount)

    // Sum contributions from issues that depend on this one
    for voterID, targets := range adj {
        for _, targetID := range targets {
            if targetID == issueID {
                outDegree := len(adj[voterID])
                if outDegree > 0 {
                    newRank += dampingFactor * pageRanks[voterID] / float64(outDegree)
                }
            }
        }
    }
    newRanks[issueID] = newRank
}
```

Or more efficiently, precompute incoming edges.

### Fix 4: Make Ready and Graph trigger computation (fixes Bug 4)

Add `EnsureRepoPageRankComputed()` calls to both endpoints, or have them call `CalculatePageRank()` directly like the Triage endpoint does.

### Fix 5: Add unit tests for PageRank calculation

Implement the tests from `testing-plan.md`, particularly:
- `TestCalculatePageRank_SimpleChain`
- `TestCalculatePageRank_StarPattern`
- `TestCalculatePageRank_SumToOne`

---

## Constraints and Risks

1. **Database compatibility:** The fix must work with both SQLite and PostgreSQL backends. Raw SQL using `?` placeholders is safe for xorm's multi-database support.
2. **Cross-repo dependencies:** The `issue_dependency` table can reference issues from different repositories. The fix must handle this correctly (only include issues from the target repo).
3. **Performance:** The current O(issues * deps) inner loop in the power iteration should be optimised to O(edges) per iteration. For small repos (< 100 issues) this is not critical, but for larger repos it will matter.
4. **Cache invalidation:** There is no mechanism to invalidate PageRank cache when dependencies change. The `InvalidateCache()` function exists but is never called from dependency CRUD operations.
5. **Closed issue filtering:** The current code filters closed issues from the dependency query but the `is_closed` field filtering may not work correctly if the xorm struct tags are broken (Bug 3).

---

## Open Questions

1. **Should PageRank direction match web PageRank or be inverted for task prioritisation?** The testing plan and E2E scenario documentation suggest that root issues (blockers) should have the highest PageRank. The E2E doc says "Issue 1 should have highest PageRank" where Issue 1 is the root that blocks Issues 2 and 3. This confirms that rank should flow from blocked issues to their blockers.

2. **Should closed issues be completely excluded or receive a baseline score?** Currently the code tries to exclude them from the graph, which seems correct for prioritisation.

3. **Should `EnsureRepoPageRankComputed` use a TTL-based check?** The `PageRankCacheTTL` setting exists (default 300 seconds) but is never used in the actual cache check -- `hasPageRankCache()` only checks whether rows exist, not their age.

4. **Is there a hook to invalidate cache when dependencies are added/removed?** Currently there is not. Should `CreateIssueDependency` and `RemoveIssueDependency` call `InvalidateCache()`?

---

## Recommended Next Steps

1. **Confirm Bug 3** by adding temporary debug logging in `CalculatePageRank()` and calling the Triage endpoint, or by running the xorm query in a test harness.
2. **Fix Bug 3** (xorm struct tags) as the highest priority -- this is the root cause of the uniform scores.
3. **Fix Bug 2** (duplicate edges) to prevent subtle correctness issues.
4. **Fix Bug 1** (PageRank direction) to match the documented expectation that blockers rank higher.
5. **Fix Bug 4** (Ready/Graph not triggering computation) so all endpoints return computed scores.
6. **Write unit tests** to prevent regression and validate mathematical correctness.
7. **Add cache invalidation** on dependency CRUD operations.

---

## Summary

The primary root cause is **Bug 3: xorm struct tags with table-qualified names fail silently**, causing all dependency data to be read as zero values. This produces a degenerate graph with a single phantom node (ID 0), whose computed PageRank is never stored for real issue IDs. Consequently, all API responses fall back to the baseline score of `1 - 0.85 = 0.15`.

Secondary issues include duplicate edges from two overlapping queries (Bug 2), inverted PageRank flow direction (Bug 1), and endpoints that never trigger computation (Bug 4).
