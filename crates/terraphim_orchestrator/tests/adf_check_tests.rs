//! End-to-end tests for `adf --check <config>` dry-run validation.
//! Invokes the compiled binary and asserts on exit code + stdout.

use std::path::PathBuf;
use std::process::Command;

fn adf_bin() -> PathBuf {
    // `CARGO_BIN_EXE_adf` is set by cargo for the adf integration test target.
    PathBuf::from(env!("CARGO_BIN_EXE_adf"))
}

fn fixture(name: &str) -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests");
    p.push("fixtures");
    p.push("multi_project");
    p.push(name);
    p
}

#[test]
fn adf_check_succeeds_on_valid_inline_config() {
    let out = Command::new(adf_bin())
        .arg("--check")
        .arg(fixture("base_inline.toml"))
        .output()
        .expect("run adf --check");

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "expected success, got {out:?}");
    assert!(stdout.contains("PROJECT"), "stdout missing header: {stdout}");
    assert!(
        stdout.contains("alpha-watcher"),
        "stdout missing alpha-watcher: {stdout}"
    );
    assert!(
        stdout.contains("beta-watcher"),
        "stdout missing beta-watcher: {stdout}"
    );
}

#[test]
fn adf_check_expands_include_and_prints_merged_agents() {
    let out = Command::new(adf_bin())
        .arg("--check")
        .arg(fixture("base_include.toml"))
        .output()
        .expect("run adf --check");

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "expected success, got {out:?}");
    // All three agents from the merged fragments must be present.
    assert!(stdout.contains("alpha-watcher"));
    assert!(stdout.contains("beta-watcher"));
    assert!(stdout.contains("beta-reviewer"));
    // Model column shows the subscription-allowed model.
    assert!(stdout.contains("sonnet"));
}

#[test]
fn adf_check_fails_on_banned_provider_with_nonzero_exit() {
    let out = Command::new(adf_bin())
        .arg("--check")
        .arg(fixture("invalid_banned.toml"))
        .output()
        .expect("run adf --check");

    assert!(!out.status.success(), "expected failure, got success");
    let code = out.status.code().unwrap_or_default();
    assert_eq!(code, 1, "expected exit code 1, got {code}");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("FAILED") || stderr.contains("google/gemini-2"),
        "stderr should mention failure or banned provider: {stderr}"
    );
}

#[test]
fn adf_check_fails_on_missing_file() {
    let out = Command::new(adf_bin())
        .arg("--check")
        .arg("/tmp/definitely-does-not-exist-adf-test.toml")
        .output()
        .expect("run adf --check");

    assert!(!out.status.success());
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn adf_check_table_is_sorted_by_project_then_agent() {
    let out = Command::new(adf_bin())
        .arg("--check")
        .arg(fixture("base_include.toml"))
        .output()
        .expect("run adf --check");

    let stdout = String::from_utf8_lossy(&out.stdout);
    let alpha_idx = stdout.find("alpha-watcher").expect("alpha-watcher present");
    let beta_rev_idx = stdout.find("beta-reviewer").expect("beta-reviewer present");
    let beta_watch_idx = stdout.find("beta-watcher").expect("beta-watcher present");

    // alpha project rows first.
    assert!(alpha_idx < beta_rev_idx);
    assert!(alpha_idx < beta_watch_idx);
    // within beta, alphabetical: reviewer before watcher.
    assert!(beta_rev_idx < beta_watch_idx);
}
