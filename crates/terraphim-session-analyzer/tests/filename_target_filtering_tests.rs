//! Comprehensive tests for filename-based target filtering in Claude Log Analyzer
//!
//! This test suite verifies that the analyzer correctly handles filename-based target filtering,
//! including:
//! - Basic filename matching
//! - Edge cases (partial matching, case sensitivity, similar names)
//! - File attribution and collaboration patterns
//! - CLI integration
//! - Empty results for non-matching files

use anyhow::Result;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use tempfile::{tempdir, NamedTempFile};
use terraphim_session_analyzer::{Analyzer, Reporter};

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

/// Create a test directory structure with the target filtering session files
fn create_target_filtering_test_directory() -> Result<tempfile::TempDir> {
    let temp_dir = tempdir()?;

    // Create a project subdirectory
    let project_dir = temp_dir
        .path()
        .join("-home-alex-projects-status-implementation");
    fs::create_dir_all(&project_dir)?;

    // Load our custom test session files
    let session1_content = include_str!("test_data/filename_target_filtering_session1.jsonl");
    let session2_content = include_str!("test_data/filename_target_filtering_session2.jsonl");
    let session3_content = include_str!("test_data/filename_target_filtering_session3.jsonl");

    fs::write(
        project_dir.join("filename-filter-session-001.jsonl"),
        session1_content,
    )?;

    fs::write(
        project_dir.join("filename-filter-session-002.jsonl"),
        session2_content,
    )?;

    // Create a different project directory for session 3
    let other_project_dir = temp_dir
        .path()
        .join("-home-alex-projects-different-project");
    fs::create_dir_all(&other_project_dir)?;

    fs::write(
        other_project_dir.join("filename-filter-session-003.jsonl"),
        session3_content,
    )?;

    Ok(temp_dir)
}

#[cfg(test)]
mod basic_filename_matching_tests {
    use super::*;

    #[test]
    fn test_exact_filename_matching_status_implementation() {
        let temp_dir = create_target_filtering_test_directory().unwrap();
        let analyzer = Analyzer::new(temp_dir.path()).unwrap();

        // Test exact filename matching for STATUS_IMPLEMENTATION.md
        let analyses = analyzer.analyze(Some("STATUS_IMPLEMENTATION.md")).unwrap();

        // Should find sessions that worked on this file
        assert!(
            !analyses.is_empty(),
            "Should find sessions with STATUS_IMPLEMENTATION.md operations"
        );

        // Verify that all file operations in results are related to the target file
        for analysis in &analyses {
            if !analysis.file_operations.is_empty() {
                for file_op in &analysis.file_operations {
                    assert!(
                        file_op.file_path.contains("STATUS_IMPLEMENTATION.md"),
                        "File operation should be related to STATUS_IMPLEMENTATION.md, found: {}",
                        file_op.file_path
                    );
                }
            }
        }

        // Should have file-to-agent attributions for the target file
        let mut found_target_file = false;
        for analysis in &analyses {
            for (file_path, attributions) in &analysis.file_to_agents {
                if file_path.contains("STATUS_IMPLEMENTATION.md") {
                    found_target_file = true;
                    assert!(
                        !attributions.is_empty(),
                        "Should have agent attributions for target file"
                    );

                    // Verify attribution structure
                    for attr in attributions {
                        assert!(
                            !attr.agent_type.is_empty(),
                            "Agent type should not be empty"
                        );
                        assert!(
                            attr.contribution_percent > 0.0,
                            "Contribution should be greater than 0"
                        );
                        assert!(
                            attr.confidence_score >= 0.0 && attr.confidence_score <= 1.0,
                            "Confidence should be between 0 and 1"
                        );
                        assert!(!attr.operations.is_empty(), "Should have operations listed");
                    }
                }
            }
        }

        assert!(found_target_file, "Should find target file in attributions");
    }

