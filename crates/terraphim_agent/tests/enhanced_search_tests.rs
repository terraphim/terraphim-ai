#![allow(clippy::bool_assert_comparison)]

use std::str::FromStr;
#[cfg(feature = "repl")]
use terraphim_agent::repl::commands::*;

/// Test basic search command parsing
#[cfg(feature = "repl")]
#[test]
fn test_basic_search_command_parsing() {
    let command = ReplCommand::from_str("/search rust programming").unwrap();

    match command {
        ReplCommand::Search {
            query,
            role,
            limit,
            semantic,
            concepts,
            ..
        } => {
            assert_eq!(query, "rust programming");
            assert_eq!(role, None);
            assert_eq!(limit, None);
            assert!(!semantic);
            assert!(!concepts);
        }
        _ => panic!("Expected Search command"),
    }
}

#[test]
fn test_search_with_role_command_parsing() {
    let command = ReplCommand::from_str("/search rust programming --role Developer").unwrap();

    match command {
        ReplCommand::Search {
            query,
            role,
            limit,
            semantic,
            concepts,
            ..
        } => {
            assert_eq!(query, "rust programming");
            assert_eq!(role, Some("Developer".to_string()));
            assert_eq!(limit, None);
            assert!(!semantic);
            assert!(!concepts);
        }
        _ => panic!("Expected Search command"),
    }
}

#[test]
fn test_search_with_limit_command_parsing() {
    let command = ReplCommand::from_str("/search rust programming --limit 10").unwrap();

    match command {
        ReplCommand::Search {
            query,
            role,
            limit,
            semantic,
            concepts,
            ..
        } => {
            assert_eq!(query, "rust programming");
            assert_eq!(role, None);
            assert_eq!(limit, Some(10));
            assert!(!semantic);
            assert!(!concepts);
        }
        _ => panic!("Expected Search command"),
    }
}

#[test]
fn test_search_semantic_flag_parsing() {
    let command = ReplCommand::from_str("/search rust programming --semantic").unwrap();

    match command {
        ReplCommand::Search {
            query,
            role,
            limit,
            semantic,
            concepts,
            ..
        } => {
            assert_eq!(query, "rust programming");
            assert_eq!(role, None);
            assert_eq!(limit, None);
            assert!(semantic);
            assert!(!concepts);
        }
        _ => panic!("Expected Search command"),
    }
}

#[test]
fn test_search_concepts_flag_parsing() {
    let command = ReplCommand::from_str("/search rust programming --concepts").unwrap();

    match command {
        ReplCommand::Search {
            query,
            role,
            limit,
            semantic,
            concepts,
            ..
        } => {
            assert_eq!(query, "rust programming");
            assert_eq!(role, None);
            assert_eq!(limit, None);
            assert!(!semantic);
            assert!(concepts);
        }
        _ => panic!("Expected Search command"),
    }
}

#[test]
fn test_search_all_flags_parsing() {
    let command = ReplCommand::from_str(
        "/search rust programming --role Developer --limit 15 --semantic --concepts",
    )
    .unwrap();

    match command {
        ReplCommand::Search {
            query,
            role,
            limit,
            semantic,
            concepts,
            ..
        } => {
            assert_eq!(query, "rust programming");
            assert_eq!(role, Some("Developer".to_string()));
            assert_eq!(limit, Some(15));
            assert!(semantic);
            assert!(concepts);
        }
        _ => panic!("Expected Search command"),
    }
}

#[test]
fn test_search_complex_query_parsing() {
    let command = ReplCommand::from_str("/search \"machine learning algorithms\" --semantic --concepts --role DataScientist --limit 20").unwrap();

    match command {
        ReplCommand::Search {
            query,
            role,
            limit,
            semantic,
            concepts,
            ..
        } => {
            assert_eq!(query, "\"machine learning algorithms\"");
            assert_eq!(role, Some("DataScientist".to_string()));
            assert_eq!(limit, Some(20));
            assert!(semantic);
            assert!(concepts);
        }
        _ => panic!("Expected Search command"),
    }
}

#[test]
fn test_search_empty_query_parsing() {
    let result = ReplCommand::from_str("/search");
    assert!(result.is_err(), "Should fail when query is empty");
}

