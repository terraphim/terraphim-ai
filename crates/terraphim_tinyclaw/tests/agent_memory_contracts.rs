//! Contract tests for the agent-memory bridge tools.
//!
//! Each test uses a shell shim script in a `tempdir` to simulate the
//! `terraphim-agent` binary, avoiding any dependency on the real binary
//! or a live evolution store.

mod common;

use std::sync::Arc;
use terraphim_tinyclaw::config::MemoryConfig;
use terraphim_tinyclaw::tools::agent_memory::{
    AgentMemoryConfig, LearnCaptureTool, MemoryApplyTool, MemoryCaptureTool, MemoryRetrieveTool,
};
use terraphim_tinyclaw::tools::{Tool, ToolError, ToolRegistry};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Write a POSIX shell shim to `dir/terraphim-agent` and make it executable.
/// Returns the path to the shim binary.
fn write_shim(dir: &std::path::Path, script: &str) -> std::path::PathBuf {
    let shim_path = dir.join("terraphim-agent");
    std::fs::write(&shim_path, script).expect("write shim");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o755);
        std::fs::set_permissions(&shim_path, perms).expect("chmod shim");
    }
    shim_path
}

fn make_config(binary: std::path::PathBuf) -> Arc<AgentMemoryConfig> {
    Arc::new(AgentMemoryConfig {
        binary,
        role: None,
        timeout_secs: 5,
        max_context_chars: 4000,
    })
}

/// Canned export JSON matching the verified shape from research.md.
const CANNED_EXPORT_JSON: &str = r#"{"agent":"test","exported_at":"2026-01-01T00:00:00Z","memory_items":[{"id":"001","item_type":"Experience","content":"test memory content about Rust","importance":"Medium","tags":[],"access_count":0,"created_at":"2026-01-01T00:00:00Z"},{"id":"002","item_type":"Experience","content":"Python is good for scripting","importance":"Low","tags":["python"],"access_count":1,"created_at":"2026-01-02T00:00:00Z"}],"lessons":[],"summary":{"memory_count":2,"lesson_count":0}}"#;

// ---------------------------------------------------------------------------
// Contract 1: All 4 tools register in the registry
// ---------------------------------------------------------------------------

#[test]
fn contract_memory_tools_register_in_registry() {
    common::scrub_env();

    let cfg = make_config(std::path::PathBuf::from("terraphim-agent"));
    let mut registry = ToolRegistry::new();
    let initial_len = registry.len();

    registry.register(Box::new(MemoryCaptureTool::new(cfg.clone())));
    registry.register(Box::new(MemoryRetrieveTool::new(cfg.clone())));
    registry.register(Box::new(MemoryApplyTool::new(cfg.clone())));
    registry.register(Box::new(LearnCaptureTool::new(cfg)));

    assert_eq!(registry.len(), initial_len + 4);
    assert!(registry.has("memory_capture"));
    assert!(registry.has("memory_retrieve"));
    assert!(registry.has("memory_apply"));
    assert!(registry.has("learn_capture"));
}

// ---------------------------------------------------------------------------
// Contract 2: MemoryCaptureTool with shim
// ---------------------------------------------------------------------------

#[tokio::test]
async fn contract_memory_capture_with_shim() {
    common::scrub_env();

    let tmp = tempfile::tempdir().unwrap();
    let shim = write_shim(
        tmp.path(),
        r#"#!/bin/sh
case "$1 $2" in
  "memory capture")
    cat > /dev/null
    echo "Memory captured: 00000000-0000-0000-0000-000000000001"
    echo "  provenance_tag: tinyclaw"
    ;;
  *)
    echo "unknown: $*" >&2
    exit 1
    ;;
esac
"#,
    );

    let cfg = make_config(shim);
    let tool = MemoryCaptureTool::new(cfg);

    let result = tool
        .execute(serde_json::json!({
            "content": "test memory",
            "provenance_tag": "tc"
        }))
        .await
        .expect("capture should succeed");

    assert!(
        result.contains("Memory captured"),
        "Expected 'Memory captured' in output, got: {}",
        result
    );
}

// ---------------------------------------------------------------------------
// Contract 3: MemoryRetrieveTool with shim (parses export JSON, filters)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn contract_memory_retrieve_with_shim() {
    common::scrub_env();

    let tmp = tempfile::tempdir().unwrap();
    let script = format!(
        r#"#!/bin/sh
case "$1 $2" in
  "memory export")
    cat <<'ENDJSON'
{}
ENDJSON
    ;;
  *)
    echo "unknown: $*" >&2
    exit 1
    ;;
