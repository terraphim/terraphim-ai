# Changelog

## 1.21.3 (2026-09-02)

Initial release to the internal `terraphim` Gitea cargo registry.

- Faithful port of the MegaHAL engine (megahal.rb + keyword.rb, Unlicense):
  five predictors, dictionary interning, brain context map, learn/reply,
  keyword extraction with Ruby Hash last-wins antonym swap, walks, surprise
  scoring, bounded rewrite.
- Canonical PCG32 RNG contract mirrored in the Ruby oracle driver; golden
  reply fixtures replayed byte-for-byte (tests/conformance.rs).
- `personalities` feature (11 upstream corpora; :default always embedded).
- `persistence` feature (Persistable brain; memory-backend round-trip test).
- `automata` feature (KG keyword seeding via terraphim_automata).
- CLI binary `megahal` with /help menu.
- Divergences documented: MHRS1 JSON brains (not Marshal), CJK heuristic
  instead of CLD, canonicalised RNG.
