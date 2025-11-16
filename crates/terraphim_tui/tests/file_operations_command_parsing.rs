// Simplified test that only tests command parsing without full handler dependencies
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

        if let ReplCommand::File { subcommand } = result.unwrap() {
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
        assert!(
            help.contains("file operations"),
            "help should mention file operations"
        );

        println!("✅ File command help available: {}", help);
    }

    #[test]
    #[cfg(feature = "repl-file")]
    fn test_variations_of_file_commands() {
        use terraphim_agent::repl::commands::ReplCommand;

        let test_commands = vec![
            "/file search \"rust async\"",
            "/file classify ./src",
            "/file analyze ./main.rs",
            "/file summarize ./README.md",
            "/file tag ./lib.rs rust,important",
            "/file metadata ./src/main.rs",
            "/file index ./docs",
            "/file find \"function\" --path ./src",
            "/file list ./src",
            "/file status indexing",
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
            "/file classify",                 // missing path
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

        let complex_commands = vec![
            "/file search \"async rust\" --path ./src --semantic --limit 10",
            "/file classify ./src --recursive --update-metadata",
            "/file analyze ./main.rs --classification --semantic --extract-entities --extract-concepts",
            "/file summarize ./README.md --detailed --key-points",
            "/file tag ./lib.rs rust,core,module --auto-suggest",
            "/file list ./src --show-metadata --show-tags --sort-by name",
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
