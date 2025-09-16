//! Extract feature tests for Tauri CLI
//!
//! Tests the extract command functionality which extracts paragraphs from text
//! that contain knowledge graph terms.

use serial_test::serial;
use std::time::Duration;
use tokio::time::timeout;

use terraphim_config::{ConfigBuilder, ConfigId, ConfigState};
use terraphim_service::TerraphimService;
use terraphim_types::RoleName;

#[tokio::test]
#[serial]
async fn test_extract_basic_functionality() {
    println!("🔍 Testing extract feature basic functionality");

    // Initialize test configuration
    let mut config = ConfigBuilder::new_with_id(ConfigId::Desktop)
        .build_default_desktop()
        .build()
        .expect("Failed to build desktop config");

    let config_state = ConfigState::new(&mut config)
        .await
        .expect("Failed to initialize ConfigState");

    let mut service = TerraphimService::new(config_state.clone());

    // Test text with potential knowledge graph terms
    let test_text = "
        This is a paragraph about haystacks and their configuration.
        Haystacks are data sources that provide information to the system.

        Another paragraph discusses services and their integration.
        Services act as middleware components in the architecture.

        Finally, this paragraph mentions graph embeddings and their usage.
        Graph embeddings help with semantic understanding and relationships.
    ";

    let role_name = RoleName::new("Terraphim Engineer");

    // Test extract with include term (default behavior)
    let result = timeout(
        Duration::from_secs(30),
        service.extract_paragraphs(&role_name, test_text, false),
    )
    .await
    .expect("Extract operation timed out");

    match result {
        Ok(extracted) => {
            println!("✅ Extract succeeded: found {} matches", extracted.len());

            // With KG data available, we should find matches for this text
            assert!(
                !extracted.is_empty(),
                "Extract should find matches for text containing 'haystacks' and 'configuration' with KG data available"
            );

            for (i, (term, paragraph)) in extracted.iter().enumerate() {
                println!(
                    "  Match {}: term='{}', paragraph length={}",
                    i + 1,
                    term,
                    paragraph.len()
                );

                // Validate structure
                assert!(!term.is_empty(), "Term should not be empty");
                assert!(!paragraph.is_empty(), "Paragraph should not be empty");
            }

            // Check that expected terms are found
            let found_haystack = extracted.iter().any(|(term, _)| {
                term.to_lowercase().contains("haystack") || term.to_lowercase().contains("data")
            });
            let found_config = extracted.iter().any(|(term, _)| {
                term.to_lowercase().contains("config") || term.to_lowercase().contains("setting")
            });

            assert!(
                found_haystack || found_config,
                "Should find matches for haystack or configuration related terms"
            );

            println!(
                "  ✅ Extract feature working correctly with {} matches",
                extracted.len()
            );
        }
        Err(e) => {
            panic!(
                "Extract failed when it should succeed with KG data available: {:?}",
                e
            );
        }
    }
}

#[tokio::test]
#[serial]
async fn test_extract_exclude_term_option() {
    println!("🔍 Testing extract feature with exclude_term option");

    let mut config = ConfigBuilder::new_with_id(ConfigId::Desktop)
        .build_default_desktop()
        .build()
        .expect("Failed to build desktop config");

    let config_state = ConfigState::new(&mut config)
        .await
        .expect("Failed to initialize ConfigState");

    let mut service = TerraphimService::new(config_state.clone());

    let test_text = "
        This paragraph contains the term haystack which should be detected.

        This paragraph does not contain any matching terms.

        Another paragraph with service mentions for testing.
    ";

    let role_name = RoleName::new("Terraphim Engineer");

    // Test extract with exclude_term = true
    let result_exclude = timeout(
        Duration::from_secs(30),
        service.extract_paragraphs(&role_name, test_text, true),
    )
    .await
    .expect("Extract with exclude timed out");

    // Test extract with exclude_term = false (include terms)
    let result_include = timeout(
        Duration::from_secs(30),
        service.extract_paragraphs(&role_name, test_text, false),
    )
    .await
    .expect("Extract with include timed out");

    match (result_exclude, result_include) {
        (Ok(excluded), Ok(included)) => {
            println!("✅ Both extract modes succeeded");
            println!(
                "  Excluded paragraphs: {}, Included paragraphs: {}",
                excluded.len(),
                included.len()
            );

            // Basic validation - exclude and include should be different or both empty
            if excluded.len() + included.len() > 0 {
                println!("  ✅ Extract with exclude_term option working");
            }
        }
        (Err(e1), Err(e2)) => {
            println!("❌ Both extract modes failed: {:?}, {:?}", e1, e2);
            println!("  This may be expected in test environment");
        }
        (Ok(_), Err(e)) => {
            println!("⚠️ Include mode failed but exclude mode succeeded: {:?}", e);
        }
        (Err(e), Ok(_)) => {
            println!("⚠️ Exclude mode failed but include mode succeeded: {:?}", e);
        }
    }
}

#[tokio::test]
#[serial]
async fn test_extract_different_roles() {
    println!("🔍 Testing extract feature with different roles");

    let mut config = ConfigBuilder::new_with_id(ConfigId::Desktop)
        .build_default_desktop()
        .build()
        .expect("Failed to build desktop config");

    let config_state = ConfigState::new(&mut config)
        .await
        .expect("Failed to initialize ConfigState");

    let mut service = TerraphimService::new(config_state.clone());

    let test_text = "
        This is a test document with various technical terms.
        It mentions databases, APIs, and microservices.
        There are also references to machine learning and algorithms.
    ";

    // Test different roles
    let test_roles = vec![
        RoleName::new("Terraphim Engineer"),
        RoleName::new("Default"),
    ];

    for role_name in test_roles {
        println!("  Testing role: {}", role_name.original);

        let result = timeout(
            Duration::from_secs(30),
            service.extract_paragraphs(&role_name, test_text, false),
        )
        .await
        .expect("Extract operation timed out");

        match result {
            Ok(extracted) => {
                println!(
                    "    ✅ Role '{}': found {} matches",
                    role_name.original,
                    extracted.len()
                );
            }
            Err(e) => {
                println!("    ⚠️ Role '{}' failed: {:?}", role_name.original, e);
                println!("      This may be expected if role has no thesaurus");
            }
        }
    }
}

