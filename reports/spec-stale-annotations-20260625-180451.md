# Stale Spec-Annotation Sweep — 2026-06-25

**Agent:** documentation-generator (Ferrox, Rust Engineer)
**Scope:** in-code doc comments referencing spec documents / spec decisions
**Trigger:** `please action the stale-spec annotations`

---

## Method

Grepped all `.rs` for spec-reference patterns (`Per spec`, `spec at .docs/`,
`per spec <Tag>-N`, `Mirrors the spec`, `[SPEC]`). For each hit, verified
whether the referenced artefact (spec file / cross-repo plan / Gitea issue)
actually exists and whether the surrounding code matches the described
behaviour.

## Findings

### Stale — ACTIONED (1 cluster, 6 files)

**`crates/terraphim_merge_coordinator/`** — every module header referenced
`.docs/spec-merge-coordinator.md`, a file that has **never existed in git
history** (absent on `origin/main` too — a dangling pointer since the crate's
inception, #1805). Headers also called the crate a "minimal skeleton" with
logic "will be implemented in follow-up commits", but the crate is fully
implemented (24 tests across 6 modules, real `evaluate_all` / `merge_and_close`
/ PID-lock / retrying Gitea client).

| File | Old stale fragment | Fix |
|------|-------------------|-----|
| `src/lib.rs` | "minimal skeleton … Full spec at .docs/spec-merge-coordinator.md will be implemented in follow-up commits" | Rewritten as accurate library overview |
| `src/types.rs` | "Mirrors the spec at .docs/spec-merge-coordinator.md. Lib code in follow-up commits will use these types" | Rewritten; "spec" pointer dropped |
| `src/evaluator.rs` | "Sequential per spec Concurrency-2 … per Failure-1 … per Failure-2" | Behavioural facts retained, dangling tag refs dropped |
| `src/pid_lock.rs` | "File-based PID lock … (Concurrency-1 per spec)" | Tag dropped; behavioural doc kept |
| `src/gitea.rs` | "retry/backoff per spec Failure-3 … never logs the token (Security-2)" | Tag dropped; facts kept |
| `src/main.rs` | "Exit codes per Operational-1" | Rewritten as plain list |

### Not stale — LEFT UNTOUCHED (surgical protocol)

- `crates/terraphim_rlm/src/executor/trait.rs:111` — `Per spec: "Ignore
  external state drift on restore"` is a self-contained inline quote of the
  behaviour, not a file pointer. Accurate. Left as-is.
- `crates/terraphim_orchestrator/src/pr_dispatch.rs`, `pr_poller.rs`,
  `tests/auto_merge_execution_tests.rs` — reference
  `cto-executive-system/plans/adf-rate-of-change-design.md` §Step N plus real
  Gitea issues (`adf-fleet#32`, `#34`). These are **cross-repo** pointers to a
  separate design repo, not workspace-internal dangling refs. Accurate. Left.
- `pr-spec-validator` identifiers in `lib_tests.rs` — those are agent / status
  names, not spec annotations. Left.

## Verification

```
cargo fmt -p terraphim_merge_coordinator -- --check   # clean
cargo build -p terraphim_merge_coordinator             # exit 0
cargo clippy -p terraphim_merge_coordinator --tests    # exit 0
cargo test -p terraphim_merge_coordinator              # 24 passed, 0 failed
RUSTDOCFLAGS="-D warnings" cargo doc -p terraphim_merge_coordinator --no-deps  # exit 0
```

No remaining `spec-merge-coordinator` / `minimal skeleton` / `will be
implemented in follow-up` references in the crate.

## Conclusion

6 stale annotations resolved across 1 crate. Zero regressions. The crate's
headers now describe actual behaviour rather than a non-existent spec and a
skeleton state that no longer applies.

Theme-ID: doc-gap
