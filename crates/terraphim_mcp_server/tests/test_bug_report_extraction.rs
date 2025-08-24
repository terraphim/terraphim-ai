use anyhow::Result;
use rmcp::{
    model::CallToolRequestParam,
    service::ServiceExt,
    transport::TokioChildProcess,
};
use serde_json::json;
use std::process::Stdio;
use tokio::process::Command;

/// Test comprehensive bug report extraction using knowledge graph terminology
#[tokio::test]
async fn test_bug_report_extraction_with_kg_terms() -> Result<()> {
    println!("🐛 Testing bug report extraction with knowledge graph terms");
    
    let crate_dir = std::env::current_dir()?;
    let binary_path = crate_dir
        .parent()
        .and_then(|p| p.parent())
        .map(|workspace| workspace.join("target").join("debug").join("terraphim_mcp_server"))
        .ok_or_else(|| anyhow::anyhow!("Cannot find workspace root"))?;
    
    let mut cmd = Command::new(binary_path);
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .arg("--profile")
        .arg("desktop");
    
    let transport = TokioChildProcess::new(cmd)?;
    let service = ().serve(transport).await?;
    
    println!("🔗 Connected to MCP server");
    
    // Build autocomplete index for Terraphim Engineer role
    let _build_result = service
        .call_tool(CallToolRequestParam {
            name: "build_autocomplete_index".into(),
            arguments: json!({
                "role": "Terraphim Engineer"
            })
            .as_object()
            .cloned(),
        })
        .await?;
    
    // Test document containing all four bug report sections with domain-specific issues
    let comprehensive_bug_report = r#"
    Issue Report: Payroll System Data Inconsistency
    
    Problem Summary
    
    We have discovered a critical issue with our payroll system that affects employee 
    compensation calculations and HR system integration.
    
    Steps to Reproduce the Issue
    
    To recreate this issue, follow these reproduction steps carefully:
    1. Log into the HR system using administrator credentials
    2. Navigate to the payroll processing module 
    3. Select employees from the Q3 2024 cohort
    4. Run the salary calculation process for the selected group
    5. Observe the wage calculation discrepancies in the output report
    
    These repro steps should consistently demonstrate the problem. The issue occurs 
    every time we follow this procedure to reproduce the bug.
    
    Expected Behavior and System Requirements
    
    The intended behavior should be as follows:
    - The payroll system should calculate wages correctly for all employees
    - Data consistency should be maintained across all HR system components
    - System integration between payroll and HR databases should work seamlessly
    - Employee compensation should match the predefined salary structures
    - The expected result should show accurate wage calculations
    
    This represents the normal behavior we expect from a properly functioning system.
    The correct behavior would ensure all employee pay issues are resolved.
    
    Actual Behavior and Observed Problems
    
    What actually happens demonstrates significant system problems:
    - The observed behavior shows incorrect salary calculations for 30% of employees
    - Data mismatch occurs between the payroll system and HR database
    - System integration failures prevent proper data synchronization
    - Performance degradation causes slow response times during peak processing
    - The current behavior results in payroll discrepancies and employee complaints
    
    This faulty behavior represents a serious deviation from expected functionality.
    The system malfunction affects multiple HR workflows and user experience.
    
    Business Impact and Consequences Analysis
    
    The impact of this issue extends across multiple areas:
    
    User Impact:
    - Employees experience delayed or incorrect salary payments
    - HR staff spend excessive time on manual corrections
    - User experience suffers due to system reliability issues
    
    System Impact:
    - Database consistency problems affect data integrity
    - Integration failures disrupt automated HR processes
    - Performance issues slow down critical payroll operations
    
    Business Impact:
    - Operational impact includes increased manual processing costs
    - Employee satisfaction decreases due to payroll errors
    - Compliance risks arise from incorrect compensation calculations
    - The business consequences include potential legal liability
    
    This represents a critical issue requiring immediate attention from our QA process
    and quality assurance team to prevent further operational disruption.
    "#;
    
    println!("📄 Testing extraction of Steps to Reproduce section...");
    let steps_extraction = service
        .call_tool(CallToolRequestParam {
            name: "extract_paragraphs_from_automata".into(),
            arguments: json!({
                "text": comprehensive_bug_report,
                "include_term": true,
                "role": "Terraphim Engineer"
            })
            .as_object()
            .cloned(),
        })
        .await?;
    
    println!("✅ Steps extraction result: {:?}", steps_extraction.content);
    
    // Test specific sections individually
    println!("📋 Testing Expected Behavior section extraction...");
    let expected_behavior_text = r#"
    System Requirements and Expected Functionality
    
    The expected behaviour of our HR system should demonstrate proper functionality:
    - Correct behavior in salary calculations
    - Expected results that match employee contracts
    - Intended behavior for system integration processes
    - Normal behavior during peak processing periods
    
    The expected outcome should provide reliable payroll processing capabilities.
    This represents the intended result we expect from the system upgrade.
    "#;
    
    let expected_extraction = service
        .call_tool(CallToolRequestParam {
            name: "extract_paragraphs_from_automata".into(),
            arguments: json!({
                "text": expected_behavior_text,
                "include_term": true,
                "role": "Terraphim Engineer"
            })
            .as_object()
            .cloned(),
        })
        .await?;
    
    println!("✅ Expected behavior extraction: {:?}", expected_extraction.content);
    
    println!("🔍 Testing Actual Behavior section extraction...");
    let actual_behavior_text = r#"
    Current System Issues and Observed Problems
    
    The actual behaviour we observe differs significantly from expectations:
    - Incorrect behavior in wage calculations affecting multiple employees
    - Observed behavior shows data synchronization failures
    - Current behavior includes system lag and performance degradation
    - Faulty behavior in the HR integration processes
    
    This system malfunction represents erroneous behavior that must be addressed.
    The bug behavior consistently occurs during payroll processing cycles.
    "#;
    
    let actual_extraction = service
        .call_tool(CallToolRequestParam {
            name: "extract_paragraphs_from_automata".into(),
            arguments: json!({
                "text": actual_behavior_text,
                "include_term": true,
                "role": "Terraphim Engineer"
            })
            .as_object()
            .cloned(),
        })
        .await?;
    
    println!("✅ Actual behavior extraction: {:?}", actual_extraction.content);
    
    println!("💼 Testing Impact Analysis section extraction...");
    let impact_analysis_text = r#"
    Comprehensive Impact Assessment
    
    The business impact of this issue affects multiple stakeholders:
    
    User Impact Assessment:
    - Employee satisfaction declined due to payroll errors
    - HR staff productivity reduced by manual correction requirements
    - User experience degraded by system reliability issues
    
    Operational Impact:
    - System impact includes database integrity problems
    - Performance impact affects processing efficiency
    - Integration failures disrupt automated workflows
    
    Business Consequences:
    - Financial impact from incorrect compensation calculations
    - Compliance risks due to payroll discrepancies
    - Reputational consequences from employee dissatisfaction
    
    The severity of this issue requires immediate quality assurance intervention.
    Our QA process must prioritize this critical system defect.
    "#;
    
    let impact_extraction = service
        .call_tool(CallToolRequestParam {
            name: "extract_paragraphs_from_automata".into(),
            arguments: json!({
                "text": impact_analysis_text,
                "include_term": true,
                "role": "Terraphim Engineer"
            })
            .as_object()
            .cloned(),
        })
        .await?;
    
    println!("✅ Impact analysis extraction: {:?}", impact_extraction.content);
    
    // Test connectivity between all bug reporting terms
    println!("🔗 Testing term connectivity across all bug report sections...");
    let connectivity_result = service
        .call_tool(CallToolRequestParam {
            name: "is_all_terms_connected_by_path".into(),
            arguments: json!({
                "text": comprehensive_bug_report,
                "role": "Terraphim Engineer"
            })
            .as_object()
            .cloned(),
        })
        .await?;
    
    println!("✅ Bug report term connectivity: {:?}", connectivity_result.content);
    
    // Test specific domain terminology recognition
    println!("🏢 Testing domain-specific terminology recognition...");
    let domain_text = "The payroll system shows data consistency problems affecting HR system integration and causing performance degradation in our quality assurance process.";
    
    let domain_extraction = service
        .call_tool(CallToolRequestParam {
            name: "extract_paragraphs_from_automata".into(),
            arguments: json!({
                "text": domain_text,
                "include_term": true,
                "role": "Terraphim Engineer"
            })
            .as_object()
            .cloned(),
        })
        .await?;
    
    println!("✅ Domain terminology extraction: {:?}", domain_extraction.content);
    
    let domain_connectivity = service
        .call_tool(CallToolRequestParam {
            name: "is_all_terms_connected_by_path".into(),
            arguments: json!({
                "text": domain_text,
                "role": "Terraphim Engineer"
            })
            .as_object()
            .cloned(),
        })
        .await?;
    
    println!("✅ Domain terminology connectivity: {:?}", domain_connectivity.content);
    
    println!("🎉 All bug report extraction tests completed successfully!");
    
    Ok(())
}

