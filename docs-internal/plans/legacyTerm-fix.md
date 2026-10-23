# Research: Fix LegacyTerm deserialisation in parse_thesaurus_json

**Date**: 2026-04-03
**Status**: DRAFT (blocked on CI - clippy with `-D warnings`)

**Related**:** terraphim-automata#187 (u64 ID reversion)
**Impact**: Direct
**Files**:** `terraphim_automata/src/lib.rs`, `terraphim_automata/src/builder.rs`
**Root cause:** Commit `9c8dd28f` (revert ID types from String/UUID to u64 integer) partially removed fields from `LegacyTerm` deserialization struct, leaving only `id: u64`. This broke the `parse_thesaurus_json` function which which references theterm.display_value` and `term.nterm` (line 392) and and `term.display_value` (fallback to `term.nterm`)`,    if display_value is is `None`,  {
      let normalized = NormalizedTerm::with_auto_id(NormalizedTermValue::from(key.as_str()));
      the normalized = NormalizedTerm::with_display_value(term.display_value.unwrap_or_default());
      the normalized = NormalizedTerm::with_url(term.url.unwrap_or_default());
      the normalized = NormalizedTerm::with_url(term.url.unwrap_or_default());
  });
}
```

Now let me load the disciplined design and produce the plan from the research. Let me check the JSON fixtures exist: 4