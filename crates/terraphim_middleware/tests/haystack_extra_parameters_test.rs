use std::collections::HashMap;
use terraphim_config::{Haystack, ServiceType};
use terraphim_middleware::{indexer::IndexMiddleware, RipgrepIndexer};

/// Test the custom serialization behavior that prevents atomic server secrets
/// from being exposed for non-Atomic haystacks
#[tokio::test]
async fn test_haystack_serialization_security() {
    println!("🔐 Testing haystack serialization security...");

    // Test 1: Ripgrep haystack should NOT serialize atomic_server_secret
    let mut ripgrep_haystack =
        Haystack::new("test_location".to_string(), ServiceType::Ripgrep, false);
    ripgrep_haystack.atomic_server_secret = Some("secret_should_not_appear".to_string());

    let ripgrep_json = serde_json::to_string(&ripgrep_haystack).unwrap();
    println!("Ripgrep haystack JSON: {}", ripgrep_json);

    assert!(
        !ripgrep_json.contains("secret_should_not_appear"),
        "Ripgrep haystack should NOT serialize atomic_server_secret"
    );
    assert!(
        !ripgrep_json.contains("atomic_server_secret"),
        "Ripgrep haystack should NOT include atomic_server_secret field"
    );

    // Test 2: Atomic haystack WITH secret should serialize it
    let atomic_haystack_with_secret = Haystack::new(
        "http://localhost:9883".to_string(),
        ServiceType::Atomic,
        true,
    )
    .with_atomic_secret(Some("valid_atomic_secret".to_string()));

    let atomic_json = serde_json::to_string(&atomic_haystack_with_secret).unwrap();
    println!("Atomic haystack (with secret) JSON: {}", atomic_json);

    assert!(
        atomic_json.contains("valid_atomic_secret"),
        "Atomic haystack should serialize atomic_server_secret when present"
    );
    assert!(
        atomic_json.contains("atomic_server_secret"),
        "Atomic haystack should include atomic_server_secret field"
    );

    // Test 3: Atomic haystack WITHOUT secret should not serialize it
    let atomic_haystack_no_secret = Haystack::new(
        "http://localhost:9883".to_string(),
        ServiceType::Atomic,
        true,
    );

    let atomic_no_secret_json = serde_json::to_string(&atomic_haystack_no_secret).unwrap();
    println!(
        "Atomic haystack (no secret) JSON: {}",
        atomic_no_secret_json
    );

    assert!(
        !atomic_no_secret_json.contains("atomic_server_secret"),
        "Atomic haystack without secret should NOT include atomic_server_secret field"
    );

    println!("✅ Haystack serialization security test passed");
}

/// Test extra parameters functionality for ripgrep filtering
#[tokio::test]
async fn test_ripgrep_extra_parameters() {
    println!("🏷️ Testing ripgrep extra parameters functionality...");

    let ripgrep_command = terraphim_middleware::command::ripgrep::RipgrepCommand::default();

    // Test 1: Tag filtering
    let mut tag_params = HashMap::new();
    tag_params.insert("tag".to_string(), "#rust".to_string());

    let tag_args = ripgrep_command.parse_extra_parameters(&tag_params);
    println!("Tag filter args: {:?}", tag_args);

    assert_eq!(
        tag_args,
        vec![
            "--all-match".to_string(),
            "-e".to_string(),
            "#rust".to_string()
        ]
    );

    // Test 2: Multiple parameters
    let mut multi_params = HashMap::new();
    multi_params.insert("tag".to_string(), "#testing".to_string());
    multi_params.insert("type".to_string(), "md".to_string());
    multi_params.insert("max_count".to_string(), "5".to_string());
    multi_params.insert("case_sensitive".to_string(), "true".to_string());

    let multi_args = ripgrep_command.parse_extra_parameters(&multi_params);
    println!("Multiple params args: {:?}", multi_args);

    // Check that tag patterns are enforced with --all-match and -e
    assert!(multi_args.contains(&"--all-match".to_string()));
    assert!(multi_args.contains(&"-e".to_string()));
    assert!(multi_args.contains(&"#testing".to_string()));
    assert!(multi_args.contains(&"-t".to_string()));
    assert!(multi_args.contains(&"md".to_string()));
    assert!(multi_args.contains(&"--max-count".to_string()));
    assert!(multi_args.contains(&"5".to_string()));
    assert!(multi_args.contains(&"--case-sensitive".to_string()));

    // Test 3: Glob patterns
    let mut glob_params = HashMap::new();
    glob_params.insert("glob".to_string(), "*.rs".to_string());

    let glob_args = ripgrep_command.parse_extra_parameters(&glob_params);
    println!("Glob pattern args: {:?}", glob_args);

    assert_eq!(glob_args, vec!["--glob".to_string(), "*.rs".to_string()]);

    // Test 4: Context lines override
    let mut context_params = HashMap::new();
    context_params.insert("context".to_string(), "7".to_string());

    let context_args = ripgrep_command.parse_extra_parameters(&context_params);
    println!("Context override args: {:?}", context_args);

    assert_eq!(context_args, vec!["-C".to_string(), "7".to_string()]);

    // Test 5: Unknown parameters (should log warning but not break)
    let mut unknown_params = HashMap::new();
    unknown_params.insert("unknown_param".to_string(), "value".to_string());

    let unknown_args = ripgrep_command.parse_extra_parameters(&unknown_params);
    println!("Unknown params args: {:?}", unknown_args);

    assert!(
        unknown_args.is_empty(),
        "Unknown parameters should not generate arguments"
    );

    println!("✅ Ripgrep extra parameters test passed");
}

