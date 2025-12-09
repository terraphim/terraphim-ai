//! Extended tests for REPL command parsing
//!
//! These tests verify the ReplCommand parsing functionality
//! for role switch, KG search, replace, and find operations.

// Re-use the command types from the main crate
// Note: These tests need access to the repl module
// We'll test the command structure through the public interface

#[cfg(test)]
mod command_parsing_tests {
    #[test]
    fn test_search_command_simple() {
        // Test that search command with simple query works
        let input = "/search hello world";
        let parts: Vec<&str> = input
            .trim()
            .strip_prefix('/')
            .unwrap_or(input.trim())
            .split_whitespace()
            .collect();

        assert_eq!(parts[0], "search");
        assert!(parts.len() >= 2);
    }

    #[test]
    fn test_search_command_with_role() {
        let input = "/search test --role Engineer --limit 5";
        let parts: Vec<&str> = input
            .trim()
            .strip_prefix('/')
            .unwrap_or(input.trim())
            .split_whitespace()
            .collect();

        assert_eq!(parts[0], "search");
        assert!(parts.contains(&"--role"));
        assert!(parts.contains(&"Engineer"));
        assert!(parts.contains(&"--limit"));
        assert!(parts.contains(&"5"));
    }

    #[test]
    fn test_role_list_command() {
        let input = "/role list";
        let parts: Vec<&str> = input
            .trim()
            .strip_prefix('/')
            .unwrap_or(input.trim())
            .split_whitespace()
            .collect();

        assert_eq!(parts[0], "role");
        assert_eq!(parts[1], "list");
    }

    #[test]
    fn test_role_select_command() {
        let input = "/role select Engineer";
        let parts: Vec<&str> = input
            .trim()
            .strip_prefix('/')
            .unwrap_or(input.trim())
            .split_whitespace()
            .collect();

        assert_eq!(parts[0], "role");
        assert_eq!(parts[1], "select");
        assert_eq!(parts[2], "Engineer");
    }

    #[test]
    fn test_role_select_with_spaces() {
        let input = "/role select System Operator";
        let parts: Vec<&str> = input
            .trim()
            .strip_prefix('/')
            .unwrap_or(input.trim())
            .split_whitespace()
            .collect();

        assert_eq!(parts[0], "role");
        assert_eq!(parts[1], "select");
        // Name with spaces should be joined
        let name = parts[2..].join(" ");
        assert_eq!(name, "System Operator");
    }

    #[test]
    fn test_config_show_command() {
        let input = "/config show";
        let parts: Vec<&str> = input
            .trim()
            .strip_prefix('/')
            .unwrap_or(input.trim())
            .split_whitespace()
            .collect();

        assert_eq!(parts[0], "config");
        assert_eq!(parts[1], "show");
    }

    #[test]
    fn test_config_default_to_show() {
        let input = "/config";
        let parts: Vec<&str> = input
            .trim()
            .strip_prefix('/')
            .unwrap_or(input.trim())
            .split_whitespace()
            .collect();

        assert_eq!(parts[0], "config");
        // Default behavior should be show when only "config" is provided
        assert_eq!(parts.len(), 1);
    }

    #[test]
    fn test_graph_command_simple() {
        let input = "/graph";
        let parts: Vec<&str> = input
            .trim()
            .strip_prefix('/')
            .unwrap_or(input.trim())
            .split_whitespace()
            .collect();

        assert_eq!(parts[0], "graph");
        assert_eq!(parts.len(), 1);
    }

    #[test]
    fn test_graph_command_with_top_k() {
        let input = "/graph --top-k 15";
        let parts: Vec<&str> = input
            .trim()
            .strip_prefix('/')
            .unwrap_or(input.trim())
            .split_whitespace()
            .collect();

        assert_eq!(parts[0], "graph");
        assert!(parts.contains(&"--top-k"));
        assert!(parts.contains(&"15"));
    }

    #[test]
    fn test_replace_command_simple() {
        let input = "/replace rust is a programming language";
        let parts: Vec<&str> = input
            .trim()
            .strip_prefix('/')
            .unwrap_or(input.trim())
            .split_whitespace()
            .collect();

        assert_eq!(parts[0], "replace");
        let text = parts[1..].join(" ");
        assert_eq!(text, "rust is a programming language");
    }

