# Research Document: Security Fixes for Issue #630

**Status**: Draft
**Author**: Agent
**Date**: 2026-04-20
**Gitea Issue**: #630
**Phase**: 1 (Research)

---

## 1. Problem Restatement and Scope

### Problem

The automated security audit (security_checklist agent) identified 5 findings blocking production deployment:

1. **RUSTSEC-2026-0098**: rustls-webpki name constraints bypass via URI names (HIGH)
2. **RUSTSEC-2026-0099**: rustls-webpki name constraints bypass via wildcards (HIGH)
3. **RUSTSEC-2026-0097**: rand unsound with custom logger (MEDIUM)
4. **RUSTSEC-2025-0141**: bincode unmaintained (MEDIUM)
5. **Port 11434 (Ollama) publicly exposed** (CRITICAL -- infrastructure)

Additionally, the project has a cross-cutting concern: **minimise rand dependencies in favour of WASM-compatible random** (`getrandom`).

### IN Scope

- Eliminate or suppress all 4 cargo-audit findings
- Document the port 11434 infrastructure fix (cannot be fixed in code)
- Reduce rand dependency surface area across workspace crates
- Update `deny.toml` to reflect current state accurately

### OUT Scope

- Serenity 0.13 / `next` branch migration (separate effort, PR #353 showed 5 break points)
- Full bincode replacement with postcard/rkyv (separate issue)
- Ollama configuration on bigbox (infrastructure, not code)
- WASM build target changes (related but separate)

---

## 2. User and Business Outcomes

### Visible Changes

- `cargo audit` returns 0 vulnerabilities on default workspace build
- `cargo deny check` passes cleanly
- Fewer rand versions in dependency tree (ideally just getrandom-backed)
- Security audit issue #630 can be closed

### Business Value

- Production deployment unblocked
- Reduced attack surface from certificate validation bypasses
- WASM compatibility improved by removing heavy rand dependencies
- Cleaner supply chain with fewer unmaintained crates

---

## 3. System Elements and Dependencies

### 3A: rustls-webpki CVE Chain

**Current state**: The workspace already has a git patch for rustls-webpki 0.103.10:

```
# Cargo.toml [patch] section
rustls-webpki = { git = "https://github.com/rustls/webpki.git", tag = "v/0.103.10" }
```

This patches the **reqwest -> hyper-rustls 0.27 -> rustls 0.23** chain to use webpki 0.103.10.

**Problem**: rustls-webpki 0.103.10 does NOT fix RUSTSEC-2026-0098/0099. The fix requires >=0.103.12 or >=0.104.0.

**Default workspace build**: No serenity. rustls-webpki 0.103.10 via reqwest chain only. This IS vulnerable to 0098/0099.

**Discord feature build** (`--features discord`): serenity 0.12.5 -> rustls 0.22.4 -> rustls-webpki 0.102.8 (separate, older, ALSO vulnerable).

| Component | Version | Vulnerable? | Path |
|-----------|---------|-------------|------|
| rustls-webpki | 0.103.10 (git patch) | YES (0098, 0099) | reqwest -> hyper-rustls 0.27 -> rustls 0.23 |
| rustls-webpki | 0.102.8 (crates.io) | YES (0098, 0099, 0049) | serenity -> hyper-rustls 0.24 -> rustls 0.22 |

**Key insight**: Discord is already removed from default features (done in prior session). So 0.102.8 only appears when `--features discord` is explicitly enabled. The 0.103.10 path affects ALL builds.

**Fix options for 0.103.10 path**:
- Update git patch tag from `v/0.103.10` to `v/0.103.12` (or later)
- This is a one-line change in the root `Cargo.toml` `[patch]` section

**Fix options for 0.102.8 path** (discord feature):
- Cannot patch -- serenity 0.12.5 pins rustls 0.22.x which pins webpki 0.102.x
- Already mitigated by removing discord from defaults
- Add RUSTSEC-2026-0098/0099 to deny.toml ignore list with same conditions as 0049

### 3B: rand Unsound (RUSTSEC-2026-0097)

**Three rand versions in the workspace**:

| Version | Path to terraphim-ai | Usage Type |
|---------|---------------------|------------|
| rand 0.10.0 | axum-test -> rust-multipart-rfc7578_2 | Dev-dependency only (terraphim_validation) |
| rand 0.9.2 | proptest (dev-dep), terraphim_multi_agent, terraphim_kg_agents | Direct dep + dev-dep |
| rand 0.8.5 | portpicker -> terraphim_server; phf_generator -> markup5ever (build-dep) | Transitive |

**Direct rand usage in our code** (2 call sites):
- `crates/terraphim_multi_agent/src/pool.rs:349-350` -- `rand::rng().random_range(0..available.len())` for load balancing
- `crates/terraphim_kg_agents/src/worker.rs:394` -- `rand::random()` for random value generation

**WASM-compatible alternatives already in workspace**:
- `getrandom 0.3` with `wasm_js` feature in `terraphim_types`, `terraphim_automata`
- `getrandom 0.2` with `js` + `wasm-bindgen` in `terraphim_atomic_client`
- `getrandom 0.4` is also present (transitive)

**The constraint**: Replace direct `rand` usage with `getrandom`-backed alternatives. For the two call sites:
- `pool.rs` random range: use `fastrand` (already in tree, WASM-compatible, no std required) or `getrandom` + manual range
- `worker.rs` random f64: use `getrandom` to fill bytes, convert to f64

**portpicker (rand 0.8.5)**: `portpicker` crate is not WASM-compatible and uses rand 0.8. Could replace with a simple `getrandom`-based port picker or use a fixed port range.

### 3C: bincode Unmaintained (RUSTSEC-2025-0141)

**Two paths**:

| Path | Affected Crate | Version |
|------|---------------|---------|
| `terraphim_automata` | Direct dependency | bincode 1.3.3 |
| `fff-search -> heed -> heed-types` | Transitive | bincode 1.3.3 |

**terraphim_automata**: Uses bincode for serialisation. Would need migration to postcard or rkyv.

**fff-search (external)**: heed (LMDB bindings) uses bincode internally. Cannot change without upstream fix.

**Current state**: Already in `deny.toml` ignore list with TODO.

### 3D: Port 11434 (Infrastructure)

Ollama listening on `0.0.0.0:11434` on bigbox. Not a code fix -- requires:
- Rebind Ollama to `127.0.0.1:11434` in systemd unit or Ollama config
- Or add firewall rule

**This is documentation-only in code.** The fix is on bigbox infrastructure.

### 3E: Existing deny.toml State

Current ignores already cover:
- `RUSTSEC-2026-0049` (rustls-webpki CRL bypass) -- same serenity chain
- `RUSTSEC-2026-0097` (rand unsound) -- already ignored
- `RUSTSEC-2025-0141` (bincode) -- already ignored

**Missing from ignore list**:
- `RUSTSEC-2026-0098` (rustls-webpki URI name constraints)
- `RUSTSEC-2026-0099` (rustls-webpki wildcard name constraints)

---

## 4. Constraints and Their Implications

| Constraint | Why It Matters | Implication |
|------------|---------------|-------------|
| WASM compatibility | Several crates target WASM (terraphim_types, terraphim_automata) | Must use getrandom/fastrand, not rand |
| serenity 0.12.5 pins rustls 0.22 | Cannot upgrade webpki 0.102.x without serenity 0.13 | Accept CVE in discord feature, document it |
| fff-search uses heed which uses bincode | External dependency we cannot change | Must keep bincode ignore in deny.toml |
| Discord already off by default | CVE path is latent, not active | Low urgency for discord chain |
| rustls-webpki 0.103.10 is patchable | We control the git patch tag | One-line fix to update to 0.103.12 |

---

## 5. Risks, Unknowns, and Assumptions

### Risks

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| rustls-webpki 0.103.12 not released or tag doesn't exist | Low | High | Check tag exists before changing; fall back to 0.104.x |
| Replacing rand in pool.rs changes load balancing behaviour | Low | Low | fastrand produces uniform distribution; behaviour equivalent |
| getrandom API differs from rand for range generation | Medium | Low | Write small helper function; trivial maths |
| portpicker replacement breaks server tests | Medium | Medium | Keep portpicker for now; flag as future work |

### Assumptions

1. `v/0.103.12` tag exists on `github.com/rustls/webpki` -- **needs verification**
2. fastrand is sufficient for the two call sites replacing rand -- likely true (simple RNG needs)
3. portpicker can be deferred to a separate issue -- assumed acceptable
4. bincode replacement is out of scope for this issue -- assumed acceptable

### Unknowns

1. Does `rustls-webpki 0.103.12` actually exist as a git tag? Must verify before committing to this approach.
2. Does the workspace build cleanly with the updated patch? Need to test.
3. Is fastrand already a dependency of any terraphim crate? (Yes -- it's transitive via tokio)

---

## 6. Context Complexity vs Simplicity Opportunities

### Sources of Complexity

- Three separate rustls-webpki CVEs affecting two different versions via two different dependency chains
- Three rand versions across workspace with different consumers
- bincode in external dependency (heed) we cannot control

### Simplification Opportunities

1. **Single git patch update** solves the main (default build) webpki CVE -- one line in Cargo.toml
2. **Two deny.toml additions** documents the discord-feature-only CVEs -- consistent with existing pattern
3. **Two code changes** replace rand with fastrand in our direct code -- small, self-contained
4. **Defer portpicker and bincode** -- already tracked, not blocking

---

## 7. Questions for Human Reviewer

1. **rustls-webpki patch target**: Should we target `v/0.103.12` specifically, or go to `v/0.104.x` (newer major)? 0.103.x is a smaller change.

2. **portpicker replacement scope**: Include in this issue or defer? It brings rand 0.8 into terraphim_server. Defering is simpler.

3. **rand -> fastrand migration**: Accept fastrand for the two call sites, or prefer direct getrandom usage? fastrand is simpler but adds a dep.

4. **Discord feature deny.toml ignores**: Add 0098/0099 alongside existing 0049, or consolidate into a single comment block for "serenity chain CVEs"?

5. **Port 11434**: Should we add a CI check / pre-deploy script that verifies Ollama is not publicly bound, or just document the fix?

---

## Appendix: Dependency Maps

### rustls-webpki dependency (default build, no features)

```
reqwest 0.12.28
  -> hyper-rustls 0.27.7
    -> rustls 0.23.37
      -> rustls-webpki 0.103.10 (git patch) <-- VULNERABLE to 0098/0099
```

### rustls-webpki dependency (discord feature)

```
terraphim_tinyclaw --features discord
  -> serenity 0.12.5
    -> hyper-rustls 0.24.x (pinned)
      -> rustls 0.22.4 (pinned)
        -> rustls-webpki 0.102.8 <-- VULNERABLE to 0049, 0098, 0099
```

### rand dependency map

```
rand 0.10.0 <-- RUSTSEC-2026-0097
  <- rust-multipart-rfc7578_2 0.9.0
    <- axum-test 19.1.1 (dev-dep)

rand 0.9.2
  <- terraphim_multi_agent (direct dep, pool.rs:349)
  <- terraphim_kg_agents (direct dep, worker.rs:394)
  <- proptest 1.11.0 (dev-dep)

rand 0.8.5
  <- portpicker 0.1.1 (terraphim_server)
  <- phf_generator 0.11.3 (build-dep via markup5ever)

getrandom 0.3.x (WASM-compatible, preferred)
  <- terraphim_types (direct, wasm_js)
  <- terraphim_automata (direct, wasm_js)

getrandom 0.2.x (WASM-compatible)
  <- terraphim_atomic_client (direct, js + wasm-bindgen)
```

### bincode dependency map

```
bincode 1.3.3 <-- RUSTSEC-2025-0141 (unmaintained)
  <- terraphim_automata (direct dep)
  <- heed-types 0.21.0
    <- heed 0.22.1
      <- fff-search (external, cannot change)
```
