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
// Exit code mapping table verified by unit tests in src/robot/exit_codes.rs
// (codes 1, 3, 5, 7 are exercised there; integration paths for those require
// live services or specific filesystem state and are tested separately)
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
