# Design & Implementation Plan: Security Fixes for Issue #630

**Status**: Draft
**Author**: Agent
**Date**: 2026-04-20
**Gitea Issue**: #630
**Phase**: 2 (Design)
**Research Doc**: `.docs/research-issue-630-security-fixes.md`

---

## 1. Summary of Target Behaviour

After implementation:
- `cargo audit` reports 0 vulnerabilities on default workspace build (no features)
- `cargo deny check advisories` passes on default build
- rustls-webpki updated to 0.103.12, fixing RUSTSEC-2026-0098 and RUSTSEC-2026-0099 on the reqwest chain
- Direct `rand` dependencies removed from `terraphim_multi_agent` and `terraphim_kg_agents`, replaced with `fastrand` (WASM-compatible, already in tree)
- `deny.toml` updated with accurate ignore entries for discord-feature-only CVEs
- Port 11434 remediation documented (infrastructure fix, not code)

---

## 2. Key Invariants and Acceptance Criteria

| # | Invariant | Acceptance Criteria |
|---|-----------|---------------------|
| I1 | Default build has no rustls-webpki CVEs | `cargo audit` returns 0 errors on `cargo build --workspace` |
| I2 | rand removed from direct deps | `cargo tree -p terraphim_multi_agent` and `-p terraphim_kg_agents` show no `rand` |
| I3 | fastrand replaces rand for RNG | LoadBalancingStrategy::Random and worker success simulation produce equivalent behaviour |
| I4 | deny.toml is accurate | `cargo deny check` passes; ignore entries have correct RUSTSEC IDs |
| I5 | Discord feature CVEs documented | RUSTSEC-2026-0098, 0099, 0049 listed in deny.toml with serenity caveat |
| I6 | Build passes | `cargo build --workspace` succeeds; `cargo clippy --workspace` succeeds |
| I7 | Tests pass | All existing tests pass unchanged |

---

## 3. High-Level Design and Boundaries

### Changes Inside Existing Components

1. **Root Cargo.toml** -- Update git patch tag for rustls-webpki from `v/0.103.10` to `v/0.103.12`
2. **deny.toml** -- Add 0098/0099 to ignore list; update 0049 comment
3. **terraphim_multi_agent** -- Replace `rand = "0.9"` dep with `fastrand = "2"`, update 1 call site
4. **terraphim_kg_agents** -- Replace `rand = "0.9"` dep with `fastrand = "2"`, update 1 call site

### No New Components

All changes are modifications to existing files.

### Complected Areas

None -- each change is isolated to a single crate or config file.

---

## 4. File/Module-Level Change Plan

| File | Action | Before | After |
|------|--------|--------|-------|
| `Cargo.toml` (root) | Modify | `tag = "v/0.103.10"` | `tag = "v/0.103.12"` |
| `Cargo.toml` (root) | Modify | Comment says "RUSTSEC-2026-0049" | Comment says "RUSTSEC-2026-0049, 0098, 0099" |
| `deny.toml` | Modify | Missing 0098/0099 ignores | Add 0098/0099 with serenity caveat |
| `crates/terraphim_multi_agent/Cargo.toml` | Modify | `rand = "0.9"` | `fastrand = "2"` |
| `crates/terraphim_multi_agent/src/pool.rs` | Modify | `rand::rng().random_range(0..available.len())` | `fastrand::Rng::new().usize(0..available.len())` (or similar) |
| `crates/terraphim_kg_agents/Cargo.toml` | Modify | `rand = "0.9"` | `fastrand = "2"` |
| `crates/terraphim_kg_agents/src/worker.rs` | Modify | `rand::random()` | `fastrand::Rng::new().f64()` (0..1 range) |

---

## 5. Step-by-Step Implementation Sequence

### Step 1: Update rustls-webpki git patch (5 min)

**Purpose**: Fix RUSTSEC-2026-0098 and RUSTSEC-2026-0099 on the default build chain.

**File**: `Cargo.toml` (root)

**Changes**:
- Change `tag = "v/0.103.10"` to `tag = "v/0.103.12"` in `[patch.crates-io]`
- Update comment on `rustls-webpki` workspace dep and patch to reference all three CVEs

**Deployable**: Yes -- `cargo build --workspace` must pass after this change.

**Verification**:
```bash
cargo build --workspace
cargo tree --workspace | grep "rustls-webpki"  # should show 0.103.12
```

### Step 2: Update deny.toml (5 min)

**Purpose**: Document all known CVEs with accurate ignore entries.

**File**: `deny.toml`

**Changes**:
- Update existing `RUSTSEC-2026-0049` comment to reference serenity chain
- Add `RUSTSEC-2026-0098` with comment: "rustls-webpki name constraints bypass (URI) -- transitive via serenity -> rustls 0.22 -> webpki 0.102.x; discord feature only"
- Add `RUSTSEC-2026-0099` with comment: "rustls-webpki name constraints bypass (wildcards) -- transitive via serenity -> rustls 0.22 -> webpki 0.102.x; discord feature only"

**Deployable**: Yes.

**Verification**:
```bash
cargo deny check advisories 2>&1
```

### Step 3: Replace rand in terraphim_multi_agent (15 min)

**Purpose**: Remove direct rand dependency, use WASM-compatible fastrand.

