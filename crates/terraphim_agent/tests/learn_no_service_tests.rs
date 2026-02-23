//! Regression tests for #542: learn commands must not require TuiService.
//!
//! These tests run `terraphim-agent learn {list,hook,capture}` from a temporary
//! directory to prove they no longer crash due to CWD-relative KG path resolution
//! or TuiService initialization failures.

use std::process::Command;

fn agent_binary() -> String {
    // Use cargo to locate the binary built by the workspace
    let output = Command::new("cargo")
        .args(["build", "-p", "terraphim_agent"])
        .output()
        .expect("cargo build should succeed");
    if !output.status.success() {
        panic!(
            "cargo build failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // The binary is at target/debug/terraphim-agent
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    workspace_root
        .join("target/debug/terraphim-agent")
        .to_string_lossy()
        .to_string()
}

#[test]
fn learn_list_succeeds_from_tmp_dir() {
    let binary = agent_binary();
    let tmp = tempfile::tempdir().expect("create temp dir");

    let output = Command::new(&binary)
        .args(["learn", "list"])
        .current_dir(tmp.path())
        .output()
        .expect("should execute learn list");

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

    // Hook reads JSON from stdin; feed it empty input so it finishes quickly.
    let mut child = Command::new(&binary)
        .args(["learn", "hook"])
        .current_dir(tmp.path())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("should spawn learn hook");

    // Close stdin immediately to signal EOF
    drop(child.stdin.take());

    let output = child
        .wait_with_output()
        .expect("should wait for learn hook");

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

    let output = Command::new(&binary)
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
        .expect("should execute learn capture");

    assert!(
        output.status.success(),
        "learn capture should succeed from temp dir.\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}
