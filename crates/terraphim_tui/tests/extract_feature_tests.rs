//! Extract feature tests for TUI CLI
//!
//! Tests the extract command functionality in both offline and server modes

use anyhow::Result;
use serial_test::serial;
use std::process::Command;
use std::str;

/// Helper function to run TUI extract command in offline mode
fn run_extract_offline(args: &[&str]) -> Result<(String, String, i32)> {
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "-p", "terraphim_tui", "--", "extract"])
        .args(args);

    let output = cmd.output()?;

    Ok((
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
        output.status.code().unwrap_or(-1),
    ))
}

/// Extract clean output without log messages
fn extract_clean_output(output: &str) -> String {
    output
        .lines()
        .filter(|line| {
            !line.contains("INFO")
                && !line.contains("WARN")
                && !line.contains("DEBUG")
                && !line.trim().is_empty()
        })
        .collect::<Vec<&str>>()
        .join("\n")
}

#[test]
#[serial]
fn test_extract_basic_functionality() -> Result<()> {
    println!("🔍 Testing TUI extract command basic functionality");

    let test_text =
        "This is a paragraph about haystacks and their configuration. Haystacks are data sources.";

    let (stdout, stderr, code) = run_extract_offline(&[test_text])?;

    println!("Extract command exit code: {}", code);

    // Extract command should succeed with KG data available
    assert_eq!(
        code, 0,
        "Extract command should succeed: exit_code={}, stderr={}",
        code, stderr
    );

    let clean_output = extract_clean_output(&stdout);

    // With KG data available, we should find matches for this text
    assert!(
        !clean_output.is_empty(),
        "Extract should find matches with KG data available"
    );
    assert!(
        !clean_output.contains("No matches found"),
        "Should find matches for haystack and configuration terms"
    );

    println!("✅ Extract found matches: {}", clean_output.lines().count());

    // Validate that matches have proper structure
    let has_match_structure = clean_output.contains("Match")
        && clean_output.contains("term:")
        && clean_output.contains("paragraph");

    assert!(
        has_match_structure,
        "Extract output should have proper match structure with term and paragraph info: {}",
        clean_output
    );

    // Check that specific expected terms are found
    let found_haystack = clean_output.contains("haystack") || clean_output.contains("data-source");
    let found_config = clean_output.contains("configuration") || clean_output.contains("config");

    assert!(
        found_haystack || found_config,
        "Should find matches for haystack or configuration terms: {}",
        clean_output
    );

    Ok(())
}

#[test]
#[serial]
fn test_extract_matching_evaluation() -> Result<()> {
    println!("🔍 Testing extract matching functionality with comprehensive evaluation");

    // Test with text that contains common technical terms that might be in any KG
    let test_cases = vec![
        ("Basic terms", "system data processing configuration"),
        ("Technical stack", "database api service middleware"),
        ("Development terms", "code repository testing deployment"),
        (
            "Architecture terms",
            "architecture pattern design framework",
        ),
    ];

    let mut total_tests = 0;
    let mut successful_extracts = 0;
    let mut no_match_cases = 0;

    for (case_name, test_text) in test_cases {
        println!("  Testing case: {}", case_name);
        total_tests += 1;

        let (stdout, stderr, code) = run_extract_offline(&[test_text])?;

        // Command should always succeed
        assert_eq!(
            code, 0,
            "Extract command should succeed for case '{}': exit_code={}, stderr={}",
            case_name, code, stderr
        );

        let clean_output = extract_clean_output(&stdout);

        if clean_output.contains("No matches found") {
            no_match_cases += 1;
            println!("    ⚠️ No matches found for: {}", case_name);
        } else if !clean_output.is_empty() {
            successful_extracts += 1;
            println!("    ✅ Found matches for: {}", case_name);

            // Validate output structure if matches are found
            let match_lines: Vec<&str> = clean_output
                .lines()
                .filter(|line| line.contains("Match") || line.contains("term:"))
                .collect();

            if !match_lines.is_empty() {
                println!("      Match format appears correct");
            }
        } else {
            println!("    ⚠️ Empty output for: {}", case_name);
        }
    }

    println!("\n📊 Extract Matching Evaluation Summary:");
    println!("  Total test cases: {}", total_tests);
    println!("  Successful extracts: {}", successful_extracts);
    println!("  No matches found: {}", no_match_cases);
    println!(
        "  Empty outputs: {}",
        total_tests - successful_extracts - no_match_cases
    );

    // With KG data available, we should find matches in most cases
    assert!(
        successful_extracts > 0,
        "Extract matching should work with KG data available. Found {} successful extracts out of {} cases",
        successful_extracts,
        total_tests
    );

    // At least 50% of test cases should find matches with available KG data
    let success_rate = (successful_extracts as f64 / total_tests as f64) * 100.0;
    assert!(
        success_rate >= 50.0,
        "Extract should have at least 50% success rate with KG data, got {:.1}%",
        success_rate
    );

    println!("✅ Extract matching functionality is working - found matches in {} cases ({:.1}% success rate)",
             successful_extracts, success_rate);

    Ok(())
}

