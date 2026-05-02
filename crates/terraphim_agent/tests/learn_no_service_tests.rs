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

#[test]
fn learn_correction_roundtrip() {
    let binary = agent_binary();
    let tmp = tempfile::tempdir().expect("create temp dir");

    // Pre-create .terraphim/learnings/ so storage_location() uses the project dir,
    // not the global fallback at ~/.local/share/terraphim/learnings/.
    let learnings_dir = tmp.path().join(".terraphim").join("learnings");
    std::fs::create_dir_all(&learnings_dir).expect("create .terraphim/learnings");

    // Step 1: capture a learning so there is an entry to correct.
    let capture_output = Command::new(&binary)
        .args([
            "learn",
            "capture",
            "npm-install",
            "--error",
            "command not found: npm",
            "--exit-code",
            "127",
        ])
        .current_dir(tmp.path())
        .output()
        .expect("should execute learn capture");

    assert!(
        capture_output.status.success(),
        "learn capture must succeed.\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&capture_output.stdout),
        String::from_utf8_lossy(&capture_output.stderr),
    );

    // Step 2: parse the file path from "Captured learning: <path>" and extract the ID.
    let capture_stdout = String::from_utf8_lossy(&capture_output.stdout);
    let file_path = capture_stdout
        .lines()
        .find(|l| l.starts_with("Captured learning:"))
        .and_then(|l| l.strip_prefix("Captured learning: "))
        .expect("capture output must contain 'Captured learning: <path>'")
        .trim()
        .to_string();

    let md_content =
        std::fs::read_to_string(&file_path).expect("captured learning file must exist");

    let learning_id = md_content
        .lines()
        .find(|l| l.starts_with("id: "))
        .and_then(|l| l.strip_prefix("id: "))
        .expect("learning file must have an 'id:' frontmatter field")
        .trim()
        .to_string();

    // Step 3: add a correction using the extracted ID.
    let correct_output = Command::new(&binary)
        .args([
            "learn",
            "correct",
            &learning_id,
            "--correction",
            "use bun install instead",
        ])
        .current_dir(tmp.path())
        .output()
        .expect("should execute learn correct");

    assert!(
        correct_output.status.success(),
        "learn correct must succeed.\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&correct_output.stdout),
        String::from_utf8_lossy(&correct_output.stderr),
    );

    // Step 4: verify the correction appears in learn list.
    let list_output = Command::new(&binary)
        .args(["learn", "list"])
        .current_dir(tmp.path())
        .output()
        .expect("should execute learn list");

    assert!(
        list_output.status.success(),
        "learn list must succeed after correction.\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&list_output.stdout),
        String::from_utf8_lossy(&list_output.stderr),
    );

    let list_stdout = String::from_utf8_lossy(&list_output.stdout);
    assert!(
        list_stdout.contains("use bun install instead"),
        "learn list output must show the correction text.\nstdout: {}",
        list_stdout,
    );
}