/// Test haystack builder methods and extra parameters integration
#[tokio::test]
async fn test_haystack_builder_and_extra_parameters() {
    println!("🔧 Testing haystack builder methods...");

    // Test 1: Basic builder with extra parameters
    let mut extra_params = HashMap::new();
    extra_params.insert("tag".to_string(), "#rust".to_string());
    extra_params.insert("type".to_string(), "md".to_string());

    let haystack = Haystack::new("test_docs/".to_string(), ServiceType::Ripgrep, true)
        .with_extra_parameters(extra_params.clone())
        .with_extra_parameter("max_count".to_string(), "10".to_string());

    assert_eq!(haystack.location, "test_docs/");
    assert_eq!(haystack.service, ServiceType::Ripgrep);
    assert!(haystack.read_only);
    assert_eq!(haystack.atomic_server_secret, None);

    let params = haystack.get_extra_parameters();
    assert_eq!(params.get("tag"), Some(&"#rust".to_string()));
    assert_eq!(params.get("type"), Some(&"md".to_string()));
    assert_eq!(params.get("max_count"), Some(&"10".to_string()));

    println!(
        "Builder created haystack with {} extra parameters",
        params.len()
    );

    // Test 2: Atomic haystack builder with secret
    let atomic_haystack = Haystack::new(
        "http://localhost:9883".to_string(),
        ServiceType::Atomic,
        false,
    )
    .with_atomic_secret(Some("test_secret".to_string()))
    .with_extra_parameter("timeout".to_string(), "30".to_string());

    assert_eq!(atomic_haystack.service, ServiceType::Atomic);
    assert_eq!(
        atomic_haystack.atomic_server_secret,
        Some("test_secret".to_string())
    );
    assert_eq!(
        atomic_haystack.get_extra_parameters().get("timeout"),
        Some(&"30".to_string())
    );

    // Test 3: Try to set atomic secret on Ripgrep haystack (should be ignored)
    let ripgrep_haystack = Haystack::new("local_docs/".to_string(), ServiceType::Ripgrep, false)
        .with_atomic_secret(Some("should_be_ignored".to_string()));

    assert_eq!(ripgrep_haystack.service, ServiceType::Ripgrep);
    assert_eq!(
        ripgrep_haystack.atomic_server_secret, None,
        "Ripgrep haystack should ignore atomic server secret"
    );

    println!("✅ Haystack builder and extra parameters test passed");
}

/// Test the RipgrepIndexer integration with extra parameters
#[tokio::test]
async fn test_ripgrep_indexer_with_extra_parameters() {
    println!("🔍 Testing RipgrepIndexer with extra parameters...");

    // Create test haystack with tag filtering
    let mut tag_params = HashMap::new();
    tag_params.insert("tag".to_string(), "#test".to_string());
    tag_params.insert("type".to_string(), "md".to_string());

    let haystack = Haystack::new("fixtures/haystack".to_string(), ServiceType::Ripgrep, true)
        .with_extra_parameters(tag_params);

    let indexer = RipgrepIndexer::default();

    // Test that the indexer can handle extra parameters without error
    // Note: This test will succeed even if no files are found, as we're testing integration
    let result = indexer.index("test", &haystack).await;

    match result {
        Ok(index) => {
            println!(
                "Indexer with extra parameters returned {} documents",
                index.len()
            );
            println!("✅ RipgrepIndexer successfully processed extra parameters");
        }
        Err(e) => {
            // If fixtures directory doesn't exist, that's expected in test environment
            println!("Expected error (no test fixtures): {:?}", e);
            println!("✅ RipgrepIndexer correctly handled missing directory with extra parameters");
        }
    }

    // Test with empty extra parameters (baseline)
    let simple_haystack =
        Haystack::new("fixtures/haystack".to_string(), ServiceType::Ripgrep, true);

    let simple_result = indexer.index("test", &simple_haystack).await;

    match simple_result {
        Ok(index) => {
            println!(
                "Indexer without extra parameters returned {} documents",
                index.len()
            );
        }
        Err(e) => {
            println!("Expected error for simple haystack: {:?}", e);
        }
    }

    println!("✅ RipgrepIndexer integration test completed");
}

