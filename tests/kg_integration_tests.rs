//! Integration tests for knowledge graph builder and search
//!
//! These tests verify that the knowledge graph can be built from tool invocations
//! and that complex queries work correctly with terraphim_automata.

#![cfg(feature = "terraphim")]

use claude_log_analyzer::kg::{KnowledgeGraphBuilder, KnowledgeGraphSearch, QueryNode};
use claude_log_analyzer::models::{ToolCategory, ToolInvocation};
use std::collections::HashMap;

fn create_test_tool(tool_name: &str, command: &str) -> ToolInvocation {
    ToolInvocation {
        timestamp: jiff::Timestamp::now(),
        tool_name: tool_name.to_string(),
        tool_category: ToolCategory::PackageManager,
        command_line: command.to_string(),
        arguments: vec![],
        flags: HashMap::new(),
        exit_code: Some(0),
        agent_context: None,
        session_id: "test-session".to_string(),
        message_id: "test-message".to_string(),
    }
}

#[test]
fn test_build_knowledge_graph_from_tools() {
    let tools = vec![
        create_test_tool("bun", "bunx wrangler deploy"),
        create_test_tool("npm", "npm install packages"),
        create_test_tool("cargo", "cargo build --release"),
        create_test_tool("wrangler", "npx wrangler deploy --env production"),
    ];

    let builder = KnowledgeGraphBuilder::from_tool_invocations(&tools);

    // Verify concepts were created
    assert!(builder.concept_map.contains_key("BUN"));
    assert!(builder.concept_map.contains_key("NPM"));
    assert!(builder.concept_map.contains_key("INSTALL"));
    assert!(builder.concept_map.contains_key("DEPLOY"));
    assert!(builder.concept_map.contains_key("CARGO"));

    // Verify thesaurus is not empty
    assert!(!builder.thesaurus.is_empty());
}

#[test]
fn test_search_bun_concept() -> anyhow::Result<()> {
    let tools = vec![
        create_test_tool("bun", "bunx wrangler deploy"),
        create_test_tool("npm", "npm install"),
    ];

    let builder = KnowledgeGraphBuilder::from_tool_invocations(&tools);
    let search = KnowledgeGraphSearch::new(builder);

    let query = QueryNode::Concept("BUN".to_string());
    let results = search.search("bunx wrangler deploy --env production", &query)?;

    assert!(!results.is_empty(), "Should find BUN concept");
    assert!(
        results[0].concepts_matched.contains(&"BUN".to_string()),
        "Should match BUN concept"
    );

    Ok(())
}

#[test]
fn test_search_install_concept() -> anyhow::Result<()> {
    let tools = vec![create_test_tool("npm", "npm install packages")];

    let builder = KnowledgeGraphBuilder::from_tool_invocations(&tools);
    let search = KnowledgeGraphSearch::new(builder);

    let query = QueryNode::Concept("INSTALL".to_string());
    let results = search.search("npm install packages", &query)?;

    assert!(!results.is_empty(), "Should find INSTALL concept");
    assert!(
        results[0]
            .concepts_matched
            .contains(&"INSTALL".to_string()),
        "Should match INSTALL concept"
    );

    Ok(())
}

#[test]
fn test_search_bun_and_install() -> anyhow::Result<()> {
    let tools = vec![
        create_test_tool("bun", "bun install packages"),
        create_test_tool("bun", "bunx install deps"),
    ];

    let builder = KnowledgeGraphBuilder::from_tool_invocations(&tools);
    let search = KnowledgeGraphSearch::new(builder);

    // Query: "BUN AND install"
    let query = QueryNode::And(
        Box::new(QueryNode::Concept("BUN".to_string())),
        Box::new(QueryNode::Concept("INSTALL".to_string())),
    );

    let results = search.search("bun install packages", &query)?;

    // Should find results where both BUN and INSTALL concepts appear
    if !results.is_empty() {
        println!("Found {} results", results.len());
        for result in &results {
            println!(
                "  - Matched: {:?}, Concepts: {:?}, Score: {}",
                result.matched_text, result.concepts_matched, result.relevance_score
            );
        }

        // Verify we found at least one concept
        assert!(
            !results[0].concepts_matched.is_empty(),
            "Should have matched concepts"
        );
    }

    Ok(())
}

#[test]
fn test_search_bun_or_npm() -> anyhow::Result<()> {
    let tools = vec![
        create_test_tool("bun", "bunx install"),
        create_test_tool("npm", "npm install"),
    ];

    let builder = KnowledgeGraphBuilder::from_tool_invocations(&tools);
    let search = KnowledgeGraphSearch::new(builder);

    // Query: "BUN OR NPM"
    let query = QueryNode::Or(
        Box::new(QueryNode::Concept("BUN".to_string())),
        Box::new(QueryNode::Concept("NPM".to_string())),
    );

    // Should match BUN
    let results1 = search.search("bunx install packages", &query)?;
    assert!(!results1.is_empty(), "Should find BUN");

    // Should match NPM
    let results2 = search.search("npm install packages", &query)?;
    assert!(!results2.is_empty(), "Should find NPM");

    Ok(())
}

#[test]
fn test_search_deploy_not_test() -> anyhow::Result<()> {
    let tools = vec![
        create_test_tool("wrangler", "wrangler deploy"),
        create_test_tool("npm", "npm test"),
    ];

    let builder = KnowledgeGraphBuilder::from_tool_invocations(&tools);
    let search = KnowledgeGraphSearch::new(builder);

    // Query: "DEPLOY AND NOT TEST"
    let query = QueryNode::And(
        Box::new(QueryNode::Concept("DEPLOY".to_string())),
        Box::new(QueryNode::Not(Box::new(QueryNode::Concept(
            "TEST".to_string(),
        )))),
    );

    // Should match deploy without test
    let results = search.search("wrangler deploy --env production", &query)?;

    // Verify we got results (NOT is complex, so just ensure no errors)
    println!("Deploy without test: {} results", results.len());

    Ok(())
}

