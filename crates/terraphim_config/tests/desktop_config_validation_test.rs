use terraphim_config::{ConfigBuilder, ConfigId};
use terraphim_types::{KnowledgeGraphInputType, RelevanceFunction};

#[tokio::test]
async fn test_desktop_config_default_role_basic() {
    // Build desktop configuration
    let config = ConfigBuilder::new_with_id(ConfigId::Desktop)
        .build_default_desktop()
        .build()
        .expect("Should build desktop config successfully");

    // Verify Default role configuration
    let default_role = config
        .roles
        .get(&"Default".into())
        .expect("Default role should exist");

    // Verify Default uses TitleScorer relevance function
    assert_eq!(
        default_role.relevance_function,
        RelevanceFunction::TitleScorer,
        "Default role should use TitleScorer relevance function"
    );

    // Verify Default has no knowledge graph (simple search)
    assert!(
        default_role.kg.is_none(),
        "Default role should not have knowledge graph configuration"
    );

    // Verify Default has haystack configuration
    assert!(
        !default_role.haystacks.is_empty(),
        "Default role should have haystack configuration"
    );

    println!("✅ Default role correctly configured for basic document search");
    println!(
        "   - Relevance function: {:?}",
        default_role.relevance_function
    );
    println!("   - Haystacks: {}", default_role.haystacks.len());
}

#[tokio::test]
async fn test_desktop_config_terraphim_engineer_uses_local_kg() {
    // Build desktop configuration
    let config = ConfigBuilder::new_with_id(ConfigId::Desktop)
        .build_default_desktop()
        .build()
        .expect("Should build desktop config successfully");

    // Verify Terraphim Engineer role configuration
    let terraphim_engineer_role = config
        .roles
        .get(&"Terraphim Engineer".into())
        .expect("Terraphim Engineer role should exist");

    // Verify Terraphim Engineer uses TerraphimGraph relevance function
    assert_eq!(
        terraphim_engineer_role.relevance_function,
        RelevanceFunction::TerraphimGraph,
        "Terraphim Engineer role should use TerraphimGraph relevance function"
    );

    // Verify Terraphim Engineer has knowledge graph configuration
    let kg = terraphim_engineer_role
        .kg
        .as_ref()
        .expect("Terraphim Engineer role should have knowledge graph configuration");

    // Verify local KG configuration points to user's data folder kg subdirectory
    let local_kg = kg
        .knowledge_graph_local
        .as_ref()
        .expect("Terraphim Engineer role should have local KG configuration");

    assert_eq!(
        local_kg.input_type,
        KnowledgeGraphInputType::Markdown,
        "Local KG should use Markdown input type"
    );

    // The path should end with kg (user's data folder + kg subdirectory)
    assert!(
        local_kg.path.to_string_lossy().ends_with("kg"),
        "Local KG path should point to kg subdirectory in user's data folder, got: {:?}",
        local_kg.path
    );

    // Verify automata path is None (will be built during startup)
    assert!(
        kg.automata_path.is_none(),
        "Terraphim Engineer role should not have pre-built automata path, will build from local KG"
    );

    println!("✅ Terraphim Engineer role correctly configured with local knowledge graph");
    println!(
        "   - Relevance function: {:?}",
        terraphim_engineer_role.relevance_function
    );
    println!("   - Local KG path: {:?}", local_kg.path);
    println!("   - Automata path: None (will be built from local KG)");
}

#[tokio::test]
async fn test_desktop_config_roles_consistency() {
    // Build desktop configuration
    let config = ConfigBuilder::new_with_id(ConfigId::Desktop)
        .build_default_desktop()
        .build()
        .expect("Should build desktop config successfully");

    // Verify expected roles exist
    assert!(
        config.roles.contains_key(&"Default".into()),
        "Default role should exist"
    );
    assert!(
        config.roles.contains_key(&"Terraphim Engineer".into()),
        "Terraphim Engineer role should exist"
    );

    // Verify we have exactly 2 roles in desktop configuration
    assert_eq!(
        config.roles.len(),
        2,
        "Desktop config should have exactly 2 roles"
    );

    // Verify default role is set correctly
    assert_eq!(
        config.default_role,
        "Terraphim Engineer".into(),
        "Default role should be Terraphim Engineer"
    );

    // Verify Terraphim Engineer role has proper configuration
    let terraphim_engineer_role = config.roles.get(&"Terraphim Engineer".into()).unwrap();
    let kg_path = terraphim_engineer_role
        .kg
        .as_ref()
        .unwrap()
        .knowledge_graph_local
        .as_ref()
        .unwrap()
        .path
        .clone();

    // Verify both roles point to user's data folder
    let default_role = config.roles.get(&"Default".into()).unwrap();
    let default_haystack_path = &default_role.haystacks[0].location;
    let terraphim_haystack_path = &terraphim_engineer_role.haystacks[0].location;

    assert_eq!(
        default_haystack_path, terraphim_haystack_path,
        "Both roles should use the same haystack location (user's data folder)"
    );

    println!("✅ Desktop configuration roles are consistent");
    println!("   - Total roles: {}", config.roles.len());
    println!("   - Default role: {}", config.default_role);
    println!("   - Terraphim Engineer KG path: {:?}", kg_path);
    println!("   - Shared haystack path: {}", default_haystack_path);
}
