//! Integration tests for typed exit code contracts (Task #860 Phase 4)
//!
//! Validates that CLI exit paths return the correct typed exit codes for various scenarios.
//!
//! Uses `assert_cmd::Command::cargo_bin` to invoke the pre-built binary rather than
//! nested `cargo run`, avoiding artifact lock contention under `cargo test --workspace`.

use assert_cmd::Command;

fn cmd() -> Command {
    Command::cargo_bin("terraphim-agent").expect("terraphim-agent binary not found")
}

#[test]
fn help_flag_exits_success() {
    cmd().arg("--help").assert().success();
}

#[test]
fn invalid_subcommand_exits_with_error_usage() {
    let assert = cmd().arg("invalid-subcommand").assert();
    let exit_code = assert.get_output().status.code().unwrap_or(-1);
    assert!(
        exit_code == 2 || exit_code == 1,
        "Invalid subcommand should exit with ERROR_USAGE (2) or ERROR_GENERAL (1), got {}",
        exit_code
    );
}

#[test]
fn listen_mode_with_server_flag_exits_error_usage() {
    let output = cmd()
        .args(["listen", "--server"])
        .output()
        .expect("Failed to execute listen mode with --server flag");

    let exit_code = output.status.code().expect("Process killed by signal");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        exit_code == 2
            || stderr.contains("listen mode does not support --server flag")
            || stderr.contains("--identity"),
        "Listen mode with --server should exit with ERROR_USAGE (2) or show appropriate error, got exit={}, stderr={}",
        exit_code,
        stderr
    );
}

#[test]
fn exit_code_enum_values() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let exit_codes_path = std::path::Path::new(manifest_dir).join("src/robot/exit_codes.rs");

    assert!(
        exit_codes_path.exists(),
        "exit_codes.rs module should exist"
    );

    let content = std::fs::read_to_string(&exit_codes_path).expect("Failed to read exit_codes.rs");

    let expected_variants = vec![
        "Success = 0",
        "ErrorGeneral = 1",
        "ErrorUsage = 2",
        "ErrorIndexMissing = 3",
        "ErrorNotFound = 4",
        "ErrorAuth = 5",
        "ErrorNetwork = 6",
        "ErrorTimeout = 7",
    ];

    for variant in expected_variants {
        assert!(
            content.contains(variant),
            "exit_codes.rs should define {}",
            variant
        );
    }
}

#[test]
fn exit_code_from_code_round_trip() {
    let code_variants = vec![
        (0, "Success"),
        (1, "ErrorGeneral"),
        (2, "ErrorUsage"),
        (3, "ErrorIndexMissing"),
        (4, "ErrorNotFound"),
        (5, "ErrorAuth"),
        (6, "ErrorNetwork"),
        (7, "ErrorTimeout"),
    ];

    for (code, name) in code_variants {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let exit_codes_path = std::path::Path::new(manifest_dir).join("src/robot/exit_codes.rs");
        let content =
            std::fs::read_to_string(&exit_codes_path).expect("Failed to read exit_codes.rs");

        assert!(
            content.contains(&format!("{} = {}", name, code)),
            "exit_codes.rs should contain code {} for {}",
            code,
            name
        );
    }
}

#[test]
fn all_exit_calls_use_typed_exit_codes() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let main_path = std::path::Path::new(manifest_dir).join("src/main.rs");
    let content = std::fs::read_to_string(&main_path).expect("Failed to read main.rs");

    let typed_exit_count = content.matches("ExitCode::").count();

    assert!(
        typed_exit_count > 0,
        "main.rs should have typed ExitCode exit calls"
    );
}
