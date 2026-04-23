//! CLI integration tests for auto-routing (design step 4, tests T8 and T9).
//!
//! These tests shell out to `cargo run -p terraphim_agent` so they exercise the
//! same dispatch path users hit. The fixture config is the existing
//! `crates/terraphim_agent/tests/test_config.json` (a single-role config), which
//! is enough for these assertions because:
//!   - T8 (explicit --role): asserts that stderr is silent on `[auto-route]`,
//!     regardless of how many roles are present.
//!   - T9 (--robot stdout purity): asserts that stdout parses as JSON and
//!     stderr contains exactly one `[auto-route]` line. With one role the
//!     routing decision is trivial, but the prefix must still appear.
//!
//! Tests scrub `RUST_LOG` and `JMAP_ACCESS_TOKEN` so dev-shell variables don't
//! poison stderr matching. Marked `#[serial]` to avoid clobbering the workspace
//! cargo build lock with peer tests in the same crate.

use std::process::Command;

use anyhow::{Context, Result};
use serde_json::Value;
use serial_test::serial;

const FIXTURE_CONFIG: &str = "tests/test_config.json";

fn run_agent(args: &[&str]) -> Result<(String, String, i32)> {
    let output = Command::new("cargo")
        .args(["run", "-p", "terraphim_agent", "--quiet", "--"])
        .args(args)
        .env_remove("RUST_LOG")
        .env_remove("JMAP_ACCESS_TOKEN")
        .output()
        .context("failed to execute terraphim-agent")?;
    Ok((
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
        output.status.code().unwrap_or(-1),
    ))
}

fn count_auto_route_lines(stderr: &str) -> usize {
    stderr
        .lines()
        .filter(|l| l.trim_start().starts_with("[auto-route]"))
        .count()
}

#[test]
#[serial]
fn t8_explicit_role_short_circuits_auto_route() -> Result<()> {
    let (stdout, stderr, code) = run_agent(&[
        "--config",
        FIXTURE_CONFIG,
        "--robot",
        "search",
        "terraphim",
        "--role",
        "Test Engineer",
        "--limit",
        "1",
    ])?;

    assert_eq!(
        code, 0,
        "explicit --role search should succeed; stderr={}",
        stderr
    );
    // The point of the test: no auto-route line on stderr.
    assert_eq!(
        count_auto_route_lines(&stderr),
        0,
        "explicit --role must not emit [auto-route]; stderr={}",
        stderr
    );
    // Sanity: stdout still has the JSON envelope.
    assert!(
        stdout.contains('{'),
        "expected JSON on stdout; stdout={}",
        stdout
    );
    Ok(())
}

#[test]
#[serial]
fn t9_robot_mode_stdout_is_pure_json_stderr_has_auto_route() -> Result<()> {
    let (stdout, stderr, code) = run_agent(&[
        "--config",
        FIXTURE_CONFIG,
        "--robot",
        "search",
        "terraphim",
        "--limit",
        "1",
    ])?;

    assert_eq!(
        code, 0,
        "auto-routed --robot search should succeed; stderr={}",
        stderr
    );

    // Exactly one [auto-route] line on stderr.
    assert_eq!(
        count_auto_route_lines(&stderr),
        1,
        "expected exactly one [auto-route] line on stderr; got:\n{}",
        stderr
    );

    let start = stdout
        .find('{')
        .with_context(|| format!("stdout has no JSON object; stdout={}", stdout))?;
    let parsed: Value = serde_json::from_str(&stdout[start..])
        .with_context(|| format!("stdout JSON did not parse; stdout={}", stdout))?;
    assert!(
        parsed.get("success").is_some(),
        "JSON envelope missing 'success' field"
    );
    assert!(
        parsed.get("meta").and_then(|m| m.get("command")).is_some(),
        "JSON envelope missing 'meta.command' field"
    );
    assert!(
        parsed.get("data").is_some(),
        "JSON envelope missing 'data' field"
    );
    Ok(())
}
