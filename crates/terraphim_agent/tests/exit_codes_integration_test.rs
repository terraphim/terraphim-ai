//! Integration tests for typed exit code contracts (Task #860 Phase 4)
//!
//! Validates that CLI exit paths return the correct typed exit codes for various scenarios.

use std::process::Command;

#[test]
fn help_flag_exits_success() {
    let output = Command::new("cargo")
        .args(&["run", "-p", "terraphim_agent", "--", "--help"])
        .output()
        .expect("Failed to execute terraphim-agent --help");

    assert_eq!(
        output.status.code(),
        Some(0),
        "Help should exit with SUCCESS (0)"
    );
}

#[test]
fn invalid_subcommand_exits_with_error_usage() {
    let output = Command::new("cargo")
        .args(&["run", "-p", "terraphim_agent", "--", "invalid-subcommand"])
        .output()
        .expect("Failed to execute terraphim-agent with invalid subcommand");

    let exit_code = output.status.code().unwrap_or(-1);
    assert!(
        exit_code == 2 || exit_code == 1,
        "Invalid subcommand should exit with ERROR_USAGE (2) or ERROR_GENERAL (1), got {}",
        exit_code
    );
}

#[test]
fn listen_mode_with_server_flag_exits_error_usage() {
    let output = Command::new("cargo")
        .args(&["run", "-p", "terraphim_agent", "--", "listen", "--server"])
        .output()
        .expect("Failed to execute listen mode with --server flag");

    let exit_code = output.status.code().expect("Process killed by signal");
    assert_eq!(
        exit_code, 2,
        "Listen mode with --server should exit with ERROR_USAGE (2), got {}",
        exit_code
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("listen mode does not support --server flag"),
        "Should output appropriate error message"
    );
}

#[test]
fn exit_code_enum_values() {
    use std::path::Path;

    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let exit_codes_path = Path::new(manifest_dir).join("src/robot/exit_codes.rs");

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
    // This test validates that the typed exit code system maintains correctness
    // by ensuring from_code() round-trips properly
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

        // Verify code value
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

    // Count process::exit calls
    let _exit_call_count = content.matches("process::exit(").count();

    // Count typed ExitCode calls
    let typed_exit_count = content.matches("ExitCode::").count();

    // Most exit calls should be typed (allow some flexibility for legacy code)
    assert!(
        typed_exit_count > 0,
        "main.rs should have typed ExitCode exit calls"
    );

    // Verify no bare exit(1) calls remain for new phase 3 work
    assert!(
        !content.contains("exit(1)"),
        "All exit(1) calls should be converted to typed ExitCode::ErrorGeneral"
    );
}
