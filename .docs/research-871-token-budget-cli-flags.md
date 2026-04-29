# Research Document: Token Budget CLI Flags and Per-Document Truncation Indicators (Issue #871)

## 1. Problem Restatement and Scope

### Problem
The robot-mode CLI surface for token budget management is incomplete. The underlying infrastructure (`RobotConfig`, `FieldMode`, `BudgetEngine`, truncation logic) is fully implemented and tested internally, but no CLI flags exist to control it. Two locations in `main.rs` hardcode `max_content_length(2000)` and `max_tokens(8000)` when `--robot` is active, and `FieldMode` is never wired to any CLI argument.

Additionally, the spec (F1.3) requires per-document truncation indicators (`body_truncated: true`, `body_original_length: 15000`) on individual result items. Currently only a response-level `truncated` flag exists on `TokenBudget`. The `SearchResultItem` struct lacks these fields.

### IN Scope
- Adding `--max-tokens`, `--max-results`, `--max-content-length`, `--fields` clap flags to robot-mode search
- Removing hardcoded `2000`/`8000` values in `main.rs`
- Adding `body_truncated` / `body_original_length` to `SearchResultItem`
- Wiring `BudgetEngine::apply()` into the search pipeline (replaces inline manual truncation)
- Removing `#[allow(dead_code)]` from robot module where it becomes unnecessary
- Integration and unit tests for new CLI flags and truncation indicators