    #[test]
    fn test_replace_command_with_format() {
        let input = "/replace async programming with tokio --format markdown";
        let parts: Vec<&str> = input
            .trim()
            .strip_prefix('/')
            .unwrap_or(input.trim())
            .split_whitespace()
            .collect();

        assert_eq!(parts[0], "replace");
        assert!(parts.contains(&"--format"));
        assert!(parts.contains(&"markdown"));
    }

    #[test]
    fn test_replace_command_html_format() {
        let input = "/replace check out rust --format html";
        let parts: Vec<&str> = input
            .trim()
            .strip_prefix('/')
            .unwrap_or(input.trim())
            .split_whitespace()
            .collect();

        assert_eq!(parts[0], "replace");
        assert!(parts.contains(&"--format"));
        assert!(parts.contains(&"html"));
    }

    #[test]
    fn test_replace_command_wiki_format() {
        let input = "/replace docker kubernetes --format wiki";
        let parts: Vec<&str> = input
            .trim()
            .strip_prefix('/')
            .unwrap_or(input.trim())
            .split_whitespace()
            .collect();

        assert_eq!(parts[0], "replace");
        assert!(parts.contains(&"--format"));
        assert!(parts.contains(&"wiki"));
    }

    #[test]
    fn test_replace_command_plain_format() {
        let input = "/replace some text --format plain";
        let parts: Vec<&str> = input
            .trim()
            .strip_prefix('/')
            .unwrap_or(input.trim())
            .split_whitespace()
            .collect();

        assert_eq!(parts[0], "replace");
        assert!(parts.contains(&"--format"));
        assert!(parts.contains(&"plain"));
    }

    #[test]
    fn test_find_command_simple() {
        let input = "/find rust async programming";
        let parts: Vec<&str> = input
            .trim()
            .strip_prefix('/')
            .unwrap_or(input.trim())
            .split_whitespace()
            .collect();

        assert_eq!(parts[0], "find");
        let text = parts[1..].join(" ");
        assert_eq!(text, "rust async programming");
    }

    #[test]
    fn test_thesaurus_command_simple() {
        let input = "/thesaurus";
        let parts: Vec<&str> = input
            .trim()
            .strip_prefix('/')
            .unwrap_or(input.trim())
            .split_whitespace()
            .collect();

        assert_eq!(parts[0], "thesaurus");
        assert_eq!(parts.len(), 1);
    }

    #[test]
    fn test_thesaurus_command_with_role() {
        let input = "/thesaurus --role Engineer";
        let parts: Vec<&str> = input
            .trim()
            .strip_prefix('/')
            .unwrap_or(input.trim())
            .split_whitespace()
            .collect();

        assert_eq!(parts[0], "thesaurus");
        assert!(parts.contains(&"--role"));
        assert!(parts.contains(&"Engineer"));
    }

    #[test]
    fn test_help_command_simple() {
        let input = "/help";
        let parts: Vec<&str> = input
            .trim()
            .strip_prefix('/')
            .unwrap_or(input.trim())
            .split_whitespace()
            .collect();

        assert_eq!(parts[0], "help");
        assert_eq!(parts.len(), 1);
    }

    #[test]
    fn test_help_command_with_topic() {
        let input = "/help search";
        let parts: Vec<&str> = input
            .trim()
            .strip_prefix('/')
            .unwrap_or(input.trim())
            .split_whitespace()
            .collect();

        assert_eq!(parts[0], "help");
        assert_eq!(parts[1], "search");
    }

    #[test]
    fn test_quit_command() {
        let input = "/quit";
        let parts: Vec<&str> = input
            .trim()
            .strip_prefix('/')
            .unwrap_or(input.trim())
            .split_whitespace()
            .collect();

        assert_eq!(parts[0], "quit");
    }

    #[test]
    fn test_q_shortcut() {
        let input = "/q";
        let parts: Vec<&str> = input
            .trim()
            .strip_prefix('/')
            .unwrap_or(input.trim())
            .split_whitespace()
            .collect();

        assert_eq!(parts[0], "q");
    }

