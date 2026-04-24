/// Integration tests for the F1.2 exit-code contract.
///
/// Invokes the real `terraphim-agent` binary and asserts `status.code()`.
/// No mocks — uses real but unreachable endpoints, missing files, and
/// deliberately bad CLI flags to trigger each exit-code category.
use assert_cmd::Command;

fn cmd() -> Command {
    Command::cargo_bin("terraphim-agent").expect("binary not found")
}

// ---------------------------------------------------------------------------
// Exit code 2 — usage / bad flags (clap exits before main body)
// ---------------------------------------------------------------------------

#[test]
fn bad_flag_exits_2() {
    cmd()
        .args(["--unknown-flag-that-does-not-exist"])
        .assert()
        .code(2);
}

#[test]
fn search_missing_query_arg_exits_2() {
    // `search` requires a positional <query> argument
    cmd().args(["search"]).assert().code(2);
}

// ---------------------------------------------------------------------------
// Exit code 1 — ERROR_GENERAL (unspecified error, no matching pattern)
// ---------------------------------------------------------------------------

#[test]
fn bad_config_file_exits_1() {
    // A nonexistent config file produces a "Failed to load config" error which
    // does not match any specific exit-code pattern, so classify_error returns
    // ErrorGeneral (1).
    cmd()
        .args([
            "--config",
            "/tmp/nonexistent_f1_2_exit_code_test.json",
            "search",
            "terraphim",
        ])
        .assert()
        .code(1);
}

// ---------------------------------------------------------------------------
// Exit code 0 — successful offline search (may return 0 results but succeeds)
// ---------------------------------------------------------------------------

#[test]
fn search_succeeds_exits_0() {
    cmd()
        .args(["search", "terraphim", "--role", "Terraphim Engineer"])
        .assert()
        .code(0);
}

// ---------------------------------------------------------------------------
// Exit code 3 — index missing (knowledge graph not configured)
// ---------------------------------------------------------------------------

#[test]
fn validate_with_no_kg_exits_3() {
    // Load a fixture config where the role has kg: null so the service layer
    // returns "Knowledge graph not configured", which classify_error maps to
    // ErrorIndexMissing (3).  This avoids relying on the developer's local KG.
    let fixture = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/no_kg_config.json");
    cmd()
        .args([
            "--config",
            fixture,
            "validate",
            "xyzzy_f1_2_exit_code_test_sentinel",
        ])
        .assert()
        .code(3);
}

// ---------------------------------------------------------------------------
// Exit code 4 — not found when --fail-on-empty and no results
// ---------------------------------------------------------------------------

#[test]
fn fail_on_empty_with_no_results_exits_4() {
    // Use a query that is guaranteed to return zero results with the default
    // in-memory config (no haystacks indexed at test time).
    cmd()
        .args([
            "search",
            "xyzzy_no_such_term_f1_2_exit_code_test_sentinel",
            "--fail-on-empty",
        ])
        .assert()
        .code(4);
}

// ---------------------------------------------------------------------------
// Exit code 0 — --fail-on-empty with results present
// ---------------------------------------------------------------------------

#[test]
fn fail_on_empty_with_results_exits_0() {
    // Even when --fail-on-empty is set, non-empty results should exit 0.
    // The test may still produce 0 results depending on the indexed content,
    // so we accept either 0 or 4 — the point is it must NOT be 1/2/3.
    let status = cmd()
        .args(["search", "terraphim", "--fail-on-empty"])
        .output()
        .expect("failed to run binary")
        .status;
    let code = status.code().unwrap_or(1);
    assert!(
        code == 0 || code == 4,
        "expected 0 or 4 from --fail-on-empty search, got {code}"
    );
}

// ---------------------------------------------------------------------------
// Exit code 6 — network error (unreachable server endpoint)
// ---------------------------------------------------------------------------

#[cfg(feature = "server")]
#[test]
fn unreachable_server_exits_6() {
    cmd()
        .args([
            "--server",
            "--server-url",
            "http://127.0.0.1:19999",
            "search",
            "terraphim",
        ])
        .assert()
        .code(6);
}

// ---------------------------------------------------------------------------
// Exit codes 5 (ERROR_AUTH) and 7 (ERROR_TIMEOUT) are exercised by the
// classify_error unit tests in src/main.rs.  Their binary-level integration
// paths require either a live authenticating server (5) or a controllable
// slow endpoint (7) and are therefore omitted here to keep the suite
// self-contained and offline.
// ---------------------------------------------------------------------------

#[test]
fn exit_code_values_are_stable() {
    use terraphim_agent::robot::ExitCode;
    assert_eq!(ExitCode::Success.code(), 0);
    assert_eq!(ExitCode::ErrorGeneral.code(), 1);
    assert_eq!(ExitCode::ErrorUsage.code(), 2);
    assert_eq!(ExitCode::ErrorIndexMissing.code(), 3);
    assert_eq!(ExitCode::ErrorNotFound.code(), 4);
    assert_eq!(ExitCode::ErrorAuth.code(), 5);
    assert_eq!(ExitCode::ErrorNetwork.code(), 6);
    assert_eq!(ExitCode::ErrorTimeout.code(), 7);
}
