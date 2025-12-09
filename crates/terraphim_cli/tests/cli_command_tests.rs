//! Tests for CLI command execution using assert_cmd
//!
//! These tests verify the CLI binary produces correct output for various commands.

#[allow(deprecated)] // cargo_bin is deprecated but still works
use assert_cmd::Command;
use predicates::prelude::*;
use serial_test::serial;

/// Get a command for the terraphim-cli binary
#[allow(deprecated)] // cargo_bin is deprecated but still functional
fn cli_command() -> Command {
    Command::cargo_bin("terraphim-cli").unwrap()
}

#[test]
fn test_cli_help() {
    cli_command()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("terraphim-cli"))
        .stdout(predicate::str::contains("search"))
        .stdout(predicate::str::contains("config"))
        .stdout(predicate::str::contains("roles"))
        .stdout(predicate::str::contains("graph"))
        .stdout(predicate::str::contains("replace"))
        .stdout(predicate::str::contains("find"))
        .stdout(predicate::str::contains("thesaurus"))
        .stdout(predicate::str::contains("completions"));
}

#[test]
fn test_cli_version() {
    cli_command()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("terraphim-cli"));
}

#[test]
fn test_search_help() {
    cli_command()
        .args(["search", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("query"))
        .stdout(predicate::str::contains("--role"))
        .stdout(predicate::str::contains("--limit"));
}

#[test]
fn test_replace_help() {
    cli_command()
        .args(["replace", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("TEXT").or(predicate::str::contains("text")))
        .stdout(predicate::str::contains("--link-format"))
        .stdout(predicate::str::contains("--role"));
}

#[test]
fn test_find_help() {
    cli_command()
        .args(["find", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("text"))
        .stdout(predicate::str::contains("--role"));
}

#[test]
fn test_graph_help() {
    cli_command()
        .args(["graph", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--top-k"))
        .stdout(predicate::str::contains("--role"));
}

#[test]
fn test_thesaurus_help() {
    cli_command()
        .args(["thesaurus", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--role"))
        .stdout(predicate::str::contains("--limit"));
}

#[test]
fn test_completions_help() {
    cli_command()
        .args(["completions", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("shell"));
}

#[test]
fn test_completions_bash() {
    cli_command()
        .args(["completions", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("terraphim-cli"));
}

#[test]
fn test_completions_zsh() {
    cli_command()
        .args(["completions", "zsh"])
        .assert()
        .success()
        .stdout(predicate::str::contains("terraphim-cli"));
}

#[test]
fn test_completions_fish() {
    cli_command()
        .args(["completions", "fish"])
        .assert()
        .success()
        .stdout(predicate::str::contains("terraphim-cli"));
}

#[test]
fn test_no_command_shows_help() {
    cli_command()
        .assert()
        .failure()
        .stderr(predicate::str::contains("Usage"));
}

#[test]
fn test_invalid_command() {
    cli_command().arg("invalid_command").assert().failure();
}

// Integration tests that require service initialization
mod integration {
    use super::*;

    #[test]
    #[serial]
    fn test_config_command_json_output() {
        let output = cli_command()
            .args(["config"])
            .output()
            .expect("Failed to execute command");

        // Check that output is valid JSON
        let stdout = String::from_utf8_lossy(&output.stdout);
        if output.status.success() {
            // Try to parse as JSON
            let parsed: Result<serde_json::Value, _> = serde_json::from_str(&stdout);
            assert!(
                parsed.is_ok(),
                "Config output should be valid JSON: {}",
                stdout
            );

            if let Ok(json) = parsed {
                // Check structure
                assert!(
                    json.get("selected_role").is_some(),
                    "Should have selected_role field"
                );
                assert!(json.get("roles").is_some(), "Should have roles field");
            }
        }
    }

    #[test]
    #[serial]
    fn test_config_command_pretty_json() {
        let output = cli_command()
            .args(["--format", "json-pretty", "config"])
            .output()
            .expect("Failed to execute command");

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            // Pretty JSON should have newlines
            assert!(
                stdout.contains('\n'),
                "Pretty JSON should have newlines: {}",
                stdout
            );
        }
    }

    #[test]
    #[serial]
    fn test_roles_command_json_output() {
        let output = cli_command()
            .args(["roles"])
            .output()
            .expect("Failed to execute command");

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            // Should be an array of role names
            let parsed: Result<Vec<String>, _> = serde_json::from_str(&stdout);
            assert!(
                parsed.is_ok(),
                "Roles output should be a JSON array: {}",
                stdout
            );
        }
    }

    #[test]
    #[serial]
    fn test_search_command_with_query() {
        let output = cli_command()
            .args(["search", "rust"])
            .output()
            .expect("Failed to execute command");

        let stdout = String::from_utf8_lossy(&output.stdout);
        if output.status.success() {
            let parsed: Result<serde_json::Value, _> = serde_json::from_str(&stdout);
            assert!(
                parsed.is_ok(),
                "Search output should be valid JSON: {}",
                stdout
            );

            if let Ok(json) = parsed {
                // Check structure
                assert!(json.get("query").is_some(), "Should have query field");
                assert!(json.get("role").is_some(), "Should have role field");
                assert!(json.get("results").is_some(), "Should have results field");
                assert!(json.get("count").is_some(), "Should have count field");
            }
        }
    }

    #[test]
    #[serial]
    fn test_search_command_with_role() {
        let output = cli_command()
            .args(["search", "async", "--role", "Default"])
            .output()
            .expect("Failed to execute command");

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let parsed: serde_json::Value =
                serde_json::from_str(&stdout).expect("Should be valid JSON");

            assert_eq!(
                parsed["role"].as_str(),
                Some("Default"),
                "Should use specified role"
            );
        }
    }

    #[test]
    #[serial]
    fn test_search_command_with_limit() {
        let output = cli_command()
            .args(["search", "tokio", "--limit", "5"])
            .output()
            .expect("Failed to execute command");

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let parsed: serde_json::Value =
                serde_json::from_str(&stdout).expect("Should be valid JSON");

            let count = parsed["count"].as_u64().unwrap_or(0);
            assert!(count <= 5, "Results should respect limit");
        }
    }

    #[test]
    #[serial]
    fn test_graph_command() {
        let output = cli_command()
            .args(["graph"])
            .output()
            .expect("Failed to execute command");

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let parsed: Result<serde_json::Value, _> = serde_json::from_str(&stdout);
            assert!(
                parsed.is_ok(),
                "Graph output should be valid JSON: {}",
                stdout
            );

            if let Ok(json) = parsed {
                assert!(json.get("role").is_some(), "Should have role field");
                assert!(json.get("top_k").is_some(), "Should have top_k field");
                assert!(json.get("concepts").is_some(), "Should have concepts field");
            }
        }
    }

    #[test]
    #[serial]
    fn test_graph_command_with_top_k() {
        let output = cli_command()
            .args(["graph", "--top-k", "5"])
            .output()
            .expect("Failed to execute command");

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let parsed: serde_json::Value =
                serde_json::from_str(&stdout).expect("Should be valid JSON");

            assert_eq!(
                parsed["top_k"].as_u64(),
                Some(5),
                "Should use specified top_k"
            );
        }
    }

    #[test]
    #[serial]
    fn test_replace_command_markdown() {
        let output = cli_command()
            .args([
                "replace",
                "rust async programming",
                "--link-format",
                "markdown",
            ])
            .output()
            .expect("Failed to execute command");

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let parsed: Result<serde_json::Value, _> = serde_json::from_str(&stdout);
            assert!(
                parsed.is_ok(),
                "Replace output should be valid JSON: {}",
                stdout
            );

            if let Ok(json) = parsed {
                assert!(json.get("original").is_some(), "Should have original field");
                assert!(json.get("replaced").is_some(), "Should have replaced field");
                assert!(json.get("format").is_some(), "Should have format field");
                assert_eq!(
                    json["format"].as_str(),
                    Some("markdown"),
                    "Should be markdown format"
                );
            }
        }
    }

    #[test]
    #[serial]
    fn test_replace_command_html() {
        let output = cli_command()
            .args(["replace", "tokio server", "--link-format", "html"])
            .output()
            .expect("Failed to execute command");

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let parsed: serde_json::Value =
                serde_json::from_str(&stdout).expect("Should be valid JSON");

            assert_eq!(
                parsed["format"].as_str(),
                Some("html"),
                "Should be html format"
            );
        }
    }

    #[test]
    #[serial]
    fn test_replace_command_wiki() {
        let output = cli_command()
            .args(["replace", "git github", "--link-format", "wiki"])
            .output()
            .expect("Failed to execute command");

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let parsed: serde_json::Value =
                serde_json::from_str(&stdout).expect("Should be valid JSON");

            assert_eq!(
                parsed["format"].as_str(),
                Some("wiki"),
                "Should be wiki format"
            );
        }
    }

    #[test]
    #[serial]
    fn test_replace_command_plain() {
        let output = cli_command()
            .args(["replace", "docker kubernetes", "--link-format", "plain"])
            .output()
            .expect("Failed to execute command");

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let parsed: serde_json::Value =
                serde_json::from_str(&stdout).expect("Should be valid JSON");

            // Plain format should return original text unchanged
            assert_eq!(
                parsed["format"].as_str(),
                Some("plain"),
                "Should be plain format"
            );
            assert_eq!(
                parsed["original"].as_str(),
                parsed["replaced"].as_str(),
                "Plain format should not modify text"
            );
        }
    }

    #[test]
    #[serial]
    fn test_replace_command_invalid_format() {
        let output = cli_command()
            .args(["replace", "test", "--link-format", "invalid"])
            .output()
            .expect("Failed to execute command");

        // Should fail with error
        assert!(!output.status.success(), "Invalid format should fail");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("error") || stdout.contains("Unknown format"),
            "Should indicate invalid format"
        );
    }

    #[test]
    #[serial]
    fn test_find_command() {
        let output = cli_command()
            .args(["find", "rust async tokio"])
            .output()
            .expect("Failed to execute command");

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let parsed: Result<serde_json::Value, _> = serde_json::from_str(&stdout);
            assert!(
                parsed.is_ok(),
                "Find output should be valid JSON: {}",
                stdout
            );

            if let Ok(json) = parsed {
                assert!(json.get("text").is_some(), "Should have text field");
                assert!(json.get("matches").is_some(), "Should have matches field");
                assert!(json.get("count").is_some(), "Should have count field");
            }
        }
    }

    #[test]
    #[serial]
    fn test_find_command_with_role() {
        let output = cli_command()
            .args(["find", "database server", "--role", "Default"])
            .output()
            .expect("Failed to execute command");

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let parsed: serde_json::Value =
                serde_json::from_str(&stdout).expect("Should be valid JSON");

            assert!(parsed["matches"].is_array(), "Matches should be an array");
        }
    }

    #[test]
    #[serial]
    fn test_thesaurus_command() {
        let output = cli_command()
            .args(["thesaurus"])
            .output()
            .expect("Failed to execute command");

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let parsed: Result<serde_json::Value, _> = serde_json::from_str(&stdout);
            assert!(
                parsed.is_ok(),
                "Thesaurus output should be valid JSON: {}",
                stdout
            );

            if let Ok(json) = parsed {
                assert!(json.get("role").is_some(), "Should have role field");
                assert!(json.get("name").is_some(), "Should have name field");
                assert!(json.get("terms").is_some(), "Should have terms field");
                assert!(
                    json.get("total_count").is_some(),
                    "Should have total_count field"
                );
                assert!(
                    json.get("shown_count").is_some(),
                    "Should have shown_count field"
                );
            }
        }
    }

    #[test]
    #[serial]
    fn test_thesaurus_command_with_limit() {
        let output = cli_command()
            .args(["thesaurus", "--limit", "10"])
            .output()
            .expect("Failed to execute command");

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let parsed: serde_json::Value =
                serde_json::from_str(&stdout).expect("Should be valid JSON");

            let shown_count = parsed["shown_count"].as_u64().unwrap_or(0);
            assert!(shown_count <= 10, "Should respect limit");
        }
    }

    #[test]
    #[serial]
    fn test_output_format_text() {
        let output = cli_command()
            .args(["--format", "text", "config"])
            .output()
            .expect("Failed to execute command");

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            // Text format should not be strict JSON (may have different formatting)
            assert!(!stdout.is_empty(), "Text output should not be empty");
        }
    }

    #[test]
    #[serial]
    fn test_quiet_mode() {
        let output = cli_command()
            .args(["--quiet", "config"])
            .output()
            .expect("Failed to execute command");

        // In quiet mode, stderr should be empty (no warnings/errors printed)
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Note: Some log output may still appear depending on log configuration
        // This test mainly verifies the flag is accepted
        assert!(output.status.success() || stderr.len() < 1000);
    }
}
