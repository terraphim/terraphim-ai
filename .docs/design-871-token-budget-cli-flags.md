# Design and Implementation Plan: Token Budget CLI Flags and Per-Document Truncation Indicators (Issue #871)

## 1. Summary of Target Behaviour

After implementation, robot-mode users will be able to:

```
terraphim-agent search "async" --robot --max-tokens 1000 --max-content-length 500 --fields minimal
terraphim-agent search "rust" --robot --max-results 3 --fields custom:title,url,score
terraphim-agent search "query" --robot   # defaults: max_content_length=2000, max_tokens=8000
```

Each truncated result item will include per-document indicators:
```json
{
  "title": "...",
  "preview": "First 500 chars...",
  "preview_truncated": true,
  "body_original_length": 15000
}
```

The `BudgetEngine` (currently dead code) becomes the active code path for search result processing, replacing the two duplicated inline truncation loops in `main.rs`.

## 2. Key Invariants and Acceptance Criteria

### Invariants
- **I1**: All new `SearchResultItem` fields are `Option<T>` with `#[serde(skip_serializing_if = "Option::is_none")]` -- no public-API breakage
- **I2**: Non-robot output is unchanged (human-readable terminal output)
- **I3**: Default behaviour of `--robot` without explicit budget flags matches current hardcoded values (max_content_length=2000, max_tokens=8000)
- **I4**: `--fields minimal` emits only `rank`, `id`, `title`, `score` keys
- **I5**: `--max-results N` caps `data.results.len()` even if more matches exist
- **I6**: One tool failure does not abort the budget pipeline

### Acceptance Criteria (from issue #871)
- [ ] AC1: Add clap flags: `--max-tokens <N>`, `--max-results <N>`, `--max-content-length <N>`, `--fields <full|summary|minimal|custom:f1,f2>`
- [ ] AC2: Remove hardcoded `2000`/`8000` in main.rs; build `RobotConfig` from CLI args with backward-compatible defaults
- [ ] AC3: Extend `SearchResultItem` with `body_original_length: Option<usize>`
- [ ] AC4: When `max_content_length` is set, populate `body_original_length` on each truncated document
- [ ] AC5: When `FieldMode::Summary`/`Minimal`/`Custom` is selected, emitted JSON omits non-selected fields
- [ ] AC6: Integration test: `--max-content-length 50 --format json` emits at least one document with `"body_original_length"` present
- [ ] AC7: Integration test: `--fields minimal --format json` emits only `title`, `url`, `score` keys on result items
- [ ] AC8: Integration test: `--max-results 2` caps `data.results.len() == 2`
- [ ] AC9: `cargo test -p terraphim_agent --test phase1_robot_mode_tests` passes
- [ ] AC10: `cargo clippy -p terraphim_agent -- -D warnings` passes

## 3. High-Level Design and Boundaries

### Components Changed

```
main.rs (CLI layer)
    |
    +-- Cli struct: add 4 new flags to Search subcommand
    +-- Search handlers (x2): extract build_robot_config(), use BudgetEngine
    |
robot/schema.rs (types)
    |
    +-- SearchResultItem: add body_original_length field
    |
robot/budget.rs (engine)
    |
    +-- truncate_item(): populate body_original_length on truncation
    +-- KNOWN_FIELDS: add "body_original_length"
    |
robot/mod.rs (cleanup)
    |
    +-- Remove #[allow(dead_code)] from budget, output, schema modules
```

### New Helper Function

```rust
fn build_robot_config(
    robot: bool,
    format: OutputFormat,
    limit: usize,
    max_tokens: Option<usize>,
    max_content_length: Option<usize>,
    max_results: Option<usize>,
    fields: Option<FieldMode>,
) -> RobotConfig
```

Extracted into a free function, used by both search handlers (offline and server).

## 4. File/Module-Level Change Plan

