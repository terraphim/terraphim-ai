# Research: Fix LegacyTerm deserialization in terraphim_automata

**Date**: 2026-04-03
**Author**: AI Agent
**Status**: DRAFT

**Related**: PR #753 (CI clippy failure), PR #229 (Gitea PR #229)

**Design Reference**: `docs/plans/u64-reversion-research.md`, `docs/plans/u64-reversion-design.md`

**Root Cause**: Commit `9c8dd28f` partially removed `nterm`, `display_value`, and `url` fields from `LegacyTerm`, causing compilation errors when these fields is accessed (line 392-394).

 It The if `term.id` is used, `term.url`, or the will use `term.id`, instead of a u64, because `term.id` is now `u64`, is the the old serde patterns). Access still works because but commit missed these the `NormalizedTerm.id` → `term.id` (u64) assignment) pattern. This conflicts with removing both fields AND accessing pattern.

 The code also uses `NormalizedTerm.id` directly in `parse_thesaurus_json` (line 380-398) and on `term.id` directly (line 392)), or the `term.display_value.unwrap_or_else(|| term.ntterm.clone())` `1` else `None`.
- **JSON fixtures with integer IDs**: Deserialized correctly. 
- **3 clippy warnings** (`clone_on_copy` on u64 IDs): CI uses `-D warnings`, causing build failure.
- **Test suite**: The `test_load_thesaurus_from_json` integration test exercises, verify JSON deserialization with both formats.
  ** `tests/autocomplete_tests.rs`: 11 test assertions use `NormalizedTerm::new(1, ...)` (expect u64). All pass.
  - `tests/paragraph_extraction_tests.rs`: 1 assertion for `NormalizedTerm::new("foo", NormalizedTermValue::from("foo"))` (expect String). Fails.

  - Test fixtures use pure integer IDss. All pass.
- **Migration scope minimal** - TheLegacyTerm` struct is local to `parse_thesaurus_json` with a hidden function. The `id` field is used solely to gate old serde legacy format. The code that uses `term.id` also now reads the term's fields via `.ntterm` instead of accessing.

 No existing code reads them through `.ntterm`, (the serialized field name). The only option is to reconstruct the legacyTerm's data from the term.nterm` and `term.id`.

3 else (reading via the existing serde),...
  }

    - **`id` is `u64` breakss the serde's Deserialize annotations to this change is minimal and risk, possible.
    - **Option B**: Use `#[serde(deny)]` or a `LegacyTerm` struct to skip the `id` field entirely. This removes the unused deserialized data but reduces struct size, clippy, and still validate JSON deserialization.

 But - **Option C**: Revert `term.nterm`, field but `String`. This makes sense only if there's a legacy-format data with `nterm` as String. }
    - **Option D**: Keep `nterm` as `String` (for backward compat with legacy thesaurus JSON files). This preserves backward compatibility with the data from the JSON file that uses the `term.nterm` field.
 Since serde's deserialize, the `term.url` field becomes `Option<String>` instead of `u64`. This maps to the `NormalizedTerm::with_url` parameter, the new signature.
    - **Option E**: Keep `term.nterm` as `String` but `NormalizedTermValue` instead of `u64`:
      - `term.nterm` is `NormalizedTermValue::from_string)`. This preserves backward compatibility.
    - Add `#[serde(default)]` to `term: Term` struct for optional `url` field.
      This `serde(default)` will deserialize the Option<String> as `None`, which would default to `None`. This makes clippy pass without losing the `id` field.

    - **Option F**: This is the minimal change with lowest risk.
    - **Option G**: This is the larger change that reconstructs `LegacyTerm` properly but restoring all fields needed for serde JSON format. This would fix the compilation error while preserving backward compatibility with legacy thesaurus JSON files.

    - **Option H**: Moderate risk - needs to find and update all existing JSON fixture that use `term.nterm` as String. Alternatively of `u64`, to verify the serde handle)` correctly.
    - **Option I**: Low risk - but **Option M**: Low risk - Easy to test, add tests in CI to confirm the fix
 **Additional context**:
- The `parse_thesaurus_json` integration test exercises verify JSON deserialization with both formats. No other callsers of the change.
  - `tests/autocomplete_tests.rs`: All 11 assertions now check for u64 instead of String
  - `tests/paragraph_extraction_tests.rs` line 1: 1 assertion checks for `NormalizedTerm::new("foo", NormalizedTermValue::from("foo"))` fails because it expects a `String`, not `u64`:
  - `test_load_thesaurus_from_json`: verify JSON parsing of both formats
- All fixture files with `.json` extension use pure integer IDs (no `"1", `"2", etc.)

  - Fixture `test-fixtures/*.json`: already uses pure integers (verified OK)

  - **CI clippy** check: CI uses `-D warnings` locally but `cargo clippy --workspace -- -D warnings 2>&1 | tail -5` to confirm

  - **Test suite**: `cargo test --package terraphim_automata` (36 tests) and `cargo test --workspace --lib` (300+ tests)
  - **Format check**: `cargo fmt --all -- --check` passes
  - All 4 checks steps must pass
  - Verify final state with test run after any changes
  - **Merge**: Squash merge after all checks pass
  - **Rebase/squash** if needed to keep clean git history
