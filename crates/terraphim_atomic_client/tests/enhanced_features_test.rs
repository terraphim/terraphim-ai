use serde_json::json;
use std::collections::HashMap;
use terraphim_atomic_client::{Agent, Commit, CommitBuilder, Config, Store};

#[test]
fn test_enhanced_agent_features() {
    // Test agent with metadata
    let agent = Agent::new();

    // Test that agent has metadata fields
    assert!(agent.created_at > 0);
    assert!(agent.name.is_none());
    assert!(!agent.subject.is_empty());
    assert!(!agent.get_public_key_base64().is_empty());

    // Test agent with name
    let agent_with_name = Agent::new_with_name(
        "Test Agent".to_string(),
        "http://localhost:9883".to_string(),
    );
    assert_eq!(agent_with_name.get_name(), Some("Test Agent"));
    assert!(agent_with_name.get_created_at() > 0);

    // Test agent from private key
    let private_key = "test_private_key_base64"; // This would be a valid base64 key in practice
    let result = Agent::new_from_private_key(
        private_key,
        "http://localhost:9883".to_string(),
        Some("Private Key Agent".to_string()),
    );
    // This should fail with invalid key, but that's expected
    assert!(result.is_err());

    println!("✅ Enhanced agent features test passed");
}

#[test]
fn test_enhanced_commit_features() {
    let agent = Agent::new();

    // Test commit with new fields
    let mut properties = HashMap::new();
    properties.insert(
        "https://atomicdata.dev/properties/name".to_string(),
        json!("Test Resource"),
    );
    properties.insert(
        "https://atomicdata.dev/properties/description".to_string(),
        json!("A test resource"),
    );

    let commit =
        Commit::new_create_or_update("http://localhost:9883/test".to_string(), properties, &agent)
            .unwrap();

    // Test that new fields are properly initialized
    assert!(commit.remove.is_none());
    assert!(commit.push.is_none());
    assert!(commit.previous_commit.is_none());
    assert!(commit.url.is_none());
    assert!(commit.set.is_some());
    assert!(commit.destroy.is_none());

    // Test commit methods
    let mut commit_copy = commit.clone();
    commit_copy.add_remove("https://atomicdata.dev/properties/old_field".to_string());
    assert!(commit_copy.remove.is_some());
    assert_eq!(commit_copy.remove.as_ref().unwrap().len(), 1);

    commit_copy.add_push(
        "https://atomicdata.dev/properties/tags".to_string(),
        json!(["tag1", "tag2"]),
    );
    assert!(commit_copy.push.is_some());
    assert_eq!(commit_copy.push.as_ref().unwrap().len(), 1);

    commit_copy.set_previous_commit("http://localhost:9883/commits/previous".to_string());
    assert!(commit_copy.previous_commit.is_some());

    commit_copy.set_url("http://localhost:9883/commits/current".to_string());
    assert!(commit_copy.url.is_some());

    // Test validation
    assert!(commit_copy.validate().is_ok());

    println!("✅ Enhanced commit features test passed");
}

#[test]
fn test_commit_builder_pattern() {
    let agent = Agent::new();

    // Test CommitBuilder
    let commit_result = CommitBuilder::new("http://localhost:9883/test".to_string())
        .set(
            "https://atomicdata.dev/properties/name".to_string(),
            json!("Built Resource"),
        )
        .set(
            "https://atomicdata.dev/properties/description".to_string(),
            json!("Built with CommitBuilder"),
        )
        .remove("https://atomicdata.dev/properties/old_field".to_string())
        .push(
            "https://atomicdata.dev/properties/tags".to_string(),
            json!(["builder", "test"]),
        )
        .set_previous_commit("http://localhost:9883/commits/previous".to_string())
        .build(&agent);

    assert!(commit_result.is_ok());
    let commit = commit_result.unwrap();

    // Verify all fields are set correctly
    assert!(commit.set.is_some());
    assert_eq!(commit.set.as_ref().unwrap().len(), 2);

    assert!(commit.remove.is_some());
    assert_eq!(commit.remove.as_ref().unwrap().len(), 1);

    assert!(commit.push.is_some());
    assert_eq!(commit.push.as_ref().unwrap().len(), 1);

    assert!(commit.previous_commit.is_some());
    assert_eq!(
        commit.previous_commit.as_ref().unwrap(),
        "http://localhost:9883/commits/previous"
    );

    // Test destroy flag
    let destroy_commit = CommitBuilder::new("http://localhost:9883/test".to_string())
        .destroy(true)
        .build(&agent)
        .unwrap();

    assert!(destroy_commit.destroy.is_some());
    assert!(destroy_commit.destroy.unwrap());

    println!("✅ CommitBuilder pattern test passed");
}