#[test]
#[serial]
fn test_extract_with_role_option() -> Result<()> {
    println!("🔍 Testing TUI extract command with role option");

    let test_text = "Testing document with various technical terms and concepts.";

    // Test with specific role
    let (stdout, stderr, code) = run_extract_offline(&[test_text, "--role", "Default"])?;

    assert!(
        code == 0 || code == 1,
        "Extract with role should complete: exit_code={}, stderr={}",
        code,
        stderr
    );

    let clean_output = extract_clean_output(&stdout);

    if code == 0 {
        println!("✅ Extract with role succeeded");
        if !clean_output.is_empty() {
            println!("  Found matches with Default role");
        }
    } else {
        println!("⚠️ Extract with role found no matches");
    }

    Ok(())
}

#[test]
#[serial]
fn test_extract_exclude_term_option() -> Result<()> {
    println!("🔍 Testing TUI extract command with exclude-term option");

    let test_text = "This paragraph contains haystack terms that should be handled properly.";

    // Test with exclude-term flag
    let (stdout, stderr, code) = run_extract_offline(&[test_text, "--exclude-term"])?;

    assert!(
        code == 0 || code == 1,
        "Extract with exclude-term should complete: exit_code={}, stderr={}",
        code,
        stderr
    );

    let clean_output = extract_clean_output(&stdout);

    if code == 0 {
        println!("✅ Extract with exclude-term succeeded");
        if !clean_output.is_empty() {
            println!("  Found excluded paragraphs");
        }
    } else {
        println!("⚠️ Extract with exclude-term found no matches");
    }

    Ok(())
}

#[test]
#[serial]
fn test_extract_combined_options() -> Result<()> {
    println!("🔍 Testing TUI extract command with combined options");

    let test_text = "Complex document with multiple technical terms for comprehensive testing.";

    // Test with both role and exclude-term options
    let (stdout, stderr, code) =
        run_extract_offline(&[test_text, "--role", "Terraphim Engineer", "--exclude-term"])?;

    assert!(
        code == 0 || code == 1,
        "Extract with combined options should complete: exit_code={}, stderr={}",
        code,
        stderr
    );

    let clean_output = extract_clean_output(&stdout);

    if code == 0 {
        println!("✅ Extract with combined options succeeded");
        if !clean_output.is_empty() {
            println!("  Found results with Terraphim Engineer role and exclude-term");
        }
    } else {
        println!("⚠️ Extract with combined options found no matches");
    }

    Ok(())
}

#[test]
#[serial]
fn test_extract_edge_cases() -> Result<()> {
    println!("🔍 Testing TUI extract command edge cases");

    // Test with empty text
    println!("  Testing empty text");
    let (_, _, code) = run_extract_offline(&[""])?;
    assert!(
        code == 0 || code == 1,
        "Extract with empty text should handle gracefully"
    );

    // Test with very short text
    println!("  Testing single word");
    let (_, _, code) = run_extract_offline(&["haystack"])?;
    assert!(
        code == 0 || code == 1,
        "Extract with single word should handle gracefully"
    );

    // Test with special characters
    println!("  Testing special characters");
    let (_, _, code) = run_extract_offline(&["Text with üñíçödé characters!"])?;
    assert!(
        code == 0 || code == 1,
        "Extract with special characters should handle gracefully"
    );

    // Test with very long text
    println!("  Testing long text");
    let long_text =
        "This is a very long paragraph with haystack terms repeated many times. ".repeat(50);
    let (_, _, code) = run_extract_offline(&[&long_text])?;
    assert!(
        code == 0 || code == 1,
        "Extract with long text should handle gracefully"
    );

    println!("✅ All edge case tests completed");
    Ok(())
}

