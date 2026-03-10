use std::str::FromStr;

#[cfg(all(test, feature = "repl", feature = "repl-web"))]
mod tests {
    use super::*;

    // Test basic command parsing - this is the core functionality we need
    #[test]
    fn test_web_get_command_parsing() {
        // Since imports are problematic, let's test the FromStr implementation directly
        let result = terraphim_agent::repl::commands::ReplCommand::from_str(
            "/web get https://httpbin.org/get",
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_web_post_command_parsing() {
        let result = terraphim_agent::repl::commands::ReplCommand::from_str(
            "/web post https://httpbin.org/post '{\"test\": \"data\"}'",
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_web_scrape_command_parsing() {
        let result = terraphim_agent::repl::commands::ReplCommand::from_str(
            "/web scrape https://example.com '.content'",
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_web_screenshot_command_parsing() {
        let result = terraphim_agent::repl::commands::ReplCommand::from_str(
            "/web screenshot https://github.com",
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_web_pdf_command_parsing() {
        let result =
            terraphim_agent::repl::commands::ReplCommand::from_str("/web pdf https://example.com");
        assert!(result.is_ok());
    }

    #[test]
    fn test_web_form_command_parsing() {
        let result = terraphim_agent::repl::commands::ReplCommand::from_str(
            "/web form https://example.com/login '{\"username\": \"test\"}'",
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_web_api_command_parsing() {
        let result = terraphim_agent::repl::commands::ReplCommand::from_str(
            "/web api https://api.github.com /users/user1,/repos/repo1",
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_web_status_command_parsing() {
        let result = terraphim_agent::repl::commands::ReplCommand::from_str(
            "/web status webop-1642514400000",
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_web_cancel_command_parsing() {
        let result = terraphim_agent::repl::commands::ReplCommand::from_str(
            "/web cancel webop-1642514400000",
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_web_history_command_parsing() {
        let result = terraphim_agent::repl::commands::ReplCommand::from_str("/web history");
        assert!(result.is_ok());
    }

    #[test]
    fn test_web_config_show_command_parsing() {
        let result = terraphim_agent::repl::commands::ReplCommand::from_str("/web config show");
        assert!(result.is_ok());
    }

    #[test]
    fn test_web_config_set_command_parsing() {
        let result = terraphim_agent::repl::commands::ReplCommand::from_str(
            "/web config set timeout_ms 45000",
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_web_config_reset_command_parsing() {
        let result = terraphim_agent::repl::commands::ReplCommand::from_str("/web config reset");
        assert!(result.is_ok());
    }

    #[test]
    fn test_web_command_error_handling() {
        // Test missing subcommand
        let result = terraphim_agent::repl::commands::ReplCommand::from_str("/web");
        assert!(result.is_err());

        // Test missing URL for GET
        let result = terraphim_agent::repl::commands::ReplCommand::from_str("/web get");
        assert!(result.is_err());

        // POST only requires a URL; body is optional/empty
        let result =
            terraphim_agent::repl::commands::ReplCommand::from_str("/web post https://example.com");
        assert!(result.is_ok());

        // Test missing operation ID for status
        let result = terraphim_agent::repl::commands::ReplCommand::from_str("/web status");
        assert!(result.is_err());

        // Test invalid subcommand
        let result = terraphim_agent::repl::commands::ReplCommand::from_str("/web invalid_command");
        assert!(result.is_err());
    }

    #[test]
    fn test_web_command_available_in_help() {
        // Test that web command is included in available commands
        let commands = terraphim_agent::repl::commands::ReplCommand::available_commands();
        assert!(commands.contains(&"web"));

        // Test that web command has help text
        let help_text = terraphim_agent::repl::commands::ReplCommand::get_command_help("web");
        assert!(help_text.is_some());
        let help_text = help_text.unwrap();
        // Help text is feature-gated and may change; assert key substrings.
        assert!(help_text.contains("Web operations"));
        assert!(help_text.contains("get"));
    }

    #[test]
    fn test_all_web_subcommands_coverage() {
        // Test that all web subcommands are properly parsed
        let test_cases = vec![
            "/web get https://example.com",
            "/web post https://example.com data",
            "/web scrape https://example.com .content",
            "/web screenshot https://example.com",
            "/web pdf https://example.com",
            "/web form https://example.com {\"key\":\"value\"}",
            "/web api https://api.example.com /endpoint1",
            "/web status op123",
            "/web cancel op123",
            "/web history",
            "/web config show",
        ];

        for test_case in test_cases {
            let result = terraphim_agent::repl::commands::ReplCommand::from_str(test_case);
            assert!(result.is_ok(), "Failed to parse: {}", test_case);

            match result.unwrap() {
                terraphim_agent::repl::commands::ReplCommand::Web { .. } => {
                    // Expected
                }
                _ => panic!("Expected Web command for: {}", test_case),
            }
        }
    }
}
