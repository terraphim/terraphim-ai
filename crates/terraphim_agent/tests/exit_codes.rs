use assert_cmd::Command;

fn cmd() -> Command {
    Command::cargo_bin("terraphim-agent").expect("binary not found")
}

#[test]
fn bad_flag_exits_2() {
    cmd()
        .args(["--unknown-flag-that-does-not-exist"])
        .assert()
        .code(2);
}

#[test]
fn search_missing_query_arg_exits_2() {
    cmd().args(["search"]).assert().code(2);
}

#[test]
fn bad_config_file_exits_1() {
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

#[test]
fn validate_with_no_kg_exits_3() {
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

#[test]
fn fail_on_empty_with_no_results_exits_4() {
    cmd()
        .args([
            "search",
            "xyzzy_no_such_term_f1_2_exit_code_test_sentinel",
            "--fail-on-empty",
        ])
        .assert()
        .code(4);
}

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
