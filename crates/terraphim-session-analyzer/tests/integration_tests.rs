//! Integration tests for Claude Log Analyzer
//!
//! These tests cover the full pipeline: parsing JSONL session files,
//! extracting agent invocations and file operations, performing analysis,
//! and generating reports.

use anyhow::Result;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use tempfile::{tempdir, NamedTempFile};
use terraphim_session_analyzer::models::*;
use terraphim_session_analyzer::utils;
use terraphim_session_analyzer::{Analyzer, Reporter, SessionParser, TimelineEventType};

/// Test data directory path
#[allow(dead_code)]
fn test_data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("test_data")
}

/// Create a test session file with given content
#[allow(dead_code)]
fn create_test_session_file(content: &str) -> Result<NamedTempFile> {
    let mut file = NamedTempFile::new()?;
    writeln!(file, "{}", content)?;
    Ok(file)
}

/// Create a test directory structure with session files
fn create_test_session_directory() -> Result<tempfile::TempDir> {
    let temp_dir = tempdir()?;

    // Create a project subdirectory
    let project_dir = temp_dir.path().join("-home-alex-projects-test-project");
    fs::create_dir_all(&project_dir)?;

    // Create multiple session files
    let session1_content = include_str!("test_data/valid_session.jsonl");
    let session2_content = include_str!("test_data/agent_collaboration_session.jsonl");

    fs::write(
        project_dir.join("b325985c-5c1c-48f1-97e2-e3185bb55886.jsonl"),
        session1_content,
    )?;

    fs::write(
        project_dir.join("a123456b-7c8d-49e1-98f2-f4296cc66997.jsonl"),
        session2_content,
    )?;

    Ok(temp_dir)
}

#[cfg(test)]
mod parsing_tests {
    use super::*;

    #[test]
    fn test_parse_valid_session_file() {
        let content = include_str!("test_data/valid_session.jsonl");
        let file = create_test_session_file(content).unwrap();

        let parser = SessionParser::from_file(file.path()).unwrap();
        let (session_id, project_path, start_time, end_time) = parser.get_session_info();

        assert_eq!(session_id, "b325985c-5c1c-48f1-97e2-e3185bb55886");
        assert!(!project_path.is_empty());
        assert!(start_time.is_some());
        assert!(end_time.is_some());
        assert!(parser.entry_count() > 0);
    }

    #[test]
    fn test_parse_malformed_session_file() {
        let malformed_content = r#"{"invalid": "json without required fields"}
{"parentUuid":null,"sessionId":"test","timestamp":"invalid-timestamp","message":{"role":"user","content":"test"},"uuid":"test-uuid","type":"user","userType":"external","cwd":"/test","version":"1.0.0","gitBranch":""}
{"parentUuid":null,"sessionId":"test","timestamp":"2025-10-01T09:05:21.902Z","message":{"role":"user","content":"valid entry"},"uuid":"test-uuid-2","type":"user","userType":"external","cwd":"/test","version":"1.0.0","gitBranch":"","isSidechain":false}"#;

        let file = create_test_session_file(malformed_content).unwrap();
        let parser = SessionParser::from_file(file.path()).unwrap();

        // Should parse successfully - at least one valid entry should be parsed
        // The parser logs warnings for malformed entries but continues
        assert!(parser.entry_count() >= 1); // At least the valid entry should be parsed
    }

    #[test]
    fn test_parse_empty_session_file() {
        let file = create_test_session_file("").unwrap();
        let parser = SessionParser::from_file(file.path()).unwrap();

        assert_eq!(parser.entry_count(), 0);
    }

    #[test]
    fn test_parse_session_directory() {
        let temp_dir = create_test_session_directory().unwrap();
        let parsers = SessionParser::from_directory(temp_dir.path()).unwrap();

        assert_eq!(parsers.len(), 2);
        assert!(parsers.iter().any(|p| {
            let (session_id, _, _, _) = p.get_session_info();
            session_id == "b325985c-5c1c-48f1-97e2-e3185bb55886"
        }));
    }

