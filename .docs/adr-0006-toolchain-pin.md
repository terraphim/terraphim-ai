# ADR-0006: Rust toolchain pin (rust-toolchain.toml)

**Status:** ADOPTED (2026-08-08)
**Deciders:** Hermes Agent (session review of TinyClaw ↔ Hermes parity work)
**Fleet mandate:** `rust-fleet-standard` §1.4

## Context

Per `rust-fleet-standard` §1.4, every fleet Rust repo MUST commit a root
`rust-toolchain.toml` with channel + `rustfmt` + `clippy` components. Drift
between the pin and workspace `rust-version` must be recorded explicitly
with rationale.

Before this ADR, `terraphim-ai` had no `rust-toolchain.toml`. The session
that delivered the TinyClaw ↔ Hermes parity epic added it, but the
initial pin (1.93) was wrong and broke the build because `sysinfo@0.39.5`
requires `rustc >= 1.95`.

## Decision

Pin `channel = "1.96"` (default stable on the reference box, 2026-04-16).

## MSRV trace

| Step | Value | Why |
|------|-------|-----|
| Workspace `rust-version` (Cargo.toml `[workspace.package]`) | 1.91 | Set by prior maintainer; reflects the lowest crate's stated MSRV |
| `sysinfo@0.39.5` transitive dep | requires `rustc >= 1.95` | Newest published version, no older 1.91-compatible line |
| Candidate pin 1.93 | REJECT | Build breaks: `error: rustc 1.93.1 is not supported by sysinfo@0.39.5` |
| Candidate pin 1.95 | ADOPTED (initial) | Builds clean; matches the tightest dep constraint |
| Final pin 1.96 | ADOPTED (this revision) | Default stable on this host; probe verified all workspace deps compile |

## Alternatives considered

- **Pin 1.95** — builds, but ties us to an older minor. 1.96 is the
  default on this host and the wider fleet baseline.
- **Pin nightly** — rejected: nightly drift breaks reproducibility and
  §1.4 specifies a stable channel.
- **Pin MSRV (1.91)** — rejected: requires downgrading `sysinfo` to a
  pre-0.39 line and blocking several other transitive deps. Out of
  scope for the fleet rollout window.
- **Set `rust-version = "1.91"` but no toolchain pin** — pre-ADR state.
  REJECT per §1.4.

## Consequences

- All CI workers + dev boxes that build `terraphim-ai` MUST have
  `rustup toolchain install 1.96` or use the auto-install path
  (`rust-toolchain.toml` triggers this automatically when `rustup` is
  present).
- Workspace `rust-version` (1.91) is now intentionally LOWER than the
  toolchain pin. This is allowed by §1.4 ("record the decision") but
  must be flagged at next dependency audit.
- If a new dep requires `rustc > 1.96`, this ADR must be revised and
  re-merged.

## Verification

```bash
cargo check -p terraphim_tinyclaw  # exits 0
cargo test -p terraphim_tinyclaw --all-targets --no-fail-fast
# Result: 349 passed, 0 failed
```

## References

- `rust-fleet-standard` skill §1.4
- `/home/alex/projects/cto-executive-system/2026-08-08-rust-fleet-standard.md`
- `memory/2026-08-08.md` (session log of the parity work)
- `memory/regressions.md` (rule: ADOPT/ADAPT/REJECT every fleet mandate violation in code)