### OUT of Scope
- Changes to table/interactive-mode output (spec explicitly excludes)
- REPL integration (separate issue #844)
- New dependencies
- Changes to `OutputFormat` or existing serialisation for non-robot mode

## 2. User and Business Outcomes

| Stakeholder | Outcome |
|---|---|
| LLM-based agents (AI callers) | Can precisely size responses to fit context windows via `--max-tokens` and `--max-content-length` |
| AI callers needing minimal data | Can select `--fields minimal` to reduce response payload to title/url/score |
| Debugging/introspection users | Per-document `body_truncated` indicator shows exactly which items were cut and by how much |
| CI/automation pipelines | `--max-results N` provides deterministic output size for scripted consumption |
| Developers maintaining robot module | Dead-code annotations (`#[allow(dead_code)]`) removed; `BudgetEngine` activated in production code path |

## 3. System Elements and Dependencies

| Element | Location | Role | Dependencies |
|---|---|---|---|
| `RobotConfig` | `robot/output.rs:100-113` | Configuration struct with all budget fields | Used by `RobotFormatter`, `BudgetEngine` |
| `FieldMode` | `robot/output.rs:63-73` | Enum: Full/Summary/Minimal/Custom | `from_str_loose()` parser at line 77 |
| `BudgetEngine` | `robot/budget.rs:19-121` | Applies truncation, field filtering, result limiting, token budget | `RobotConfig`, `SearchResultItem` |
| `SearchResultItem` | `robot/schema.rs:299` | Individual search result struct | Needs 2 new optional fields |
| `TokenBudget` | `robot/schema.rs:164` | Response-level token budget metadata | `BudgetEngine` populates this |
| `RobotFormatter` | `robot/output.rs:193-233` | Content truncation with word-boundary awareness | `RobotConfig` |
| `Cli` struct | `main.rs:675-696` | Top-level clap CLI definition | Has `--robot` flag already |
| Search handler (offline) | `main.rs:2074-2100` | Offline search path, hardcoded config | Primary change target |
| Search handler (server) | `main.rs:4053-4099` | Server/hybrid search path, hardcoded config | Primary change target |
| `robot/mod.rs` | `robot/mod.rs` | Module root with `#[allow(dead_code)]` annotations | Cleanup target |
| Spec F1.3 | `docs/specifications/terraphim-agent-session-search-spec.md:108-131` | Requirements definition | Source of truth |
| Tests | `tests/phase1_robot_mode_tests.rs`, `tests/robot_search_output_regression_tests.rs` | Existing test coverage | Extend with new tests |

### Critical Dependency: BudgetEngine vs. Inline Truncation

The current search handlers manually truncate results inline rather than using `BudgetEngine::apply()`. The design must decide whether to:
1. **Replace** inline truncation with `BudgetEngine::apply()` (activates dead code, cleaner)
2. **Extend** inline truncation with CLI flag values (minimal change, leaves BudgetEngine unused)

Option 1 is strongly preferred -- it resolves the dead-code cluster and provides a single code path.

## 4. Constraints and Their Implications

| Constraint | Why It Matters | Implication |
|---|---|---|
| No public-API breakage | Robot mode is used by automation | All new fields must be `Option<T>` with `#[serde(skip_serializing_if = "Option::is_none")]` |
| Backward-compatible defaults | Existing `--robot` users expect current behaviour | When flags absent, defaults should match current hardcoded values (2000 content, 8000 tokens) OR be unlimited (spec says "sensible defaults") |
| No new dependencies | Cargo.toml must not change | Use existing `clap` (already a dependency), `serde_json` for field filtering |
| Robot mode only | Non-robot output must not change | Flags only apply when `--robot` is active |
| `BudgetEngine` requires `SearchResultItem` | Field filtering serialises to JSON then filters keys | Must work with the new optional fields |
| Clippy clean | `cargo clippy -p terraphim_agent -- -D warnings` must pass | Removing `#[allow(dead_code)]` must not produce warnings |

### Backward Compatibility Decision

**Default values when `--robot` is set but budget flags are not provided:**
- Current hardcoded: `max_content_length=2000`, `max_tokens=8000`
- Issue suggests: "sensible defaults (unlimited when flag absent)"
- Risk: Existing automation may depend on current 2000/8000 truncation
- Recommendation: Keep 2000/8000 as defaults for `--robot` to avoid breaking existing callers, but allow explicit override

## 5. Risks, Unknowns, and Assumptions

### Assumptions
- [ASSUMPTION] The `BudgetEngine::apply()` interface is compatible with the search pipeline's result format. It returns `BudgetedResults` which contains `Vec<SearchResultItem>` -- needs verification.
- [ASSUMPTION] The `from_str_loose()` parser for `FieldMode` is sufficient for CLI use (it defaults unknown strings to `Full` -- should probably error instead for CLI).
- [ASSUMPTION] The two search handler locations (offline and server) are the only places that hardcode robot config.
- [ASSUMPTION] Existing integration tests in `robot_search_output_regression_tests.rs` will continue to pass since they don't assert on specific truncation values.

### Unknowns
- Does `BudgetEngine::apply()` produce `SearchResultItem` with `preview_truncated` already set, or does it need modification to also set `body_truncated`/`body_original_length`?
- Are there other callers of the search handlers (REPL, service) that also hardcode robot config?
- Does the `--limit` flag (already exists for search) conflict with `--max-results` for robot mode?

### Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Changing defaults breaks existing automation | Medium | High | Keep current defaults, allow explicit override |
| `BudgetEngine` interface mismatch with search pipeline | Low | Medium | Verify in research phase; adapter layer if needed |
| `from_str_loose()` silently accepts invalid FieldMode | Medium | Low | Add clap `value_parser` that validates and returns error |
| Per-document fields bloat response for non-truncated items | Low | Low | Use `skip_serializing_if = "Option::is_none"` |
| Removing `#[allow(dead_code)]` exposes other unused items | Low | Medium | Incremental removal; verify clippy after each step |

## 6. Context Complexity vs. Simplicity Opportunities

### Sources of Complexity
1. **Two search handler locations** with nearly identical hardcoded config (offline at 2074, server at 4053) -- duplication
2. **Two separate truncation mechanisms**: inline manual loop AND `BudgetEngine` (unused) -- dual paths
3. **Two response truncation levels**: response-level `TokenBudget.truncated` and per-document `body_truncated` (to be added) -- conceptual overlap

### Simplification Strategies
1. **Extract a `build_robot_config()` helper** that reads CLI flags and constructs `RobotConfig` once, used by both search handlers. Eliminates duplication.
2. **Replace inline truncation with `BudgetEngine::apply()`** in both handlers. Single code path, activates dead code, easier to test.
3. **Unify truncation indicators**: The response-level `TokenBudget.truncated` stays as a summary; per-document `body_truncated` is additive, not redundant.

## 7. Questions for Human Reviewer

1. **Default values**: Should `--robot` without explicit budget flags use the current hardcoded 2000/8000, or switch to unlimited? (Recommendation: keep 2000/8000 for backward compatibility)
2. **`--max-results` vs `--limit`**: The search command already has `--limit`. Should `--max-results` be a robot-mode-specific alias, or should `--limit` be reused? (Recommendation: use `--limit` for robot mode too, no new flag needed)
3. **FieldMode validation**: Should invalid `--fields` values produce an error or silently default to `Full`? (Recommendation: error via clap `value_parser`)
4. **`body_truncated` field naming**: The existing field is `preview_truncated`. Should the new field be `body_truncated` (as per spec) or `content_truncated` for consistency with `max_content_length`? (Recommendation: `body_truncated` to match spec F1.3 exactly)
5. **Scope of dead-code cleanup**: Remove all `#[allow(dead_code)]` from robot module, or only those that become unnecessary with this change? (Recommendation: only those that become unnecessary -- surgical)