#[test]
fn test_search_only_flags_parsing() {
    let result = ReplCommand::from_str("/search --semantic --concepts");
    assert!(result.is_err(), "Should fail when query is missing");
}

#[test]
fn test_search_invalid_limit_parsing() {
    let result = ReplCommand::from_str("/search test --limit invalid");
    assert!(result.is_err(), "Should fail with invalid limit value");
}

#[test]
fn test_search_missing_role_value_parsing() {
    let result = ReplCommand::from_str("/search test --role");
    assert!(result.is_err(), "Should fail when role value is missing");
}

#[test]
fn test_search_missing_limit_value_parsing() {
    let result = ReplCommand::from_str("/search test --limit");
    assert!(result.is_err(), "Should fail when limit value is missing");
}

#[test]
fn test_search_with_multiple_words_and_spaces() {
    let command =
        ReplCommand::from_str("/search    rust    async    programming    --semantic").unwrap();

    match command {
        ReplCommand::Search {
            query,
            semantic,
            concepts,
            ..
        } => {
            assert_eq!(query, "rust async programming");
            assert!(semantic);
            assert!(!concepts);
        }
        _ => panic!("Expected Search command"),
    }
}

#[test]
fn test_search_flags_order_independence() {
    let commands = vec![
        "/search test --role Dev --semantic",
        "/search test --semantic --role Dev",
        "/search test --role Dev --limit 5 --semantic",
        "/search test --semantic --limit 5 --role Dev",
        "/search test --limit 5 --semantic --role Dev",
    ];

    for cmd_str in commands {
        let command = ReplCommand::from_str(cmd_str).unwrap();

        match command {
            ReplCommand::Search {
                query,
                role,
                limit,
                semantic,
                concepts,
                ..
            } => {
                assert_eq!(query, "test");
                assert_eq!(role, Some("Dev".to_string()));
                assert!(semantic);
                assert!(!concepts);
                if cmd_str.contains("--limit 5") {
                    assert_eq!(limit, Some(5));
                } else {
                    assert_eq!(limit, None);
                }
            }
            _ => panic!("Expected Search command for: {}", cmd_str),
        }
    }
}

#[test]
fn test_search_with_special_characters() {
    let command =
        ReplCommand::from_str("/search \"C++ templates\" --concepts --role CppDeveloper").unwrap();

    match command {
        ReplCommand::Search {
            query,
            role,
            semantic,
            concepts,
            ..
        } => {
            assert_eq!(query, "\"C++ templates\"");
            assert_eq!(role, Some("CppDeveloper".to_string()));
            assert!(!semantic);
            assert!(concepts);
        }
        _ => panic!("Expected Search command"),
    }
}

#[test]
fn test_search_concepts_flag_multiple_times() {
    let command = ReplCommand::from_str("/search test --concepts --concepts").unwrap();

    match command {
        ReplCommand::Search {
            query,
            semantic,
            concepts,
            ..
        } => {
            assert_eq!(query, "test");
            assert!(!semantic);
            assert!(concepts); // Should still be true even with multiple flags
        }
        _ => panic!("Expected Search command"),
    }
}

#[test]
fn test_search_semantic_flag_multiple_times() {
    let command = ReplCommand::from_str("/search test --semantic --semantic").unwrap();

    match command {
        ReplCommand::Search {
            query,
            semantic,
            concepts,
            ..
        } => {
            assert_eq!(query, "test");
            assert!(semantic); // Should still be true even with multiple flags
            assert!(!concepts);
        }
        _ => panic!("Expected Search command"),
    }
}

#[test]
fn test_search_case_insensitive_flags() {
    let commands = vec![
        "/search test --ROLE Developer",
        "/search test --role DEVELOPER",
        "/search test --LIMIT 10",
        "/search test --limit 10",
    ];

    for cmd_str in commands {
        let result = ReplCommand::from_str(cmd_str);
        assert!(
            result.is_ok(),
            "Should parse case-insensitive flags: {}",
            cmd_str
        );
    }
}

#[test]
fn test_search_with_very_long_query() {
    let long_query = "a".repeat(1000);
    let command = ReplCommand::from_str(&format!("/search {} --semantic", long_query)).unwrap();

    match command {
        ReplCommand::Search {
            query, semantic, ..
        } => {
            assert_eq!(query.len(), 1000);
            assert!(semantic);
        }
        _ => panic!("Expected Search command"),
    }
}

