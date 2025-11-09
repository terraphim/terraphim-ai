//! Test that thesaurus is prewarmed (built) when switching to a KG-enabled role
//!
//! This test validates that when a user switches to a role with a knowledge graph configured,
//! the thesaurus is built immediately rather than waiting for "first use".

use serial_test::serial;
use std::time::Duration;
use tokio::time::timeout;
use tracing::Level;

use terraphim_config::{ConfigBuilder, ConfigId, ConfigState, KnowledgeGraph};
use terraphim_service::TerraphimService;
use terraphim_types::{KnowledgeGraphInputType, RoleName};

#[tokio::test]
#[serial]
async fn test_thesaurus_prewarm_on_role_switch() {
    // Initialize logging for debugging
    let _ = tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .try_init();

    println!("🧪 Testing thesaurus prewarm on role switch");

    // Step 1: Initialize memory-only persistence
    println!("📝 Step 1: Initializing memory-only persistence");
    terraphim_persistence::DeviceStorage::init_memory_only()
        .await
        .expect("Failed to initialize memory-only persistence");

    // Step 2: Create desktop config with Terraphim Engineer role
    println!("🔧 Step 2: Creating desktop configuration");
    let mut config = ConfigBuilder::new_with_id(ConfigId::Desktop)
        .build_default_desktop()
        .build()
        .expect("Failed to build desktop config");

    // Ensure Terraphim Engineer role exists with KG configured
    let role_name = RoleName::new("Terraphim Engineer");
    if let Some(role) = config.roles.get_mut(&role_name) {
        // Update role to ensure KG is configured
        if role.kg.is_none() {
            role.kg = Some(KnowledgeGraph {
                automata_path: None,
                knowledge_graph_local: Some(terraphim_config::KnowledgeGraphLocal {
                    input_type: KnowledgeGraphInputType::Markdown,
                    path: std::path::PathBuf::from("./docs/src/kg"),
                }),
                public: false,
                publish: false,
            });
        }
    }

    let config_state = ConfigState::new(&mut config)
        .await
        .expect("Failed to initialize ConfigState");

    // Step 3: Create service and switch to a different role first
    println!("🚀 Step 3: Creating service and switching to Default role");
    let mut service = TerraphimService::new(config_state.clone());
    let default_role = RoleName::new("Default");

    service
        .update_selected_role(default_role.clone())
        .await
        .expect("Failed to switch to Default role");

    println!("  ✅ Switched to Default role");

    // Step 4: Switch to Terraphim Engineer - thesaurus should be built
    println!("🔄 Step 4: Switching to Terraphim Engineer (should trigger thesaurus build)");
    service
        .update_selected_role(role_name.clone())
        .await
        .expect("Failed to switch to Terraphim Engineer");

    println!("  ✅ Switched to Terraphim Engineer");

    // Step 5: Verify thesaurus is loaded (not just "will be built on first use")
    println!("🔍 Step 5: Verifying thesaurus is loaded");
    let thesaurus_result = timeout(
        Duration::from_secs(60),
        service.ensure_thesaurus_loaded(&role_name),
    )
    .await
    .expect("Thesaurus load timed out");

    assert!(
        thesaurus_result.is_ok(),
        "Thesaurus should be loaded after role switch, got error: {:?}",
        thesaurus_result.err()
    );

    let thesaurus = thesaurus_result.unwrap();
    assert!(
        !thesaurus.is_empty(),
        "Thesaurus should not be empty after building"
    );

    println!(
        "  ✅ Thesaurus prewarm test passed: {} terms loaded for role '{}'",
        thesaurus.len(),
        role_name.original
    );
}
