use terraphim_config::{ConfigBuilder, ConfigId};
use terraphim_types::{KnowledgeGraphInputType, RelevanceFunction};

#[tokio::test]
async fn test_desktop_config_engineer_uses_local_kg() {
    // Build desktop configuration
    let config = ConfigBuilder::new_with_id(ConfigId::Desktop)
        .build_default_desktop()
        .build()
        .expect("Should build desktop config successfully");

    // Verify Engineer role configuration
    let engineer_role = config.roles.get(&"Engineer".into())
        .expect("Engineer role should exist");

    // Verify Engineer uses TerraphimGraph relevance function
    assert_eq!(engineer_role.relevance_function, RelevanceFunction::TerraphimGraph,
               "Engineer role should use TerraphimGraph relevance function");

    // Verify Engineer has knowledge graph configuration
    let kg = engineer_role.kg.as_ref()
        .expect("Engineer role should have knowledge graph configuration");

    // Verify local KG configuration points to docs/src/kg
    let local_kg = kg.knowledge_graph_local.as_ref()
        .expect("Engineer role should have local KG configuration");

    assert_eq!(local_kg.input_type, KnowledgeGraphInputType::Markdown,
               "Local KG should use Markdown input type");

    // The path should end with docs/src/kg (relative to user's default data path)
    assert!(local_kg.path.to_string_lossy().contains("docs/src/kg"),
            "Local KG path should point to docs/src/kg directory, got: {:?}", local_kg.path);

    // Verify automata path is local (not remote)
    let automata_path = kg.automata_path.as_ref()
        .expect("Engineer role should have automata path");
    
    // Check that automata path points to a local file (not a remote URL)
    assert!(!automata_path.to_string().starts_with("http"),
            "Engineer role should use local automata path, not remote URL: {:?}", automata_path);

    println!("✅ Engineer role correctly configured with local knowledge graph");
    println!("   - Relevance function: {:?}", engineer_role.relevance_function);
    println!("   - Local KG path: {:?}", local_kg.path);
    println!("   - Automata path: {:?}", automata_path);
}

#[tokio::test]
async fn test_desktop_config_terraphim_engineer_uses_local_kg() {
    // Build desktop configuration
    let config = ConfigBuilder::new_with_id(ConfigId::Desktop)
        .build_default_desktop()
        .build()
        .expect("Should build desktop config successfully");

    // Verify Terraphim Engineer role configuration
    let terraphim_engineer_role = config.roles.get(&"Terraphim Engineer".into())
        .expect("Terraphim Engineer role should exist");

    // Verify Terraphim Engineer uses TerraphimGraph relevance function
    assert_eq!(terraphim_engineer_role.relevance_function, RelevanceFunction::TerraphimGraph,
               "Terraphim Engineer role should use TerraphimGraph relevance function");

    // Verify Terraphim Engineer has knowledge graph configuration
    let kg = terraphim_engineer_role.kg.as_ref()
        .expect("Terraphim Engineer role should have knowledge graph configuration");

    // Verify local KG configuration points to docs/src/kg
    let local_kg = kg.knowledge_graph_local.as_ref()
        .expect("Terraphim Engineer role should have local KG configuration");

    assert_eq!(local_kg.input_type, KnowledgeGraphInputType::Markdown,
               "Local KG should use Markdown input type");

    // The path should end with docs/src/kg
    assert!(local_kg.path.to_string_lossy().contains("docs/src/kg"),
            "Local KG path should point to docs/src/kg directory, got: {:?}", local_kg.path);

    // Verify automata path is local (not remote)
    let automata_path = kg.automata_path.as_ref()
        .expect("Terraphim Engineer role should have automata path");
    
    // Check that automata path points to a local file (not a remote URL)
    assert!(!automata_path.to_string().starts_with("http"),
            "Terraphim Engineer role should use local automata path, not remote URL: {:?}", automata_path);

    println!("✅ Terraphim Engineer role correctly configured with local knowledge graph");
    println!("   - Relevance function: {:?}", terraphim_engineer_role.relevance_function);
    println!("   - Local KG path: {:?}", local_kg.path);
    println!("   - Automata path: {:?}", automata_path);
}

#[tokio::test]
async fn test_desktop_config_roles_consistency() {
    // Build desktop configuration
    let config = ConfigBuilder::new_with_id(ConfigId::Desktop)
        .build_default_desktop()
        .build()
        .expect("Should build desktop config successfully");

    // Verify expected roles exist
    assert!(config.roles.contains_key(&"Default".into()), "Default role should exist");
    assert!(config.roles.contains_key(&"Engineer".into()), "Engineer role should exist");
    assert!(config.roles.contains_key(&"Terraphim Engineer".into()), "Terraphim Engineer role should exist");
    assert!(config.roles.contains_key(&"System Operator".into()), "System Operator role should exist");

    // Verify both Engineer roles use the same KG configuration
    let engineer_role = config.roles.get(&"Engineer".into()).unwrap();
    let terraphim_engineer_role = config.roles.get(&"Terraphim Engineer".into()).unwrap();

    let engineer_kg_path = engineer_role.kg.as_ref().unwrap()
        .knowledge_graph_local.as_ref().unwrap().path.clone();
    let terraphim_engineer_kg_path = terraphim_engineer_role.kg.as_ref().unwrap()
        .knowledge_graph_local.as_ref().unwrap().path.clone();

    assert_eq!(engineer_kg_path, terraphim_engineer_kg_path,
               "Both Engineer roles should use the same KG path");

    println!("✅ Desktop configuration roles are consistent");
    println!("   - Total roles: {}", config.roles.len());
    println!("   - Default role: {}", config.default_role);
    println!("   - Engineer KG path: {:?}", engineer_kg_path);
} 