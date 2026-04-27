//! Integration tests covering the spawner's task-body delivery contract
//! for Gitea issue #1020.
//!
//! These tests exercise the real `AgentSpawner` against a small bash
//! fixture (`tests/fixtures/echo_args_env.sh`) which records argv plus
//! selected env vars to a temp file. The tests then read the record file
//! and assert what the child actually saw.
//!
//! No mocks are used per repository policy; the fixture is a real
//! `tokio::process::Command` invocation through the spawner pipeline.

use std::path::PathBuf;
use std::time::Duration;

use tempfile::NamedTempFile;
use terraphim_spawner::{AgentSpawner, SpawnContext, SpawnRequest};
use terraphim_types::capability::{Capability, Provider, ProviderType};

/// Build a `Provider` whose CLI command is `/bin/bash`. The spawner's
/// `infer_args` for `bash` returns `["-c"]`, so `bash -c "<task>"` is
/// the expected invocation shape.
fn bash_provider(working_dir: PathBuf) -> Provider {
    Provider::new(
        "@bash-fixture",
        "Bash Fixture",
        ProviderType::Agent {
            agent_id: "@bash-fixture".to_string(),
            cli_command: "/bin/bash".to_string(),
            working_dir,
        },
        vec![Capability::CodeGeneration],
    )
}

/// Locate the bundled fixture script (resolved at compile time to keep
/// the test path stable regardless of CWD when `cargo test` runs).
fn fixture_script_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/echo_args_env.sh")
}

/// Wait briefly for a child to exit and the record file to be written.
/// Bash on the fixture exits within a few ms; 1 s is plenty.
async fn wait_for_record(path: &std::path::Path) -> String {
    let start = std::time::Instant::now();
    loop {
        if let Ok(contents) = tokio::fs::read_to_string(path).await {
            if !contents.is_empty() {
                return contents;
            }
        }
        if start.elapsed() > Duration::from_secs(2) {
            panic!(
                "fixture record file at {} did not become non-empty within 2s",
                path.display()
            );
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

/// Regression for #1020: when the spawner receives a multi-line bash
/// script via `task`, it must invoke `bash -c "<script>"` so the script
/// runs as a unit. Previously the orchestrator passed a short
/// informational summary instead, which produced `bash -c "Build/test
/// verdict ..."` and a `command not found` (exit 127).
#[tokio::test]
async fn bash_provider_runs_multi_line_task_via_dash_c() {
    let record = NamedTempFile::new().expect("create temp record file");
    let record_path = record.path().to_path_buf();

    let fixture = fixture_script_path();
    assert!(
        fixture.exists(),
        "fixture script missing at {}",
        fixture.display()
    );

    // Multi-line script body of the same shape as a real ADF agent
    // task: invokes the fixture, relies on `RECORD_FILE` from env.
    // Invoke the fixture via `bash <path>` so the test does not depend
    // on the fixture's executable bit (which `chmod` cannot set in the
    // sandboxed environment).
    let task_body = format!("set -e\nbash '{}' first second\n", fixture.display(),);

    let provider = bash_provider(PathBuf::from("/tmp"));
    let request = SpawnRequest::new(provider, &task_body);

    let ctx = SpawnContext::global()
        .with_env("RECORD_FILE", record_path.to_string_lossy().to_string())
        .with_env("ADF_TASK_SUMMARY", "informational summary line")
        .with_env("ADF_FIXTURE_MARKER", "marker-value");

    let spawner = AgentSpawner::new();
    let _handle = spawner
        .spawn_with_fallback(&request, ctx)
        .await
        .expect("spawn must succeed");

    let contents = wait_for_record(&record_path).await;

    // The fixture got two argv entries (`first`, `second`) which means
    // the spawner ran it inside a `bash -c` shell rather than treating
    // the multi-line body as a literal command name.
    assert!(
        contents.contains("argc=2"),
        "expected argc=2 (fixture must execute), got: {contents}"
    );
    assert!(
        contents.contains("argv[1]=first"),
        "missing first argv: {contents}"
    );
    assert!(
        contents.contains("argv[2]=second"),
        "missing second argv: {contents}"
    );
}

/// Regression for #1020: the orchestrator must be able to layer a
/// runtime informational summary as `ADF_TASK_SUMMARY` so future TOML
/// scripts can reference it. Verifies the spawn context env override is
/// visible to the spawned child process.
#[tokio::test]
async fn env_overrides_reach_child_process() {
    let record = NamedTempFile::new().expect("create temp record file");
    let record_path = record.path().to_path_buf();

    let fixture = fixture_script_path();
    let task_body = format!("set -e\nbash '{}' no-args\n", fixture.display(),);

    let provider = bash_provider(PathBuf::from("/tmp"));
    let request = SpawnRequest::new(provider, &task_body);

    let ctx = SpawnContext::global()
        .with_env("RECORD_FILE", record_path.to_string_lossy().to_string())
        .with_env("ADF_TASK_SUMMARY", "Build/test verdict for PR #999")
        .with_env("ADF_FIXTURE_MARKER", "marker-value");

    let spawner = AgentSpawner::new();
    let _handle = spawner
        .spawn_with_fallback(&request, ctx)
        .await
        .expect("spawn must succeed");

    let contents = wait_for_record(&record_path).await;
    assert!(
        contents.contains("ADF_TASK_SUMMARY=Build/test verdict for PR #999"),
        "ADF_TASK_SUMMARY env not visible to child, got: {contents}"
    );
    assert!(
        contents.contains("ADF_FIXTURE_MARKER=marker-value"),
        "ADF_FIXTURE_MARKER env not visible to child, got: {contents}"
    );
}