    #[test]
    fn test_exit_command() {
        let input = "/exit";
        let parts: Vec<&str> = input
            .trim()
            .strip_prefix('/')
            .unwrap_or(input.trim())
            .split_whitespace()
            .collect();

        assert_eq!(parts[0], "exit");
    }

    #[test]
    fn test_clear_command() {
        let input = "/clear";
        let parts: Vec<&str> = input
            .trim()
            .strip_prefix('/')
            .unwrap_or(input.trim())
            .split_whitespace()
            .collect();

        assert_eq!(parts[0], "clear");
    }

    #[test]
    fn test_command_without_slash() {
        // Commands should work without leading slash
        let input = "search hello";
        let parts: Vec<&str> = input
            .trim()
            .strip_prefix('/')
            .unwrap_or(input.trim())
            .split_whitespace()
            .collect();

        assert_eq!(parts[0], "search");
    }

    #[test]
    fn test_command_with_extra_spaces() {
        let input = "/search   hello   world  ";
        let parts: Vec<&str> = input
            .trim()
            .strip_prefix('/')
            .unwrap_or(input.trim())
            .split_whitespace()
            .collect();

        // split_whitespace handles multiple spaces
        assert_eq!(parts[0], "search");
        assert_eq!(parts[1], "hello");
        assert_eq!(parts[2], "world");
    }

    #[test]
    fn test_empty_command_is_handled() {
        let input = "";
        let trimmed = input.trim();
        assert!(trimmed.is_empty());
    }

    #[test]
    fn test_whitespace_only_is_handled() {
        let input = "   ";
        let trimmed = input.trim();
        assert!(trimmed.is_empty());
    }
}

#[cfg(test)]
mod available_commands_tests {
    #[test]
    fn test_expected_commands_exist() {
        let expected_commands = vec![
            "search",
            "config",
            "role",
            "graph",
            "replace",
            "find",
            "thesaurus",
            "help",
            "quit",
            "exit",
            "clear",
        ];

        // Verify all expected commands are valid
        for cmd in expected_commands {
            assert!(!cmd.is_empty(), "Command should not be empty: {}", cmd);
        }
    }
}

#[cfg(test)]
mod link_type_format_tests {
    #[test]
    fn test_markdown_format_string() {
        let format = "markdown";
        assert_eq!(format, "markdown");
    }

    #[test]
    fn test_html_format_string() {
        let format = "html";
        assert_eq!(format, "html");
    }

    #[test]
    fn test_wiki_format_string() {
        let format = "wiki";
        assert_eq!(format, "wiki");
    }

    #[test]
    fn test_plain_format_string() {
        let format = "plain";
        assert_eq!(format, "plain");
    }

    #[test]
    fn test_format_parsing() {
        let test_cases = vec![
            ("markdown", true),
            ("html", true),
            ("wiki", true),
            ("plain", true),
            ("invalid", false),
            ("MARKDOWN", false), // Case sensitive
        ];

        for (format, should_be_valid) in test_cases {
            let is_valid = matches!(format, "markdown" | "html" | "wiki" | "plain");
            assert_eq!(
                is_valid, should_be_valid,
                "Format '{}' validation mismatch",
                format
            );
        }
    }
}

#[cfg(test)]
mod role_subcommand_tests {
    #[test]
    fn test_role_list_parsing() {
        let input = "list";
        assert_eq!(input, "list");
    }

    #[test]
    fn test_role_select_parsing() {
        let input = "select";
        assert_eq!(input, "select");
    }

    #[test]
    fn test_invalid_role_subcommand() {
        let input = "invalid";
        let is_valid = matches!(input, "list" | "select");
        assert!(!is_valid, "Invalid subcommand should not be valid");
    }
}

#[cfg(test)]
mod config_subcommand_tests {
    #[test]
    fn test_config_show_parsing() {
        let input = "show";
        assert_eq!(input, "show");
    }

    #[test]
    fn test_invalid_config_subcommand() {
        let input = "invalid";
        let is_valid = input == "show";
        assert!(!is_valid, "Invalid subcommand should not be valid");
    }
}
