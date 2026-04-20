<h3>Summary</h3>

This PR adds the `handoff_ledger` persistence layer with a new SQLite
schema and a retry loop around transient write failures. The core logic
is sound but the retry loop does not guard non-retryable errors, and
there is one hard-coded path that blocks deployment to the staging
environment.

Key changes:

- **handoff.rs**: new `persist_ledger` function with configurable retry.
- **migrations/**: schema `202604201100_handoff_ledger.sql`.
- **tests**: integration coverage for happy path and transient failure.

What was done well: schema is forward-compatible, integration tests hit
a real SQLite file instead of a mock. The retry loop itself is cleanly
separated from the business logic.

Acceptance criteria:

- [x] schema migration lands
- [x] happy-path persistence covered by tests
- [ ] retry guards non-retryable errors

<h3>Confidence Score: 3/5</h3>

- Safe to merge with caution — retry-loop correctness and the staging
  hard-coded path must be addressed before or shortly after merging.
- Zero P0. One P1 (retry loop does not guard 4xx-equivalent SQLite
  errors). Two P2 items are hygiene.
- `crates/terraphim_orchestrator/src/handoff.rs` requires attention.

<h3>Important Files Changed</h3>

| Filename | Overview |
|----------|----------|
| `crates/terraphim_orchestrator/src/handoff.rs` | Adds persistence; retry loop needs to short-circuit non-retryable errors. |
| `crates/terraphim_orchestrator/migrations/202604201100_handoff_ledger.sql` | New schema, fine as-is. |

<h3>Inline Findings</h3>

**P1 crates/terraphim_orchestrator/src/handoff.rs, line 324**: **Retry loop does not guard non-retryable errors**

`persist_ledger` retries on every `sqlx::Error`, including
`Error::Database(e) if e.is_unique_violation()`. This wastes the full
retry budget on errors that will never succeed and obscures the real
cause in logs.

```rust
match err {
    sqlx::Error::Database(e) if !is_retryable(&e) => return Err(err),
    _ => { /* retry */ }
}
```

**P2 crates/terraphim_orchestrator/src/handoff.rs, line 98**: **Hard-coded staging DSN**

`const STAGING_DSN: &str = "sqlite:///var/lib/adf/handoff.db";` should
come from `TerraphimSettings` so the same binary can target dev.

**P2 crates/terraphim_orchestrator/tests/handoff_persistence_tests.rs, line 17**: **Duplicated setup helper**

The `setup_temp_db` helper is copy-pasted from `agent_run_record_tests.rs`.
Extract to a shared `common` module.

<sub>Last reviewed commit: 3be4e599 | Reviews (1)</sub>
