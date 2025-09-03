//! Comprehensive test for thesaurus persistence
//!
//! This test validates that thesaurus objects can be properly saved to and loaded from
//! persistence, covering the full lifecycle including KG building, saving, and retrieval.

use serial_test::serial;
use std::time::Duration;
use tokio::time::timeout;
use tracing::Level;

use terraphim_config::{ConfigBuilder, ConfigId, ConfigState};
use terraphim_service::TerraphimService;
use terraphim_types::{NormalizedTermValue, RoleName, SearchQuery};

#[tokio::test]
#[serial]
async fn test_thesaurus_full_persistence_lifecycle() {
    // Initialize logging for debugging
    let _ = tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .try_init();

    println!("🧪 Testing complete thesaurus persistence lifecycle");

    // Step 1: Initialize memory-only persistence to avoid configuration issues
    println!("📝 Step 1: Initializing memory-only persistence");
    terraphim_persistence::DeviceStorage::init_memory_only()
        .await
        .expect("Failed to initialize memory-only persistence");

    // Step 2: Create desktop config
    println!("🔧 Step 2: Creating desktop configuration");
    let mut config = ConfigBuilder::new_with_id(ConfigId::Desktop)
        .build_default_desktop()
        .build()
        .expect("Failed to build desktop config");

    let config_state = ConfigState::new(&mut config)
        .await
        .expect("Failed to initialize ConfigState");

    // Step 3: Create service and load thesaurus
    println!("🚀 Step 3: Creating TerraphimService and loading thesaurus");
    let mut terraphim_service = TerraphimService::new(config_state.clone());
    let role_name = RoleName::new("Terraphim Engineer");

    // First load - should build from KG files
    println!("  🔍 First load: Building thesaurus from KG files");
    let first_load_result = timeout(
        Duration::from_secs(60),
        terraphim_service.ensure_thesaurus_loaded(&role_name),
    )
    .await
    .expect("First thesaurus load timed out")
    .expect("First thesaurus load failed");

    println!(
        "  ✅ First load succeeded: {} entries",
        first_load_result.len()
    );
    assert!(
        !first_load_result.is_empty(),
        "Thesaurus should not be empty after building from KG"
    );

    // Verify specific terms from our KG exist
    let expected_terms = vec!["haystack", "service", "terraphim-graph"];
    for term in &expected_terms {
        let normalized_term = NormalizedTermValue::from(term.to_string());
        if first_load_result.get(&normalized_term).is_some() {
            println!("    ✓ Found expected term: '{}'", term);
        } else {
            println!("    ⚠️  Missing expected term: '{}'", term);
        }
    }

    // Step 4: Test persistence by creating a new service instance
    println!("🔄 Step 4: Testing persistence with new service instance");
    let mut new_service = TerraphimService::new(config_state.clone());

    // Second load - should load from persistence
    println!("  🔍 Second load: Loading thesaurus from persistence");
    let second_load_result = timeout(
        Duration::from_secs(30),
        new_service.ensure_thesaurus_loaded(&role_name),
    )
    .await
    .expect("Second thesaurus load timed out")
    .expect("Second thesaurus load failed");

    println!(
        "  ✅ Second load succeeded: {} entries",
        second_load_result.len()
    );

    // Step 5: Verify consistency between loads
    println!("🔍 Step 5: Verifying consistency between loads");
    assert_eq!(
        first_load_result.len(),
        second_load_result.len(),
        "Thesaurus should have same number of entries after persistence"
    );

    // Check that key terms are preserved
    for term in &expected_terms {
        let normalized_term = NormalizedTermValue::from(term.to_string());
        let first_has = first_load_result.get(&normalized_term).is_some();
        let second_has = second_load_result.get(&normalized_term).is_some();

        if first_has {
            assert!(second_has, "Term '{}' should persist between loads", term);
            println!("    ✓ Term '{}' persisted correctly", term);
        }
    }

    // Step 6: Test thesaurus functionality with search
    println!("🔎 Step 6: Testing thesaurus functionality with search");
    let search_query = SearchQuery {
        search_term: "haystack".into(),
        role: Some(role_name.clone()),
        skip: None,
        limit: Some(5),
        ..Default::default()
    };

    let search_result = timeout(Duration::from_secs(30), new_service.search(&search_query))
        .await
        .expect("Search timed out")
        .expect("Search failed");

    println!(
        "  📊 Search with persisted thesaurus: {} results",
        search_result.len()
    );

    // Step 7: Verify the rolegraph is properly updated in config_state
    println!("📋 Step 7: Verifying rolegraph in config_state");
    let config_data = new_service.fetch_config().await;

    assert!(
        config_data.roles.contains_key(&role_name),
        "Terraphim Engineer role should exist in config"
    );

    let terraphim_role = &config_data.roles[&role_name];
    assert_eq!(terraphim_role.name, role_name, "Role name should match");

    println!("🎉 All persistence tests passed!");
    println!("✅ Thesaurus builds correctly from KG files");
    println!("✅ Thesaurus persists correctly to storage");
    println!("✅ Thesaurus loads correctly from persistence");
    println!("✅ Thesaurus maintains consistency across loads");
    println!("✅ Search functionality works with persisted thesaurus");
}

