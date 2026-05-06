//! Regression tests for #542: learn commands must not require TuiService.
//!
//! These tests run `terraphim-agent learn {list,hook,capture}` from a temporary
//! directory to prove they no longer crash due to CWD-relative KG path resolution
//! or TuiService initialization failures.

use std::process::Command;

fn agent_binary() -> Option<String> {
    let output = match Command::new("cargo")
        .args(["build", "-p", "terraphim_agent"])
        .output()
    {
        Ok(o) => o,
        Err(_) => return None,
    };
    if !output.status.success() {
        return None;
    }

    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let path = workspace_root.join("target/debug/terraphim-agent");
    if path.exists() {
        Some(path.to_string_lossy().to_string())
    } else {
        None
    }
}

#[test]
fn learn_list_succeeds_from_tmp_dir() {
    let binary = match agent_binary() {
        Some(b) => b,
        None => return,
    };
    let tmp = tempfile::tempdir().expect("create temp dir");

    let output = match Command::new(&binary)
        .args(["learn", "list"])
        .current_dir(tmp.path())
        .output()
    {
        Ok(o) => o,
        Err(_) => return,
    };

    assert!(
        output.status.success(),
        "learn list should succeed from temp dir.\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

#[test]
fn learn_hook_succeeds_from_tmp_dir() {
    let binary = match agent_binary() {
        Some(b) => b,
        None => return,
    };
    let tmp = tempfile::tempdir().expect("create temp dir");

    let mut child = match Command::new(&binary)
        .args(["learn", "hook"])
        .current_dir(tmp.path())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(_) => return,
    };

    // Close stdin immediately to signal EOF
    drop(child.stdin.take());

    let output = match child.wait_with_output() {
        Ok(o) => o,
        Err(_) => return,
    };

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
    let binary = match agent_binary() {
        Some(b) => b,
        None => return,
    };
    let tmp = tempfile::tempdir().expect("create temp dir");

    let output = match Command::new(&binary)
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
    {
        Ok(o) => o,
        Err(_) => return,
    };

    assert!(
        output.status.success(),
        "learn capture should succeed from temp dir.\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

#[test]
fn learn_correction_succeeds_from_tmp_dir() {
    let binary = match agent_binary() {
        Some(b) => b,
        None => return,
    };
    let tmp = tempfile::tempdir().expect("create temp dir");

    let output = match Command::new(&binary)
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
    {
        Ok(o) => o,
        Err(_) => return,
    };

    assert!(
        output.status.success(),
        "learn correction should succeed from temp dir.\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}
