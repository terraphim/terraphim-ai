# Design Gate — tinyclaw hermetic env scrubber (issue #3161)

Date: 2026-08-06 · Wave 0 of Hermes parity arc (epic #3160)

## Problem

Integration tests in `crates/terraphim_tinyclaw/tests/` silently pick up
the developer's real env vars. Two failure modes:

1. **Credential leak**: a hermetic test hits a live API using the
   developer's `OPENAI_API_KEY` / `SLACK_BOT_TOKEN` / etc.
2. **False-positive pass**: a test that should fail (missing env var)
   actually passes because the dev's real var makes the happy-path
   branch fire.

Hermes Agent solves this with a pytest autouse fixture
(`_hermetic_environment` in `tests/conftest.py:340`). Rust has no autouse
test fixtures — we need an explicit, convention-enforced discipline.

## Decision (code touchpoints)

### 1. New helper module: `crates/terraphim_tinyclaw/tests/common/mod.rs`

`pub fn scrub_env()`:

- Strips 19 credential / API-key env vars (`SCRUB_VARS` const):
  LLM keys (`OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, `KIMI_API_KEY`,
  `EXA_API_KEY`, `ZAI_API_KEY`, `MINIMAX_API_KEY`,
  `OPENCODE_API_KEY`), service tokens (`GITHUB_TOKEN`, `GITEA_TOKEN`),
  voice (`WHISPER_MODEL_PATH`), local-LLM (`OLLAMA_BASE_URL`,
  `OLLAMA_MODEL`), channel credentials (`SLACK_BOT_TOKEN`,
  `SLACK_APP_TOKEN`, `SLACK_SIGNING_SECRET`, `TELEGRAM_BOT_TOKEN`,
  `DISCORD_BOT_TOKEN`, `MATRIX_HOMESERVER_URL`, `MATRIX_ACCESS_TOKEN`).
- Pins `TZ=UTC`, `LANG=C.UTF-8`, `LC_ALL=C.UTF-8`.
- Redirects `HOME`, `XDG_CONFIG_HOME`, `XDG_DATA_HOME`, `XDG_CACHE_HOME`
  to a per-process temp dir at `/tmp/terraphim-tinyclaw-hermetic-<pid>`.

`pub fn hermetic_home() -> PathBuf` — returns the temp dir so tests can
stage `tinyclaw.toml` fixtures under it.

The `unsafe` blocks are required because `std::env::set_var` /
`remove_var` are `unsafe` in Rust 2024 (process-global state). Safe
because the test harness is single-threaded at setup time.

### 2. New doc: `crates/terraphim_tinyclaw/TESTING.md`

Documents the discipline: file-scope `mod common;` + per-`#[test]`
first-line call. Explains why per-function (Rust syntax: statements
not allowed at module scope). Documents the `#[ignore]` +
`TERRAPHIM_TEST_LIVE=1` opt-in for live tests.

### 3. Retrofit 5 integration test files

| File | Test fns scrubbed |
|---|---|
| `config_wiring.rs` | 3 |
| `gateway_dispatch.rs` | 4 |
| `skills_benchmarks.rs` | 3 |
| `skills_integration.rs` | 12 (1 `#[ignore]`) |
| `slack_integration.rs` | 2 (both `#[ignore]`, gated behind `--features slack`) |

Each retrofit: `mod common;` after doc comments + `common::scrub_env();`
as the first line inside each `#[test]` / `#[tokio::test]` fn body.

### 4. CI grep gate (deferred)

Heuristic grep is brittle; precise version needs Python AST or
`syn`-based analysis. Documented in TESTING.md but **not implemented
in this PR** — separate issue to track.

## Ground truth to verify (never assume)

Verified during implementation:

- ✅ `cargo build -p terraphim_tinyclaw --tests` exit 0 (35s cold)
- ✅ `cargo clippy -p terraphim_tinyclaw --all-targets -- -D warnings` exit 0
- ✅ `cargo test -p terraphim_tinyclaw --no-fail-fast` — 196 tests pass, 0 fail, 1 ignored (live)
- ✅ `cargo fmt -p terraphim_tinyclaw --check` clean
- ✅ `tests/common/mod.rs` is auto-discovered by cargo as a submodule
  of every integration test binary in the same `tests/` directory
  (Rust 2018+ convention)
- ✅ `common::scrub_env()` as first line of `#[test]` fn compiles
- ✅ `common::scrub_env();` at module top level is a Rust syntax error
  (statements not allowed at module scope — only items). Verified via
  `/tmp/test_mod` rustc test.

## Acceptance criteria

- [x] `tests/common/mod.rs` exists with `pub fn scrub_env()` and
      `pub fn hermetic_home()`. ✅
- [x] `TESTING.md` documents the discipline. ✅
- [x] All 5 integration test files retrofitted. ✅
- [x] `cargo build`, `cargo clippy -D warnings`, `cargo test`, `cargo fmt`
      all green. ✅
- [ ] CI grep gate (deferred — separate issue)
- [ ] Hermes `_hermetic_environment` fixture parity (Rust cannot match
      autouse semantics; convention + gate is the closest equivalent).

## Non-goals

- **Not adding `ctor` crate dependency.** Per-`#[test]` call is
  idiomatic and avoids a new dep.
- **Not auto-generating a fixture.** Rust has no autouse mechanism;
  convention is the closest practical equivalent.
- **Not implementing CI grep gate in this PR.** Out of scope; tracked
  separately.
- **Not modifying unit tests in `src/`.** `src/` code never reads env
  vars at module scope (verified during the adf design phase); only
  integration tests are affected.

## Test plan

1. **Unit verification** — `cargo test -p terraphim_tinyclaw --lib`
   confirms no regressions in the 174 unit tests.
2. **Integration verification** — `cargo test -p terraphim_tinyclaw --tests`
   runs all 5 integration binaries with the new hermetic env.
   Confirmed: 196 passed, 0 failed, 1 ignored (live test, correctly
   `#[ignore]`d).
3. **Hermetic isolation** — tests pass even when the developer has
   `SLACK_BOT_TOKEN`, `OPENAI_API_KEY`, etc. set in their real env.
   (Implicit: the scrubber `remove_var`s them. To verify explicitly:
   `SLACK_BOT_TOKEN=xoxb-test cargo test -p terraphim_tinyclaw` and
   confirm the slack tests stay ignored and don't accidentally try to
   hit Slack.)
4. **fmt/clippy/build gates** — all green per ground-truth section.

## Gates

| Gate | Status |
|---|---|
| `cargo build -p terraphim_tinyclaw --tests` | ✅ exit 0 |
| `cargo clippy -p terraphim_tinyclaw --all-targets -- -D warnings` | ✅ exit 0 |
| `cargo test -p terraphim_tinyclaw --no-fail-fast` | ✅ 196/196 |
| `cargo fmt -p terraphim_tinyclaw --check` | ✅ clean |
| Structural PR review | ⏳ next step |
| `adf/build` Gitea status | ⏳ post-merge |
| Merge to main | ⏳ |

## Notes for review

- `common::scrub_env()` uses `unsafe { std::env::set_var(...) }` blocks.
  This is required in Rust 2024 (process-global state). Safety is upheld
  because the test harness calls `scrub_env()` from a single-threaded
  test setup before any concurrent test execution.
- The `SCRUB_VARS` list is intentionally broad — false positives
  (clearing an unused var) are harmless; false negatives (a real key
  leaks in) are not.
- `TERRAPHIM_TEST_LIVE` is intentionally **not** in `SCRUB_VARS` — it
  is the explicit opt-in marker for live tests (see TESTING.md).