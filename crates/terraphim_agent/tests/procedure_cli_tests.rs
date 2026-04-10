//! Integration tests for the procedural memory CLI (`learn procedure` subcommands).
//!
//! These tests exercise the full binary to verify that procedures can be created,
//! steps added, confidence updated, listed, and shown via the CLI.

use std::process::Command;

fn agent_binary() -> String {
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

/// Run a procedure subcommand, returning (stdout, stderr, success).
fn run_procedure_cmd(binary: &str, args: &[&str], env_home: &str) -> (String, String, bool) {
    let mut full_args = vec!["learn", "procedure"];
    full_args.extend_from_slice(args);

    let output = Command::new(binary)
        .args(&full_args)
        // Override HOME so procedures.jsonl is written to a temp dir
        .env("HOME", env_home)
        // Also override XDG_DATA_HOME to control where dirs::data_dir() resolves
        .env("XDG_DATA_HOME", format!("{}/data", env_home))
        .output()
        .expect("should execute procedure command");

    (
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
        output.status.success(),
    )
}

#[test]
fn procedure_list_empty() {
    let binary = agent_binary();
    let tmp = tempfile::tempdir().expect("create temp dir");
    let home = tmp.path().to_string_lossy().to_string();

    let (stdout, _stderr, success) = run_procedure_cmd(&binary, &["list"], &home);
    assert!(success, "list on empty store should succeed");
    assert!(
        stdout.contains("No procedures found"),
        "expected empty message, got: {}",
        stdout
    );
}

#[test]
fn procedure_record_and_show() {
    let binary = agent_binary();
    let tmp = tempfile::tempdir().expect("create temp dir");
    let home = tmp.path().to_string_lossy().to_string();

    // Record a new procedure
    let (stdout, _stderr, success) = run_procedure_cmd(
        &binary,
        &[
            "record",
            "Build Rust project",
            "--description",
            "Steps to build a Rust project from scratch",
        ],
        &home,
    );
    assert!(success, "record should succeed");
    assert!(
        stdout.contains("Created procedure:"),
        "expected creation message, got: {}",
        stdout
    );

    // Extract the procedure ID from output
    let id = stdout
        .trim()
        .strip_prefix("Created procedure: ")
        .expect("should have procedure ID")
        .to_string();

    // Show it
    let (stdout, _stderr, success) = run_procedure_cmd(&binary, &["show", &id], &home);
    assert!(success, "show should succeed");
    assert!(stdout.contains("Build Rust project"), "title in output");
    assert!(
        stdout.contains("Steps to build a Rust project from scratch"),
        "description in output"
    );
    assert!(stdout.contains("Steps (0):"), "zero steps initially");
}

#[test]
fn procedure_add_step_and_list() {
    let binary = agent_binary();
    let tmp = tempfile::tempdir().expect("create temp dir");
    let home = tmp.path().to_string_lossy().to_string();

    // Record
    let (stdout, _, _) = run_procedure_cmd(&binary, &["record", "Deploy app"], &home);
    let id = stdout
        .trim()
        .strip_prefix("Created procedure: ")
        .unwrap()
        .to_string();

    // Add steps
    let (stdout, _, success) = run_procedure_cmd(
        &binary,
        &[
            "add-step",
            &id,
            "cargo build --release",
            "--precondition",
            "Rust toolchain installed",
            "--postcondition",
            "Binary exists in target/release",
        ],
        &home,
    );
    assert!(success, "add-step should succeed");
    assert!(stdout.contains("Added step 1"), "first step added");

    let (stdout, _, success) = run_procedure_cmd(
        &binary,
        &["add-step", &id, "scp target/release/app server:/opt/"],
        &home,
    );
    assert!(success, "second add-step should succeed");
    assert!(stdout.contains("Added step 2"), "second step added");

    // Show with steps
    let (stdout, _, success) = run_procedure_cmd(&binary, &["show", &id], &home);
    assert!(success);
    assert!(stdout.contains("Steps (2):"), "two steps");
    assert!(stdout.contains("cargo build --release"));
    assert!(stdout.contains("pre: Rust toolchain installed"));
    assert!(stdout.contains("post: Binary exists in target/release"));
    assert!(stdout.contains("scp target/release/app server:/opt/"));

    // List
    let (stdout, _, success) = run_procedure_cmd(&binary, &["list"], &home);
    assert!(success);
    assert!(stdout.contains("Deploy app"));
    assert!(stdout.contains("2 steps"));
}

#[test]
fn procedure_success_and_failure_update_confidence() {
    let binary = agent_binary();
    let tmp = tempfile::tempdir().expect("create temp dir");
    let home = tmp.path().to_string_lossy().to_string();

    // Record
    let (stdout, _, _) = run_procedure_cmd(&binary, &["record", "Test procedure"], &home);
    let id = stdout
        .trim()
        .strip_prefix("Created procedure: ")
        .unwrap()
        .to_string();

    // Record successes
    let (_, _, success) = run_procedure_cmd(&binary, &["success", &id], &home);
    assert!(success);
    let (_, _, success) = run_procedure_cmd(&binary, &["success", &id], &home);
    assert!(success);

    // Record a failure
    let (_, _, success) = run_procedure_cmd(&binary, &["failure", &id], &home);
    assert!(success);

    // Show to verify confidence: 2 successes, 1 failure = 67%
    let (stdout, _, success) = run_procedure_cmd(&binary, &["show", &id], &home);
    assert!(success);
    assert!(
        stdout.contains("67%"),
        "expected 67% confidence, got: {}",
        stdout
    );
    assert!(stdout.contains("2 successes"));
    assert!(stdout.contains("1 failures"));
}

#[test]
fn procedure_success_nonexistent_fails() {
    let binary = agent_binary();
    let tmp = tempfile::tempdir().expect("create temp dir");
    let home = tmp.path().to_string_lossy().to_string();

    let (_, _stderr, success) = run_procedure_cmd(&binary, &["success", "nonexistent-id"], &home);
    assert!(!success, "success on nonexistent procedure should fail");
}