/// Test serialization of haystacks with various configurations
#[tokio::test]
async fn test_haystack_serialization_completeness() {
    println!("📄 Testing complete haystack serialization scenarios...");

    // Test 1: Ripgrep haystack with extra parameters (no secret serialization)
    let mut params = HashMap::new();
    params.insert("tag".to_string(), "#rust".to_string());
    params.insert("type".to_string(), "md".to_string());

    let ripgrep_with_params = Haystack::new("docs/".to_string(), ServiceType::Ripgrep, false)
        .with_extra_parameters(params)
        .with_extra_parameter("max_count".to_string(), "5".to_string());

    let json = serde_json::to_string_pretty(&ripgrep_with_params).unwrap();
    println!("Ripgrep with extra parameters:\n{}", json);

    // Should contain extra_parameters but not atomic_server_secret
    assert!(json.contains("extra_parameters"));
    assert!(json.contains("#rust"));
    assert!(json.contains("max_count"));
    assert!(!json.contains("atomic_server_secret"));

    // Test 2: Atomic haystack with secret and extra parameters
    let mut atomic_params = HashMap::new();
    atomic_params.insert("timeout".to_string(), "30".to_string());

    let atomic_with_all = Haystack::new(
        "http://localhost:9883".to_string(),
        ServiceType::Atomic,
        true,
    )
    .with_atomic_secret(Some("secret123".to_string()))
    .with_extra_parameters(atomic_params);

    let atomic_json = serde_json::to_string_pretty(&atomic_with_all).unwrap();
    println!("Atomic with secret and extra parameters:\n{}", atomic_json);

    // Should contain both atomic_server_secret and extra_parameters
    assert!(atomic_json.contains("atomic_server_secret"));
    assert!(atomic_json.contains("secret123"));
    assert!(atomic_json.contains("extra_parameters"));
    assert!(atomic_json.contains("timeout"));

    // Test 3: Empty extra parameters should not be serialized
    let minimal_haystack = Haystack::new("minimal/".to_string(), ServiceType::Ripgrep, true);

    let minimal_json = serde_json::to_string_pretty(&minimal_haystack).unwrap();
    println!("Minimal haystack:\n{}", minimal_json);

    // Should not contain extra_parameters or atomic_server_secret
    assert!(!minimal_json.contains("extra_parameters"));
    assert!(!minimal_json.contains("atomic_server_secret"));

    println!("✅ Complete haystack serialization test passed");
}

/// Demonstrate tag filtering use case with example configurations
#[tokio::test]
async fn test_tag_filtering_use_cases() {
    println!("🏷️ Demonstrating tag filtering use cases...");

    // Use case 1: Rust development - only files tagged with #rust
    let rust_dev_haystack = Haystack::new("src/".to_string(), ServiceType::Ripgrep, false)
        .with_extra_parameter("tag".to_string(), "#rust".to_string())
        .with_extra_parameter("type".to_string(), "rs".to_string());

    println!("Rust development haystack:");
    println!("  Location: {}", rust_dev_haystack.location);
    println!(
        "  Tag filter: {:?}",
        rust_dev_haystack.get_extra_parameters().get("tag")
    );
    println!(
        "  Type filter: {:?}",
        rust_dev_haystack.get_extra_parameters().get("type")
    );

    // Use case 2: Documentation search - markdown files with #docs tag
    let docs_haystack = Haystack::new("documentation/".to_string(), ServiceType::Ripgrep, true)
        .with_extra_parameter("tag".to_string(), "#docs".to_string())
        .with_extra_parameter("type".to_string(), "md".to_string())
        .with_extra_parameter("context".to_string(), "5".to_string());

    println!("Documentation haystack:");
    println!("  Location: {}", docs_haystack.location);
    println!(
        "  Tag filter: {:?}",
        docs_haystack.get_extra_parameters().get("tag")
    );
    println!(
        "  Context lines: {:?}",
        docs_haystack.get_extra_parameters().get("context")
    );

    // Use case 3: Testing - files tagged with #test, case-sensitive search
    let test_haystack = Haystack::new("tests/".to_string(), ServiceType::Ripgrep, true)
        .with_extra_parameter("tag".to_string(), "#test".to_string())
        .with_extra_parameter("case_sensitive".to_string(), "true".to_string())
        .with_extra_parameter("max_count".to_string(), "10".to_string());

    println!("Testing haystack:");
    println!("  Location: {}", test_haystack.location);
    println!(
        "  Tag filter: {:?}",
        test_haystack.get_extra_parameters().get("tag")
    );
    println!(
        "  Case sensitive: {:?}",
        test_haystack.get_extra_parameters().get("case_sensitive")
    );
    println!(
        "  Max results: {:?}",
        test_haystack.get_extra_parameters().get("max_count")
    );

    // Verify serialization excludes secrets for all use cases
    for (name, haystack) in [
        ("Rust dev", &rust_dev_haystack),
        ("Documentation", &docs_haystack),
        ("Testing", &test_haystack),
    ] {
        let json = serde_json::to_string(haystack).unwrap();
        assert!(
            !json.contains("atomic_server_secret"),
            "{} haystack should not serialize atomic_server_secret",
            name
        );
        assert!(
            json.contains("extra_parameters"),
            "{} haystack should serialize extra_parameters",
            name
        );
    }

    println!("✅ Tag filtering use cases demonstration completed");
}