#[test]
fn test_commit_validation() {
    let agent = Agent::new();

    // Test circular parent reference detection
    let mut properties = HashMap::new();
    properties.insert(
        "https://atomicdata.dev/properties/parent".to_string(),
        json!("http://localhost:9883/test"),
    );

    let commit =
        Commit::new_create_or_update("http://localhost:9883/test".to_string(), properties, &agent)
            .unwrap();

    // This should fail validation due to circular reference
    assert!(commit.validate().is_err());

    // Test valid commit
    let mut valid_properties = HashMap::new();
    valid_properties.insert(
        "https://atomicdata.dev/properties/name".to_string(),
        json!("Valid Resource"),
    );

    let valid_commit = Commit::new_create_or_update(
        "http://localhost:9883/test".to_string(),
        valid_properties,
        &agent,
    )
    .unwrap();

    assert!(valid_commit.validate().is_ok());

    println!("✅ Commit validation test passed");
}

#[test]
fn test_atomic_haystack_compatibility() {
    // Test that the enhanced client maintains API compatibility
    let config = Config {
        server_url: "http://localhost:9883".to_string(),
        agent: Some(Agent::new()),
    };

    // This should create without errors
    let store_result = Store::new(config);
    assert!(store_result.is_ok());

    // Test that we can still create basic commits (backward compatibility)
    let agent = Agent::new();
    let mut properties = HashMap::new();
    properties.insert(
        "https://atomicdata.dev/properties/name".to_string(),
        json!("Test Document"),
    );

    let commit_result = Commit::new_create_or_update(
        "http://localhost:9883/documents/test".to_string(),
        properties,
        &agent,
    );
    assert!(commit_result.is_ok());

    let commit = commit_result.unwrap();
    assert!(commit.signature.is_none()); // Should be unsigned initially

    // Test that signing still works
    let signed_commit_result = commit.sign(&agent);
    assert!(signed_commit_result.is_ok());

    let signed_commit = signed_commit_result.unwrap();
    assert!(signed_commit.signature.is_some());

    println!("✅ Atomic haystack compatibility test passed");
}

#[test]
fn test_agent_creation_methods() {
    // Test various agent creation methods
    let agent1 = Agent::new();
    assert!(agent1.created_at > 0);
    assert!(agent1.name.is_none());

    let agent2 = Agent::new_with_name(
        "Named Agent".to_string(),
        "http://localhost:9883".to_string(),
    );
    assert_eq!(agent2.get_name(), Some("Named Agent"));
    assert!(agent2.get_created_at() > 0);

    // Test agent mutation
    let mut agent3 = Agent::new();
    agent3.set_name("Modified Agent".to_string());
    assert_eq!(agent3.get_name(), Some("Modified Agent"));

    println!("✅ Agent creation methods test passed");
}

#[test]
fn test_commit_serialization() {
    let agent = Agent::new();

    // Create a commit with all possible fields
    let commit = CommitBuilder::new("http://localhost:9883/test".to_string())
        .set(
            "https://atomicdata.dev/properties/name".to_string(),
            json!("Full Featured Resource"),
        )
        .set(
            "https://atomicdata.dev/properties/description".to_string(),
            json!("Has all features"),
        )
        .remove("https://atomicdata.dev/properties/old_field".to_string())
        .push(
            "https://atomicdata.dev/properties/tags".to_string(),
            json!(["tag1", "tag2"]),
        )
        .set_previous_commit("http://localhost:9883/commits/previous".to_string())
        .build(&agent)
        .unwrap();

    // Test JSON serialization
    let json_result = commit.to_json();
    assert!(json_result.is_ok());

    let json = json_result.unwrap();
    assert!(json.is_object());

    let obj = json.as_object().unwrap();
    assert!(obj.contains_key("https://atomicdata.dev/properties/subject"));
    assert!(obj.contains_key("https://atomicdata.dev/properties/set"));
    assert!(obj.contains_key("https://atomicdata.dev/properties/remove"));
    assert!(obj.contains_key("https://atomicdata.dev/properties/push"));
    assert!(obj.contains_key("https://atomicdata.dev/properties/previousCommit"));

    println!("✅ Commit serialization test passed");
}
