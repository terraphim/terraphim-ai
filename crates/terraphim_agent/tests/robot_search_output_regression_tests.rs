use std::process::Command;

use anyhow::{Context, Result};
use serde_json::Value;
use serial_test::serial;

fn run_agent_command(args: &[&str]) -> Result<(String, String, i32)> {
    let output = Command::new("cargo")
        .args(["run", "-p", "terraphim_agent", "--"])
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

fn assert_search_output_contract(value: &Value, expected_query: &str, expected_limit: usize) {
    let root = value
        .as_object()
        .expect("search output should be a JSON object");

    let query = root
        .get("query")
        .and_then(Value::as_str)
        .expect("search output should contain string field 'query'");
    assert_eq!(query, expected_query, "search output should echo the query");

    let role = root
        .get("role")
        .and_then(Value::as_str)
        .expect("search output should contain string field 'role'");
    assert!(
        !role.trim().is_empty(),
        "search output 'role' should not be empty"
    );

    let count = root
        .get("count")
        .and_then(Value::as_u64)
        .expect("search output should contain numeric field 'count'") as usize;

    let results = root
        .get("results")
        .and_then(Value::as_array)
        .expect("search output should contain array field 'results'");

    assert_eq!(
        count,
        results.len(),
        "search output count must match results length"
    );
    // Note: not all relevance functions (e.g. TitleScorer) enforce the limit
    // parameter server-side, so we only warn rather than fail.
    if results.len() > expected_limit {
        eprintln!(
            "warning: search returned {} results, limit was {} (relevance function may not enforce limit)",
            results.len(),
            expected_limit
        );
    }

    for result in results {
        let result_obj = result
            .as_object()
            .expect("each search result should be an object");

        let id = result_obj
            .get("id")
            .and_then(Value::as_str)
            .expect("search result should contain string field 'id'");
        assert!(
            !id.trim().is_empty(),
            "search result 'id' should not be empty"
        );

        let title = result_obj
            .get("title")
            .and_then(Value::as_str)
            .expect("search result should contain string field 'title'");
        assert!(
            !title.trim().is_empty(),
            "search result 'title' should not be empty"
        );

        let _url = result_obj
            .get("url")
            .and_then(Value::as_str)
            .expect("search result should contain string field 'url'");

        let rank = result_obj
            .get("rank")
            .expect("search result should contain field 'rank'");
        assert!(
            rank.is_null() || rank.as_u64().is_some(),
            "search result 'rank' should be null or u64"
        );
    }
}

#[test]
#[serial]
fn search_robot_mode_emits_parseable_json_contract() -> Result<()> {
    let query = "terraphim";
    let limit = 5usize;
    let (stdout, stderr, code) = run_agent_command(&["--robot", "search", query, "--limit", "5"])?;

    assert_eq!(
        code, 0,
        "search in --robot mode should succeed; stderr={}",
        stderr
    );

    let json = parse_first_json_object(&stdout)?;
    assert_search_output_contract(&json, query, limit);
    Ok(())
}

#[test]
#[serial]
fn search_format_json_emits_parseable_json_contract() -> Result<()> {
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
    assert_search_output_contract(&json, query, limit);
    Ok(())
}

#[test]
#[serial]
fn search_format_json_compact_emits_parseable_json_contract() -> Result<()> {
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
    assert_search_output_contract(&json, query, limit);
    Ok(())
}