**Files**:
- `crates/terraphim_multi_agent/Cargo.toml` -- replace `rand = "0.9"` with `fastrand = "2"`
- `crates/terraphim_multi_agent/src/pool.rs` -- replace lines 349-350

**Before**:
```rust
LoadBalancingStrategy::Random => {
    use rand::Rng;
    rand::rng().random_range(0..available.len())
}
```

**After**:
```rust
LoadBalancingStrategy::Random => {
    fastrand::usize(0..available.len())
}
```

**Deployable**: Yes.

**Verification**:
```bash
cargo build -p terraphim_multi_agent
cargo test -p terraphim_multi_agent
cargo tree -p terraphim_multi_agent | grep rand  # should show no direct rand
```

### Step 4: Replace rand in terraphim_kg_agents (15 min)

**Purpose**: Remove direct rand dependency, use WASM-compatible fastrand.

**Files**:
- `crates/terraphim_kg_agents/Cargo.toml` -- replace `rand = "0.9"` with `fastrand = "2"`
- `crates/terraphim_kg_agents/src/worker.rs` -- replace line 394

**Before**:
```rust
let random_value: f64 = rand::random();
```

**After**:
```rust
let random_value: f64 = fastrand::f64();
```

**Deployable**: Yes.

**Verification**:
```bash
cargo build -p terraphim_kg_agents
cargo test -p terraphim_kg_agents
cargo tree -p terraphim_kg_agents | grep rand  # should show no direct rand
```

### Step 5: Full workspace verification (15 min)

**Purpose**: Confirm all changes work together.

**Verification**:
```bash
cargo build --workspace
cargo clippy --workspace -- -D warnings
cargo test --workspace
cargo tree --workspace | grep "rustls-webpki"  # only 0.103.12
cargo tree -p terraphim_multi_agent | grep -E "^.*rand"  # no direct rand
cargo tree -p terraphim_kg_agents | grep -E "^.*rand"    # no direct rand
cargo deny check advisories 2>&1
```

### Step 6: Commit and update Gitea (5 min)

```bash
git add -A
git commit -m "fix(security): resolve RUSTSEC-2026-0098/0099, replace rand with fastrand Refs #630"
git push
```

Update Gitea issue #630 with summary comment.

---

## 6. Testing and Verification Strategy

| Acceptance Criteria | Test Type | Verification |
|---------------------|-----------|--------------|
| I1: No webpki CVEs on default build | Build + tree inspection | `cargo tree --workspace \| grep rustls-webpki` shows only 0.103.12 |
| I2: rand removed from direct deps | Dependency inspection | `cargo tree -p terraphim_multi_agent -i rand` empty; same for kg_agents |
| I3: fastrand produces equivalent results | Unit (existing) | Existing tests for pool load balancing and worker simulation pass |
| I4: deny.toml accurate | `cargo deny check` | Exits 0 on default build |
| I5: Discord CVEs documented | File inspection | deny.toml has 0049, 0098, 0099 entries |
| I6: Build passes | CI | `cargo build --workspace && cargo clippy --workspace` |
| I7: Tests pass | CI | `cargo test --workspace` |

### Note on fastrand behavioural equivalence

- `rand::rng().random_range(0..n)` and `fastrand::usize(0..n)` both produce uniform random indices in `[0, n)`. Behaviour is equivalent.
- `rand::random::<f64>()` produces `[0, 1)` and `fastrand::f64()` also produces `[0, 1)`. Behaviour is equivalent.
- Neither call site requires cryptographic randomness -- both are for load balancing and simulation.

---

## 7. Risk and Complexity Review

| Risk (from Phase 1) | Mitigation | Residual Risk |
|----------------------|------------|---------------|
| rustls-webpki 0.103.12 tag doesn't exist | Verified: tag exists at `27131d47` | None |
| fastrand changes load balancing behaviour | fastrand produces uniform distribution; equivalent to rand | None |
| getrandom API differs from rand | Using fastrand, not getrandom directly -- API maps 1:1 | None |
| portpicker still pulls rand 0.8 | Deferred to separate issue -- out of scope | Low (test-only code path) |
| bincode still present | Already in deny.toml ignore; out of scope | Low (external dep) |
| Discord feature still has webpki 0.102.8 CVEs | Documented in deny.toml; feature off by default | Low (opt-in only) |

---

## 8. Open Questions / Decisions for Human Review

1. **fastrand vs getrandom**: This plan uses `fastrand` for the two call sites because it's simpler (one function call vs byte buffer + conversion) and already in the tree. Acceptable?

2. **portpicker deferral**: The `portpicker` crate pulls `rand 0.8` into `terraphim_server`. Defer to a separate issue? It's only used in tests and main.rs for dev-mode port picking.

3. **rand 0.10.0 via axum-test**: This is a dev-dependency only (terraphim_validation). Defer?

4. **Commit message format**: Use `fix(security): ... Refs #630` or prefer different convention?

---

## Deferred Items (Not In Scope)

| Item | Reason | Tracking |
|------|--------|----------|
| Replace portpicker (rand 0.8) | Test-only code, non-blocking | Separate issue |
| Replace bincode in terraphim_automata | Requires migration to postcard/rkyv | RUSTSEC-2025-0141 in deny.toml |
| Port 11434 firewall on bigbox | Infrastructure, not code | Document in issue comment |
| Serenity 0.13 upgrade | Breaking API changes (5 break points) | PR #353 context |
