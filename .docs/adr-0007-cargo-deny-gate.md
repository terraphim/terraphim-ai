# ADR-0007: Cargo-deny gate activation

**Status:** ADOPTED (2026-08-08)
**Deciders:** Hermes Agent (session review of TinyClaw ↔ Hermes parity work)
**Fleet mandate:** `rust-fleet-standard` §1.5

## Context

Per `rust-fleet-standard` §1.5, every PR gate must include `cargo deny check`. The configuration file `deny.toml` was committed to the repo but **cargo-deny itself was never installed** and the gate had never been run.

Running `cargo deny check` for the first time on 2026-08-08 against `terraphim-ai` workspace revealed:

```
advisories FAILED, bans ok, licenses FAILED, sources ok
```

Two real findings of fleet-standard significance:

1. **Unlicensed path-deps introduced by Wave 4 of this session.** `jmap_client` (1.0.0) and `haystack_core` (0.2.0) — both from `terraphim-private` workspace — do not declare a `license` field in their Cargo.toml. **This is a supply-chain hygiene gap I introduced by adding `jmap_client` as a path dep without first verifying the sibling's license metadata.**

   **2026-08-08 update:** Terraphim AI workspace DOES have MIT-licensed haystack crates (`atlassian_haystack 1.0.0`, `discourse_haystack 1.0.0` — both on terraphim registry). But there is no MIT-licensed `jmap_client` in `terraphim-ai`; the only JMAP client is the unlicensed path-dep in `terraphim-private/`. So `jmap_client` path dep remains the right call capability-wise, but the license gap is real and the action item (file issue in `terraphim-private`) is still required.

2. **Cargo.lock vulnerability in `crossbeam-epoch 0.9.18`** (CVE in `fmt::Pointer` impl for `Atomic`/`Shared`). Dev-only dep via `criterion` → `rayon`. Not in our runtime, but still flagged by the gate.

## Decision

**Activate the cargo-deny gate.** Block merge on FAILED status. Document the current state and the path to green.

## Action items (in priority order)

1. **File Gitea issue** in `terraphim-private` asking for `license = "Apache-2.0 OR MIT"` to be added to `jmap_client` and `haystack_core` Cargo.toml files. **(fleet-standard supply-chain fix)**

2. **Add `MIT-0` to `deny.toml` allow list** (single-line cleanup, lets `borrow-or-share 0.2.4` pass).

3. **Verify `quick-xml 0.38.4` vulnerability status** — the deny output is ambiguous because the CVE is filed against 0.37.5. If 0.38.4 is also flagged, bump opendal. If only 0.37.5 is flagged, no action needed.

4. **Clean up stale `ignore` entries** in `deny.toml` (5 RUSTSEC IDs no longer match anything in the dep graph).

5. **Track as fleet-rollout issue** in `terraphim-ai` per §1.6 (issue → gitea-robot claim → design gate → fix).

## Verification

```bash
cargo install cargo-deny --locked  # ~3 min compile
cargo deny check 2>&1 | tail -3
# Current: advisories FAILED, bans ok, licenses FAILED, sources ok
# Target:  all 4 sections ok
```

## What This Means for the Merge Bar

**Workspace is NOT currently fleet-standard §1.5 compliant.** All previous PRs (including this session's Wave 4 Hermes parity work) bypassed the cargo-deny gate because the gate was never wired.

The 355-test pass + clippy clean + fmt clean is necessary but **not sufficient**.

## Lessons Learned

1. **The fleet standard is right.** This is exactly the kind of issue the standard was designed to catch — a new path-dep was added without verifying supply-chain metadata. Without cargo-deny in the merge gate, this would have shipped uncorrected.

2. **Configuration is not enforcement.** `deny.toml` was committed but `cargo deny` was never installed. The gate existed in form but not in function. **A standard is only as strong as its enforcement.**

3. **Audit-before-implement.** I should have run `cargo deny check` BEFORE adding `jmap_client` as a path dep, not after. The discipline pattern: check the store before adding to the cart.

## References

- `/home/alex/projects/cto-executive-system/2026-08-08-cargo-deny-findings.md` — full findings report
- `deny.toml` — gate config (untouched, gated work is at the action items above)
- `rust-fleet-standard` skill §1.5 (validation stack mandate)
- `memory/regressions.md` §5 (claim-sudo-without-checking) — sister "audit before action" lesson