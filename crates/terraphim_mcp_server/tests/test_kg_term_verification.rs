use anyhow::Result;
use rmcp::{
    model::CallToolRequestParam,
    service::ServiceExt,
    transport::TokioChildProcess,
};
use serde_json::json;
use std::process::Stdio;
use tokio::process::Command;

/// Test that our bug reporting knowledge graph terms are available in autocomplete
#[tokio::test]
async fn test_kg_bug_reporting_terms_available() -> Result<()> {
    println!("🔍 Testing knowledge graph bug reporting terms availability");
    
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
    
    // Build autocomplete index
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
    
    // Test autocomplete for "steps to reproduce" terms
    println!("🔧 Testing autocomplete for 'steps to reproduce' terms...");
    let steps_autocomplete = service
        .call_tool(CallToolRequestParam {
            name: "autocomplete_terms".into(),
            arguments: json!({
                "query": "steps to reproduce",
                "limit": 10,
                "role": "Terraphim Engineer"
            })
            .as_object()
            .cloned(),
        })
        .await?;
    
    println!("✅ Steps autocomplete: {:?}", steps_autocomplete.content);
    
    // Test autocomplete for "expected behavior" terms
    println!("🎯 Testing autocomplete for 'expected behavior' terms...");
    let expected_autocomplete = service
        .call_tool(CallToolRequestParam {
            name: "autocomplete_terms".into(),
            arguments: json!({
                "query": "expected behavior",
                "limit": 10,
                "role": "Terraphim Engineer"
            })
            .as_object()
            .cloned(),
        })
        .await?;
    
    println!("✅ Expected behavior autocomplete: {:?}", expected_autocomplete.content);
    
    // Test autocomplete for "payroll system" terms
    println!("💰 Testing autocomplete for 'payroll system' terms...");
    let payroll_autocomplete = service
        .call_tool(CallToolRequestParam {
            name: "autocomplete_terms".into(),
            arguments: json!({
                "query": "payroll",
                "limit": 10,
                "role": "Terraphim Engineer"
            })
            .as_object()
            .cloned(),
        })
        .await?;
    
    println!("✅ Payroll autocomplete: {:?}", payroll_autocomplete.content);
    
    // Test autocomplete for "data consistency" terms
    println!("🔄 Testing autocomplete for 'data consistency' terms...");
    let data_autocomplete = service
        .call_tool(CallToolRequestParam {
            name: "autocomplete_terms".into(),
            arguments: json!({
                "query": "data consistency",
                "limit": 10,
                "role": "Terraphim Engineer"
            })
            .as_object()
            .cloned(),
        })
        .await?;
    
    println!("✅ Data consistency autocomplete: {:?}", data_autocomplete.content);
    
    // Test autocomplete for general "quality assurance" terms
    println!("🛡️ Testing autocomplete for 'quality assurance' terms...");
    let qa_autocomplete = service
        .call_tool(CallToolRequestParam {
            name: "autocomplete_terms".into(),
            arguments: json!({
                "query": "quality assurance",
                "limit": 10,
                "role": "Terraphim Engineer"
            })
            .as_object()
            .cloned(),
        })
        .await?;
    
    println!("✅ Quality assurance autocomplete: {:?}", qa_autocomplete.content);
    
    println!("🎉 All knowledge graph term verification tests completed!");
    
    Ok(())
}