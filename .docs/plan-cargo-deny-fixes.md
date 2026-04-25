---
date: 2026-04-25
type: plan
status: in-progress
---

# Plan: Make `cargo deny check` green in CI

## Findings (from CI run 24936561372, Security Scan job 73023044889)

```
error[unlicensed]:    fcctl-core = 0.1.0 is unlicensed
error[vulnerability]: idna accepts Punycode labels that do not produce
                      any non-ASCII when decoded   (RUSTSEC-2024-0421)
```

## Root causes

### 1. `fcctl-core` unlicensed

`fcctl-core` is a git dependency from the private `terraphim/firecracker-rust`
repo. None of the four crates in that repo (`fcctl-core`, `fcctl`, `fcctl-repl`,
`fcctl-web`) declared a `license` field in their `Cargo.toml`, and the repo
had no top-level `LICENSE` file.

### 2. `idna 0.4.0` carrying RUSTSEC-2024-0421

The dep tree had two versions of `idna`:
- `idna 1.1.0` -- pulled by `url -> reqwest` (current, unaffected)
- `idna 0.4.0` -- pulled by `trust-dns-proto -> trust-dns-resolver` (vulnerable)

`trust-dns` was renamed to `hickory-dns` in 2024 and the old crates are
unmaintained. The fix could have been a migration to `hickory-resolver`,
but `terraphim_rlm` only **declared** the `trust-dns-resolver = { version =
"0.23", optional = true }` dep behind a `dns-security` feature: there is
**no `use trust_dns_resolver` anywhere in the source**. Same pattern as
the `ring` dead-dep cleanup. Just remove it.

## Actions

### A. License fcctl-core upstream **DONE**

- Branch: `terraphim/firecracker-rust@main`
- Commit: `ffb4c094` -- declare Apache-2.0 license on all four crates and
  add a top-level `LICENSE` file (Apache 2.0 verbatim).
- Local lockfile updated: `cargo update -p fcctl-core` bumps the git ref
  from `07265b36` -> `ffb4c094`. Lockfile delta committed alongside the
  workspace deps cleanup.

### B. Drop dead `trust-dns-resolver` dep from `terraphim_rlm` **DONE**

- `crates/terraphim_rlm/Cargo.toml`: removed the `trust-dns-resolver`
  declaration and the `dns-security` feature (no consumers in source).
- `cargo update` drops `trust-dns-{resolver,proto}`, `quinn-*`, `idna 0.4.0`
  from the lockfile.
- Verification: `grep -c "trust-dns" Cargo.lock` returns 0;
  `grep "name = \"idna\"" Cargo.lock` shows only `1.1.0`.

### C. Re-run cargo deny on CI **PENDING (validation)**

The next `ci-main` run on commit after this plan should report 0 errors
from `cargo deny check`. If a new advisory surfaces, repeat the same
analysis: find the dep chain, confirm the consumer is dead, drop it. If
the consumer is live, prefer migration to a maintained successor over
adding to `deny.toml` ignore list.

## Why not `deny.toml` ignore?

Adding RUSTSEC-2024-0421 to the ignore list would silence the alarm
without removing the vulnerable code from the binary. We pulled `idna
0.4` only to satisfy a feature that no source code uses -- the surgical
fix is to remove the dead dep, not to mute the alarm.

## Follow-ups (not in this PR)

- Watch upstream `idna` and `url` for further advisories; both are at
  current versions and should not regress.
- The transitive `ring` dependency (via `reqwest -> hyper-rustls -> rustls`)
  is a separate, larger piece of work -- swap rustls's CryptoProvider to
  `rustls-rustcrypto` for full WASM compatibility. Out of scope here.