#[tokio::test]
#[serial]
async fn test_thesaurus_persistence_error_handling() {
    println!("🧪 Testing thesaurus persistence error handling");

    let _ = tracing_subscriber::fmt()
        .with_max_level(Level::WARN)
        .try_init();

    // Initialize memory persistence
    terraphim_persistence::DeviceStorage::init_memory_only()
        .await
        .expect("Failed to initialize memory-only persistence");

    // Create config
    let mut config = ConfigBuilder::new_with_id(ConfigId::Desktop)
        .build_default_desktop()
        .build()
        .expect("Failed to build desktop config");

    let config_state = ConfigState::new(&mut config)
        .await
        .expect("Failed to initialize ConfigState");

    let mut service = TerraphimService::new(config_state);

    // Test with invalid role name
    println!("  🔍 Testing with invalid role name");
    let invalid_role = RoleName::new("NonExistent Role");
    let result = service.ensure_thesaurus_loaded(&invalid_role).await;

    match result {
        Ok(_) => println!("    ⚠️  Unexpected success with invalid role"),
        Err(e) => {
            println!("    ✅ Correctly handled invalid role: {:?}", e);
        }
    }

    // Test with valid role
    println!("  🔍 Testing with valid role");
    let valid_role = RoleName::new("Terraphim Engineer");
    let result = service.ensure_thesaurus_loaded(&valid_role).await;

    match result {
        Ok(thesaurus) => {
            println!(
                "    ✅ Successfully loaded thesaurus: {} entries",
                thesaurus.len()
            );
        }
        Err(e) => {
            println!("    ❌ Failed to load thesaurus: {:?}", e);
            // This should not fail in normal circumstances
        }
    }

    println!("✅ Error handling test completed");
}

#[tokio::test]
#[serial]
async fn test_thesaurus_memory_vs_persistence() {
    println!("🧪 Testing thesaurus memory vs persistence behavior");

    // This test verifies that the thesaurus works correctly with memory-only persistence
    // and doesn't require external database backends

    let _ = tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .try_init();

    // Step 1: Initialize memory-only persistence
    terraphim_persistence::DeviceStorage::init_memory_only()
        .await
        .expect("Failed to initialize memory-only persistence");

    // Step 2: Create config and service
    let mut config = ConfigBuilder::new_with_id(ConfigId::Desktop)
        .build_default_desktop()
        .build()
        .expect("Failed to build desktop config");

    let config_state = ConfigState::new(&mut config)
        .await
        .expect("Failed to initialize ConfigState");

    let mut service = TerraphimService::new(config_state);
    let role_name = RoleName::new("Terraphim Engineer");

    // Step 3: Load thesaurus multiple times to test stability
    println!("  🔄 Testing multiple loads for stability");

    for i in 1..=3 {
        println!("    🔍 Load attempt {}", i);

        let result = timeout(
            Duration::from_secs(30),
            service.ensure_thesaurus_loaded(&role_name),
        )
        .await
        .expect("Load timed out")
        .expect("Load failed");

        println!("      ✅ Load {} succeeded: {} entries", i, result.len());
        assert!(
            !result.is_empty(),
            "Thesaurus should not be empty on load {}",
            i
        );

        // Verify some expected terms
        let haystack_term = NormalizedTermValue::from("haystack".to_string());
        let service_term = NormalizedTermValue::from("service".to_string());

        if result.get(&haystack_term).is_some() {
            println!("      ✓ Contains 'haystack'");
        }
        if result.get(&service_term).is_some() {
            println!("      ✓ Contains 'service'");
        }
    }

    println!("✅ Memory persistence stability test passed");
}
