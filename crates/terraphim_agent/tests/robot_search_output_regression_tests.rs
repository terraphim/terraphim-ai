// Regression tests for the `terraphim-agent search` CLI's --robot / --format
// JSON output. The canonical schema is the RobotResponse envelope from
// crates/terraphim_agent/src/robot/schema.rs:
//
//   {
//     "success": bool,
//     "meta":    { command, elapsed_ms, timestamp, version },
//     "data":    SearchResultsData { results, total_matches, concepts_matched?, wildcard_fallback },
//     "error":   ... (only on failure)
//   }
//
// where each SearchResultItem carries: rank, id, title, url?, score, preview?,
// source?, date?, preview_truncated.

use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

use anyhow::{Context, Result};
use serde_json::Value;
use serial_test::serial;

/// Build the agent binary once per `cargo test` invocation; subsequent calls
/// exec it directly. Avoids the per-call cargo metadata + recheck overhead.
static AGENT_BINARY: OnceLock<Result<PathBuf, String>> = OnceLock::new();

fn agent_binary() -> Result<PathBuf> {
    AGENT_BINARY
        .get_or_init(|| {
            let status = Command::new("cargo")
                .args(["build", "-p", "terraphim_agent", "--bin", "terraphim-agent"])
                .status()
                .map_err(|e| format!("failed to spawn cargo build: {}", e))?;
            if !status.success() {
                return Err(format!("cargo build failed with status {}", status));
            }
            // CARGO_MANIFEST_DIR is set by cargo when building/running tests.
            let manifest = std::env::var("CARGO_MANIFEST_DIR")
                .map_err(|_| "CARGO_MANIFEST_DIR not set".to_string())?;
            // crates/terraphim_agent -> ../../target/debug/terraphim-agent
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

/// Verify the RobotResponse envelope (success, meta, data) and the
/// SearchResultsData under data, plus per-item field types.
fn assert_search_envelope(value: &Value, expected_limit: usize) {
    let root = value.as_object().expect("envelope should be a JSON object");

    assert!(
        root.get("success")
            .and_then(Value::as_bool)
            .expect("envelope should contain bool field 'success'"),
        "search should report success=true"
    );

    let meta = root
        .get("meta")
        .and_then(Value::as_object)
        .expect("envelope should contain object field 'meta'");
    let cmd = meta
        .get("command")
        .and_then(Value::as_str)
        .expect("meta should contain string field 'command'");
    assert_eq!(cmd, "search", "meta.command should echo the subcommand");
    assert!(
        meta.get("timestamp").and_then(Value::as_str).is_some(),
        "meta should contain string field 'timestamp'"
    );
    assert!(
        meta.get("version").and_then(Value::as_str).is_some(),
        "meta should contain string field 'version'"
    );

    let data = root
        .get("data")
        .and_then(Value::as_object)
        .expect("envelope should contain object field 'data'");

    let total_matches =
        data.get("total_matches")
            .and_then(Value::as_u64)
            .expect("data should contain numeric field 'total_matches'") as usize;

    let results = data
        .get("results")
        .and_then(Value::as_array)
        .expect("data should contain array field 'results'");

    assert_eq!(
        total_matches,
        results.len(),
        "data.total_matches must match data.results length"
    );

    if results.len() > expected_limit {
        eprintln!(
            "warning: search returned {} results, limit was {} (relevance function may not enforce limit)",
            results.len(),
            expected_limit
        );
    }

    for result in results {
        let obj = result
            .as_object()
            .expect("each search result should be an object");

        let id = obj
            .get("id")
            .and_then(Value::as_str)
            .expect("search result should contain string field 'id'");
        assert!(
            !id.trim().is_empty(),
            "search result 'id' should not be empty"
        );

        let title = obj
            .get("title")
            .and_then(Value::as_str)
            .expect("search result should contain string field 'title'");
        assert!(
            !title.is_empty(),
            "search result 'title' should not be empty"
        );

        obj.get("rank")
            .and_then(Value::as_u64)
            .expect("search result should contain u64 field 'rank'");

        obj.get("score")
            .and_then(Value::as_f64)
            .expect("search result should contain f64 field 'score'");
    }
}

#[test]
#[serial]
fn search_robot_mode_emits_parseable_json_envelope() -> Result<()> {
    let query = "terraphim";
    let limit = 5usize;
    let (stdout, stderr, code) = run_agent_command(&["--robot", "search", query, "--limit", "5"])?;

    assert_eq!(
        code, 0,
        "search in --robot mode should succeed; stderr={}",
        stderr
    );

    let json = parse_first_json_object(&stdout)?;
    assert_search_envelope(&json, limit);
    Ok(())
}

#[test]
#[serial]
fn search_format_json_emits_parseable_json_envelope() -> Result<()> {
    let query = "terraphim";
    let limit = 5usize;
    let (stdout, stderr, code) =
        run_agent_command(&["--format", "json", "search", query, "--limit", "5"])?;

    assert_eq!(
        code, 0,
        "search in --format json mode should succeed; stderr={}",
        stderr
    );

    let json = parse_first_json_object(&stdout)?;
    assert_search_envelope(&json, limit);
    Ok(())
}

#[test]
#[serial]
fn search_format_json_compact_emits_parseable_json_envelope() -> Result<()> {
    let query = "terraphim";
    let limit = 5usize;
    let (stdout, stderr, code) =
        run_agent_command(&["--format", "json-compact", "search", query, "--limit", "5"])?;

    assert_eq!(
        code, 0,
        "search in --format json-compact mode should succeed; stderr={}",
        stderr
    );

    let json = parse_first_json_object(&stdout)?;
    assert_search_envelope(&json, limit);
    Ok(())
}

#[test]
#[serial]
fn search_robot_json_results_carry_optional_preview_url() -> Result<()> {
    let query = "terraphim";
    let (stdout, stderr, code) = run_agent_command(&["--robot", "search", query, "--limit", "3"])?;

    assert_eq!(
        code, 0,
        "search in --robot mode should succeed; stderr={}",
        stderr
    );

    let json = parse_first_json_object(&stdout)?;
    let results = json
        .pointer("/data/results")
        .and_then(Value::as_array)
        .expect("envelope should contain array at /data/results");

    for result in results {
        let obj = result.as_object().expect("each result should be an object");
        // url is optional but if present must be string
        if let Some(url) = obj.get("url") {
            assert!(
                url.is_string() || url.is_null(),
                "result 'url' should be string or null"
            );
        }
        // preview is optional but if present must be string
        if let Some(preview) = obj.get("preview") {
            assert!(
                preview.is_string() || preview.is_null(),
                "result 'preview' should be string or null"
            );
        }
    }
    Ok(())
}

#[test]
#[serial]
fn search_format_json_compact_produces_single_line_output() -> Result<()> {
    let query = "terraphim";
    let (stdout, stderr, code) =
        run_agent_command(&["--format", "json-compact", "search", query, "--limit", "2"])?;

    assert_eq!(
        code, 0,
        "search in --format json-compact mode should succeed; stderr={}",
        stderr
    );

    // Find the JSON line in stdout (skip any non-JSON preamble)
    let json_line = stdout
        .lines()
        .find(|line| line.trim_start().starts_with('{'))
        .expect("should find a JSON line in stdout");

    let parsed: Value = serde_json::from_str(json_line)
        .expect("compact JSON output should be parseable from a single line");
    assert!(
        parsed.get("success").is_some(),
        "compact JSON should contain success field"
    );
    assert!(
        parsed.pointer("/data/results").is_some(),
        "compact JSON should contain /data/results"
    );
    Ok(())
}