    #[test]
    fn test_exact_filename_matching_revised_estimates() {
        let temp_dir = create_target_filtering_test_directory().unwrap();
        let analyzer = Analyzer::new(temp_dir.path()).unwrap();

        // Test exact filename matching for REVISED_STATUS_IMPLEMENTATION_ESTIMATES.md
        let analyses = analyzer
            .analyze(Some("REVISED_STATUS_IMPLEMENTATION_ESTIMATES.md"))
            .unwrap();

        // Should find sessions that worked on this file
        assert!(
            !analyses.is_empty(),
            "Should find sessions with REVISED_STATUS_IMPLEMENTATION_ESTIMATES.md operations"
        );

        // Verify file operations are related to target
        for analysis in &analyses {
            if !analysis.file_operations.is_empty() {
                for file_op in &analysis.file_operations {
                    assert!(
                        file_op.file_path.contains("REVISED_STATUS_IMPLEMENTATION_ESTIMATES.md"),
                        "File operation should be related to REVISED_STATUS_IMPLEMENTATION_ESTIMATES.md, found: {}",
                        file_op.file_path
                    );
                }
            }
        }
    }

    #[test]
    fn test_partial_filename_matching() {
        let temp_dir = create_target_filtering_test_directory().unwrap();
        let analyzer = Analyzer::new(temp_dir.path()).unwrap();

        // Test partial filename matching - should find both files
        let analyses = analyzer.analyze(Some("STATUS_IMPLEMENTATION")).unwrap();

        assert!(
            !analyses.is_empty(),
            "Should find sessions with STATUS_IMPLEMENTATION* files"
        );

        // Should find both STATUS_IMPLEMENTATION.md and REVISED_STATUS_IMPLEMENTATION_ESTIMATES.md
        let mut found_main_file = false;
        let mut found_estimates_file = false;

        for analysis in &analyses {
            for file_op in &analysis.file_operations {
                if file_op.file_path.contains("STATUS_IMPLEMENTATION.md")
                    && !file_op.file_path.contains("REVISED")
                    && !file_op.file_path.contains("ESTIMATES")
                {
                    found_main_file = true;
                }
                if file_op
                    .file_path
                    .contains("REVISED_STATUS_IMPLEMENTATION_ESTIMATES.md")
                {
                    found_estimates_file = true;
                }
            }
        }

        assert!(
            found_main_file,
            "Should find operations on main STATUS_IMPLEMENTATION.md file"
        );
        assert!(
            found_estimates_file,
            "Should find operations on REVISED_STATUS_IMPLEMENTATION_ESTIMATES.md file"
        );
    }