esac
"#,
        CANNED_EXPORT_JSON
    );
    let shim = write_shim(tmp.path(), &script);

    let cfg = make_config(shim);
    let tool = MemoryRetrieveTool::new(cfg);

    // Query "rust" should match item 001 but not 002
    let result = tool
        .execute(serde_json::json!({"query": "rust", "limit": 5}))
        .await
        .expect("retrieve should succeed");

    let items: Vec<serde_json::Value> = serde_json::from_str(&result).unwrap();
    assert_eq!(
        items.len(),
        1,
        "Expected 1 match for 'rust', got: {}",
        result
    );
    assert_eq!(items[0]["id"], "001");
}

#[tokio::test]
async fn contract_memory_retrieve_no_match() {
    common::scrub_env();

    let tmp = tempfile::tempdir().unwrap();
    let script = format!(
        r#"#!/bin/sh
case "$1 $2" in
  "memory export")
    cat <<'ENDJSON'
{}
ENDJSON
    ;;
  *)
    echo "unknown: $*" >&2
    exit 1
    ;;
esac
"#,
        CANNED_EXPORT_JSON
    );
    let shim = write_shim(tmp.path(), &script);

    let cfg = make_config(shim);
    let tool = MemoryRetrieveTool::new(cfg);

    let result = tool
        .execute(serde_json::json!({"query": "nonexistent_xyz"}))
        .await
        .expect("retrieve should succeed even with no matches");

    let items: Vec<serde_json::Value> = serde_json::from_str(&result).unwrap();
    assert!(items.is_empty(), "Expected empty array, got: {}", result);
}

#[tokio::test]
async fn contract_stdout_cap_rejects_oversized_export() {
    // A runaway `memory export` must be rejected with a clear error, not
    // slurped into memory (PR review P2: no stdout size cap).
    common::scrub_env();

    let tmp = tempfile::tempdir().unwrap();
    let script = r#"#!/bin/sh
case "$1 $2" in
  "memory export")
    # Emit > 1 MiB of JSON-ish junk.
    head -c $((1024*1024+16)) /dev/zero | tr '\0' 'x'
    echo
    ;;
  *)
    echo "unknown: $*" >&2
    exit 1
    ;;
esac
"#;
    let shim = write_shim(tmp.path(), script);
    let cfg = make_config(shim);

    let tool = MemoryRetrieveTool::new(cfg);
    let err = tool
        .execute(serde_json::json!({"query": "anything"}))
        .await
        .expect_err("oversized export must be rejected");

    let msg = format!("{err}");
    assert!(msg.contains("exceeds cap"), "unexpected error: {msg}");
    assert!(
        msg.contains("1 MiB") || msg.contains("1048576"),
        "unexpected error: {msg}"
    );
}

// ---------------------------------------------------------------------------
// Contract 4: LearnCaptureTool with shim
// ---------------------------------------------------------------------------

#[tokio::test]
async fn contract_learn_capture_with_shim() {
    common::scrub_env();

    let tmp = tempfile::tempdir().unwrap();
    let shim = write_shim(
        tmp.path(),
        r#"#!/bin/sh
case "$1 $2" in
  "learn capture")
    echo "Captured learning: /tmp/learn-001.json"
    ;;
  *)
    echo "unknown: $*" >&2
    exit 1
    ;;
esac
"#,
    );

    let cfg = make_config(shim);
    let tool = LearnCaptureTool::new(cfg);

    let result = tool
        .execute(serde_json::json!({
            "command": "npm install",
            "error": "command not found",
            "exit_code": 127
        }))
        .await
        .expect("learn capture should succeed");

    assert!(
        result.contains("Captured learning"),
        "Expected 'Captured learning' in output, got: {}",
        result
    );
}

// ---------------------------------------------------------------------------
// Contract 5: Missing binary returns ExecutionFailed
// ---------------------------------------------------------------------------

