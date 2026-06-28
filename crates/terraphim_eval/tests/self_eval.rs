//! Live integration tests: invoke real `cargo clippy`/`cargo test` against a
//! tiny fixture crate.
//!
//! These are `#[ignore]` by default because they spawn cargo subprocesses
//! (which is slow and would violate the "no recursive cargo" convention if
//! run during normal `cargo test --workspace`). Run them explicitly with:
//!
//! ```sh
//! cargo test -p terraphim_eval --test self_eval -- --ignored
//! ```
//!
//! They prove the end-to-end executor path works against a real toolchain.

use std::path::PathBuf;

use terraphim_eval::{PassFail, run_clippy, run_test};

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join("mini_crate")
}

#[test]
#[ignore = "spawns cargo subprocess; run with --ignored"]
fn run_clippy_on_clean_fixture_returns_pass() {
    let record = run_clippy(fixture_dir()).expect("clippy should run");
    assert_eq!(record.metric_id, "clippy");
    assert_eq!(record.tool, "cargo-clippy");
    // Fixture is clippy-clean under default lints.
    assert_eq!(record.pass_fail, PassFail::Pass);
    assert_eq!(record.counts.errors, 0);
}

#[test]
#[ignore = "spawns cargo subprocess; run with --ignored"]
fn run_test_on_passing_fixture_returns_pass() {
    let record = run_test(fixture_dir()).expect("test should run");
    assert_eq!(record.metric_id, "test");
    assert_eq!(record.tool, "cargo-test");
    assert_eq!(record.pass_fail, PassFail::Pass);
    assert!(record.counts.passed >= 1, "fixture has one passing test");
    assert_eq!(record.counts.failed, 0);
}