#[test]
#[serial]
fn test_extract_output_format() -> Result<()> {
    println!("🔍 Testing TUI extract command output format");

    let test_text =
        "This is a test paragraph with haystack and service terms for format validation.";

    let (stdout, stderr, code) = run_extract_offline(&[test_text])?;

    if code == 0 {
        let clean_output = extract_clean_output(&stdout);

        if !clean_output.is_empty() {
            println!("✅ Extract produced output:");

            // Validate output format
            let lines: Vec<&str> = clean_output.lines().collect();

            // Should have some information about matches
            let has_match_info = lines.iter().any(|line| {
                line.contains("Match")
                    || line.contains("paragraph")
                    || line.contains("Found")
                    || line.contains("term:")
            });

            if has_match_info {
                println!("  ✅ Output format appears correct");
                for (i, line) in lines.iter().enumerate().take(5) {
                    println!("    Line {}: {}", i + 1, line);
                }
            } else {
                println!("  ⚠️ Output format may not be optimal");
            }
        } else if code == 0 {
            // Empty output with success code suggests "no matches found" scenario
            println!("  ✅ Clean 'no matches' behavior");
        }
    } else {
        println!("⚠️ Extract command found no matches: code={}", code);

        // Validate error handling
        assert!(
            code == 1,
            "Should exit with code 1 for no matches, got {}",
            code
        );
    }

    Ok(())
}

#[test]
#[serial]
fn test_extract_help_and_usage() -> Result<()> {
    println!("🔍 Testing TUI extract command help and usage");

    // Test help output
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "-p", "terraphim_tui", "--", "extract", "--help"]);

    let output = cmd.output()?;
    let help_text = String::from_utf8_lossy(&output.stdout);

    // Help should be displayed successfully
    assert_eq!(
        output.status.code().unwrap_or(-1),
        0,
        "Help should display successfully"
    );

    let help_content = help_text.to_lowercase();
    assert!(
        help_content.contains("extract") && help_content.contains("text"),
        "Help should mention extract and text"
    );

    // Check for key options
    let expected_options = ["--role", "--exclude-term"];
    for option in &expected_options {
        assert!(
            help_content.contains(option),
            "Help should mention option: {}",
            option
        );
    }

    println!("✅ Help output validated");

    // Test invalid arguments
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "-p", "terraphim_tui", "--", "extract"]);

    let output = cmd.output()?;
    let exit_code = output.status.code().unwrap_or(-1);

    // Should fail gracefully with missing required text argument
    assert_ne!(
        exit_code, 0,
        "Should fail when required text argument is missing"
    );

    println!("✅ Error handling for missing arguments validated");

    Ok(())
}

#[test]
#[serial]
fn test_extract_different_roles() -> Result<()> {
    println!("🔍 Testing TUI extract command with different roles");

    let test_text = "Document with various technical concepts for role testing.";

    let test_roles = ["Default", "Terraphim Engineer"];

    for role in &test_roles {
        println!("  Testing role: {}", role);

        let (stdout, stderr, code) = run_extract_offline(&[test_text, "--role", role])?;

        assert!(
            code == 0 || code == 1,
            "Extract with role '{}' should complete: exit_code={}, stderr={}",
            role,
            code,
            stderr
        );

        let clean_output = extract_clean_output(&stdout);

        if code == 0 && !clean_output.is_empty() {
            println!("    ✅ Role '{}' found matches", role);
        } else {
            println!("    ⚠️ Role '{}' found no matches", role);
        }
    }

    println!("✅ Role testing completed");
    Ok(())
}

#[test]
#[serial]
fn test_extract_performance() -> Result<()> {
    println!("⚡ Testing TUI extract command performance");

    // Create a reasonably large text for performance testing
    let large_text = format!(
        "{}{}{}",
        "This paragraph discusses haystacks and data sources extensively. ".repeat(20),
        "Another section covers services and middleware concepts in detail. ".repeat(20),
        "Final part examines graph embeddings and semantic relationships. ".repeat(20)
    );

    println!("  Testing with large text ({} chars)", large_text.len());

    let start = std::time::Instant::now();
    let (stdout, stderr, code) = run_extract_offline(&[&large_text])?;
    let duration = start.elapsed();

    println!("  ⏱️ Extract completed in {:?}", duration);

    assert!(
        code == 0 || code == 1,
        "Extract performance test should complete: exit_code={}, stderr={}",
        code,
        stderr
    );

    // Performance threshold - should complete within reasonable time
    assert!(
        duration.as_secs() < 30,
        "Extract should complete within 30 seconds, took {:?}",
        duration
    );

    let clean_output = extract_clean_output(&stdout);

    if code == 0 && !clean_output.is_empty() {
        println!("  ✅ Performance test passed with matches found");
    } else {
        println!("  ✅ Performance test passed (no matches expected in test env)");
    }

    Ok(())
}