    #[test]
    fn test_nonexistent_filename_returns_empty() {
        let temp_dir = create_target_filtering_test_directory().unwrap();
        let analyzer = Analyzer::new(temp_dir.path()).unwrap();

        // Test with a filename that doesn't exist in any session
        let analyses = analyzer.analyze(Some("NONEXISTENT_FILE.md")).unwrap();

        // Should have sessions but no file operations matching the target
        for analysis in &analyses {
            assert!(
                analysis.file_operations.is_empty(),
                "Should have no file operations for nonexistent file"
            );
        }
    }
}

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_case_sensitivity() {
        let temp_dir = create_target_filtering_test_directory().unwrap();
        let analyzer = Analyzer::new(temp_dir.path()).unwrap();

        // Test with different case - should still match (depending on implementation)
        let analyses_lower = analyzer.analyze(Some("status_implementation.md")).unwrap();
        let _analyses_upper = analyzer.analyze(Some("STATUS_IMPLEMENTATION.MD")).unwrap();
        let _analyses_mixed = analyzer.analyze(Some("Status_Implementation.md")).unwrap();

        // Note: Current implementation uses contains() which is case-sensitive
        // So these should return empty results
        for analysis in &analyses_lower {
            for file_op in &analysis.file_operations {
                // Should be empty or contain the exact case match
                if !file_op.file_path.contains("status_implementation.md") {
                    assert!(
                        analysis.file_operations.is_empty(),
                        "Case-sensitive search should not match different case"
                    );
                }
            }
        }

        // Test that exact case still works
        let analyses_exact = analyzer.analyze(Some("STATUS_IMPLEMENTATION.md")).unwrap();
        let mut found_exact = false;
        for analysis in &analyses_exact {
            if !analysis.file_operations.is_empty() {
                found_exact = true;
                break;
            }
        }
        assert!(found_exact, "Exact case matching should still work");
    }

    #[test]
    fn test_similar_filename_distinction() {
        let temp_dir = create_target_filtering_test_directory().unwrap();
        let analyzer = Analyzer::new(temp_dir.path()).unwrap();

        // Test that similar filenames are properly distinguished
        let analyses_report = analyzer
            .analyze(Some("STATUS_REPORT_IMPLEMENTATION.md"))
            .unwrap();
        let analyses_main = analyzer.analyze(Some("STATUS_IMPLEMENTATION.md")).unwrap();

        // Should find different files for each search
        let mut report_file_count = 0;
        let mut main_file_count = 0;

        for analysis in &analyses_report {
            for file_op in &analysis.file_operations {
                if file_op
                    .file_path
                    .contains("STATUS_REPORT_IMPLEMENTATION.md")
                {
                    report_file_count += 1;
                }
            }
        }

        for analysis in &analyses_main {
            for file_op in &analysis.file_operations {
                if file_op.file_path.contains("STATUS_IMPLEMENTATION.md")
                    && !file_op.file_path.contains("REPORT")
                    && !file_op.file_path.contains("REVISED")
                {
                    main_file_count += 1;
                }
            }
        }

        assert!(
            report_file_count > 0,
            "Should find STATUS_REPORT_IMPLEMENTATION.md operations"
        );
        assert!(
            main_file_count > 0,
            "Should find STATUS_IMPLEMENTATION.md operations"
        );
    }

    #[test]
    fn test_multiple_sessions_same_file() {
        let temp_dir = create_target_filtering_test_directory().unwrap();
        let analyzer = Analyzer::new(temp_dir.path()).unwrap();

        // Test that multiple sessions working on the same file are properly captured
        let analyses = analyzer.analyze(Some("STATUS_IMPLEMENTATION")).unwrap();

        let mut session_count = 0;
        let mut total_operations = 0;

        for analysis in &analyses {
            if !analysis.file_operations.is_empty() {
                session_count += 1;
                for file_op in &analysis.file_operations {
                    if file_op.file_path.contains("STATUS_IMPLEMENTATION") {
                        total_operations += 1;
                    }
                }
            }
        }

        assert!(
            session_count >= 1,
            "Should find at least one session working on STATUS_IMPLEMENTATION files"
        );
        assert!(
            total_operations >= 3,
            "Should find multiple operations across sessions (Write, Edit, MultiEdit)"
        );
    }

    #[test]
    fn test_empty_target_string() {
        let temp_dir = create_target_filtering_test_directory().unwrap();
        let analyzer = Analyzer::new(temp_dir.path()).unwrap();

        // Test with empty string - should be treated as no filtering
        let analyses_all = analyzer.analyze(None).unwrap();
        let analyses_empty = analyzer.analyze(Some("")).unwrap();

        // Both should return all sessions, but empty string filtering might behave differently
        assert!(
            !analyses_all.is_empty(),
            "Should return all sessions when no filter"
        );

        // Empty string contains check should match all files
        let mut empty_has_operations = false;
        for analysis in &analyses_empty {
            if !analysis.file_operations.is_empty() {
                empty_has_operations = true;
                break;
            }
        }
        assert!(
            empty_has_operations,
            "Empty string filter should still return file operations"
        );
    }
}

#[cfg(test)]
mod collaboration_and_attribution_tests {
    use super::*;

