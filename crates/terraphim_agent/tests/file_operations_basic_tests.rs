#[cfg(test)]
mod file_operations_tests {
    use std::str::FromStr;

    // Test file operations command parsing - this is the core functionality we need
    #[test]
    fn test_file_search_command_parsing() {
        #[cfg(feature = "repl-file")]
        {
            let result = terraphim_agent::repl::commands::ReplCommand::from_str(
                "/file search \"async rust\" --path ./src --semantic --limit 5",
            );
            assert!(result.is_ok());

            match result.unwrap() {
                terraphim_agent::repl::commands::ReplCommand::File { subcommand } => {
                    match subcommand {
                        terraphim_agent::repl::commands::FileSubcommand::Search {
                            query,
                            path,
                            file_types,
                            semantic,
                            limit,
                        } => {
                            assert_eq!(query, "async rust");
                            assert_eq!(path, Some("./src".to_string()));
                            assert_eq!(semantic, true);
                            assert_eq!(limit, Some(5));
                            assert!(file_types.is_none());
                        }
                        _ => panic!("Expected Search subcommand"),
                    }
                }
                _ => panic!("Expected File command"),
            }
        }
    }

    #[test]
    fn test_file_classify_command_parsing() {
        #[cfg(feature = "repl-file")]
        {
            let result = terraphim_agent::repl::commands::ReplCommand::from_str(
                "/file classify ./src --recursive --update-metadata",
            );
            assert!(result.is_ok());

            match result.unwrap() {
                terraphim_agent::repl::commands::ReplCommand::File { subcommand } => {
                    match subcommand {
                        terraphim_agent::repl::commands::FileSubcommand::Classify {
                            path,
                            recursive,
                            update_metadata,
                        } => {
                            assert_eq!(path, "./src");
                            assert_eq!(recursive, true);
                            assert_eq!(update_metadata, true);
                        }
                        _ => panic!("Expected Classify subcommand"),
                    }
                }
                _ => panic!("Expected File command"),
            }
        }
    }

    #[test]
    fn test_file_analyze_command_parsing() {
        #[cfg(feature = "repl-file")]
        {
            let result = terraphim_agent::repl::commands::ReplCommand::from_str(
                "/file analyze ./src/main.rs --classification --semantic --extract-entities",
            );
            assert!(result.is_ok());

            match result.unwrap() {
                terraphim_agent::repl::commands::ReplCommand::File { subcommand } => {
                    match subcommand {
                        terraphim_agent::repl::commands::FileSubcommand::Analyze {
                            file_path,
                            analysis_types,
                            config: _,
                        } => {
                            assert_eq!(file_path, "./src/main.rs");
                            assert!(analysis_types.len() >= 2); // At least classification and semantic
                        }
                        _ => panic!("Expected Analyze subcommand"),
                    }
                }
                _ => panic!("Expected File command"),
            }
        }
    }

    #[test]
    fn test_file_summarize_command_parsing() {
        #[cfg(feature = "repl-file")]
        {
            let result = terraphim_agent::repl::commands::ReplCommand::from_str(
                "/file summarize ./README.md --detailed --key-points",
            );
            assert!(result.is_ok());

            match result.unwrap() {
                terraphim_agent::repl::commands::ReplCommand::File { subcommand } => {
                    match subcommand {
                        terraphim_agent::repl::commands::FileSubcommand::Summarize {
                            file_path,
                            detail_level,
                            include_key_points,
                        } => {
                            assert_eq!(file_path, "./README.md");
                            assert_eq!(detail_level, Some("detailed".to_string()));
                            assert_eq!(include_key_points, true);
                        }
                        _ => panic!("Expected Summarize subcommand"),
                    }
                }
                _ => panic!("Expected File command"),
            }
        }
    }

    #[test]
    fn test_file_tag_command_parsing() {
        #[cfg(feature = "repl-file")]
        {
            let result = terraphim_agent::repl::commands::ReplCommand::from_str(
                "/file tag ./src/lib.rs rust,core,module --auto-suggest",
            );
            assert!(result.is_ok());

            match result.unwrap() {
                terraphim_agent::repl::commands::ReplCommand::File { subcommand } => {
                    match subcommand {
                        terraphim_agent::repl::commands::FileSubcommand::Tag {
                            file_path,
                            tags,
                            auto_suggest,
                        } => {
                            assert_eq!(file_path, "./src/lib.rs");
                            assert_eq!(tags, vec!["rust", "core", "module"]);
                            assert_eq!(auto_suggest, true);
                        }
                        _ => panic!("Expected Tag subcommand"),
                    }
                }
                _ => panic!("Expected File command"),
            }
        }
    }

    #[test]
    fn test_file_command_error_handling() {
        #[cfg(feature = "repl-file")]
        {
            // Test missing subcommand
            let result = terraphim_agent::repl::commands::ReplCommand::from_str("/file");
            assert!(result.is_err());

            // Test missing file path for search
            let result = terraphim_agent::repl::commands::ReplCommand::from_str("/file search");
            assert!(result.is_err());

            // Test missing file path for classify
            let result = terraphim_agent::repl::commands::ReplCommand::from_str("/file classify");
            assert!(result.is_err());

            // Test invalid subcommand
            let result = terraphim_agent::repl::commands::ReplCommand::from_str(
                "/file invalid_command ./src",
            );
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_file_command_available_in_help() {
        #[cfg(feature = "repl-file")]
        {
            // Test that file command is included in available commands
            let commands = terraphim_agent::repl::commands::ReplCommand::available_commands();
            assert!(commands.contains(&"file"));

            // Test that file command has help text
            let help_text = terraphim_agent::repl::commands::ReplCommand::get_command_help("file");
            assert!(help_text.is_some());
            let help_text = help_text.unwrap();
            assert!(help_text.contains("file operations"));
            assert!(help_text.contains("semantic"));
        }
    }

    #[test]
    fn test_all_file_subcommands_coverage() {
        #[cfg(feature = "repl-file")]
        {
            // Test that all file subcommands are properly parsed
            let test_cases = vec![
                "/file search \"test query\" --path ./src",
                "/file classify ./src --recursive",
                "/file suggest --context \"error handling\" --limit 10",
                "/file analyze ./src/main.rs --classification",
                "/file summarize ./README.md --brief",
                "/file metadata ./src/lib.rs --extract-concepts --extract-entities",
                "/file index ./docs --recursive --force-reindex",
                "/file find \"function_name\" --path ./src --type rs",
                "/file list ./src --show-metadata --show-tags --sort-by name",
                "/file tag ./src/main.rs rust,important --auto-suggest",
                "/file status indexing",
            ];

            for test_case in test_cases {
                let result = terraphim_agent::repl::commands::ReplCommand::from_str(test_case);
                assert!(result.is_ok(), "Failed to parse: {}", test_case);

                match result.unwrap() {
                    terraphim_agent::repl::commands::ReplCommand::File { .. } => {
                        // Expected
                    }
                    _ => panic!("Expected File command for: {}", test_case),
                }
            }
        }
    }
}
