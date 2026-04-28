//! Integration tests for robot-mode JSON output on non-search commands.
//!
//! Mirrors the regression-test style in `robot_search_output_regression_tests.rs`
//! but targets the commands addressed by issue #1013:
//!   - roles list
//!   - config show
//!   - graph
//!
//! Each test shells out to the built `terraphim-agent` binary and asserts that
//! stdout is a parseable `RobotResponse<T>` envelope.

use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

use anyhow::{Context, Result};
use serde_json::Value;
use serial_test::serial;

static AGENT_BINARY: OnceLock<Result<PathBuf, String>> = OnceLock::new();

fn agent_binary() -> Result<PathBuf> {
    AGENT_BINARY
        .get_or_init(|| {
            let status = Command::new("cargo")
                .args([
                    "build",
                    "-p",
                    "terraphim_agent",
                    "--bin",
                    "terraphim-agent",
                    "--quiet",
                ])
                .status()
                .map_err(|e| format!("failed to spawn cargo build: {}", e))?;
            if !status.success() {
                return Err(format!("cargo build failed with status {}", status));
            }
            let manifest = std::env::var("CARGO_MANIFEST_DIR")
                .map_err(|_| "CARGO_MANIFEST_DIR not set".to_string())?;
            let bin = PathBuf::from(manifest)
                .parent()
                .and_then(|p| p.parent())
                .ok_or("could not derive workspace root from CARGO_MANIFEST_DIR")?
                .join("target/debug/terraphim-agent");
            Ok(bin)
        })
        .clone()
        .map_err(anyhow::Error::msg)
}

fn run_agent_command(args: &[&str]) -> Result<(String, String, i32)> {
    let bin = agent_binary().context("agent binary build failed")?;
    let output = Command::new(&bin)
        .args(args)
        .output()
        .context("failed to execute terraphim-agent command")?;

    Ok((
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
        output.status.code().unwrap_or(-1),
    ))
}

fn parse_first_json_object(stdout: &str) -> Result<Value> {
    let start = stdout
        .find('{')
        .context("stdout does not contain a JSON object start '{'")?;

    let mut stream = serde_json::Deserializer::from_str(&stdout[start..]).into_iter::<Value>();
    stream
        .next()
        .transpose()?
        .context("failed to parse first JSON value from stdout")
}

fn assert_robot_envelope(value: &Value, expected_command: &str) {
    let root = value.as_object().expect("envelope should be a JSON object");

    assert!(
        root.get("success")
            .and_then(Value::as_bool)
            .expect("envelope should contain bool field 'success'"),
        "response should report success=true"
    );

    let meta = root
        .get("meta")
        .and_then(Value::as_object)
        .expect("envelope should contain object field 'meta'");
    let cmd = meta
        .get("command")
        .and_then(Value::as_str)
        .expect("meta should contain string field 'command'");
    assert_eq!(
        cmd, expected_command,
        "meta.command should echo the subcommand"
    );
    assert!(
        meta.get("timestamp").and_then(Value::as_str).is_some(),
        "meta should contain string field 'timestamp'"
    );
    assert!(
        meta.get("version").and_then(Value::as_str).is_some(),
        "meta should contain string field 'version'"
    );

    assert!(
        root.get("data").is_some(),
        "envelope should contain field 'data'"
    );
}

const FIXTURE_CONFIG: &str = "tests/test_config.json";

#[test]
#[serial]
fn roles_list_format_json_emits_parseable_envelope() -> Result<()> {
    let (stdout, stderr, code) = run_agent_command(&[
        "--config",
        FIXTURE_CONFIG,
        "--format",
        "json",
        "roles",
        "list",
    ])?;

    assert_eq!(
        code, 0,
        "roles list in --format json mode should succeed; stderr={}",
        stderr
    );

    let json = parse_first_json_object(&stdout)?;
    assert_robot_envelope(&json, "roles list");

    let data = json
        .get("data")
        .and_then(Value::as_object)
        .expect("envelope should contain object field 'data'");

    let roles = data
        .get("roles")
        .and_then(Value::as_array)
        .expect("data should contain array field 'roles'");
    assert!(!roles.is_empty(), "roles array should not be empty");

    let selected = data
        .get("selected")
        .and_then(Value::as_str)
        .expect("data should contain string field 'selected'");
    assert!(!selected.is_empty(), "selected role should not be empty");

    for role in roles {
        let obj = role.as_object().expect("each role should be an object");
        let name = obj
            .get("name")
            .and_then(Value::as_str)
            .expect("role should contain string field 'name'");
        assert!(!name.is_empty(), "role 'name' should not be empty");

        let selected_flag = obj
            .get("selected")
            .and_then(Value::as_bool)
            .expect("role should contain bool field 'selected'");
        assert!(
            selected_flag == (name == selected),
            "role 'selected' should match the top-level selected field"
        );
    }

    Ok(())
}

#[test]
#[serial]
fn config_show_format_json_emits_parseable_envelope() -> Result<()> {
    let (stdout, stderr, code) = run_agent_command(&[
        "--config",
        FIXTURE_CONFIG,
        "--format",
        "json",
        "config",
        "show",
    ])?;

    assert_eq!(
        code, 0,
        "config show in --format json mode should succeed; stderr={}",
        stderr
    );

    let json = parse_first_json_object(&stdout)?;
    assert_robot_envelope(&json, "config show");

    let data = json
        .get("data")
        .and_then(Value::as_object)
        .expect("envelope should contain object field 'data'");

    assert!(
        data.get("config").is_some(),
        "data should contain field 'config'"
    );

    Ok(())
}

#[test]
#[serial]
fn graph_format_json_emits_parseable_envelope() -> Result<()> {
    let (stdout, stderr, code) =
        run_agent_command(&["--config", FIXTURE_CONFIG, "--format", "json", "graph"])?;

    assert_eq!(
        code, 0,
        "graph in --format json mode should succeed; stderr={}",
        stderr
    );

    let json = parse_first_json_object(&stdout)?;
    assert_robot_envelope(&json, "graph");

    let data = json
        .get("data")
        .and_then(Value::as_object)
        .expect("envelope should contain object field 'data'");

    let role = data
        .get("role")
        .and_then(Value::as_str)
        .expect("data should contain string field 'role'");
    assert!(!role.is_empty(), "graph 'role' should not be empty");

    assert!(
        data.get("concepts").and_then(Value::as_array).is_some(),
        "data should contain array field 'concepts'"
    );

    Ok(())
}

#[test]
#[serial]
fn graph_format_json_with_role_flag_emits_parseable_envelope() -> Result<()> {
    let (stdout, stderr, code) = run_agent_command(&[
        "--config",
        FIXTURE_CONFIG,
        "--format",
        "json",
        "graph",
        "--role",
        "Test Engineer",
    ])?;

    assert_eq!(
        code, 0,
        "graph with --role in --format json mode should succeed; stderr={}",
        stderr
    );

    let json = parse_first_json_object(&stdout)?;
    assert_robot_envelope(&json, "graph");

    let data = json
        .get("data")
        .and_then(Value::as_object)
        .expect("envelope should contain object field 'data'");

    let role = data
        .get("role")
        .and_then(Value::as_str)
        .expect("data should contain string field 'role'");
    assert_eq!(role, "Test Engineer", "graph should use the requested role");

    Ok(())
}