    #[test]
    fn test_parse_nonexistent_file() {
        let result = SessionParser::from_file("/nonexistent/path/file.jsonl");
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod agent_identification_tests {
    use super::*;

    #[test]
    fn test_extract_agent_invocations() {
        let content = include_str!("test_data/task_invocations.jsonl");
        let file = create_test_session_file(content).unwrap();

        let parser = SessionParser::from_file(file.path()).unwrap();
        let agents = parser.extract_agent_invocations();

        assert!(!agents.is_empty());

        // Check specific agent types
        let agent_types: Vec<String> = agents.iter().map(|a| a.agent_type.clone()).collect();

        assert!(agent_types.contains(&"architect".to_string()));
        assert!(agent_types.contains(&"developer".to_string()));
        assert!(agent_types.contains(&"test-writer-fixer".to_string()));

        // Verify agent invocation structure
        let first_agent = &agents[0];
        assert!(!first_agent.task_description.is_empty());
        assert!(!first_agent.session_id.is_empty());
        assert!(!first_agent.parent_message_id.is_empty());
    }

    #[test]
    fn test_agent_context_lookup() {
        let content = include_str!("test_data/task_invocations.jsonl");
        let file = create_test_session_file(content).unwrap();

        let parser = SessionParser::from_file(file.path()).unwrap();

        // Test finding active agent context
        let agent_context = parser.find_active_agent("some-message-id");
        // This may be None if the specific message ID isn't in test data
        // but the function should not panic
        assert!(agent_context.is_some() || agent_context.is_none());
    }

    #[test]
    fn test_get_agent_types() {
        let content = include_str!("test_data/task_invocations.jsonl");
        let file = create_test_session_file(content).unwrap();

        let parser = SessionParser::from_file(file.path()).unwrap();
        let agent_types = parser.get_agent_types();

        assert!(!agent_types.is_empty());
        // Should be sorted
        for i in 1..agent_types.len() {
            assert!(agent_types[i - 1] <= agent_types[i]);
        }
    }
}

#[cfg(test)]
mod file_operation_tests {
    use super::*;

    #[test]
    fn test_extract_file_operations() {
        let content = include_str!("test_data/file_operations.jsonl");
        let file = create_test_session_file(content).unwrap();

        let parser = SessionParser::from_file(file.path()).unwrap();
        let file_ops = parser.extract_file_operations();

        assert!(!file_ops.is_empty());

        // Check operation types
        let operation_types: Vec<String> = file_ops
            .iter()
            .map(|op| format!("{:?}", op.operation))
            .collect();

        assert!(operation_types.contains(&"Read".to_string()));
        assert!(operation_types.contains(&"Write".to_string()));
        assert!(operation_types.contains(&"Edit".to_string()));

        // Verify file operation structure
        let first_op = &file_ops[0];
        assert!(!first_op.file_path.is_empty());
        assert!(!first_op.session_id.is_empty());
        assert!(!first_op.message_id.is_empty());
    }

    #[test]
    fn test_file_path_extraction() {
        // Test extract_file_path utility function
        let input = serde_json::json!({
            "file_path": "/home/user/test.rs",
            "content": "test content"
        });

        let path = extract_file_path(&input);
        assert_eq!(path, Some("/home/user/test.rs".to_string()));

        // Test MultiEdit case
        let multi_edit_input = serde_json::json!({
            "file_path": "/home/user/multi.rs",
            "edits": [
                {"old_string": "old", "new_string": "new"}
            ]
        });

        let path = extract_file_path(&multi_edit_input);
        assert_eq!(path, Some("/home/user/multi.rs".to_string()));

        // Test missing file path
        let no_path_input = serde_json::json!({
            "description": "No file path here"
        });

        let path = extract_file_path(&no_path_input);
        assert_eq!(path, None);
    }

    #[test]
    fn test_file_operation_types() {
        use std::str::FromStr;

        // Test FileOpType parsing
        assert!(matches!(
            FileOpType::from_str("Read").unwrap(),
            FileOpType::Read
        ));
        assert!(matches!(
            FileOpType::from_str("Write").unwrap(),
            FileOpType::Write
        ));
        assert!(matches!(
            FileOpType::from_str("Edit").unwrap(),
            FileOpType::Edit
        ));
        assert!(matches!(
            FileOpType::from_str("MultiEdit").unwrap(),
            FileOpType::MultiEdit
        ));
        assert!(matches!(
            FileOpType::from_str("Delete").unwrap(),
            FileOpType::Delete
        ));
        assert!(matches!(
            FileOpType::from_str("Glob").unwrap(),
            FileOpType::Glob
        ));
        assert!(matches!(
            FileOpType::from_str("Grep").unwrap(),
            FileOpType::Grep
        ));

        // Test invalid operation type
        assert!(FileOpType::from_str("InvalidOp").is_err());
    }
}

#[cfg(test)]
mod analysis_tests {
    use super::*;

    #[test]
    fn test_full_session_analysis() {
        let temp_dir = create_test_session_directory().unwrap();
        let analyzer = Analyzer::new(temp_dir.path()).unwrap();

        let analyses = analyzer.analyze(None).unwrap();
        assert!(!analyses.is_empty());

        let analysis = &analyses[0];

        // Verify analysis structure
        assert!(!analysis.session_id.is_empty());
        assert!(!analysis.project_path.is_empty());
        // duration_ms is u64, always >= 0

        // Check that agents and file operations are extracted
        if !analysis.agents.is_empty() {
            assert!(!analysis.agent_stats.is_empty());
        }
    }

    #[test]
    fn test_target_file_filtering() {
        let temp_dir = create_test_session_directory().unwrap();
        let analyzer = Analyzer::new(temp_dir.path()).unwrap();

        // Test filtering by specific file
        let analyses = analyzer.analyze(Some("test.rs")).unwrap();

        // All file operations should relate to the target file
        for analysis in &analyses {
            for file_op in &analysis.file_operations {
                assert!(file_op.file_path.contains("test.rs"));
            }
        }
    }

    #[test]
    fn test_agent_statistics_calculation() {
        let temp_dir = create_test_session_directory().unwrap();
        let analyzer = Analyzer::new(temp_dir.path()).unwrap();

        let analyses = analyzer.analyze(None).unwrap();

        for analysis in &analyses {
            for (agent_type, stats) in &analysis.agent_stats {
                assert_eq!(stats.agent_type, *agent_type);
                assert!(stats.total_invocations > 0);
                // files_touched is u64, always >= 0
                assert!(!stats.tools_used.is_empty() || stats.files_touched == 0);
            }
        }
    }

    #[test]
    fn test_file_attribution() {
        let temp_dir = create_test_session_directory().unwrap();
        let analyzer = Analyzer::new(temp_dir.path()).unwrap();

        let analyses = analyzer.analyze(None).unwrap();

        for analysis in &analyses {
            for (file_path, attributions) in &analysis.file_to_agents {
                assert!(!file_path.is_empty());

                // Total contribution should be approximately 100%
                let total_contribution: f32 = attributions
                    .iter()
                    .map(|attr| attr.contribution_percent)
                    .sum();

                if !attributions.is_empty() {
                    assert!(total_contribution > 90.0 && total_contribution <= 110.0);
                }

                // Each attribution should have valid data
                for attr in attributions {
                    assert!(!attr.agent_type.is_empty());
                    assert!(attr.contribution_percent >= 0.0);
                    assert!(attr.confidence_score >= 0.0 && attr.confidence_score <= 1.0);
                    assert!(!attr.operations.is_empty());
                }
            }
        }
    }

    #[test]
    fn test_collaboration_pattern_detection() {
        let content = include_str!("test_data/agent_collaboration_session.jsonl");
        let file = create_test_session_file(content).unwrap();

        let analyzer = Analyzer::new(file.path()).unwrap();
        let analyses = analyzer.analyze(None).unwrap();

        if !analyses.is_empty() {
            let analysis = &analyses[0];

            // Check if collaboration patterns are detected
            for pattern in &analysis.collaboration_patterns {
                assert!(!pattern.pattern_type.is_empty());
                assert!(!pattern.agents.is_empty());
                assert!(!pattern.description.is_empty());
                assert!(pattern.frequency > 0);
                assert!(pattern.confidence >= 0.0 && pattern.confidence <= 1.0);
            }
        }
    }

    #[test]
    fn test_analyzer_configuration() {
        let config = AnalyzerConfig {
            session_dirs: vec!["/test/dir".to_string()],
            agent_confidence_threshold: 0.8,
            file_attribution_window_ms: 600_000,
            exclude_patterns: vec!["*.tmp".to_string()],
        };

        let temp_dir = create_test_session_directory().unwrap();
        let analyzer = Analyzer::new(temp_dir.path()).unwrap().with_config(config);

        let analyses = analyzer.analyze(None).unwrap();

        // Files matching exclude patterns should not appear in results
        for analysis in &analyses {
            for file_path in analysis.file_to_agents.keys() {
                assert!(!file_path.ends_with(".tmp"));
            }
        }
    }

    #[test]
    fn test_summary_statistics() {
        let temp_dir = create_test_session_directory().unwrap();
        let analyzer = Analyzer::new(temp_dir.path()).unwrap();

        let summary = analyzer.get_summary_stats().unwrap();

        assert!(summary.total_sessions > 0);
        // total_agents is u64, always >= 0
        // total_files is u64, always >= 0
        // unique_agent_types is u64, always >= 0

        // Most active agents should be sorted by frequency
        for i in 1..summary.most_active_agents.len() {
            assert!(summary.most_active_agents[i - 1].1 >= summary.most_active_agents[i].1);
        }
    }
}

#[cfg(test)]
mod timeline_tests {
    use super::*;

    #[test]
    fn test_timeline_generation() {
        let content = include_str!("test_data/valid_session.jsonl");
        let file = create_test_session_file(content).unwrap();

        let parser = SessionParser::from_file(file.path()).unwrap();
        let timeline = parser.build_timeline();

        // Timeline should be sorted by timestamp
        for i in 1..timeline.len() {
            assert!(timeline[i - 1].timestamp <= timeline[i].timestamp);
        }

        // Check event types
        for event in &timeline {
            assert!(!event.description.is_empty());
            match event.event_type {
                TimelineEventType::AgentInvocation => {
                    assert!(event.agent.is_some());
                }
                TimelineEventType::FileOperation => {
                    assert!(event.file.is_some());
                }
                TimelineEventType::UserMessage => {
                    // User messages might not have agent/file context
                }
            }
        }
    }

    #[test]
    fn test_entries_in_window() {
        let content = include_str!("test_data/valid_session.jsonl");
        let file = create_test_session_file(content).unwrap();

        let parser = SessionParser::from_file(file.path()).unwrap();

        // Get session time bounds
        let (_, _, start_time, end_time) = parser.get_session_info();

        if let (Some(start), Some(end)) = (start_time, end_time) {
            let entries = parser.entries_in_window(start, end);
            assert!(entries.len() <= parser.entry_count());

            // All entries should be within the time window
            for entry in entries {
                if let Ok(timestamp) = parse_timestamp(&entry.timestamp) {
                    assert!(timestamp >= start && timestamp <= end);
                }
            }
        }
    }
}

#[cfg(test)]
mod reporting_tests {
    use super::*;

    #[test]
    fn test_json_export() {
        let temp_dir = create_test_session_directory().unwrap();
        let analyzer = Analyzer::new(temp_dir.path()).unwrap();
        let analyses = analyzer.analyze(None).unwrap();

        let reporter = Reporter::new();
        let json_output = reporter.to_json(&analyses).unwrap();

        // Should be valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&json_output).unwrap();
        assert!(parsed.is_array() || parsed.is_object());
    }

    #[test]
    fn test_csv_export() {
        let temp_dir = create_test_session_directory().unwrap();
        let analyzer = Analyzer::new(temp_dir.path()).unwrap();
        let analyses = analyzer.analyze(None).unwrap();

        let reporter = Reporter::new();
        let csv_output = reporter.to_csv(&analyses).unwrap();

        // Should contain CSV headers
        assert!(csv_output.contains("Session ID") || csv_output.contains("session_id"));

        // Should contain data rows
        let lines: Vec<&str> = csv_output.lines().collect();
        assert!(!lines.is_empty()); // At least header row
    }

    #[test]
    fn test_markdown_export() {
        let temp_dir = create_test_session_directory().unwrap();
        let analyzer = Analyzer::new(temp_dir.path()).unwrap();
        let analyses = analyzer.analyze(None).unwrap();

        let reporter = Reporter::new();
        let markdown_output = reporter.to_markdown(&analyses).unwrap();

        // Should contain Markdown formatting
        assert!(markdown_output.contains("#") || markdown_output.contains("*"));
    }

    #[test]
    fn test_terminal_output() {
        let temp_dir = create_test_session_directory().unwrap();
        let analyzer = Analyzer::new(temp_dir.path()).unwrap();
        let analyses = analyzer.analyze(None).unwrap();

        let reporter = Reporter::new().with_colors(false);

        // This should not panic
        reporter.print_terminal(&analyses);
    }
}

#[cfg(test)]
mod utility_tests {
    use super::*;

    #[test]
    fn test_is_session_file() {
        assert!(utils::is_session_file(
            "b325985c-5c1c-48f1-97e2-e3185bb55886.jsonl"
        ));
        assert!(!utils::is_session_file("regular-file.txt"));
        assert!(!utils::is_session_file("short.jsonl"));
        assert!(!utils::is_session_file(
            "too-long-filename-that-exceeds-42-chars.jsonl"
        ));
    }

    #[test]
    fn test_extract_project_name() {
        let path = "/home/alex/.claude/projects/-home-alex-projects-test-project/session.jsonl";
        let project = utils::extract_project_name(path);
        assert!(project.is_some());

        let project_name = project.unwrap();
        assert!(project_name.contains("/home/alex/projects"));
    }

    #[test]
    fn test_get_default_session_dir() {
        let dir = utils::get_default_session_dir();
        assert!(dir.is_some());

        let path = dir.unwrap();
        assert!(path.to_string_lossy().contains(".claude"));
        assert!(path.to_string_lossy().contains("projects"));
    }

    #[test]
    fn test_filter_by_project() {
        let temp_dir = create_test_session_directory().unwrap();
        let analyzer = Analyzer::new(temp_dir.path()).unwrap();
        let analyses = analyzer.analyze(None).unwrap();

        if !analyses.is_empty() {
            let filtered = utils::filter_by_project(&analyses, "test-project");

            // All filtered results should contain the filter term
            for analysis in filtered {
                assert!(analysis.project_path.contains("test-project"));
            }
        }
    }

    #[test]
    fn test_get_unique_agents() {
        let temp_dir = create_test_session_directory().unwrap();
        let analyzer = Analyzer::new(temp_dir.path()).unwrap();
        let analyses = analyzer.analyze(None).unwrap();

        let unique_agents = utils::get_unique_agents(&analyses);

        // Should be sorted and unique
        for i in 1..unique_agents.len() {
            assert!(unique_agents[i - 1] <= unique_agents[i]);
        }

        // Check for duplicates
        let mut seen = std::collections::HashSet::new();
        for agent in &unique_agents {
            assert!(seen.insert(agent.clone()));
        }
    }

    #[test]
    fn test_agent_utilities() {
        // Test normalize_agent_name
        assert_eq!(
            normalize_agent_name("rust-performance-expert"),
            "rust_performance_expert"
        );
        assert_eq!(
            normalize_agent_name("Backend Architect"),
            "backend_architect"
        );

        // Test get_agent_category
        assert_eq!(get_agent_category("architect"), "architecture");
        assert_eq!(get_agent_category("rust-performance-expert"), "rust-expert");
        assert_eq!(get_agent_category("debugger"), "testing");
        assert_eq!(get_agent_category("technical-writer"), "documentation");
        assert_eq!(get_agent_category("unknown-agent"), "other");
    }

    #[test]
    fn test_session_utility_functions() {
        let temp_dir = create_test_session_directory().unwrap();
        let analyzer = Analyzer::new(temp_dir.path()).unwrap();
        let analyses = analyzer.analyze(None).unwrap();

        if !analyses.is_empty() {
            // Test total_session_time
            let _total_time = utils::total_session_time(&analyses);
            // total_time is u64, always >= 0

            // Test most_productive_session
            let most_productive = utils::most_productive_session(&analyses);
            assert!(most_productive.is_some());

            // Test sessions_with_agent
            let architect_sessions = utils::sessions_with_agent(&analyses, "architect");
            for session in architect_sessions {
                assert!(session.agents.iter().any(|a| a.agent_type == "architect"));
            }
        }
    }

    #[test]
    fn test_timestamp_parsing() {
        // Test valid timestamps
        let valid_timestamps = vec![
            "2025-10-01T09:05:21.902Z",
            "2025-10-01T14:30:45.123Z",
            "2025-12-31T23:59:59.999Z",
        ];

        for timestamp_str in valid_timestamps {
            let result = parse_timestamp(timestamp_str);
            assert!(
                result.is_ok(),
                "Failed to parse timestamp: {}",
                timestamp_str
            );
        }

        // Test invalid timestamps
        let invalid_timestamps = vec![
            "invalid-timestamp",
            "2025-13-01T09:05:21.902Z", // Invalid month
            "not-a-date",
            "",
        ];

        for timestamp_str in invalid_timestamps {
            let result = parse_timestamp(timestamp_str);
            assert!(
                result.is_err(),
                "Should have failed to parse: {}",
                timestamp_str
            );
        }
    }
}

#[cfg(test)]
mod cli_tests {
    use super::*;
    use std::process::Command;

    #[test]
    fn test_cli_help_command() {
        let output = Command::new("cargo")
            .args(["run", "--bin", "tsa", "--", "--help"])
            .output()
            .expect("Failed to execute CLI help command");

        assert!(output.status.success());
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(stdout.contains("Terraphim Session Analyzer"));
        assert!(stdout.contains("analyze"));
        assert!(stdout.contains("list"));
    }

    #[test]
    fn test_cli_version_command() {
        let output = Command::new("cargo")
            .args(["run", "--bin", "tsa", "--", "--version"])
            .output()
            .expect("Failed to execute CLI version command");

        assert!(output.status.success());
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(stdout.contains("terraphim-session-analyzer"));
    }

    #[test]
    fn test_cli_analyze_with_invalid_path() {
        let output = Command::new("cargo")
            .args(["run", "--bin", "cla", "--", "analyze", "/nonexistent/path"])
            .output()
            .expect("Failed to execute CLI analyze command");

        // Should exit with error for nonexistent path
        assert!(!output.status.success());
    }

    #[test]
    fn test_cli_analyze_with_test_data() {
        let temp_dir = create_test_session_directory().unwrap();

        let output = Command::new("cargo")
            .args([
                "run",
                "--bin",
                "cla",
                "--",
                "analyze",
                temp_dir.path().to_str().unwrap(),
                "--format",
                "json",
            ])
            .output()
            .expect("Failed to execute CLI analyze command");

        let stdout = String::from_utf8(output.stdout).unwrap();
        let stderr = String::from_utf8(output.stderr).unwrap();

        if output.status.success() {
            // Check if there's any JSON output
            if !stdout.trim().is_empty() {
                // Find the start of JSON content
                let lines: Vec<&str> = stdout.lines().collect();
                let mut json_start = None;

                for (i, line) in lines.iter().enumerate() {
                    if line.trim().starts_with('[') || line.trim().starts_with('{') {
                        json_start = Some(i);
                        break;
                    }
                }

                if let Some(start_idx) = json_start {
                    let json_content = lines[start_idx..].join("\n");
                    let result = serde_json::from_str::<serde_json::Value>(&json_content);
                    if let Err(e) = result {
                        println!("JSON parse error: {}", e);
                        println!(
                            "JSON content starts at line {}: '{}'",
                            start_idx, json_content
                        );
                        panic!("CLI should produce valid JSON");
                    }
                } else {
                    println!("No JSON found in output");
                }
            } else {
                // Empty output is acceptable if no sessions are found
                println!("CLI produced no output (likely no sessions found)");
            }
        } else {
            // Print error information for debugging
            println!("CLI command failed with exit code: {}", output.status);
            println!("Stderr: {}", stderr);
            println!("Stdout: {}", stdout);
            panic!("CLI command should succeed");
        }
    }
}

#[cfg(test)]
mod error_handling_tests {
    use super::*;

    #[test]
    fn test_malformed_json_handling() {
        let malformed_content = r#"{"incomplete": "json"
{"valid": "entry", "parentUuid":null,"sessionId":"test","timestamp":"2025-10-01T09:05:21.902Z","message":{"role":"user","content":"test"},"uuid":"test-uuid","type":"user","userType":"external","cwd":"/test","version":"1.0.0","gitBranch":"","isSidechain":false}
invalid line here
{"another": "valid", "parentUuid":null,"sessionId":"test","timestamp":"2025-10-01T09:05:21.902Z","message":{"role":"user","content":"test2"},"uuid":"test-uuid-2","type":"user","userType":"external","cwd":"/test","version":"1.0.0","gitBranch":"","isSidechain":false}"#;

        let file = create_test_session_file(malformed_content).unwrap();
        let parser = SessionParser::from_file(file.path()).unwrap();

        // Should handle malformed entries gracefully - parse valid entries and skip invalid ones
        assert!(parser.entry_count() >= 1); // Should parse at least one valid entry
    }

    #[test]
    fn test_missing_required_fields() {
        let incomplete_content = r#"
{"parentUuid":null,"sessionId":"test","message":{"role":"user","content":"test"},"uuid":"test-uuid","type":"user"}
{"parentUuid":null,"timestamp":"2025-10-01T09:05:21.902Z","message":{"role":"user","content":"test"},"uuid":"test-uuid-2","type":"user"}
        "#;

        let file = create_test_session_file(incomplete_content).unwrap();

        // Should handle missing required fields gracefully
        let result = SessionParser::from_file(file.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_invalid_tool_inputs() {
        let invalid_tool_content = r#"
{"parentUuid":"parent-uuid","sessionId":"test-session","timestamp":"2025-10-01T09:05:21.902Z","message":{"role":"assistant","content":[{"type":"tool_use","id":"tool-id","name":"Task","input":{"invalid_field":"no_subagent_type"}}]},"type":"assistant","uuid":"msg-uuid"}
{"parentUuid":"parent-uuid","sessionId":"test-session","timestamp":"2025-10-01T09:05:21.902Z","message":{"role":"assistant","content":[{"type":"tool_use","id":"tool-id","name":"UnknownTool","input":{"file_path":"/test.rs"}}]},"type":"assistant","uuid":"msg-uuid-2"}
        "#;

        let file = create_test_session_file(invalid_tool_content).unwrap();
        let parser = SessionParser::from_file(file.path()).unwrap();

        // Should handle invalid tool inputs gracefully
        let agents = parser.extract_agent_invocations();
        let file_ops = parser.extract_file_operations();

        // Should not crash, but might have empty results
        assert!(agents.is_empty()); // No valid agent invocations
        assert!(file_ops.is_empty()); // No valid file operations
    }

    #[test]
    fn test_analyzer_with_empty_sessions() {
        let temp_dir = tempdir().unwrap();

        // Create empty session file
        let empty_session = temp_dir.path().join("empty.jsonl");
        fs::write(&empty_session, "").unwrap();

        let analyzer = Analyzer::new(&empty_session).unwrap();
        let analyses = analyzer.analyze(None).unwrap();

        // Should handle empty sessions gracefully
        assert!(analyses.len() <= 1);
        if !analyses.is_empty() {
            let analysis = &analyses[0];
            assert!(analysis.agents.is_empty());
            assert!(analysis.file_operations.is_empty());
        }
    }

    #[test]
    fn test_reporter_with_empty_data() {
        let reporter = Reporter::new();
        let empty_analyses = vec![];

        // Should not panic with empty data
        let json_result = reporter.to_json(&empty_analyses);
        assert!(json_result.is_ok());

        let csv_result = reporter.to_csv(&empty_analyses);
        assert!(csv_result.is_ok());

        let markdown_result = reporter.to_markdown(&empty_analyses);
        assert!(markdown_result.is_ok());

        // Terminal output should also not panic
        reporter.print_terminal(&empty_analyses);
    }

    #[test]
    fn test_agent_file_attribution_for_revised_status_estimates() {
        let temp_dir = tempdir().unwrap();

        // Create a realistic session file that mimics the structure of
        // session 05db0b56-9c09-4715-b597-0c12077274d3 where REVISED_STATUS_IMPLEMENTATION_ESTIMATES.md was created
        let session_file = temp_dir
            .path()
            .join("05db0b56-9c09-4715-b597-0c12077274d3.jsonl");

        let session_content = [
            "{\"parentUuid\":null,\"isSidechain\":false,\"userType\":\"external\",\"cwd\":\"/home/alex/projects/zestic-at/charm\",\"sessionId\":\"05db0b56-9c09-4715-b597-0c12077274d3\",\"version\":\"1.0.111\",\"gitBranch\":\"\",\"type\":\"user\",\"message\":{\"role\":\"user\",\"content\":\"Create comprehensive status management estimates\"},\"uuid\":\"user-message-1\",\"timestamp\":\"2025-10-01T10:00:00.000Z\"}",
            "{\"parentUuid\":\"user-message-1\",\"isSidechain\":false,\"userType\":\"external\",\"cwd\":\"/home/alex/projects/zestic-at/charm\",\"sessionId\":\"05db0b56-9c09-4715-b597-0c12077274d3\",\"version\":\"1.0.111\",\"gitBranch\":\"\",\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",\"content\":[{\"type\":\"tool_use\",\"id\":\"architect-task-1\",\"name\":\"Task\",\"input\":{\"subagent_type\":\"architect\",\"description\":\"Analyze requirements\",\"prompt\":\"Create detailed status management estimates\"}}]},\"uuid\":\"architect-invoke-1\",\"timestamp\":\"2025-10-01T10:01:00.000Z\"}",
            "{\"parentUuid\":\"architect-invoke-1\",\"isSidechain\":false,\"userType\":\"external\",\"cwd\":\"/home/alex/projects/zestic-at/charm\",\"sessionId\":\"05db0b56-9c09-4715-b597-0c12077274d3\",\"version\":\"1.0.111\",\"gitBranch\":\"\",\"type\":\"user\",\"message\":{\"role\":\"user\",\"content\":[{\"tool_use_id\":\"architect-task-1\",\"type\":\"tool_result\",\"content\":[{\"type\":\"text\",\"text\":\"Analysis complete. Creating detailed estimates document.\"}]}]},\"uuid\":\"architect-result-1\",\"timestamp\":\"2025-10-01T10:05:00.000Z\"}",
            "{\"parentUuid\":\"architect-result-1\",\"isSidechain\":false,\"userType\":\"external\",\"cwd\":\"/home/alex/projects/zestic-at/charm\",\"sessionId\":\"05db0b56-9c09-4715-b597-0c12077274d3\",\"version\":\"1.0.111\",\"gitBranch\":\"\",\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",\"content\":[{\"type\":\"tool_use\",\"id\":\"write-estimates\",\"name\":\"Write\",\"input\":{\"file_path\":\"/home/alex/projects/zestic-at/charm/REVISED_STATUS_IMPLEMENTATION_ESTIMATES.md\",\"content\":\"Test Document\"}}]},\"uuid\":\"write-message-1\",\"timestamp\":\"2025-10-01T10:06:00.000Z\"}",
            "{\"parentUuid\":\"write-message-1\",\"isSidechain\":false,\"userType\":\"external\",\"cwd\":\"/home/alex/projects/zestic-at/charm\",\"sessionId\":\"05db0b56-9c09-4715-b597-0c12077274d3\",\"version\":\"1.0.111\",\"gitBranch\":\"\",\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",\"content\":[{\"type\":\"tool_use\",\"id\":\"studio-task-1\",\"name\":\"Task\",\"input\":{\"subagent_type\":\"studio-producer\",\"description\":\"Review estimates\",\"prompt\":\"Review the status implementation estimates for feasibility\"}}]},\"uuid\":\"producer-invoke-1\",\"timestamp\":\"2025-10-01T10:07:00.000Z\"}"
        ].join("\n");

        fs::write(&session_file, session_content).unwrap();

        // Test 1: Analyze without target - should show all files
        let analyzer = Analyzer::new(&session_file).unwrap();
        let all_analyses = analyzer.analyze(None).unwrap();

        // Should find the session and file operations
        assert!(!all_analyses.is_empty(), "Should find session data");
        let analysis = &all_analyses[0];
        assert!(
            !analysis.file_operations.is_empty(),
            "Should find file operations"
        );
        assert!(!analysis.agents.is_empty(), "Should find agent invocations");

        // Should find at least one file operation for REVISED_STATUS_IMPLEMENTATION_ESTIMATES.md
        let target_file_ops: Vec<_> = analysis
            .file_operations
            .iter()
            .filter(|op| {
                op.file_path
                    .contains("REVISED_STATUS_IMPLEMENTATION_ESTIMATES.md")
            })
            .collect();
        assert!(
            !target_file_ops.is_empty(),
            "Should find file operations for REVISED_STATUS_IMPLEMENTATION_ESTIMATES.md"
        );

        // Test 2: Analyze with target - should ONLY show results for that specific file
        let target_analyses = analyzer
            .analyze(Some("REVISED_STATUS_IMPLEMENTATION_ESTIMATES.md"))
            .unwrap();

        // CRITICAL: When targeting a specific file, should only show results if that file was found
        if !target_analyses.is_empty() {
            let target_analysis = &target_analyses[0];

            // Should ONLY contain file operations for the target file
            assert!(
                target_analysis.file_operations.iter().all(|op| op
                    .file_path
                    .contains("REVISED_STATUS_IMPLEMENTATION_ESTIMATES.md")),
                "When targeting a specific file, should only show operations for that file"
            );

            // Should contain the agents that worked on this file
            assert!(
                !target_analysis.agents.is_empty(),
                "Should find agents that worked on the target file"
            );

            // Should contain proper agent attribution
            let architect_agents: Vec<_> = target_analysis
                .agents
                .iter()
                .filter(|a| a.agent_type == "architect")
                .collect();
            assert!(
                !architect_agents.is_empty(),
                "Should find architect agent that created the file"
            );

            // Test file-to-agent attribution
            assert!(
                target_analysis
                    .file_to_agents
                    .contains_key("REVISED_STATUS_IMPLEMENTATION_ESTIMATES.md")
                    || target_analysis
                        .file_to_agents
                        .keys()
                        .any(|k| k.contains("REVISED_STATUS_IMPLEMENTATION_ESTIMATES.md")),
                "Should have file-to-agent attribution for the target file"
            );
        } else {
            panic!("Should find results when targeting REVISED_STATUS_IMPLEMENTATION_ESTIMATES.md");
        }

        // Test 3: Target file filtering precision
        let non_existent_analyses = analyzer.analyze(Some("NON_EXISTENT_FILE.md")).unwrap();

        // Should return empty or no results for non-existent files
        if !non_existent_analyses.is_empty() {
            let non_existent_analysis = &non_existent_analyses[0];
            assert!(
                non_existent_analysis.file_operations.is_empty()
                    || non_existent_analysis
                        .file_operations
                        .iter()
                        .all(|op| !op.file_path.contains("NON_EXISTENT_FILE.md")),
                "Should not show results for non-existent target files"
            );
        }
    }
}
