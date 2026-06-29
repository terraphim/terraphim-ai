# Structural PR Review: BlockerKind Classification + Contamination Gate

**PRs**: Ref #2465, Ref #2409
**Author**: AI Agent
**Reviewed**: 2026-06-29
**Commit range**: 218d9e2a3..4e007d737
**Files**: 6 changed, +395/-52 lines

## Summary

This PR adds two independently valuable features to the merge coordinator:
1. **BlockerKind classification**: queries Gitea's CI status API to distinguish CI failures (`ci_failed`, `ci_pending`) from policy/confidence holds (`not_mergeable`), so operators can triage blocked PRs at a glance.
2. **Contamination gate**: scans PR file lists for artefact contamination (`.sessions/`, `.review_tmp/`, `.handoff/`, `.beads/`) before mergeability evaluation, preventing ADF agent noise from entering main.

The implementation is clean and well-structured. The `PrEvaluation` struct gains a backward-compatible `Option<BlockerKind>` field. `evaluate_one` is now async to support CI status lookups, with an `Option<&GiteaClient>` parameter for testability. The contamination check runs first (blocking artefact PRs immediately), followed by CI-status-based blocker classification.

**What was done well**:
- Clean separation of concerns: `classify_blocker`, `check_contamination`, and `evaluate_one` each have single responsibilities.
- Testability: `evaluate_one` accepts `Option<&GiteaClient>` so unit tests can pass `None` and skip network calls.
- Pagination: `list_pr_files` correctly uses `X-Total-Count` header with a page loop and guards against infinite loops via `page_len == 0`.
- Test coverage: 35 tests (34 existing + 1 new pattern-match test) all pass.

**What remains problematic**:
- One P1 finding: `list_pr_files` retry-with-backoff on non-retryable 404/4xx responses may waste time.
- One P2 finding: `check_contamination` pattern matching uses `contains` which could produce false positives on filenames that coincidentally contain pattern strings.

## Confidence Score: 4/5

Safe to merge with awareness of one P1 retry-noisiness issue. The retry-on-404 behaviour pre-exists in the `send_with_retry` helper (not introduced here), so it is consistent with the rest of the codebase. The contamination pattern false-positive risk is theoretical (no known collision in practice). Tests pass, clippy clean, fmt clean. No security or data-loss risks.

## Important Files Changed

| Filename | Overview |
|----------|----------|
| `types.rs` | Added `BlockerKind` enum with `Display` + `Serialize`, 2 new tests. Clean. |
| `gitea.rs` | Added `head_sha` to PrSummary, `CommitCombinedStatus` struct, paginated `list_pr_files`, `get_commit_status`. One P1 finding (retry on 404). |
| `evaluator.rs` | Core changes: `evaluate_one` async, `check_contamination`, `classify_blocker`. Updated all 5 existing tests + 1 new test. `#[allow(clippy::collapsible_match)]` on `evaluate_one`. |
| `lib.rs` | Extended `extract_fixes` to match `closes/close/fixes/fix/resolves/resolve` (was only `fixes`). Deduplication via `BTreeSet`. |
| `main.rs` | Minor doc/import sync. |
| `pid_lock.rs` | Minor doc sync. |

## Diagram

```mermaid
%%{init: {'theme': 'neutral'}}%%
flowchart TD
    A[evaluate_all: fetch open PRs] --> B[evaluate_one: per PR]
    B --> C{head_sha + GiteaClient?}
    C -->|Yes| D[check_contamination]
    C -->|No / test mode| G[Skip contamination]
    D --> E{file list has artefacts?}
    E -->|Yes: .sessions/ etc| F[Hold: contaminated]
    E -->|No| G
    G --> H{pr.mergeable?}
    H -->|No| I[classify_blocker]
    H -->|Yes| J[Merge]
    I --> K[GET /commits/{sha}/status]
    K --> L{CI state?}
    L -->|failure/error| M[Hold: ci_failed]
    L -->|pending| N[Hold: ci_pending]
    L -->|no data| O[Hold: ci_no_status]
    L -->|success| P[Hold: not_mergeable]
    
    style D fill:#d4edda,stroke:#28a745
    style I fill:#d4edda,stroke:#28a745
    style F fill:#fff3cd,stroke:#ffc107
    style M fill:#fff3cd,stroke:#ffc107
    style N fill:#fff3cd,stroke:#ffc107
    style O fill:#fff3cd,stroke:#ffc107
    style P fill:#fff3cd,stroke:#ffc107
```

## Inline Findings

**P1 `gitea.rs`, line 175**: **`list_pr_files` retries on 4xx/non-retryable errors**

The `get_with_retry` helper (called by `list_pr_files` to fetch each page) retries with backoff on ANY non-success status, including 404 (PR deleted), 403 (token expired), and 422 (malformed). A 404 on page 1 will retry 4 times (1s+2s+4s = 7s wasted) before giving up. A 404 on page N (after successful page 1..N-1) will also retry. This behaviour is inherited from `send_with_retry` (pre-existing in the codebase), so it is consistent, but it still adds latency to failure paths.

**Suggestion**: Consider adding a `non_retryable` status check to `send_with_retry` for 400-499 range, or wrap the page-loop in code that handles 404 specially (break early, return accumulated files). Alternatively, accept as-is since merge coordinator runs are infrequent and 7s latency on failure is negligible.

**P2 `evaluator.rs`, line 77**: **`check_contamination` uses `contains` which risks false positives**

The pattern match uses `file.starts_with(pattern) || file.contains(pattern)`. The `contains` check could match a path like `src/sessions_parser.rs` against `.sessions/` (though the `/` in the pattern makes this extremely unlikely). A more precise check would use `file.starts_with(pattern) || file.contains(&format!("/{}", pattern.trim_end_matches('/')))`.

**Impact**: Theoretical only. No known collision in practice. The check is a hold, not a permanent block — a human can override it. Acceptable as-is for initial implementation.

## Comments Outside Diff

*No findings on unchanged code.*

---

*Last reviewed commit: 4e007d737 | Reviews (1)*
