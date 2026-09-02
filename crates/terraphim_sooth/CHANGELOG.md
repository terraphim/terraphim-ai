# Changelog

## 1.21.3 (2026-09-02)

Initial release to the internal `terraphim` Gitea cargo registry.

- Deterministic Rust port of the Sooth predictor (Jason Hutchens,
  Unlicense): `observe`, `count`, `select` (seeded weighted draw via the
  injected `rand_core::Rng`), `select_limit` (positional, mirroring the
  upstream C cumulative scan), `surprise`, `uncertainty`.
- BTreeMap-only state; no OS entropy; serde round-trip; wasm32 green.
- Ruby-gem golden fixtures (scripts/generate_fixtures.rb).
