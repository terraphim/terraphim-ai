use anyhow::{Context, Result};
use serde_json::Value;
use std::process::Command;

mod support;
use support::cli_test_env::apply_hermetic_env;

fn run_agent(args: &[&str]) -> Result<(String, String, i32)> {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_terraphim-agent"));
    cmd.args(args);
    apply_hermetic_env(&mut cmd)?;

    let output = cmd.output().context("run terraphim-agent")?;

    Ok((
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
        output.status.code().unwrap_or(-1),
    ))
}

#[test]
fn global_json_compact_guard_emits_single_line_json() -> Result<()> {
    let (stdout, stderr, code) =
        run_agent(&["--format", "json-compact", "guard", "git reset --hard HEAD"])?;

    assert_eq!(
        code, 0,
        "machine-readable guard should preserve --json compatibility; stderr={stderr}"
    );
    assert!(
        !stderr.contains("BLOCKED:"),
        "human BLOCKED text must not be emitted for global JSON format: {stderr}"
    );

    let trimmed = stdout.trim();
    assert!(!trimmed.is_empty(), "stdout should contain JSON");
    assert_eq!(
        trimmed.lines().count(),
        1,
        "json-compact should be single-line"
    );
    let json: Value = serde_json::from_str(trimmed)?;
    assert_eq!(json["decision"], "block");
    assert_eq!(json["command"], "git reset --hard HEAD");
    Ok(())
}

#[test]
fn global_json_validate_emits_parseable_json() -> Result<()> {
    let (stdout, stderr, code) = run_agent(&["--format", "json", "validate", "terraphim"])?;

    assert_eq!(code, 0, "validate should succeed; stderr={stderr}");
    let json: Value = serde_json::from_str(stdout.trim())?;
    assert!(
        json.get("matched_count").is_some() || json.get("error").is_some(),
        "unexpected validate JSON: {json}"
    );
    Ok(())
}

#[test]
fn global_json_guard_emits_pretty_json() -> Result<()> {
    let (stdout, stderr, code) =
        run_agent(&["--format", "json", "guard", "git reset --hard HEAD"])?;

    assert_eq!(
        code, 0,
        "global --format json guard should succeed; stderr={stderr}"
    );
    assert!(
        !stderr.contains("BLOCKED:"),
        "human BLOCKED text must not be emitted for global JSON format: {stderr}"
    );
    let json = assert_pretty_json(&stdout)?;
    assert_eq!(json["decision"], "block");
    assert_eq!(json["command"], "git reset --hard HEAD");
    Ok(())
}

#[test]
fn global_json_compact_suggest_emits_parseable_json() -> Result<()> {
    let (stdout, stderr, code) = run_agent(&[
        "--format",
        "json-compact",
        "suggest",
        "terraphim",
        "--limit",
        "3",
    ])?;

    assert_eq!(code, 0, "suggest should succeed; stderr={stderr}");
    assert_single_line_json(&stdout)?;
    Ok(())
}

#[test]
fn legacy_guard_json_stays_single_line_compact() -> Result<()> {
    let (stdout, stderr, code) = run_agent(&["guard", "--json", "git reset --hard HEAD"])?;

    assert_eq!(
        code, 0,
        "legacy guard --json compatibility; stderr={stderr}"
    );
    assert!(
        !stderr.contains("BLOCKED:"),
        "legacy JSON guard should not emit human BLOCKED text: {stderr}"
    );
    let json = assert_single_line_json(&stdout)?;
    assert_eq!(json["decision"], "block");
    Ok(())
}

#[test]
fn legacy_validate_json_stays_single_line_compact() -> Result<()> {
    let (stdout, stderr, code) = run_agent(&["validate", "--json", "terraphim"])?;

    assert_eq!(
        code, 0,
        "legacy validate --json compatibility; stderr={stderr}"
    );
    let json = assert_single_line_json(&stdout)?;
    assert!(
        json.get("matched_count").is_some() || json.get("error").is_some(),
        "unexpected validate JSON: {json}"
    );
    Ok(())
}

#[test]
fn legacy_suggest_json_stays_single_line_compact() -> Result<()> {
    let (stdout, stderr, code) = run_agent(&["suggest", "--json", "terraphim", "--limit", "3"])?;

    assert_eq!(
        code, 0,
        "legacy suggest --json compatibility; stderr={stderr}"
    );
    assert_single_line_json(&stdout)?;
    Ok(())
}

#[test]
fn server_mode_global_json_compact_guard_emits_single_line_json() -> Result<()> {
    let (stdout, stderr, code) = run_agent(&[
        "--server",
        "--format",
        "json-compact",
        "guard",
        "git reset --hard HEAD",
    ])?;

    assert_eq!(
        code, 0,
        "machine-readable server-mode guard should preserve --json compatibility; stderr={stderr}"
    );
    assert!(
        !stderr.contains("BLOCKED:"),
        "human BLOCKED text must not be emitted for server-mode global JSON format: {stderr}"
    );
    let json = assert_single_line_json(&stdout)?;
    assert_eq!(json["decision"], "block");
    Ok(())
}

#[test]
fn server_mode_global_json_validate_error_is_parseable_json() -> Result<()> {
    let (stdout, stderr, code) = run_agent(&[
        "--server",
        "--format",
        "json-compact",
        "validate",
        "terraphim",
    ])?;

    assert_eq!(code, 1, "server-mode validate should remain unavailable");
    assert!(
        !stderr.contains("Validate command is only available in offline mode"),
        "machine-readable unavailable error must be stdout JSON, not human stderr: {stderr}"
    );
    let json = assert_single_line_json(&stdout)?;
    assert_eq!(
        json["error"],
        "Validate command is only available in offline mode"
    );
    Ok(())
}

#[test]
fn server_mode_global_json_compact_suggest_error_is_parseable_json() -> Result<()> {
    let (stdout, stderr, code) = run_agent(&[
        "--server",
        "--format",
        "json-compact",
        "suggest",
        "terraphim",
        "--limit",
        "3",
    ])?;

    assert_eq!(code, 1, "server-mode suggest should remain unavailable");
    assert!(
        !stderr.contains("Suggest command is only available in offline mode"),
        "machine-readable unavailable error must be stdout JSON, not human stderr: {stderr}"
    );
    let json = assert_single_line_json(&stdout)?;
    assert_eq!(
        json["error"],
        "Suggest command is only available in offline mode"
    );
    Ok(())
}

fn assert_single_line_json(stdout: &str) -> Result<Value> {
    let trimmed = stdout.trim();
    assert!(!trimmed.is_empty(), "stdout should contain JSON");
    assert_eq!(
        trimmed.lines().count(),
        1,
        "json-compact should be single-line"
    );
    Ok(serde_json::from_str(trimmed)?)
}

fn assert_pretty_json(stdout: &str) -> Result<Value> {
    let trimmed = stdout.trim();
    assert!(!trimmed.is_empty(), "stdout should contain JSON");
    assert!(
        trimmed.lines().count() > 1,
        "--format json should be pretty multi-line JSON; stdout={trimmed}"
    );
    Ok(serde_json::from_str(trimmed)?)
}