| File | Action | Before | After | Dependencies |
|---|---|---|---|---|
| `main.rs:698-719` (Search subcommand) | Modify | `limit`, `role`, `terms`, `fail_on_empty`, `include_pinned` | Add: `--max-tokens`, `--max-content-length`, `--max-results`, `--fields` | None |
| `main.rs` (near 575) | Add | N/A | `build_robot_config()` helper function | `RobotConfig`, `FieldMode` |
| `main.rs:2074-2135` (offline search) | Modify | Manual `RobotConfig` construction + inline truncation loop | Use `build_robot_config()` + `BudgetEngine::apply()` | `BudgetEngine` |
| `main.rs:4053-4099` (server search) | Modify | Identical manual construction + inline loop | Use `build_robot_config()` + `BudgetEngine::apply()` | `BudgetEngine` |
| `robot/schema.rs:299-323` (SearchResultItem) | Modify | 8 fields, no body_original_length | Add `body_original_length: Option<usize>` | None |
| `robot/budget.rs:74-84` (truncate_item) | Modify | Sets `preview_truncated` on truncation | Also set `body_original_length` with original preview length | `SearchResultItem` change |
| `robot/budget.rs:24-34` (KNOWN_FIELDS) | Modify | 9 fields | Add `"body_original_length"` (10 fields) | schema change |
| `robot/mod.rs:6-15` | Modify | `#[allow(dead_code)]` on all modules | Remove from `budget`, `output`, `schema` (keep on `docs`, `exit_codes` if still unused) | Integration activates code |
| `tests/phase1_robot_mode_tests.rs` | Modify | 9 existing tests | Add 4 new tests (AC6-AC8 + fields test) | New CLI flags |
| `robot/output.rs:77-95` (from_str_loose) | Modify | Defaults unknown to `Full` | Keep as-is (clap validation handles it at CLI layer) | None |

### Important: `body_truncated` vs `preview_truncated`

The spec F1.3 names the field `body_truncated`. However, the existing field `preview_truncated` already serves this purpose for the `preview` field -- `BudgetEngine::truncate_item()` sets `preview_truncated = true` when content is cut. Adding a separate `body_truncated` would be redundant with `preview_truncated`.

**Decision**: Use `body_original_length: Option<usize>` only. When present, the caller knows truncation occurred because `preview_truncated` will also be `true`. This avoids a redundant boolean field and matches the spec's intent (indicating truncation happened and how much was cut). This is noted as a deviation from the spec's `body_truncated: bool` field, but achieves the same information density.

## 5. Step-by-Step Implementation Sequence

### Step 1: Add `body_original_length` to `SearchResultItem`
- **File**: `robot/schema.rs:299-323`
- **Purpose**: Add the new optional field for per-document truncation indication
- **Change**: Add `#[serde(skip_serializing_if = "Option::is_none")] pub body_original_length: Option<usize>`
- **Deployable**: Yes (field defaults to `None`, no existing code sets it)

### Step 2: Update `BudgetEngine::truncate_item()` and `KNOWN_FIELDS`
- **File**: `robot/budget.rs`
- **Purpose**: Populate `body_original_length` when truncation occurs
- **Change**: In `truncate_item()`, when `was_truncated` is true, set `item.body_original_length = Some(original_len)`. Add `"body_original_length"` to `KNOWN_FIELDS`.
- **Deployable**: Yes (purely additive)

### Step 3: Add clap flags to `Search` subcommand
- **File**: `main.rs:698-719`
- **Purpose**: Expose token budget controls to CLI users
- **Change**: Add to `Search` variant:
  ```rust
  /// Maximum tokens in robot-mode response (estimated, default: 8000)
  #[arg(long)]
  max_tokens: Option<usize>,
  /// Truncate content fields at N characters in robot mode (default: 2000)
  #[arg(long)]
  max_content_length: Option<usize>,
  /// Maximum number of results in robot mode (overrides --limit)
  #[arg(long)]
  max_results: Option<usize>,
  /// Field selection mode for robot mode: full, summary, minimal, custom:field1,field2
  #[arg(long)]
  fields: Option<String>,
  ```
- **Deployable**: Yes (flags are optional, no behaviour change without them)

