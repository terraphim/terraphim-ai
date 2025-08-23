#!/usr/bin/env cargo +nightly -Zscript --edition 2021
//! Quick test to verify thesaurus fix

/*
[dependencies]
tokio = { version = "1", features = ["full"] }
terraphim_config = { path = "crates/terraphim_config" }
terraphim_service = { path = "crates/terraphim_service" }
terraphim_types = { path = "crates/terraphim_types" }
*/

use terraphim_config::{ConfigBuilder, ConfigId, ConfigState};
use terraphim_service::TerraphimService;
use terraphim_types::RoleName;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    println!("🧪 Testing thesaurus fix...");

    // Create desktop config like in the actual test
    let mut config = ConfigBuilder::new_with_id(ConfigId::Desktop)
        .build_default_desktop()
        .build()?;

    let config_state = ConfigState::new(&mut config).await?;
    let mut service = TerraphimService::new(config_state);

    // Try to load the Terraphim Engineer thesaurus
    let role_name = RoleName::new("Terraphim Engineer");
    
    println!("📚 Loading thesaurus for: {}", role_name);
    let result = service.ensure_thesaurus_loaded(&role_name).await;

    match result {
        Ok(thesaurus) => {
            println!("✅ SUCCESS: Thesaurus loaded with {} entries", thesaurus.len());
            
            // Test a few entries
            if thesaurus.contains_key("haystack") {
                println!("  ✓ Contains 'haystack' entry");
            }
            if thesaurus.contains_key("service") {
                println!("  ✓ Contains 'service' entry");
            }
            if thesaurus.contains_key("terraphim-graph") {
                println!("  ✓ Contains 'terraphim-graph' entry");
            }
            
            println!("🎉 Thesaurus loading fix works correctly!");
        }
        Err(e) => {
            println!("❌ FAILED: Could not load thesaurus: {:?}", e);
            println!("💡 The fix may need more work or there's a different issue.");
        }
    }

    Ok(())
}