    #[test]
    fn test_agent_attribution_with_target_filtering() {
        let temp_dir = create_target_filtering_test_directory().unwrap();
        let analyzer = Analyzer::new(temp_dir.path()).unwrap();

        // Test that agent attribution works correctly with target filtering
        let analyses = analyzer.analyze(Some("STATUS_IMPLEMENTATION.md")).unwrap();

        let mut found_multiple_agents = false;

        for analysis in &analyses {
            for (file_path, attributions) in &analysis.file_to_agents {
                if file_path.contains("STATUS_IMPLEMENTATION.md") {
                    assert!(!attributions.is_empty(), "Should have agent attributions");

                    // Check that attributions are properly calculated
                    let total_contribution: f32 = attributions
                        .iter()
                        .map(|attr| attr.contribution_percent)
                        .sum();

                    assert!(
                        total_contribution > 90.0 && total_contribution <= 110.0,
                        "Total contribution should be approximately 100%, got: {}",
                        total_contribution
                    );

                    // Check for expected agent types from our test data
                    let agent_types: Vec<&str> = attributions
                        .iter()
                        .map(|attr| attr.agent_type.as_str())
                        .collect();

                    // Should have architect, developer, technical-writer, or rust-performance-expert based on our test data
                    let expected_agents = [
                        "architect",
                        "developer",
                        "technical-writer",
                        "rust-performance-expert",
                    ];
                    let has_expected_agent = agent_types
                        .iter()
                        .any(|agent| expected_agents.contains(agent));

                    assert!(
                        has_expected_agent,
                        "Should have expected agent types, found: {:?}",
                        agent_types
                    );

                    if agent_types.len() > 1 {
                        found_multiple_agents = true;
                    }
                }
            }
        }

        // May or may not find multiple agents depending on how the data is structured
        // This is informational rather than a hard requirement
        if found_multiple_agents {
            println!("Found collaborative work on STATUS_IMPLEMENTATION.md");
        }
    }

    #[test]
    fn test_collaboration_patterns_with_filtering() {
        let temp_dir = create_target_filtering_test_directory().unwrap();
        let analyzer = Analyzer::new(temp_dir.path()).unwrap();

        // Test that collaboration patterns are detected even with filtering
        let analyses = analyzer.analyze(Some("STATUS_IMPLEMENTATION")).unwrap();

        for analysis in &analyses {
            // Check if collaboration patterns are detected
            for pattern in &analysis.collaboration_patterns {
                assert!(
                    !pattern.pattern_type.is_empty(),
                    "Pattern type should not be empty"
                );
                assert!(!pattern.agents.is_empty(), "Pattern should have agents");
                assert!(
                    pattern.frequency > 0,
                    "Pattern frequency should be positive"
                );
                assert!(
                    pattern.confidence >= 0.0 && pattern.confidence <= 1.0,
                    "Pattern confidence should be between 0 and 1"
                );
            }

            // Check agent statistics
            for (agent_type, stats) in &analysis.agent_stats {
                assert_eq!(&stats.agent_type, agent_type, "Agent type should match key");
                assert!(stats.total_invocations > 0, "Should have invocations");

                // With filtering, files_touched might be 0 if agent didn't work on target files
                if stats.files_touched > 0 {
                    assert!(
                        !stats.tools_used.is_empty(),
                        "Should have tools used if files were touched"
                    );
                }
            }
        }
    }

    #[test]
    fn test_file_operation_agent_context() {
        let temp_dir = create_target_filtering_test_directory().unwrap();
        let analyzer = Analyzer::new(temp_dir.path()).unwrap();

        // Test that file operations have proper agent context when filtered
        let analyses = analyzer
            .analyze(Some("REVISED_STATUS_IMPLEMENTATION_ESTIMATES.md"))
            .unwrap();

        let mut operations_with_context = 0;
        let mut total_operations = 0;

        for analysis in &analyses {
            for file_op in &analysis.file_operations {
                total_operations += 1;
                if let Some(agent_context) = &file_op.agent_context {
                    operations_with_context += 1;

                    // Verify the agent context is reasonable
                    assert!(
                        !agent_context.is_empty(),
                        "Agent context should not be empty"
                    );

                    // Should be one of our expected agent types
                    let valid_agents = [
                        "architect",
                        "developer",
                        "technical-writer",
                        "rust-performance-expert",
                        "general-purpose",
                    ];
                    assert!(
                        valid_agents.contains(&agent_context.as_str()),
                        "Unexpected agent context: {}",
                        agent_context
                    );
                }
            }
        }

        assert!(
            total_operations > 0,
            "Should have file operations for target file"
        );

        // Most operations should have agent context
        let context_ratio = operations_with_context as f64 / total_operations as f64;
        assert!(
            context_ratio > 0.5,
            "Most operations should have agent context, got {}/{}",
            operations_with_context,
            total_operations
        );
    }
}

