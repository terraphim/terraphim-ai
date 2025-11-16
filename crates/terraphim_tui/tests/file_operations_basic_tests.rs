#[cfg(test)]
mod file_operations_tests {
    use std::str::FromStr;

    // Test file operations command parsing - this is the core functionality we need
    #[test]
    fn test_file_search_command_parsing() {
        #[cfg(feature = "repl-file")]
        {
            let result =
                terraphim_agent::repl::commands::ReplCommand::from_str("/file search \"async rust\"");
            assert!(result.is_ok());

            match result.unwrap() {
                terraphim_agent::repl::commands::ReplCommand::File { subcommand } => match subcommand
                {
                    terraphim_agent::repl::commands::FileSubcommand::Search { query } => {
                        assert_eq!(query, "\"async rust\"");
                    }
                    _ => panic!("Expected Search subcommand"),
                },
                _ => panic!("Expected File command"),
            }
        }
    }

    #[test]
    fn test_file_list_command_parsing() {
        #[cfg(feature = "repl-file")]
        {
            let result = terraphim_agent::repl::commands::ReplCommand::from_str("/file list");
            assert!(result.is_ok());

            match result.unwrap() {
                terraphim_agent::repl::commands::ReplCommand::File { subcommand } => match subcommand
                {
                    terraphim_agent::repl::commands::FileSubcommand::List => {
                        // List command has no fields
                    }
                    _ => panic!("Expected List subcommand"),
                },
                _ => panic!("Expected File command"),
            }
        }
    }

    #[test]
    fn test_file_info_command_parsing() {
        #[cfg(feature = "repl-file")]
        {
            let result =
                terraphim_agent::repl::commands::ReplCommand::from_str("/file info ./src/main.rs");
            assert!(result.is_ok());

            match result.unwrap() {
                terraphim_agent::repl::commands::ReplCommand::File { subcommand } => match subcommand
                {
                    terraphim_agent::repl::commands::FileSubcommand::Info { path } => {
                        assert_eq!(path, "./src/main.rs");
                    }
                    _ => panic!("Expected Info subcommand"),
                },
                _ => panic!("Expected File command"),
            }
        }
    }

    #[test]
    fn test_file_command_help_available() {
        #[cfg(feature = "repl-file")]
        {
            let commands = terraphim_agent::repl::commands::ReplCommand::available_commands();
            assert!(
                commands.iter().any(|cmd| cmd.contains("file")),
                "File command should be in available commands"
            );
        }
    }

    #[test]
    fn test_file_command_invalid_subcommand() {
        #[cfg(feature = "repl-file")]
        {
            let result =
                terraphim_agent::repl::commands::ReplCommand::from_str("/file invalid_subcommand");
            assert!(result.is_err(), "Expected error for invalid subcommand");
        }
    }

    #[test]
    fn test_file_command_no_args() {
        #[cfg(feature = "repl-file")]
        {
            let result = terraphim_agent::repl::commands::ReplCommand::from_str("/file");
            assert!(result.is_err(), "Expected error for no subcommand");
        }
    }

    // Test complex queries with spaces and quotes
    #[test]
    fn test_file_search_complex_query() {
        #[cfg(feature = "repl-file")]
        {
            let result = terraphim_agent::repl::commands::ReplCommand::from_str(
                "/file search \"async rust patterns\" --recursive",
            );
            // This should parse successfully, though we only extract the basic query
            assert!(result.is_ok());

            match result.unwrap() {
                terraphim_agent::repl::commands::ReplCommand::File { subcommand } => match subcommand
                {
                    terraphim_agent::repl::commands::FileSubcommand::Search { query } => {
                        assert_eq!(query, "\"async rust patterns\" --recursive");
                    }
                    _ => panic!("Expected Search subcommand"),
                },
                _ => panic!("Expected File command"),
            }
        }
    }
}
