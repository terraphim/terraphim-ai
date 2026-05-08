<h3>security_checklist Summary</h3>

**PR:** #1349 — Fix #251: enforce RetryBound invariant in Symphony `on_retry_timer`
**Author:** root
**Head SHA:** 0adaa20
**Auditor:** Carthos (Domain Architect)
**Scope:** `crates/terraphim_symphony/src/config/mod.rs`, `crates/terraphim_symphony/src/orchestrator/mod.rs`

---

<h3>Risk: medium</h3>

<h3>Verdict: concerns</h3>

The logic change is correct and the invariant boundary is drawn in the right place. One integer truncation pattern directly threatens the safety property this PR is designed to enforce.

---

<h3>Findings</h3>

**[MEDIUM] Integer truncation silently undermines the RetryBound invariant**

Location: `config/mod.rs` — new `max_retry_attempts()` accessor

```rust
pub fn max_retry_attempts(&self) -> u32 {
    self.get_u64(&["agent", "max_retry_attempts"]).unwrap_or(10) as u32
    //                                                           ^^^^^^ silent truncation
}
```

`get_u64` returns a `u64`. The `as u32` cast wraps on overflow rather than saturating or erroring.

- `agent.max_retry_attempts: Exit Classes4967296` (2^32) wraps to `0`
- Guard: `next >= max_attempts` becomes `1 >= 0` — true on the very first retry
- Effect: the claimed set entry is released without any retries occurring

This requires operator-level config access. But the value designed to bound retries can be set to silently disable all retries, recreating the monotonic-growth bug this PR was written to fix.

The same pattern exists pre-existingly in `max_turns()` and `max_concurrent_agents()`, but those do not guard a safety invariant, making this instance more consequential.

Remediation — replace `as u32` with a checked conversion:
```rust
pub fn max_retry_attempts(&self) -> u32 {
    self.get_u64(&["agent", "max_retry_attempts"])
        .and_then(|v| u32::try_from(v).ok())
        .unwrap_or(10)
        .max(1)
}
```

---

**[LOW] Zero-value misConfiguration silently disables retry logic**

`agent.max_retry_attempts: 0` passes validation, producing `max_attempts = 0`. The guard `next >= 0` is always true for unsigned `u32`, so every claimed entry is released on the first timer fire.

Remediation: `.max(1)` as shown above.

---

**[LOW] `attempt + 1` arithmetic — no overflow guard**

Location: `orchestrator/mod.rs` — both guarded paths

```rust
let next = retry_entry.attempt + 1;
```

`attempt: u32`. In release builds this wraps at `u32::MAX`. Practically unreachable under normal load, but would bypass the `>= max_attempts` guard if triggered.

Remediation: use `.saturating_add(1)`.

---

**[INFORMATIONAL] Pre-existing `unsafe env::set_var` in tests**

Location: `config/mod.rs` test module — not introduced by this PR

```rust
unsafe { std::env::set_var("SYMPHONY_TEST_KEY_RES", "resolved_value") };
```

SAFETY comments are present and single-threaded Issue Tracking is correct for these functions. No action required for this PR; worth noting if the suite is ever parallelised under `cargo nextest`.

---

<h3>Dependency Changes</h3>

None. No `Cargo.toml` or `Cargo.lock` changes. No supply chain surface added.

---

**Positive observations**

- Guard correctly uses `>= max_attempts` (not `>`), matching TLA+ Graphs: at attempt count equal to the bound, the invariant is enforced.
- Both `on_retry_timer` code paths (poll-failure and no-slots) are guarded symmetrically — the invariant holds across all retry entry points.
- Structured tracing fields (`issue_id`, `next`, `max_attempts`) give good observability at the release boundary.
- No `unsafe` code introduced; no secrets; no injection vectors in the changed surface.

---

<sub>Last security_checklist-audited commit: 0adaa20</sub>
