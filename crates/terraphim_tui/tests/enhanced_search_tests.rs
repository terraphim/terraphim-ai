#[cfg(feature = "repl-custom")]
use std::str::FromStr;
#[cfg(feature = "repl-custom")]
use terraphim_tui::repl::commands::ReplCommand;

/// Test basic search command parsing
#[cfg(feature = "repl-custom")]
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
        } => {
            assert_eq!(query, "rust programming");
            assert_eq!(role, None);
            assert_eq!(limit, None);
            assert_eq!(semantic, false);
            assert_eq!(concepts, false);
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
        } => {
            assert_eq!(query, "rust programming");
            assert_eq!(role, Some("Developer".to_string()));
            assert_eq!(limit, None);
            assert_eq!(semantic, false);
            assert_eq!(concepts, false);
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
        } => {
            assert_eq!(query, "rust programming");
            assert_eq!(role, None);
            assert_eq!(limit, Some(10));
            assert_eq!(semantic, false);
            assert_eq!(concepts, false);
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
        } => {
            assert_eq!(query, "rust programming");
            assert_eq!(role, None);
            assert_eq!(limit, None);
            assert_eq!(semantic, true);
            assert_eq!(concepts, false);
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
        } => {
            assert_eq!(query, "rust programming");
            assert_eq!(role, None);
            assert_eq!(limit, None);
            assert_eq!(semantic, false);
            assert_eq!(concepts, true);
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
        } => {
            assert_eq!(query, "rust programming");
            assert_eq!(role, Some("Developer".to_string()));
            assert_eq!(limit, Some(15));
            assert_eq!(semantic, true);
            assert_eq!(concepts, true);
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
        } => {
            assert_eq!(query, "\"machine learning algorithms\"");
            assert_eq!(role, Some("DataScientist".to_string()));
            assert_eq!(limit, Some(20));
            assert_eq!(semantic, true);
            assert_eq!(concepts, true);
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
            role,
            limit,
            semantic,
            concepts,
        } => {
            assert_eq!(query, "rust async programming");
            assert_eq!(semantic, true);
            assert_eq!(concepts, false);
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
            } => {
                assert_eq!(query, "test");
                assert_eq!(role, Some("Dev".to_string()));
                assert_eq!(semantic, true);
                assert_eq!(concepts, false);
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
            limit,
            semantic,
            concepts,
        } => {
            assert_eq!(query, "\"C++ templates\"");
            assert_eq!(role, Some("CppDeveloper".to_string()));
            assert_eq!(semantic, false);
            assert_eq!(concepts, true);
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
            role,
            limit,
            semantic,
            concepts,
        } => {
            assert_eq!(query, "test");
            assert_eq!(semantic, false);
            assert_eq!(concepts, true); // Should still be true even with multiple flags
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
            role,
            limit,
            semantic,
            concepts,
        } => {
            assert_eq!(query, "test");
            assert_eq!(semantic, true); // Should still be true even with multiple flags
            assert_eq!(concepts, false);
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
            query,
            role,
            limit,
            semantic,
            concepts,
        } => {
            assert_eq!(query.len(), 1000);
            assert_eq!(semantic, true);
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
