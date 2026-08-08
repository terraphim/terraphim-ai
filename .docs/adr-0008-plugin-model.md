# ADR-0008: TinyClaw Plugin Model — Compile-Time vs Runtime Discovery

**Status**: ACCEPTED (2026-08-08)
**Deciders**: Hermes Agent (Wave 6 of Hermes parity arc)
**Fleet mandate**: `terraphim-skills` catalog, plugin discoverability
**Issue**: terraphim-ai#3167

## Context

Hermes Agent supports a **runtime plugin model**: third parties ship
Python packages with a `hermes-plugin` entry-point, and Hermes discovers
them at startup. This enables the Hermes ecosystem to grow without
modifying core.

TinyClaw currently supports a **compile-time feature-flag model**:
plugins are gated behind `Cargo.toml` features (`telegram`, `slack`,
`discord`, etc.). This is more conservative but less extensible.

## Decision

**TinyClaw retains the compile-time feature-flag model for now.**
Runtime plugin discovery via `inventory` / `libloading` / entry-points is
**explicitly deferred** to a future wave (Wave 7+).

## Rationale

1. **Maturity**: tinyclaw is pre-1.0 and has <10 plugins. Compile-time
   gates are simpler and catch plugin-related compile errors at build
   time, not at first use.
2. **Build budget**: every new dep (`inventory` adds 30+ transitive
   crates; `libloading` is platform-specific) inflates the build graph.
   Our fleet standard §1.1 (`kache`) helps, but it's not free.
3. **Type safety**: Rust plugins via dynamic loading would require
   `unsafe` + `libc::dlsym` plumbing. Not worth it for tinyclaw's
   current plugin count.
4. **Hermes parity not blocking**: the Hermes parity arc doesn't
   require runtime plugin discovery; it's a Hermes implementation
   detail.

## Rejected Alternatives

### Runtime discovery via `inventory` crate

- **Pros**: Zero `unsafe`, automatic registration via static linking.
- **Cons**: Forces static plugin registration in core; harder to test
  plugins in isolation; only Rust crate plugins (not third-party).
- **Verdict**: Rejected. Solves a problem we don't have yet.

### Runtime discovery via `libloading`

- **Pros**: True dynamic loading (.so / .dll).
- **Cons**: `unsafe` API surface; platform-specific (Linux/macOS/Windows
  build matrices); ABI stability concerns; deployment complexity.
- **Verdict**: Rejected. Overkill for tinyclaw's plugin count.

### Runtime discovery via Cargo `[features]` + workspace inheritance

- **Pros**: Already in use. No new deps. Zero `unsafe`.
- **Cons**: Plugins must be in the workspace (no external plugins).
- **Verdict**: **Accepted as current model.** This is what we ship.

## Consequences

- **Positive**: Simple, type-safe, deterministic build graph.
- **Positive**: Compile errors surface at build time, not runtime.
- **Negative**: New plugins require recompiling the whole binary.
- **Negative**: Plugin authors must commit to the tinyclaw repo (or use
  git deps). No third-party plugin marketplace.

## Future Trigger

When tinyclaw reaches **20+ plugins** OR **3+ external contributors**
OR a user requests a specific feature that requires runtime plugins,
revisit this decision. The path forward is:

1. Add `inventory` as direct dep (one-time).
2. Define `pub trait Plugin` for static discovery.
3. Refactor one feature-flag-gated module (e.g., `channels::telegram`)
   to `impl Plugin for TelegramPlugin`.
4. Verify the build graph and feature interactions.
5. Migrate one module per release until feature flags are optional.

## References

- Hermes `plugins/model-providers/` — runtime plugin reference
- `crates/terraphim_tinyclaw/Cargo.toml` — current feature-gated deps
- `terraphim/terraphim-ai/.docs/adr-0008-plugin-model.md` (this file)
- Wave 6 spec: gitea/terraphim-ai#3167