//! Regression tests for #542: learn commands must not require TuiService.
//!
//! These tests run `terraphim-agent learn {list,hook,capture}` from a temporary
//! directory to prove they no longer crash due to CWD-relative KG path resolution
//! or TuiService initialization failures.

use std::process::Command;

// Use the already-compiled binary rather than spawning a nested `cargo build`.
// `CARGO_BIN_EXE_terraphim-agent` is set by Cargo for integration tests so no
// separate build step is needed and there is no contention on the cargo file lock.
fn agent_binary() -> &'static str {
    env!("CARGO_BIN_EXE_terraphim-agent")
}

#[test]
fn learn_list_succeeds_from_tmp_dir() {
    let binary = agent_binary();
    let tmp = tempfile::tempdir().expect("create temp dir");

    let output = Command::new(binary)
        .args(["learn", "list"])
        .current_dir(tmp.path())
        .output()
        .expect("failed to run terraphim-agent learn list");

    assert!(
        output.status.success(),
        "learn list should succeed from temp dir.\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

#[test]
fn learn_hook_succeeds_from_tmp_dir() {
    let binary = agent_binary();
    let tmp = tempfile::tempdir().expect("create temp dir");

    let mut child = Command::new(binary)
        .args(["learn", "hook"])
        .current_dir(tmp.path())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("failed to spawn terraphim-agent learn hook");

    // Close stdin immediately to signal EOF
    drop(child.stdin.take());

    let output = child
        .wait_with_output()
        .expect("failed to wait on terraphim-agent learn hook");

    // Hook with empty input may return non-zero (invalid JSON), but it must NOT
    // crash with a panic or TuiService initialization error.
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("TuiService") && !stderr.contains("panicked"),
        "learn hook must not fail due to TuiService init.\nstderr: {}",
        stderr,
    );
}

#[test]
fn learn_capture_succeeds_from_tmp_dir() {
    let binary = agent_binary();
    let tmp = tempfile::tempdir().expect("create temp dir");

    let output = Command::new(binary)
        .args([
            "learn",
            "capture",
            "fake-cmd",
            "--error",
            "something went wrong",
            "--exit-code",
            "1",
        ])
        .current_dir(tmp.path())
        .output()
        .expect("failed to run terraphim-agent learn capture");

    assert!(
        output.status.success(),
        "learn capture should succeed from temp dir.\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

#[test]
fn learn_correction_succeeds_from_tmp_dir() {
    let binary = agent_binary();
    let tmp = tempfile::tempdir().expect("create temp dir");

    let output = Command::new(binary)
        .args([
            "learn",
            "correction",
            "--original",
            "agent-suggestion",
            "--corrected",
            "user-fix",
            "--correction-type",
            "naming",
            "--context",
            "tmp-dir-test",
        ])
        .current_dir(tmp.path())
        .output()
        .expect("failed to run terraphim-agent learn correction");

    assert!(
        output.status.success(),
        "learn correction should succeed from temp dir.\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}