#[tokio::test]
async fn contract_missing_binary_returns_error() {
    common::scrub_env();

    let cfg = Arc::new(AgentMemoryConfig {
        binary: std::path::PathBuf::from("/nonexistent/terraphim-agent"),
        role: None,
        timeout_secs: 5,
        max_context_chars: 4000,
    });

    let tool = MemoryCaptureTool::new(cfg);
    let result = tool.execute(serde_json::json!({"content": "test"})).await;

    match result {
        Err(ToolError::ExecutionFailed { message, .. }) => {
            assert!(
                message.contains("not found"),
                "Error should mention 'not found', got: {}",
                message
            );
        }
        other => panic!("Expected ExecutionFailed, got: {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// Contract 6: Timeout returns ToolError::Timeout
// ---------------------------------------------------------------------------

#[tokio::test]
async fn contract_timeout_returns_error() {
    common::scrub_env();

    let tmp = tempfile::tempdir().unwrap();
    // Shim that sleeps longer than the configured timeout.
    let shim = write_shim(
        tmp.path(),
        r#"#!/bin/sh
# Sleep for 60 seconds (well beyond the 1-second timeout)
sleep 60
"#,
    );

    let cfg = Arc::new(AgentMemoryConfig {
        binary: shim,
        role: None,
        timeout_secs: 1,
        max_context_chars: 4000,
    });

    let tool = MemoryApplyTool::new(cfg);
    let result = tool.execute(serde_json::json!({"prompt": "test"})).await;

    assert!(
        matches!(result, Err(ToolError::Timeout { .. })),
        "Expected Timeout error, got: {:?}",
        result
    );
}

// ---------------------------------------------------------------------------
// Contract 7: MemoryApplyTool with shim
// ---------------------------------------------------------------------------

#[tokio::test]
async fn contract_memory_apply_with_shim() {
    common::scrub_env();

    let tmp = tempfile::tempdir().unwrap();
    let shim = write_shim(
        tmp.path(),
        r#"#!/bin/sh
case "$1 $2" in
  "memory apply")
    echo "Memory apply: showing what hooks would inject for prompt"
    echo "  prompt: $4"
    echo "  Context: Use Rust for performance-critical code"
    ;;
  *)
    echo "unknown: $*" >&2
    exit 1
    ;;
esac
"#,
    );

    let cfg = make_config(shim);
    let tool = MemoryApplyTool::new(cfg);

    let result = tool
        .execute(serde_json::json!({"prompt": "how should I optimise?"}))
        .await
        .expect("apply should succeed");

    assert!(
        result.contains("Memory apply"),
        "Expected 'Memory apply' in output, got: {}",
        result
    );
}

// ---------------------------------------------------------------------------
// Contract 8: create_default_registry registers memory tools when enabled
// ---------------------------------------------------------------------------

#[tokio::test]
async fn contract_registry_includes_memory_tools_when_enabled() {
    common::scrub_env();

    let mem_cfg = MemoryConfig {
        enabled: true,
        ..Default::default()
    };

    let registry =
        terraphim_tinyclaw::tools::create_default_registry(None, None, Some(&mem_cfg)).await;

    assert!(registry.has("memory_capture"), "memory_capture missing");
    assert!(registry.has("memory_retrieve"), "memory_retrieve missing");
    assert!(registry.has("memory_apply"), "memory_apply missing");
    assert!(registry.has("learn_capture"), "learn_capture missing");
}

#[tokio::test]
async fn contract_registry_excludes_memory_tools_when_disabled() {
    common::scrub_env();

    let mem_cfg = MemoryConfig {
        enabled: false,
        ..Default::default()
    };

    let registry =
        terraphim_tinyclaw::tools::create_default_registry(None, None, Some(&mem_cfg)).await;

    assert!(
        !registry.has("memory_capture"),
        "memory_capture should not be registered"
    );
    assert!(
        !registry.has("memory_retrieve"),
        "memory_retrieve should not be registered"
    );
}

// ---------------------------------------------------------------------------
// Contract 9: Non-zero exit code from shim returns ExecutionFailed
// ---------------------------------------------------------------------------

#[tokio::test]
async fn contract_nonzero_exit_returns_execution_failed() {
    common::scrub_env();

    let tmp = tempfile::tempdir().unwrap();
    let shim = write_shim(
        tmp.path(),
        r#"#!/bin/sh
echo "store is corrupt" >&2
exit 42
"#,
    );

    let cfg = make_config(shim);
    let tool = MemoryApplyTool::new(cfg);

    let result = tool.execute(serde_json::json!({"prompt": "test"})).await;

    match result {
        Err(ToolError::ExecutionFailed { message, .. }) => {
            assert!(
                message.contains("42"),
                "Error should mention exit code 42, got: {}",
                message
            );
            assert!(
                message.contains("store is corrupt"),
                "Error should include stderr, got: {}",
                message
            );
        }
        other => panic!("Expected ExecutionFailed, got: {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// Contract 10: retrieve prefers `memory retrieve --format json` (#3226)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn contract_retrieve_prefers_upstream_json_when_supported() {
    common::scrub_env();

    let tmp = tempfile::tempdir().unwrap();
    // The shim serves ranked JSON from `memory retrieve` and fails hard on
    // `memory export`: if the tool fell back to export, the test fails.
    let shim = write_shim(
        tmp.path(),
        r#"#!/bin/sh
case "$1 $2" in
  "memory retrieve")
    cat <<'ENDJSON'
{"memory_items":[{"id":"r1","item_type":"Experience","content":"ranked by upstream","importance":"Medium","tags":[],"access_count":0,"created_at":"2026-01-01T00:00:00Z"}]}
ENDJSON
    ;;
  "memory export")
    echo "export path must not be used when retrieve --format json works" >&2
    exit 9
    ;;
  *)
    echo "unknown: $*" >&2
    exit 1
    ;;
esac
"#,
    );

    let cfg = make_config(shim);
    let tool = MemoryRetrieveTool::new(cfg);

    let result = tool
        .execute(serde_json::json!({"query": "ranked", "limit": 5}))
        .await
        .expect("retrieve via upstream JSON path should succeed");

    let items: Vec<serde_json::Value> = serde_json::from_str(&result).unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["id"], "r1");
}