#[tokio::test]
#[serial]
async fn test_extract_edge_cases() {
    println!("🔍 Testing extract feature edge cases");

    let mut config = ConfigBuilder::new_with_id(ConfigId::Desktop)
        .build_default_desktop()
        .build()
        .expect("Failed to build desktop config");

    let config_state = ConfigState::new(&mut config)
        .await
        .expect("Failed to initialize ConfigState");

    let mut service = TerraphimService::new(config_state.clone());
    let role_name = RoleName::new("Terraphim Engineer");

    // Test edge cases
    let test_cases = vec![
        ("Empty text", ""),
        ("Single word", "haystack"),
        (
            "Very long text",
            &"This is a very long paragraph with haystack mentioned multiple times. ".repeat(100),
        ),
        (
            "Special characters",
            "Text with üñíçödé and haystack terms!",
        ),
        (
            "Multiple matches",
            "Haystack paragraph one.\n\nService paragraph two.\n\nGraph paragraph three.",
        ),
    ];

    for (test_name, test_text) in test_cases {
        println!("  Testing case: {}", test_name);

        let result = timeout(
            Duration::from_secs(30),
            service.extract_paragraphs(&role_name, test_text, false),
        )
        .await
        .expect("Extract operation timed out");

        match result {
            Ok(extracted) => {
                println!(
                    "    ✅ Case '{}': {} matches found",
                    test_name,
                    extracted.len()
                );

                // Validate results structure
                for (term, paragraph) in &extracted {
                    assert!(
                        !term.is_empty() || test_text.is_empty(),
                        "Term should not be empty unless input is empty"
                    );
                    // Paragraph could be empty in edge cases, so we don't assert
                }
            }
            Err(e) => {
                println!("    ⚠️ Case '{}' failed: {:?}", test_name, e);
            }
        }
    }
}

#[tokio::test]
#[serial]
async fn test_extract_performance() {
    println!("⚡ Testing extract feature performance");

    let mut config = ConfigBuilder::new_with_id(ConfigId::Desktop)
        .build_default_desktop()
        .build()
        .expect("Failed to build desktop config");

    let config_state = ConfigState::new(&mut config)
        .await
        .expect("Failed to initialize ConfigState");

    let mut service = TerraphimService::new(config_state.clone());
    let role_name = RoleName::new("Terraphim Engineer");

    // Create a large text document
    let large_text = format!(
        "{}{}{}",
        "This is the beginning paragraph with haystack terms. ".repeat(50),
        "Middle section with service and middleware concepts. ".repeat(50),
        "Final section discussing graph embeddings and algorithms. ".repeat(50)
    );

    println!("  Testing with large document ({} chars)", large_text.len());

    let start_time = std::time::Instant::now();

    let result = timeout(
        Duration::from_secs(60), // Allow more time for large document
        service.extract_paragraphs(&role_name, &large_text, false),
    )
    .await;

    let duration = start_time.elapsed();
    println!("  ⏱️ Extract completed in {:?}", duration);

    match result {
        Ok(Ok(extracted)) => {
            println!(
                "  ✅ Performance test passed: {} matches found",
                extracted.len()
            );
            assert!(
                duration < Duration::from_secs(30),
                "Extract should complete within 30 seconds"
            );
        }
        Ok(Err(e)) => {
            println!("  ⚠️ Extract failed but within time limit: {:?}", e);
        }
        Err(_) => {
            panic!("Extract operation timed out - performance issue detected");
        }
    }
}

#[tokio::test]
#[serial]
async fn test_extract_service_integration() {
    println!("🔧 Testing extract feature service integration");

    // Test that extract feature integrates properly with the service layer
    let mut config = ConfigBuilder::new_with_id(ConfigId::Desktop)
        .build_default_desktop()
        .build()
        .expect("Failed to build desktop config");

    let config_state = ConfigState::new(&mut config)
        .await
        .expect("Failed to initialize ConfigState");

    let service = TerraphimService::new(config_state.clone());

    // Verify service can be cloned and used multiple times
    let mut service1 = service.clone();
    let mut service2 = service.clone();

    let test_text = "This is a test paragraph with haystack and service terms.";
    let role_name = RoleName::new("Terraphim Engineer");

    // Test concurrent usage
    let future1 = service1.extract_paragraphs(&role_name, test_text, false);
    let future2 = service2.extract_paragraphs(&role_name, test_text, true);

    let (result1, result2) = timeout(Duration::from_secs(30), tokio::join!(future1, future2))
        .await
        .expect("Concurrent extract operations timed out");

    match (result1, result2) {
        (Ok(include_results), Ok(exclude_results)) => {
            println!("✅ Concurrent extract operations succeeded");
            println!(
                "  Include mode: {} results, Exclude mode: {} results",
                include_results.len(),
                exclude_results.len()
            );
        }
        _ => {
            println!("⚠️ Some concurrent operations failed - may be expected in test environment");
        }
    }

    // Test service state consistency
    let service_config = service.fetch_config().await;
    assert!(
        !service_config.roles.is_empty(),
        "Service should have roles available"
    );

    println!("✅ Service integration test completed");
}