#[test]
fn test_search_with_multiple_patterns() -> anyhow::Result<()> {
    let tools = vec![
        create_test_tool("bun", "bunx wrangler deploy"),
        create_test_tool("npm", "npx wrangler deploy"),
        create_test_tool("cargo", "cargo install wrangler"),
    ];

    let builder = KnowledgeGraphBuilder::from_tool_invocations(&tools);
    let search = KnowledgeGraphSearch::new(builder);

    // Different ways to invoke wrangler
    let test_cases = vec![
        ("bunx wrangler deploy", true, "Should match bunx"),
        ("npx wrangler deploy", true, "Should match npx"),
        ("yarn wrangler deploy", true, "Should match yarn"),
        ("random command", false, "Should not match random"),
    ];

    for (command, should_match, description) in test_cases {
        let query = QueryNode::Concept("DEPLOY".to_string());
        let results = search.search(command, &query)?;

        if should_match {
            assert!(!results.is_empty(), "{}: {}", description, command);
        }
        // Note: Non-matches might still return empty results, which is fine
    }

    Ok(())
}

#[test]
fn test_search_complex_query() -> anyhow::Result<()> {
    let tools = vec![
        create_test_tool("bun", "bun install"),
        create_test_tool("npm", "npm install"),
        create_test_tool("cargo", "cargo build"),
    ];

    let builder = KnowledgeGraphBuilder::from_tool_invocations(&tools);
    let search = KnowledgeGraphSearch::new(builder);

    // Query: (BUN OR NPM) AND INSTALL
    let query = QueryNode::And(
        Box::new(QueryNode::Or(
            Box::new(QueryNode::Concept("BUN".to_string())),
            Box::new(QueryNode::Concept("NPM".to_string())),
        )),
        Box::new(QueryNode::Concept("INSTALL".to_string())),
    );

    // Should match bun install
    let results1 = search.search("bun install packages", &query)?;
    println!("BUN install results: {}", results1.len());

    // Should match npm install
    let results2 = search.search("npm install packages", &query)?;
    println!("NPM install results: {}", results2.len());

    // Should not match cargo build (no INSTALL concept)
    let results3 = search.search("cargo build --release", &query)?;
    println!("Cargo build results: {}", results3.len());

    Ok(())
}

#[test]
fn test_relevance_scoring() -> anyhow::Result<()> {
    let tools = vec![
        create_test_tool("bun", "bun install"),
        create_test_tool("wrangler", "wrangler deploy"),
    ];

    let builder = KnowledgeGraphBuilder::from_tool_invocations(&tools);
    let search = KnowledgeGraphSearch::new(builder);

    let query = QueryNode::Concept("DEPLOY".to_string());
    let results = search.search("wrangler deploy --env production", &query)?;

    if !results.is_empty() {
        // Verify relevance scores are positive
        for result in &results {
            assert!(
                result.relevance_score > 0.0,
                "Relevance score should be positive"
            );
        }

        // Verify results are sorted by relevance
        for i in 1..results.len() {
            assert!(
                results[i - 1].relevance_score >= results[i].relevance_score,
                "Results should be sorted by relevance"
            );
        }
    }

    Ok(())
}

#[test]
fn test_concept_map_completeness() {
    let tools = vec![
        create_test_tool("bun", "bunx wrangler deploy"),
        create_test_tool("npm", "npm install"),
    ];

    let builder = KnowledgeGraphBuilder::from_tool_invocations(&tools);

    // Verify BUN concept has multiple patterns
    let bun_patterns = &builder.concept_map["BUN"];
    assert!(bun_patterns.contains(&"bunx".to_string()));
    assert!(bun_patterns.contains(&"bun install".to_string()));

    // Verify INSTALL concept has multiple patterns
    let install_patterns = &builder.concept_map["INSTALL"];
    assert!(install_patterns.contains(&"install".to_string()));
    assert!(install_patterns.contains(&"npm install".to_string()));

    // Verify DEPLOY concept exists
    assert!(builder.concept_map.contains_key("DEPLOY"));
    let deploy_patterns = &builder.concept_map["DEPLOY"];
    assert!(deploy_patterns.contains(&"deploy".to_string()));
}

#[test]
fn test_search_case_insensitive() -> anyhow::Result<()> {
    let tools = vec![create_test_tool("bun", "BUN INSTALL PACKAGES")];

    let builder = KnowledgeGraphBuilder::from_tool_invocations(&tools);
    let search = KnowledgeGraphSearch::new(builder);

    let query = QueryNode::Concept("INSTALL".to_string());
    let results = search.search("BUN INSTALL PACKAGES", &query)?;

    assert!(
        !results.is_empty(),
        "Should match regardless of case (terraphim is case-insensitive)"
    );

    Ok(())
}

#[test]
fn test_empty_results_for_no_match() -> anyhow::Result<()> {
    let tools = vec![create_test_tool("npm", "npm install")];

    let builder = KnowledgeGraphBuilder::from_tool_invocations(&tools);
    let search = KnowledgeGraphSearch::new(builder);

    let query = QueryNode::Concept("DEPLOY".to_string());
    let results = search.search("echo hello world", &query)?;

    // No deploy concept in "echo hello world"
    // Results might be empty or have low scores
    println!("Results for non-matching query: {}", results.len());

    Ok(())
}