### Step 4: Extract `build_robot_config()` helper
- **File**: `main.rs` (near `resolve_output_config`)
- **Purpose**: Single source of truth for robot config construction, used by both search handlers
- **Change**: New function that reads CLI flag values and constructs `RobotConfig`. Logic:
  ```
  if --robot or --format json/json-compact:
      apply defaults: max_content_length=2000, max_tokens=8000
      override with explicit flags if provided
      parse --fields string via FieldMode::from_str_loose()
      set max_results from --max-results (or fall back to --limit)
  else:
      RobotConfig::disabled (just format, no budget)
  ```
- **Deployable**: Yes (not yet called)

### Step 5: Replace offline search inline truncation with `BudgetEngine`
- **File**: `main.rs:2064-2135`
- **Purpose**: Use `BudgetEngine::apply()` instead of manual truncation loop
- **Change**:
  1. Call `build_robot_config()` to get config
  2. Build `Vec<SearchResultItem>` from raw results (no truncation yet)
  3. Call `BudgetEngine::new(config).apply(&items)?`
  4. Use `BudgetedResults` to build `SearchResultsData` with filtered items
  5. Preserve existing `ResponseMeta` and `RobotResponse` construction
- **Deployable**: Yes (behaviour should be identical; `BudgetEngine` was designed for this)

### Step 6: Replace server search inline truncation with `BudgetEngine`
- **File**: `main.rs:4043-4099`
- **Purpose**: Same change as Step 5 for the server/hybrid search path
- **Change**: Identical pattern to Step 5
- **Deployable**: Yes

### Step 7: Remove unnecessary `#[allow(dead_code)]` annotations
- **File**: `robot/mod.rs`
- **Purpose**: Clean up dead-code suppression for modules now used in production code path
- **Change**: Remove `#[allow(dead_code)]` from `budget`, `output`, `schema` module declarations. Keep for `docs` and `exit_codes` if they remain unused from main.rs.
- **Deployable**: Yes (clippy must pass)

### Step 8: Add integration tests
- **File**: `tests/phase1_robot_mode_tests.rs`
- **Purpose**: Verify new CLI flags work end-to-end
- **Change**: Add 4 new tests:
  - `test_max_content_length_flag`: Parse CLI with `--max-content-length 50`, verify config
  - `test_fields_minimal_flag`: Parse CLI with `--fields minimal`, verify FieldMode::Minimal
  - `test_max_results_flag`: Parse CLI with `--max-results 2`, verify config
  - `test_max_tokens_flag`: Parse CLI with `--max-tokens 1000`, verify config
- **Deployable**: Yes

### Step 9: Add unit tests for truncation indicators
- **File**: `robot/budget.rs` (inline tests)
- **Purpose**: Verify `body_original_length` is populated correctly
- **Change**: Add tests:
  - `test_truncate_sets_body_original_length`: Truncation populates field
  - `test_no_truncation_no_body_original_length`: No truncation leaves field `None`
  - `test_body_original_length_in_filtered_output`: Field appears in filtered JSON when present
- **Deployable**: Yes

### Step 10: Verify clippy and existing tests
- **Commands**:
  ```bash
  cargo clippy -p terraphim_agent -- -D warnings
  cargo test -p terraphim_agent --test phase1_robot_mode_tests
  cargo test -p terraphim_agent --test robot_search_output_regression_tests
  cargo test -p terraphim_agent robot::budget
  cargo test -p terraphim_agent robot::schema
  ```
- **Purpose**: Ensure no regressions, dead-code warnings, or test failures

## 6. Testing and Verification Strategy