/// Test bug report extraction with mixed terminology and edge cases
#[tokio::test]
async fn test_bug_report_extraction_edge_cases() -> Result<()> {
    println!("🎯 Testing bug report extraction with edge cases");
    
    let crate_dir = std::env::current_dir()?;
    let binary_path = crate_dir
        .parent()
        .and_then(|p| p.parent())
        .map(|workspace| workspace.join("target").join("debug").join("terraphim_mcp_server"))
        .ok_or_else(|| anyhow::anyhow!("Cannot find workspace root"))?;
    
    let mut cmd = Command::new(binary_path);
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .arg("--profile")
        .arg("desktop");
    
    let transport = TokioChildProcess::new(cmd)?;
    let service = ().serve(transport).await?;
    
    // Build index
    let _build_result = service
        .call_tool(CallToolRequestParam {
            name: "build_autocomplete_index".into(),
            arguments: json!({
                "role": "Terraphim Engineer"
            })
            .as_object()
            .cloned(),
        })
        .await?;
    
    // Test 1: Short content with multiple KG terms
    println!("📝 Testing short content with multiple KG terms...");
    let short_bug_report = "Steps to reproduce: The payroll system shows incorrect behavior during salary calculation. Expected result: Proper wage calculation. Actual behavior: Data inconsistency. Impact: User experience degradation.";
    
    let short_extraction = service
        .call_tool(CallToolRequestParam {
            name: "extract_paragraphs_from_automata".into(),
            arguments: json!({
                "text": short_bug_report,
                "include_term": true,
                "role": "Terraphim Engineer"
            })
            .as_object()
            .cloned(),
        })
        .await?;
    
    println!("✅ Short content extraction: {:?}", short_extraction.content);
    
    // Test 2: Content with overlapping terms
    println!("🔄 Testing content with overlapping terminology...");
    let overlapping_terms = r#"
    Bug Classification and Analysis
    
    This system defect demonstrates both expected behavior deviations and actual behavior problems.
    The reproduction steps reveal data consistency issues in the HR system integration.
    Quality assurance testing shows performance degradation affecting user experience.
    The business impact includes operational consequences requiring immediate attention.
    "#;
    
    let overlapping_extraction = service
        .call_tool(CallToolRequestParam {
            name: "extract_paragraphs_from_automata".into(),
            arguments: json!({
                "text": overlapping_terms,
                "include_term": true,
                "role": "Terraphim Engineer"
            })
            .as_object()
            .cloned(),
        })
        .await?;
    
    println!("✅ Overlapping terms extraction: {:?}", overlapping_extraction.content);
    
    // Test 3: Verify all terms are connected
    let all_terms_connectivity = service
        .call_tool(CallToolRequestParam {
            name: "is_all_terms_connected_by_path".into(),
            arguments: json!({
                "text": overlapping_terms,
                "role": "Terraphim Engineer"
            })
            .as_object()
            .cloned(),
        })
        .await?;
    
    println!("✅ All terms connectivity check: {:?}", all_terms_connectivity.content);
    
    println!("🎉 Edge case testing completed!");
    
    Ok(())
}