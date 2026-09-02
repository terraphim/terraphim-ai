//! Hermetic test environment for `terraphim_tinyclaw` integration tests.
//!
//! Integration tests in `crates/terraphim_tinyclaw/tests/*.rs` MUST do
//! the following, in order, at the top of each file (after doc comments):
//!
//! 1. Declare this module: `mod common;` (file-scope `mod` declaration)
//!
//! And the **first executable line inside every `#[test]` / `#[tokio::test]`
//! function** MUST be:
//!
//! ```ignore
//! common::scrub_env();
//! ```
//!
//! Rationale: Rust has no autouse fixtures. Calling `scrub_env()` at
//! module top level is a syntax error (statements are not allowed at
//! module scope; only items like `use`, `mod`, `fn`, `struct` are).
//! Hence the per-`#[test]` call. The discipline is enforced by
//! convention + a CI grep gate (see `TESTING.md`). A test that omits
//! the call will silently pick up the developer's real env vars and
//! either hit a live API or pass when it should fail.
//!
//! What `scrub_env()` does:
//! 1. Strips credential / API-key env vars so live credentials cannot leak
//!    into a test that should be hermetic.
//! 2. Pins `TZ` and `LANG` so time-zone and locale-dependent code paths
//!    are deterministic.
//! 3. Redirects home-relative config resolution (`env_home::env_home_dir`,
//!    `~/.config/...`) to a per-process temp dir so tests cannot pick up
//!    the developer's real `~/.config/terraphim/tinyclaw.toml`.
//!
//! ## Why this exists
//!
//! Hermes Agent's test suite uses a `_hermetic_environment` fixture in
//! `tests/conftest.py:340` that runs for every test. The Rust equivalent
//! can't be automatic, so we require the explicit first-line call.

#![allow(dead_code)] // many symbols are used by individual tests, not all of them

use std::path::PathBuf;

/// Env vars that must be removed before integration tests run so they
/// cannot affect behaviour that should be hermetic.
///
/// Pattern: anything that looks like a credential, an API key, or a
/// service URL. The list is intentionally comprehensive — false positives
/// (clearing an unused var) are harmless; false negatives (a real key
/// leaks in) are not.
const SCRUB_VARS: &[&str] = &[
    // LLM / provider keys
    "EXA_API_KEY",
    "KIMI_API_KEY",
    "MINIMAX_API_KEY",
    "ZAI_API_KEY",
    "OPENAI_API_KEY",
    "ANTHROPIC_API_KEY",
    "OPENCODE_API_KEY",
    "GITHUB_TOKEN",
    "GITEA_TOKEN",
    // Voice
    "WHISPER_MODEL_PATH",
    // Skills / local
    "OLLAMA_BASE_URL",
    "OLLAMA_MODEL",
    // Slack / channel credentials
    "SLACK_BOT_TOKEN",
    "SLACK_APP_TOKEN",
    "SLACK_SIGNING_SECRET",
    "TELEGRAM_BOT_TOKEN",
    "DISCORD_BOT_TOKEN",
    "MATRIX_HOMESERVER_URL",
    "MATRIX_ACCESS_TOKEN",
    // Opt-in marker for live tests (NOT scrubbed, but documented)
    // "TERRAPHIM_TEST_LIVE" — see TESTING.md
];

/// Force UTC for the duration of the test process so that any time-zone
/// dependent code paths (rollover, locale, etc.) are deterministic.
const PIN_TZ: &str = "UTC";
const PIN_LANG: &str = "C.UTF-8";

/// Install a hermetic environment for integration tests in this process.
///
/// Calling this more than once in the same process is a no-op for the env
/// (scrub is idempotent, TZ/LANG are re-set to the same values) but the
/// temp-dir setup runs fresh every call.
pub fn scrub_env() {
    // 1. Scrub credentials / API keys.
    for var in SCRUB_VARS {
        // std::env::remove_var is `unsafe` in Rust 2024 (process-global state).
        // The test harness calls this from a single-threaded test setup, so
        // the safety invariants hold. Wrap in unsafe block explicitly.
        unsafe { std::env::remove_var(var) };
    }

    // 2. Pin TZ/LANG for deterministic time-zone / locale behaviour.
    unsafe {
        std::env::set_var("TZ", PIN_TZ);
        std::env::set_var("LANG", PIN_LANG);
        std::env::set_var("LC_ALL", PIN_LANG);
    }

    // 3. Redirect home-relative config resolution to a per-process temp dir.
    let tmp = hermetic_home();
    unsafe {
        std::env::set_var("HOME", &tmp);
        std::env::set_var("XDG_CONFIG_HOME", tmp.join(".config"));
        std::env::set_var("XDG_DATA_HOME", tmp.join(".local/share"));
        std::env::set_var("XDG_CACHE_HOME", tmp.join(".cache"));
    }
}

/// The hermetic temp dir for the current process. Useful for tests that
/// need to provide a `tinyclaw.toml` fixture — they can write it under
/// this dir and the rest of the world will pick it up via `env_home`.
///
/// Returned path is created on call (idempotent).
pub fn hermetic_home() -> PathBuf {
    let tmp = std::env::temp_dir().join(format!(
        "terraphim-tinyclaw-hermetic-{}",
        std::process::id()
    ));
    let _ = std::fs::create_dir_all(&tmp);
    tmp
}