#[cfg(test)]
mod cli_integration_tests {
    use super::*;

    #[test]
    fn test_cli_analyze_with_target_filename() {
        let temp_dir = create_target_filtering_test_directory().unwrap();

        let output = Command::new("cargo")
            .args([
                "run",
                "--bin",
                "cla",
                "--",
                "analyze",
                temp_dir.path().to_str().unwrap(),
                "--target",
                "STATUS_IMPLEMENTATION.md",
                "--format",
                "json",
            ])
            .output()
            .expect("Failed to execute CLI analyze command with target");

        let stdout = String::from_utf8(output.stdout).unwrap();
        let stderr = String::from_utf8(output.stderr).unwrap();

        if !output.status.success() {
            println!("CLI command failed:");
            println!("Stderr: {}", stderr);
            println!("Stdout: {}", stdout);
            panic!("CLI command should succeed");
        }

        // Should produce JSON output
        if !stdout.trim().is_empty() {
            // Find JSON content in the output
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
                let parsed: serde_json::Value = serde_json::from_str(&json_content)
                    .expect("CLI should produce valid JSON with target filtering");

                // Verify the JSON structure contains expected fields
                if parsed.is_array() {
                    let analyses = parsed.as_array().unwrap();
                    for analysis in analyses {
                        assert!(
                            analysis.get("session_id").is_some(),
                            "Should have session_id"
                        );
                        assert!(
                            analysis.get("file_operations").is_some(),
                            "Should have file_operations"
                        );
                        assert!(
                            analysis.get("file_to_agents").is_some(),
                            "Should have file_to_agents"
                        );

                        // Check that file operations are related to target
                        if let Some(file_ops) =
                            analysis.get("file_operations").and_then(|v| v.as_array())
                        {
                            for file_op in file_ops {
                                if let Some(file_path) =
                                    file_op.get("file_path").and_then(|v| v.as_str())
                                {
                                    assert!(
                                        file_path.contains("STATUS_IMPLEMENTATION.md"),
                                        "File operation should be related to target file: {}",
                                        file_path
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_cli_analyze_with_partial_target() {
        let temp_dir = create_target_filtering_test_directory().unwrap();

        let output = Command::new("cargo")
            .args([
                "run",
                "--bin",
                "cla",
                "--",
                "analyze",
                temp_dir.path().to_str().unwrap(),
                "--target",
                "STATUS_IMPLEMENTATION",
                "--format",
                "csv",
            ])
            .output()
            .expect("Failed to execute CLI analyze command with partial target");

        let stdout = String::from_utf8(output.stdout).unwrap();

        if output.status.success() && !stdout.trim().is_empty() {
            // Should produce CSV output
            let lines: Vec<&str> = stdout.lines().collect();

            // Find CSV content (skip any header info)
            let mut csv_start = None;
            for (i, line) in lines.iter().enumerate() {
                if line.contains("Session ID") || line.contains("session_id") || line.contains(",")
                {
                    csv_start = Some(i);
                    break;
                }
            }

            if let Some(start_idx) = csv_start {
                let csv_content = lines[start_idx..].join("\n");
                assert!(csv_content.contains(","), "Should contain CSV data");
            }
        }
    }

    #[test]
    fn test_cli_analyze_with_nonexistent_target() {
        let temp_dir = create_target_filtering_test_directory().unwrap();

        let output = Command::new("cargo")
            .args([
                "run",
                "--bin",
                "cla",
                "--",
                "analyze",
                temp_dir.path().to_str().unwrap(),
                "--target",
                "NONEXISTENT_FILE.md",
                "--format",
                "json",
            ])
            .output()
            .expect("Failed to execute CLI analyze command with nonexistent target");

        let stdout = String::from_utf8(output.stdout).unwrap();

        // Should succeed but might have empty results or "No matching sessions found" message
        if output.status.success() {
            // Either empty JSON array or informational message
            assert!(
                stdout.contains("[]")
                    || stdout.contains("No matching sessions found")
                    || stdout.trim().is_empty(),
                "Should handle nonexistent target gracefully"
            );
        }
    }

    #[test]
    fn test_cli_files_only_flag_with_target() {
        let temp_dir = create_target_filtering_test_directory().unwrap();

        let output = Command::new("cargo")
            .args([
                "run",
                "--bin",
                "cla",
                "--",
                "analyze",
                temp_dir.path().to_str().unwrap(),
                "--target",
                "STATUS_IMPLEMENTATION.md",
                "--files-only",
                "--format",
                "json",
            ])
            .output()
            .expect("Failed to execute CLI analyze command with files-only flag");

        let stdout = String::from_utf8(output.stdout).unwrap();

        if output.status.success() && !stdout.trim().is_empty() {
            // When using --files-only, should only return sessions that modified files
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
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&json_content) {
                    if let Some(analyses) = parsed.as_array() {
                        for analysis in analyses {
                            // Each analysis should have file_to_agents with content
                            if let Some(file_to_agents) = analysis.get("file_to_agents") {
                                assert!(
                                    !file_to_agents.as_object().unwrap().is_empty(),
                                    "With --files-only, each session should have modified files"
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod performance_and_error_handling_tests {
    use super::*;

    #[test]
    fn test_target_filtering_performance() {
        let temp_dir = create_target_filtering_test_directory().unwrap();
        let analyzer = Analyzer::new(temp_dir.path()).unwrap();

        // Measure time for filtered vs unfiltered analysis
        let start = std::time::Instant::now();
        let analyses_all = analyzer.analyze(None).unwrap();
        let time_all = start.elapsed();

        let start = std::time::Instant::now();
        let analyses_filtered = analyzer.analyze(Some("STATUS_IMPLEMENTATION.md")).unwrap();
        let time_filtered = start.elapsed();

        // Filtering might be faster due to reduced result set processing
        println!(
            "All sessions: {:?}, Filtered: {:?}",
            time_all, time_filtered
        );

        // Verify results are reasonable
        assert!(
            !analyses_all.is_empty(),
            "Should have sessions without filtering"
        );

        // Filtered results should be subset of all results
        let all_session_count = analyses_all.len();
        let filtered_session_count = analyses_filtered.len();

        // The number of sessions might be the same (since filtering happens at file operation level)
        // but the file operations within should be reduced
        assert!(
            filtered_session_count <= all_session_count,
            "Filtered results should not exceed unfiltered results"
        );
    }

    #[test]
    fn test_special_characters_in_target() {
        let temp_dir = create_target_filtering_test_directory().unwrap();
        let analyzer = Analyzer::new(temp_dir.path()).unwrap();

        // Test with special characters that might cause issues
        let special_targets = vec![
            "STATUS_IMPLEMENTATION.md", // underscores and dots
            "STATUS*",                  // wildcard (should be treated literally)
            "STATUS[IMPLEMENTATION]",   // brackets
            "STATUS(IMPLEMENTATION)",   // parentheses
            ".md",                      // just extension
        ];

        for target in special_targets {
            let result = analyzer.analyze(Some(target));
            assert!(
                result.is_ok(),
                "Should handle special characters in target: {}",
                target
            );

            // Most should return empty results except the first one
            let analyses = result.unwrap();
            if target == "STATUS_IMPLEMENTATION.md" {
                // This one should find results
                let has_operations = analyses.iter().any(|a| !a.file_operations.is_empty());
                assert!(
                    has_operations,
                    "Should find operations for exact filename match"
                );
            }
            // Others might or might not find results depending on implementation
        }
    }

    #[test]
    fn test_very_long_target_filename() {
        let temp_dir = create_target_filtering_test_directory().unwrap();
        let analyzer = Analyzer::new(temp_dir.path()).unwrap();

        // Test with very long filename that doesn't exist
        let long_target = "A".repeat(1000) + ".md";
        let result = analyzer.analyze(Some(&long_target));

        assert!(result.is_ok(), "Should handle very long target names");
        let analyses = result.unwrap();

        // Should return empty file operations
        for analysis in &analyses {
            assert!(
                analysis.file_operations.is_empty(),
                "Should have no operations for very long nonexistent filename"
            );
        }
    }

    #[test]
    fn test_unicode_characters_in_target() {
        let temp_dir = create_target_filtering_test_directory().unwrap();
        let analyzer = Analyzer::new(temp_dir.path()).unwrap();

        // Test with Unicode characters
        let unicode_targets = vec![
            "STATUS_实现.md",             // Chinese characters
            "СТАТУС_РЕАЛИЗАЦИЯ.md",       // Cyrillic
            "STATUS_IMPLÉMENTATION.md",   // Accented characters
            "📝STATUS_IMPLEMENTATION.md", // Emoji
        ];

        for target in unicode_targets {
            let result = analyzer.analyze(Some(target));
            assert!(
                result.is_ok(),
                "Should handle Unicode characters in target: {}",
                target
            );

            // Should return empty results since these files don't exist in test data
            let analyses = result.unwrap();
            for analysis in &analyses {
                assert!(
                    analysis.file_operations.is_empty(),
                    "Should have no operations for Unicode filename that doesn't exist"
                );
            }
        }
    }
}

/// Integration test that verifies the complete pipeline works correctly
#[cfg(test)]
mod complete_pipeline_tests {
    use super::*;

    #[test]
    fn test_complete_filename_filtering_pipeline() {
        let temp_dir = create_target_filtering_test_directory().unwrap();
        let analyzer = Analyzer::new(temp_dir.path()).unwrap();

        // Test the complete pipeline for a specific target
        let target = "STATUS_IMPLEMENTATION.md";
        let analyses = analyzer.analyze(Some(target)).unwrap();

        // Should have results
        assert!(!analyses.is_empty(), "Should find sessions");

        // Test that we can generate reports from filtered results
        let reporter = Reporter::new();

        // Test JSON export
        let json_result = reporter.to_json(&analyses);
        assert!(
            json_result.is_ok(),
            "Should generate JSON report from filtered results"
        );

        let json_output = json_result.unwrap();
        assert!(!json_output.is_empty(), "JSON output should not be empty");

        // Verify JSON is valid
        let parsed: serde_json::Value = serde_json::from_str(&json_output).unwrap();
        assert!(
            parsed.is_array() || parsed.is_object(),
            "Should be valid JSON structure"
        );

        // Test CSV export
        let csv_result = reporter.to_csv(&analyses);
        assert!(
            csv_result.is_ok(),
            "Should generate CSV report from filtered results"
        );

        // Test Markdown export
        let markdown_result = reporter.to_markdown(&analyses);
        assert!(
            markdown_result.is_ok(),
            "Should generate Markdown report from filtered results"
        );

        // Test terminal output (should not panic)
        reporter.print_terminal(&analyses);

        // Verify that all file operations in the results are related to the target
        for analysis in &analyses {
            for file_op in &analysis.file_operations {
                assert!(
                    file_op.file_path.contains(target),
                    "All file operations should be related to target {}, found: {}",
                    target,
                    file_op.file_path
                );
            }

            // Verify file-to-agent mappings only contain target file
            for (file_path, _) in &analysis.file_to_agents {
                assert!(
                    file_path.contains(target),
                    "All file attributions should be for target {}, found: {}",
                    target,
                    file_path
                );
            }
        }

        println!("✅ Complete pipeline test passed for target: {}", target);
        println!(
            "   Found {} session(s) with operations on target file",
            analyses.len()
        );

        let total_operations: usize = analyses.iter().map(|a| a.file_operations.len()).sum();
        println!("   Total file operations on target: {}", total_operations);

        let unique_agents: std::collections::HashSet<String> = analyses
            .iter()
            .flat_map(|a| a.agents.iter().map(|ag| ag.agent_type.clone()))
            .collect();
        println!("   Unique agents involved: {:?}", unique_agents);
    }
}