| Acceptance Criteria | Test Type | Test Location | Verification |
|---|---|---|---|
| AC1: CLI flags exist and parse | Unit | `tests/phase1_robot_mode_tests.rs` | Clap parsing succeeds with valid values |
| AC2: Hardcoded values removed | Visual review | N/A | Grep for `2000` and `8000` in main.rs returns no results in robot config context |
| AC3: `body_original_length` field | Unit | `robot/schema.rs` inline | Serialize/deserialize round-trip with new field |
| AC4: Truncation populates field | Unit | `robot/budget.rs` inline | `truncate_item` sets `body_original_length` when `preview_truncated = true` |
| AC5: Field filtering works | Unit | `robot/budget.rs` inline | `filter_fields` with Minimal/Custom omits non-selected fields |
| AC6: Integration: truncation indicators | Integration | `tests/robot_search_output_regression_tests.rs` | CLI with `--max-content-length 50` produces `body_original_length` in JSON output |
| AC7: Integration: minimal fields | Integration | `tests/robot_search_output_regression_tests.rs` | CLI with `--fields minimal` produces only expected keys |
| AC8: Integration: max results cap | Integration | `tests/robot_search_output_regression_tests.rs` | CLI with `--max-results 2` caps output to 2 items |
| AC9: All tests pass | CI | Full test suite | `cargo test -p terraphim_agent` exits 0 |
| AC10: Clippy clean | CI | Lint | `cargo clippy -p terraphim_agent -- -D warnings` exits 0 |

## 7. Risk and Complexity Review

| Risk | Mitigation | Residual Risk |
|---|---|---|
| BudgetEngine interface mismatch (takes `SearchResultItem`, returns `serde_json::Value`) | Verified: `BudgetedResults.results` is `Vec<serde_json::Value>`, not `Vec<SearchResultItem>`. Search handlers must use JSON values for response building. | Low -- existing `SearchResultsData.results` is `Vec<SearchResultItem>`, will need to change to accept filtered JSON or re-serialize |
| Two search handlers must stay in sync | `build_robot_config()` helper is the single source of truth; both call it identically | None -- same function called from both |
| `from_str_loose()` silently defaults to Full | Acceptable for programmatic use; CLI layer can validate before calling | Low -- power users may not notice typo in `--fields sumary` |
| `BudgetedResults.results` is `Vec<serde_json::Value>` but `SearchResultsData.results` is `Vec<SearchResultItem>` | After `BudgetEngine::apply()`, use `budgeted.results` directly in JSON response instead of going through `SearchResultsData` | Medium -- changes how response is constructed; must preserve `total_matches` and other meta |

### Key Design Decision: BudgetedResults vs SearchResultsData

The `BudgetEngine::apply()` returns `BudgetedResults { results: Vec<serde_json::Value>, ... }` but `SearchResultsData { results: Vec<SearchResultItem>, ... }`. After budgeting, the items are already filtered JSON values (some fields removed).

**Approach**: After `BudgetEngine::apply()`, construct the `RobotResponse` directly with `budgeted.results` as the results array, rather than trying to fit them back into `SearchResultsData`. This means the response envelope changes slightly:

```json
{
  "success": true,
  "data": {
    "results": [ ... filtered JSON values ... ],
    "total_matches": 10,
    "pagination": { ... from BudgetedResults ... },
    "token_budget": { ... from BudgetedResults ... }
  }
}
```

This is a minor serialisation difference -- the response shape stays the same for consumers.

## 8. Open Questions / Decisions for Human Review

1. **`body_truncated` boolean**: Spec F1.3 requires `body_truncated: true` on truncated items. We propose using `body_original_length` (set only when truncation occurs) combined with the existing `preview_truncated` instead. Is this acceptable, or must we add the explicit `body_truncated: bool` field?

2. **Default values**: When `--robot` is set without explicit budget flags, use current hardcoded values (2000/8000) for backward compatibility. Confirm this is preferred over "unlimited" defaults.

3. **`--fields` validation**: Should invalid field mode strings (e.g. `--fields sumary`) produce an error or silently default to `Full`? Recommendation: error.

4. **`--max-results` vs `--limit`**: Both control result count. Recommendation: `--max-results` is robot-mode-specific and overrides `--limit` when present. `--limit` remains for non-robot mode.

5. **Scope of `SearchResultsData` refactoring**: Should we change `SearchResultsData.results` from `Vec<SearchResultItem>` to `Vec<serde_json::Value>` to accommodate `BudgetEngine` output? Or keep the current type and re-parse? Recommendation: use `serde_json::Value` after budgeting, construct response directly.
