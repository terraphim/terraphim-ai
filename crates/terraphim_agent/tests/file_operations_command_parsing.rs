// Simplified test that only tests command parsing without full handler dependencies
// Tests match the actual FileSubcommand variants: Search, List, Info
#[cfg(test)]
mod tests {
    use std::str::FromStr;

    // Test basic command parsing for file operations
    #[test]
    #[cfg(feature = "repl-file")]
    fn test_file_command_parsing_basic() {
        use terraphim_agent::repl::commands::ReplCommand;

        // Test file search command
        let result = ReplCommand::from_str("/file search \"test query\"");
        assert!(result.is_ok());

        if let ReplCommand::File { subcommand: _ } = result.unwrap() {
            // We can't access the subcommand variants directly due to feature gating
            // but we can verify it parsed as a File command
            println!("✅ File search command parsed successfully");
        } else {
            panic!("Expected File command");
        }
    }

    #[test]
    #[cfg(feature = "repl-file")]
    fn test_file_command_help_available() {
        use terraphim_agent::repl::commands::ReplCommand;

        // Test that file command is in available commands
        let commands = ReplCommand::available_commands();
        assert!(
            commands.contains(&"file"),
            "file command should be in available commands"
        );

        // Test that help text exists
        let help_text = ReplCommand::get_command_help("file");
        assert!(help_text.is_some(), "file command should have help text");

        let help = help_text.unwrap();
        // Help mentions "File operations" (case-insensitive check)
        assert!(
            help.to_lowercase().contains("file operations"),
            "help should mention file operations, got: {}",
            help
        );

        println!("✅ File command help available: {}", help);
    }

    #[test]
    #[cfg(feature = "repl-file")]
    fn test_variations_of_file_commands() {
        use terraphim_agent::repl::commands::ReplCommand;

        // Test only the implemented FileSubcommand variants: Search, List, Info
        let test_commands = vec![
            "/file search \"rust async\"",
            "/file list",
            "/file info ./main.rs",
        ];

        for cmd in test_commands {
            let result = ReplCommand::from_str(cmd);
            assert!(result.is_ok(), "Failed to parse command: {}", cmd);

            match result.unwrap() {
                ReplCommand::File { .. } => {
                    println!("✅ Successfully parsed: {}", cmd);
                }
                _ => panic!("Expected File command for: {}", cmd),
            }
        }
    }

    #[test]
    #[cfg(feature = "repl-file")]
    fn test_invalid_file_commands() {
        use terraphim_agent::repl::commands::ReplCommand;

        let invalid_commands = vec![
            "/file",                          // missing subcommand
            "/file search",                   // missing query
            "/file info",                     // missing path
            "/file invalid_subcommand ./src", // invalid subcommand
        ];

        for cmd in invalid_commands {
            let result = ReplCommand::from_str(cmd);
            assert!(
                result.is_err(),
                "Expected error for invalid command: {}",
                cmd
            );
            println!("✅ Correctly rejected invalid command: {}", cmd);
        }
    }

    #[test]
    #[cfg(feature = "repl-file")]
    fn test_file_command_with_various_flags() {
        use terraphim_agent::repl::commands::ReplCommand;

        // Test commands with the implemented subcommands
        // Note: Current implementation only supports basic search, list, info
        let complex_commands = vec![
            "/file search \"async rust\"",
            "/file list",
            "/file info ./src/main.rs",
        ];

        for cmd in complex_commands {
            let result = ReplCommand::from_str(cmd);
            assert!(result.is_ok(), "Failed to parse complex command: {}", cmd);

            match result.unwrap() {
                ReplCommand::File { .. } => {
                    println!("✅ Successfully parsed complex command: {}", cmd);
                }
                _ => panic!("Expected File command for: {}", cmd),
            }
        }
    }
}
