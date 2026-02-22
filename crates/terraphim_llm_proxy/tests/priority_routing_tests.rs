//! Priority-based routing tests

use tempfile::TempDir;
use terraphim_llm_proxy::rolegraph_client::RoleGraphClient;
use terraphim_llm_proxy::rolegraph_client::RoutingRule;
use terraphim_llm_proxy::router::Priority;

#[tokio::test]
async fn test_priority_parsing() {
    // Create a temporary directory for test taxonomy files
    let temp_dir = TempDir::new().unwrap();
    let taxonomy_path = temp_dir.path();

    // Create routing_scenarios directory
    let scenarios_dir = taxonomy_path.join("routing_scenarios");
    std::fs::create_dir_all(&scenarios_dir).unwrap();

    // Create a test markdown file with priority directive
    let test_file = scenarios_dir.join("test_rule.md");
    std::fs::write(
        &test_file,
        r#"
# Test Rule

priority:: 85
route:: openai,gpt-4o

This is a test rule for priority parsing.

synonyms:: test, example, priority
"#,
    )
    .unwrap();

    // Create RoleGraphClient and load taxonomy
    let mut client = RoleGraphClient::new(taxonomy_path).unwrap();
    client.load_taxonomy().unwrap();

    // Test that priority was parsed correctly
    let rules = client.get_routing_rules();
    println!("Number of rules found: {}", rules.len());
    for rule in rules.iter() {
        println!(
            "Rule name: '{}', provider: {}, model: {}",
            rule.name, rule.provider, rule.model
        );
    }
    assert!(!rules.is_empty(), "No routing rules were created");

    let test_rule = rules
        .iter()
        .find(|r| r.name.contains("test_rule"))
        .expect("No rule found containing 'test_rule'");
    assert_eq!(test_rule.priority.value(), 3); // 85 maps to High
    assert_eq!(test_rule.provider, "openai");
    assert_eq!(test_rule.model, "gpt-4o");
}

#[tokio::test]
async fn test_priority_routing() {
    // Create a temporary directory for test taxonomy files
    let temp_dir = TempDir::new().unwrap();
    let taxonomy_path = temp_dir.path();

    // Create routing_scenarios directory
    let scenarios_dir = taxonomy_path.join("routing_scenarios");
    std::fs::create_dir_all(&scenarios_dir).unwrap();

    // Create a high priority rule
    let high_priority_file = scenarios_dir.join("urgent.md");
    std::fs::write(
        &high_priority_file,
        r#"
# Urgent Tasks

priority:: 95
route:: openai,gpt-4o

Handles urgent tasks that require immediate attention.

synonyms:: urgent, emergency, critical, asap
"#,
    )
    .unwrap();

    // Create a medium priority rule
    let medium_priority_file = scenarios_dir.join("standard.md");
    std::fs::write(
        &medium_priority_file,
        r#"
# Standard Tasks

priority:: 50
route:: anthropic,claude-3-sonnet

Handles standard development tasks.

synonyms:: standard, normal, regular, typical
"#,
    )
    .unwrap();

    // Create RoleGraphClient and load taxonomy
    let mut client = RoleGraphClient::new(taxonomy_path).unwrap();
    client.load_taxonomy().unwrap();

    // Test priority-based routing
    let urgent_match = client.query_routing_priority("urgent bug fix needed");
    assert!(urgent_match.is_some());

    let urgent_match = urgent_match.unwrap();
    assert_eq!(urgent_match.priority.value(), 3); // 95 maps to High
    assert_eq!(urgent_match.provider, "openai");
    assert_eq!(urgent_match.model, "gpt-4o");

    let standard_match = client.query_routing_priority("standard feature development");
    assert!(standard_match.is_some());

    let standard_match = standard_match.unwrap();
    assert_eq!(standard_match.priority.value(), 2); // 50 maps to Medium
    assert_eq!(standard_match.provider, "anthropic");
    assert_eq!(standard_match.model, "claude-3-sonnet");
}

#[tokio::test]
async fn test_default_priority() {
    // Create a temporary directory for test taxonomy files
    let temp_dir = TempDir::new().unwrap();
    let taxonomy_path = temp_dir.path();

    // Create routing_scenarios directory
    let scenarios_dir = taxonomy_path.join("routing_scenarios");
    std::fs::create_dir_all(&scenarios_dir).unwrap();

    // Create a rule without priority directive (should default to 50)
    let test_file = scenarios_dir.join("default_priority.md");
    std::fs::write(
        &test_file,
        r#"
# Default Priority Rule

route:: ollama,qwen2.5-coder:latest

This rule should get default priority.

synonyms:: default, normal, fallback
"#,
    )
    .unwrap();

    // Create RoleGraphClient and load taxonomy
    let mut client = RoleGraphClient::new(taxonomy_path).unwrap();
    client.load_taxonomy().unwrap();

    // Test that default priority was applied
    let rules = client.get_routing_rules();
    for rule in rules.iter() {
        println!(
            "Default test - Rule name: '{}', provider: {}, model: {}",
            rule.name, rule.provider, rule.model
        );
    }
    let test_rule = rules
        .iter()
        .find(|r| r.name.contains("default_priority"))
        .expect("No rule found containing 'default_priority'");
    assert_eq!(test_rule.priority.value(), 2); // Default medium priority (50 maps to Medium)
}

#[test]
fn test_priority_constants() {
    assert_eq!(Priority::High.value(), 3);
    assert_eq!(Priority::Medium.value(), 2);
    assert_eq!(Priority::Low.value(), 1);
    assert_eq!(Priority::Critical.value(), 4);
}

#[test]
fn test_priority_bounds() {
    // Test priority mapping ranges
    assert_eq!(Priority::new(150).value(), 4); // Above 100 -> Critical
    assert_eq!(Priority::new(0).value(), 1); // 0-25 -> Low
    assert_eq!(Priority::new(3).value(), 1); // 0-25 -> Low
}

#[test]
fn test_routing_rule_creation() {
    let rule = RoutingRule::new(
        "test-rule".to_string(),
        "Test Rule".to_string(),
        vec!["test.*pattern".to_string()],
        "openai".to_string(),
        "gpt-4".to_string(),
        Priority::High,
    );

    assert_eq!(rule.id, "test-rule");
    assert_eq!(rule.concept, "Test Rule");
    assert_eq!(rule.patterns, vec!["test.*pattern".to_string()]);
    assert_eq!(rule.priority, Priority::High);
    assert_eq!(rule.provider, "openai");
    assert_eq!(rule.model, "gpt-4");
    assert!(rule.enabled);
}
