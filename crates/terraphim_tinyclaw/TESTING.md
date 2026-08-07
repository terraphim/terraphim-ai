# `terraphim_tinyclaw` Integration Test Discipline

## Why

Hermes Agent's test suite uses `_hermetic_environment` (an autouse pytest
fixture in `tests/conftest.py`) that strips credentials and pins timezone
for every test. The Rust equivalent cannot be automatic — Rust has no
test-fixture autouse mechanism — so we enforce hermetic env by **explicit
convention + CI grep gate**.

Without this discipline, integration tests in `crates/terraphim_tinyclaw/tests/`
silently pick up the developer's real env vars. Two failure modes result:

1. **Credential leak**: a test that should be hermetic actually hits a
   live API using the developer's real `OPENAI_API_KEY` / `SLACK_BOT_TOKEN`.
   Costs money. May produce non-deterministic results that pass on the
   dev's machine and fail in CI.
2. **False-positive pass**: a test that should fail (because the env var
   is missing) actually passes because the dev's real var makes the
   happy-path code branch fire.

## What `common::scrub_env()` does

The hermetic helper (`crates/terraphim_tinyclaw/tests/common/mod.rs`):

1. **Strips credential / API-key env vars** before the test runs.
   See `SCRUB_VARS` in `common/mod.rs` for the full list (LLM keys,
   voice model path, local LLM URLs, Slack/Telegram/Discord/Matrix
   tokens, GitHub/Gitea tokens).
2. **Pins `TZ=UTC`, `LANG=C.UTF-8`, `LC_ALL=C.UTF-8`** so time-zone
   and locale-dependent code paths are deterministic.
3. **Redirects `HOME`, `XDG_CONFIG_HOME`, `XDG_DATA_HOME`,
   `XDG_CACHE_HOME`** to a per-process temp dir
   (`/tmp/terraphim-tinyclaw-hermetic-<pid>`). Tests that need to
   provide a `tinyclaw.toml` fixture can write it under this dir
   and the rest of the world will pick it up via `env_home`.

## Required pattern in every `tests/*.rs` file

### File scope (top of file, after doc comments)

```rust
//! <doc comments>
//!
//! <description of what this test file does>

mod common;

use std::...;
use terraphim_tinyclaw::...;
```

`mod common;` declares the shared hermetic helper as a submodule of
the test binary. It must appear before any `use` statement that loads
a config-aware or env-aware module.

### Inside every test function (first executable line)

```rust
#[test]
fn test_xxx() {
    common::scrub_env();
    // ... rest of test body
}

#[tokio::test]
async fn test_yyy() {
    common::scrub_env();
    // ... rest of test body
}
```

The `common::scrub_env();` call MUST be the first statement inside the
function body, before any code that might read an env var or home-relative
config path.

### Why per-function and not per-file?

`common::scrub_env();` at module top level is a **Rust syntax error**:
statements are not allowed at module scope (only items like `use`, `mod`,
`fn`, `struct` are). The Rust compiler emits `expected one of ! or ::,
found (`. Per-function calls are the idiomatic alternative.

## Opt-in live tests (`#[ignore]` + `TERRAPHIM_TEST_LIVE=1`)

Tests that need real credentials (e.g. `slack_integration.rs`,
`test_skill_execution_with_defaults`) are marked `#[ignore]` and
gated behind an explicit opt-in env var:

```bash
TERRAPHIM_TEST_LIVE=1 SLACK_BOT_TOKEN=xoxb-... SLACK_APP_TOKEN=xapp-... \
    cargo test -p terraphim_tinyclaw --features slack \
        --test slack_integration -- --ignored
```

`TERRAPHIM_TEST_LIVE` is **NOT** in `SCRUB_VARS` — it is the explicit
"the developer accepts this test will hit live services" marker.

## CI grep gate

The discipline is enforced by a CI grep gate. Add to
`.gitea/workflows/` or `terraphim-ci.yml` (TBD — see issue #3161 follow-up):

```bash
# Gate 1: every test file declares `mod common;`
for f in crates/terraphim_tinyclaw/tests/*.rs; do
    grep -q "^mod common;" "$f" || {
        echo "ERROR: $f missing 'mod common;' declaration"
        exit 1
    }
done

# Gate 2: every test fn calls scrub_env() as first statement
# (heuristic: every `^fn test_` is followed within 3 lines by `common::scrub_env()`)
# A precise version uses a Python AST pass — TBD.
```

## Adding a new integration test

1. Create `crates/terraphim_tinyclaw/tests/<name>.rs`.
2. Add `//!` doc comments describing what the file tests.
3. Add `mod common;` at the top (after docs).
4. Add `use` statements.
5. Add `#[test]` / `#[tokio::test]` functions. First line of each fn:
   `common::scrub_env();`.
6. If the test needs live credentials, mark `#[ignore]` and document
   the env vars in the file's doc comment.

## References

- `crates/terraphim_tinyclaw/tests/common/mod.rs` — the scrubber
- Hermes Agent equivalent: `_hermetic_environment` in
  `tests/conftest.py:340`
- Issue: Gitea #3161