#[test]
fn test_search_available_commands() {
    let commands = ReplCommand::available_commands();
    assert!(
        commands.contains(&"search"),
        "Available commands should include 'search'"
    );
}

#[test]
fn test_search_help_command() {
    let help_text = ReplCommand::get_command_help("search");
    assert!(
        help_text.is_some(),
        "Should have help text for search command"
    );

    let help = help_text.unwrap();
    assert!(help.contains("search"), "Help text should mention search");
    assert!(
        help.contains("semantic"),
        "Help text should mention semantic option"
    );
    assert!(
        help.contains("concepts"),
        "Help text should mention concepts option"
    );
}

#[test]
fn test_search_edge_cases() {
    // Test with only flags, no query
    assert!(ReplCommand::from_str("/search --semantic").is_err());

    // Test with invalid limit
    assert!(ReplCommand::from_str("/search test --limit -5").is_err());

    // Test with flag at end
    let cmd = ReplCommand::from_str("/search test --semantic").unwrap();
    if let ReplCommand::Search { semantic, .. } = cmd {
        assert!(semantic);
    } else {
        panic!("Expected Search command");
    }
}

/// Test --format flag parsing
#[cfg(feature = "repl")]
#[test]
fn test_search_format_json_parsing() {
    use terraphim_agent::robot::OutputFormat;
    let cmd = ReplCommand::from_str("/search rust --format json").unwrap();
    match cmd {
        ReplCommand::Search {
            query,
            format,
            robot,
            ..
        } => {
            assert_eq!(query, "rust");
            assert_eq!(format, Some(OutputFormat::Json));
            assert!(!robot);
        }
        _ => panic!("Expected Search command"),
    }
}

/// Test --format jsonl flag parsing
#[cfg(feature = "repl")]
#[test]
fn test_search_format_jsonl_parsing() {
    use terraphim_agent::robot::OutputFormat;
    let cmd = ReplCommand::from_str("/search rust --format jsonl").unwrap();
    match cmd {
        ReplCommand::Search { format, .. } => {
            assert_eq!(format, Some(OutputFormat::Jsonl));
        }
        _ => panic!("Expected Search command"),
    }
}

/// Test --format minimal flag parsing
#[cfg(feature = "repl")]
#[test]
fn test_search_format_minimal_parsing() {
    use terraphim_agent::robot::OutputFormat;
    let cmd = ReplCommand::from_str("/search rust --format minimal").unwrap();
    match cmd {
        ReplCommand::Search { format, .. } => {
            assert_eq!(format, Some(OutputFormat::Minimal));
        }
        _ => panic!("Expected Search command"),
    }
}

/// Test --robot flag parsing
#[cfg(feature = "repl")]
#[test]
fn test_search_robot_flag_parsing() {
    let cmd = ReplCommand::from_str("/search rust --robot").unwrap();
    match cmd {
        ReplCommand::Search { robot, format, .. } => {
            assert!(robot);
            assert_eq!(format, None);
        }
        _ => panic!("Expected Search command"),
    }
}

/// Test --robot and --format together
#[cfg(feature = "repl")]
#[test]
fn test_search_robot_with_format_parsing() {
    use terraphim_agent::robot::OutputFormat;
    let cmd = ReplCommand::from_str("/search rust --robot --format jsonl").unwrap();
    match cmd {
        ReplCommand::Search { robot, format, .. } => {
            assert!(robot);
            assert_eq!(format, Some(OutputFormat::Jsonl));
        }
        _ => panic!("Expected Search command"),
    }
}

/// Test invalid format value returns error
#[cfg(feature = "repl")]
#[test]
fn test_search_invalid_format_returns_error() {
    let result = ReplCommand::from_str("/search rust --format invalid_fmt");
    assert!(result.is_err(), "Invalid format should produce an error");
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("Valid formats"),
        "Error should list valid formats, got: {}",
        err
    );
}

/// Test --format missing value returns error
#[cfg(feature = "repl")]
#[test]
fn test_search_format_missing_value_returns_error() {
    let result = ReplCommand::from_str("/search rust --format");
    assert!(
        result.is_err(),
        "--format without value should produce an error"
    );
}