// ---------------------------------------------------------------------------
// Contract 11: role is scoped client-side on the upstream path (#3226)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn contract_retrieve_upstream_results_are_role_scoped() {
    common::scrub_env();

    let tmp = tempfile::tempdir().unwrap();
    // Upstream returns items for two roles; the tool must drop the
    // reviewer-scoped one because the config role is "developer".
    let shim = write_shim(
        tmp.path(),
        r#"#!/bin/sh
case "$1 $2" in
  "memory retrieve")
    cat <<'ENDJSON'
[{"id":"dev","item_type":"Experience","content":"run cargo nextest","importance":"Medium","tags":["role:developer"],"access_count":0,"created_at":"2026-01-01T00:00:00Z"},{"id":"rev","item_type":"Experience","content":"reviewer checklist","importance":"Medium","tags":["role:reviewer"],"access_count":0,"created_at":"2026-01-01T00:00:00Z"}]
ENDJSON
    ;;
  *)
    echo "unknown: $*" >&2
    exit 1
    ;;
esac
"#,
    );

    let cfg = Arc::new(AgentMemoryConfig {
        binary: shim,
        role: Some("developer".to_string()),
        timeout_secs: 5,
        max_context_chars: 4000,
    });
    let tool = MemoryRetrieveTool::new(cfg);

    let result = tool
        .execute(serde_json::json!({"query": "checklist"}))
        .await
        .expect("retrieve should succeed");

    let items: Vec<serde_json::Value> = serde_json::from_str(&result).unwrap();
    assert_eq!(items.len(), 1, "expected role scoping, got: {}", result);
    assert_eq!(items[0]["id"], "dev");
}

// ---------------------------------------------------------------------------
// Contract 12: fallback export path is ranked and role-scoped (#3226)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn contract_retrieve_fallback_ranks_and_scopes() {
    common::scrub_env();

    let tmp = tempfile::tempdir().unwrap();
    // Shim without `memory retrieve` support (exit 2 like clap does for an
    // unrecognised argument): the tool must fall back to export and rank.
    let export_json = r#"{"agent":"test","exported_at":"2026-01-01T00:00:00Z","memory_items":[{"id":"incidental","item_type":"Experience","content":"The escargot nextestimonial dinner was lovely","importance":"Low","tags":[],"access_count":0,"created_at":"2026-01-01T00:00:00Z"},{"id":"exact","item_type":"Experience","content":"Use cargo nextest for faster Rust test runs","importance":"High","tags":["role:developer"],"access_count":3,"created_at":"2026-01-02T00:00:00Z"},{"id":"other-role","item_type":"Experience","content":"cargo nextest guidance for reviewers","importance":"Medium","tags":["role:reviewer"],"access_count":1,"created_at":"2026-01-03T00:00:00Z"}],"lessons":[],"summary":{"memory_count":3,"lesson_count":0}}"#;
    let script = format!(
        r#"#!/bin/sh
case "$1 $2" in
  "memory retrieve")
    echo "error: unrecognized argument" >&2
    exit 2
    ;;
  "memory export")
    cat <<'ENDJSON'
{}
ENDJSON
    ;;
  *)
    echo "unknown: $*" >&2
    exit 1
    ;;
esac
"#,
        export_json
    );
    let shim = write_shim(tmp.path(), &script);

    let cfg = Arc::new(AgentMemoryConfig {
        binary: shim,
        role: Some("developer".to_string()),
        timeout_secs: 5,
        max_context_chars: 4000,
    });
    let tool = MemoryRetrieveTool::new(cfg);

    let result = tool
        .execute(serde_json::json!({"query": "cargo nextest", "limit": 5}))
        .await
        .expect("fallback retrieve should succeed");

    let items: Vec<serde_json::Value> = serde_json::from_str(&result).unwrap();
    assert!(
        !items.is_empty(),
        "expected ranked fallback results, got: {}",
        result
    );
    assert_eq!(
        items[0]["id"], "exact",
        "exact concept match must rank first, got: {}",
        result
    );
    let ids: Vec<&str> = items.iter().filter_map(|i| i["id"].as_str()).collect();
    assert!(
        !ids.contains(&"incidental"),
        "incidental substring match must not surface, got: {:?}",
        ids
    );
    assert!(
        !ids.contains(&"other-role"),
        "reviewer-scoped item must be excluded for developer role, got: {:?}",
        ids
    );
}
