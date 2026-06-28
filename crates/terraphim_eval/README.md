# terraphim_eval

Codebase evaluation **metrics runner**: executes `cargo clippy` and
`cargo test`, parses their machine-readable JSON output, and emits
normalised [`MetricRecord`][records]s for downstream comparison.

[records]: https://terraphim.ai

Implements the smallest actionable slice of

## Parsers

The pure parsing logic is exposed separately so it can be unit-tested without
spawning subprocesses:

- [`parse_clippy_json`][crate::parse_clippy_json] — counts `warning`/`error`
  diagnostics from `cargo clippy --message-format=json`.
- [`parse_test_json`][crate::parse_test_json] — counts `ok`/`failed`/`ignored`
  events from `cargo test --message-format=json`.

## Tests

- **Unit tests** (`cargo test -p terraphim_eval`): parse canned JSON fixtures,
  no subprocess, run in milliseconds.
- **Live integration tests** (`cargo test -p terraphim_eval -- --ignored`):
  invoke real cargo against `fixtures/mini_crate/`. Opt-in to honour the
  "no recursive cargo" convention during normal workspace tests.
