use serial_test::serial;
use tempfile::TempDir;
use terraphim_config::{ConfigBuilder, Haystack, Role, ServiceType};
use terraphim_middleware::{indexer::IndexMiddleware, FffIndexer};
use terraphim_types::{RelevanceFunction, RoleName};

fn create_test_role() -> Role {
    let mut role = Role::new("Test");
    role.shortname = Some("Test".to_string());
    role.relevance_function = RelevanceFunction::TitleScorer;
    role.theme = "default".to_string();
    role.haystacks = vec![Haystack {
        location: "../../terraphim_server/fixtures/haystack".to_string(),
        service: ServiceType::Ripgrep,
        read_only: true,
        fetch_content: false,
        atomic_server_secret: None,
        extra_parameters: std::collections::HashMap::new(),
    }];
    role
}

fn create_test_config() -> terraphim_config::Config {
    ConfigBuilder::new()
        .global_shortcut("Ctrl+T")
        .add_role("Test", create_test_role())
        .build()
        .unwrap()
}

#[tokio::test]
#[serial]
async fn test_fff_indexer_basic() {
    let _config = create_test_config();
    let haystack = Haystack {
        location: "../../terraphim_server/fixtures/haystack".to_string(),
        service: ServiceType::Ripgrep,
        read_only: true,
        fetch_content: false,
        atomic_server_secret: None,
        extra_parameters: std::collections::HashMap::new(),
    };
    let indexer = FffIndexer::default();
    let index = indexer.index("test", &haystack).await.unwrap();
    println!("FffIndexer basic test: indexed {} documents", index.len());
    assert!(
        !index.is_empty(),
        "Expected at least one document for 'test' needle"
    );
}

#[tokio::test]
#[serial]
async fn test_fff_search_graph() {
    let _config = create_test_config();
    let haystack = Haystack {
        location: "../../terraphim_server/fixtures/haystack".to_string(),
        service: ServiceType::Ripgrep,
        read_only: true,
        fetch_content: false,
        atomic_server_secret: None,
        extra_parameters: std::collections::HashMap::new(),
    };
    let indexer = FffIndexer::default();
    let index = indexer.index("graph", &haystack).await.unwrap();
    println!("FffIndexer graph search: indexed {} documents", index.len());
    assert!(
        !index.is_empty(),
        "Expected at least one document for 'graph' needle"
    );
}

#[tokio::test]
#[serial]
async fn test_fff_search_machine_learning() {
    let _config = create_test_config();
    let haystack = Haystack {
        location: "../../terraphim_server/fixtures/haystack".to_string(),
        service: ServiceType::Ripgrep,
        read_only: true,
        fetch_content: false,
        atomic_server_secret: None,
        extra_parameters: std::collections::HashMap::new(),
    };

    let indexer = FffIndexer::default();
    let index = indexer.index("graph", &haystack).await.unwrap();
    println!(
        "FffIndexer machine learning search: indexed {} documents",
        index.len()
    );
    for doc in index.get_all_documents() {
        println!("  Document: {} ({})", doc.title, doc.id);
        assert!(!doc.title.is_empty(), "Document title should not be empty");
        assert!(!doc.body.is_empty(), "Document body should not be empty");
    }
}

#[tokio::test]
#[serial]
async fn test_fff_role_configuration() {
    let config = create_test_config();

    // Test that roles are configured correctly
    assert!(config.roles.contains_key(&RoleName::new("Test")));

    // Test haystack configuration
    let test_role = config.roles.get(&RoleName::new("Test")).unwrap();
    assert_eq!(test_role.haystacks.len(), 1);
    assert_eq!(test_role.haystacks[0].service, ServiceType::Ripgrep);
    assert_eq!(test_role.haystacks[0].atomic_server_secret, None);
}

#[tokio::test]
#[serial]
async fn test_fff_indexer_performance() {
    let haystack = Haystack {
        location: "../../terraphim_server/fixtures/haystack".to_string(),
        service: ServiceType::Ripgrep,
        read_only: true,
        fetch_content: false,
        atomic_server_secret: None,
        extra_parameters: std::collections::HashMap::new(),
    };
    let indexer = FffIndexer::default();

    let start = std::time::Instant::now();
    let index = indexer.index("test", &haystack).await.unwrap();
    let first_duration = start.elapsed();

    let start = std::time::Instant::now();
    let _index = indexer.index("test", &haystack).await.unwrap();
    let second_duration = start.elapsed();

    println!("First query: {:?}", first_duration);
    println!("Cached query: {:?}", second_duration);
    println!("Documents found: {}", index.len());

    // Cached query should be significantly faster
    assert!(
        second_duration < first_duration || first_duration < std::time::Duration::from_millis(10),
        "Cached query should be faster, or first query was already very fast"
    );
}

#[tokio::test]
#[serial]
async fn test_fff_update_document() {
    use tempfile::NamedTempFile;
    use tokio::fs;

    let temp_file = NamedTempFile::with_suffix(".md").unwrap();
    let path = temp_file.path().to_string_lossy().to_string();

    // Write initial content
    fs::write(&path, "# Test Document\n\nOriginal content.\n")
        .await
        .unwrap();

    let document = terraphim_types::Document {
        id: "test_update".to_string(),
        title: "Test".to_string(),
        url: path.clone(),
        body: "# Updated Document\n\nNew content.\n".to_string(),
        description: None,
        summarization: None,
        stub: None,
        tags: None,
        rank: None,
        source_haystack: None,
        doc_type: terraphim_types::DocumentType::KgEntry,
        synonyms: None,
        route: None,
        priority: None,
        quality_score: None,
    };

    let indexer = FffIndexer::default();
    indexer.update_document(&document).await.unwrap();

    let updated_content = fs::read_to_string(&path).await.unwrap();
    assert_eq!(updated_content, "# Updated Document\n\nNew content.\n");
}

#[tokio::test]
#[serial]
async fn test_fff_with_kg_scorer() {
    use std::sync::Arc;
    use terraphim_file_search::kg_scorer::KgPathScorer;
    use terraphim_types::{NormalizedTerm, NormalizedTermValue, Thesaurus};

    let haystack = Haystack {
        location: "../../terraphim_server/fixtures/haystack".to_string(),
        service: ServiceType::Ripgrep,
        read_only: true,
        fetch_content: false,
        atomic_server_secret: None,
        extra_parameters: std::collections::HashMap::new(),
    };

    // Create a thesaurus that matches "machine" in file paths
    let mut thesaurus = Thesaurus::new("test".to_string());
    let key = NormalizedTermValue::from("machine".to_string());
    let term = NormalizedTerm {
        id: 1,
        value: NormalizedTermValue::from("machine".to_string()),
        display_value: None,
        url: None,
        action: None,
        priority: None,
        trigger: None,
        pinned: false,
    };
    thesaurus.insert(key, term);

    let scorer = Arc::new(KgPathScorer::new(thesaurus));
    let indexer = FffIndexer::default().with_kg_scorer(scorer);

    let index = indexer.index("test", &haystack).await.unwrap();
    println!(
        "FffIndexer with KG scorer: indexed {} documents",
        index.len()
    );
    assert!(!index.is_empty(), "Expected documents with KG scorer");
}

#[tokio::test]
#[serial]
async fn test_fff_default_has_no_kg_scorer() {
    let indexer = FffIndexer::default();
    let haystack = Haystack {
        location: "../../terraphim_server/fixtures/haystack".to_string(),
        service: ServiceType::Ripgrep,
        read_only: true,
        fetch_content: false,
        atomic_server_secret: None,
        extra_parameters: std::collections::HashMap::new(),
    };

    let index = indexer.index("test", &haystack).await.unwrap();
    assert!(!index.is_empty(), "Expected documents without KG scorer");
}

#[cfg(test)]
mod nested_tests {
    use super::*;
    use terraphim_middleware::Result;

    #[tokio::test]
    async fn test_nested_search() -> Result<()> {
        let config = create_test_config();
        let _role = config.roles.get(&RoleName::new("Test")).unwrap();

        // Test basic role existence
        assert!(!config.roles.is_empty());

        Ok(())
    }
}

#[tokio::test]
#[serial]
async fn test_fff_does_not_index_rs_file_by_default() {
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let rs_file = temp_dir.path().join("example.rs");
    tokio::fs::write(&rs_file, "pub fn hello() { println!(\"hello\"); }")
        .await
        .unwrap();

    let haystack = Haystack {
        location: temp_dir.path().to_string_lossy().to_string(),
        service: ServiceType::Ripgrep,
        read_only: true,
        fetch_content: false,
        atomic_server_secret: None,
        extra_parameters: std::collections::HashMap::new(),
    };

    let indexer = FffIndexer::default();
    let index = indexer.index("fn", &haystack).await.unwrap();
    assert_eq!(
        index.len(),
        0,
        "Default FffIndexer should not index .rs files"
    );
}

#[tokio::test]
#[serial]
async fn test_fff_indexes_rs_file_when_extension_configured() {
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let rs_file = temp_dir.path().join("example.rs");
    tokio::fs::write(&rs_file, "pub fn hello() { println!(\"hello\"); }")
        .await
        .unwrap();

    let mut extra_params = std::collections::HashMap::new();
    extra_params.insert("extensions".to_string(), "rs".to_string());

    let haystack = Haystack {
        location: temp_dir.path().to_string_lossy().to_string(),
        service: ServiceType::Ripgrep,
        read_only: true,
        fetch_content: false,
        atomic_server_secret: None,
        extra_parameters: extra_params,
    };

    let indexer = FffIndexer::default();
    let index = indexer.index("fn", &haystack).await.unwrap();
    assert!(
        !index.is_empty(),
        "FffIndexer with extensions=rs should index .rs files"
    );
}

#[tokio::test]
#[serial]
async fn test_fff_with_kg_scorer_uses_stateful_path() {
    use std::sync::Arc;
    use terraphim_file_search::kg_scorer::KgPathScorer;
    use terraphim_types::{NormalizedTerm, NormalizedTermValue, Thesaurus};

    let mut extra_params = std::collections::HashMap::new();
    extra_params.insert("extensions".to_string(), "md".to_string());

    let haystack = Haystack {
        location: "../../terraphim_server/fixtures/haystack".to_string(),
        service: ServiceType::Ripgrep,
        read_only: true,
        fetch_content: false,
        atomic_server_secret: None,
        extra_parameters: extra_params,
    };

    let mut thesaurus = Thesaurus::new("test".to_string());
    let key = NormalizedTermValue::from("machine".to_string());
    let term = NormalizedTerm {
        id: 1,
        value: NormalizedTermValue::from("machine".to_string()),
        display_value: None,
        url: None,
        action: None,
        priority: None,
        trigger: None,
        pinned: false,
    };
    thesaurus.insert(key, term);

    let scorer = Arc::new(KgPathScorer::new(thesaurus));
    let indexer = FffIndexer::default().with_kg_scorer(scorer);

    let index = indexer.index("graph", &haystack).await.unwrap();
    assert!(
        !index.is_empty(),
        "Expected documents when using stateful KG scorer path"
    );
}

#[tokio::test]
#[serial]
async fn test_fff_with_kg_scorer_state_is_not_discarded() {
    use std::sync::Arc;
    use terraphim_file_search::kg_scorer::KgPathScorer;
    use terraphim_types::{NormalizedTerm, NormalizedTermValue, Thesaurus};

    let temp_dir = TempDir::new().unwrap();
    let md_file = temp_dir.path().join("test.md");
    tokio::fs::write(&md_file, "# Machine Learning\n\nContent about ML.\n")
        .await
        .unwrap();

    let mut extra_params = std::collections::HashMap::new();
    extra_params.insert("extensions".to_string(), "md".to_string());

    let haystack = Haystack {
        location: temp_dir.path().to_string_lossy().to_string(),
        service: ServiceType::Ripgrep,
        read_only: true,
        fetch_content: false,
        atomic_server_secret: None,
        extra_parameters: extra_params,
    };

    let mut thesaurus = Thesaurus::new("test".to_string());
    let key = NormalizedTermValue::from("machine".to_string());
    let term = NormalizedTerm {
        id: 1,
        value: NormalizedTermValue::from("machine".to_string()),
        display_value: None,
        url: None,
        action: None,
        priority: None,
        trigger: None,
        pinned: false,
    };
    thesaurus.insert(key, term);

    let scorer = Arc::new(KgPathScorer::new(thesaurus));
    let indexer = FffIndexer::default().with_kg_scorer(scorer);

    let index = indexer.index("Machine", &haystack).await.unwrap();
    assert!(
        !index.is_empty(),
        "KG scorer path should produce results; scorer state must not be discarded"
    );
}

fn create_terraphim_graph_role_with_haystack(location: String) -> Role {
    let mut role = Role::new("KGTest");
    role.shortname = Some("KGTest".to_string());
    role.relevance_function = RelevanceFunction::TerraphimGraph;
    role.theme = "default".to_string();
    role.haystacks = vec![Haystack {
        location,
        service: ServiceType::Ripgrep,
        read_only: true,
        fetch_content: false,
        atomic_server_secret: None,
        extra_parameters: std::collections::HashMap::new(),
    }];
    role
}

fn create_terraphim_graph_role() -> Role {
    create_terraphim_graph_role_with_haystack(
        "../../terraphim_server/fixtures/haystack".to_string(),
    )
}

#[tokio::test]
#[serial]
async fn test_search_haystacks_injects_kg_scorer_for_terraphim_graph_role() {
    use terraphim_config::ConfigState;
    use terraphim_middleware::search_haystacks;
    use terraphim_rolegraph::RoleGraphSync;
    use terraphim_types::{NormalizedTerm, NormalizedTermValue, SearchQuery, Thesaurus};

    let mut config = ConfigBuilder::new()
        .global_shortcut("Ctrl+T")
        .add_role("KGTest", create_terraphim_graph_role())
        .build()
        .unwrap();

    let mut config_state = ConfigState::new(&mut config).await.unwrap();

    let mut thesaurus = Thesaurus::new("kg_test".to_string());
    let key = NormalizedTermValue::from("graph".to_string());
    let term = NormalizedTerm {
        id: 1,
        value: NormalizedTermValue::from("graph".to_string()),
        display_value: None,
        url: None,
        action: None,
        priority: None,
        trigger: None,
        pinned: false,
    };
    thesaurus.insert(key, term);

    let rolegraph = terraphim_rolegraph::RoleGraph::new(RoleName::new("KGTest"), thesaurus)
        .await
        .unwrap();
    config_state
        .roles
        .insert(RoleName::new("KGTest"), RoleGraphSync::from(rolegraph));

    let query = SearchQuery {
        search_term: "graph".into(),
        search_terms: None,
        operator: None,
        skip: None,
        limit: None,
        role: Some(RoleName::new("KGTest")),
        layer: terraphim_types::Layer::default(),
        include_pinned: false,
        min_quality: None,
    };

    let result = search_haystacks(config_state, query).await;
    assert!(
        result.is_ok(),
        "search_haystacks should succeed for TerraphimGraph role with thesaurus"
    );
    let index = result.unwrap();
    assert!(!index.is_empty(), "Should find documents for 'graph' query");
}

#[tokio::test]
#[serial]
async fn test_search_haystacks_no_scorer_for_title_scorer_role() {
    use terraphim_config::ConfigState;
    use terraphim_middleware::search_haystacks;
    use terraphim_types::SearchQuery;

    let mut config = create_test_config();
    let config_state = ConfigState::new(&mut config).await.unwrap();

    let query = SearchQuery {
        search_term: "graph".into(),
        search_terms: None,
        operator: None,
        skip: None,
        limit: None,
        role: Some(RoleName::new("Test")),
        layer: terraphim_types::Layer::default(),
        include_pinned: false,
        min_quality: None,
    };

    let result = search_haystacks(config_state, query).await;
    assert!(
        result.is_ok(),
        "search_haystacks should succeed for TitleScorer role: {:?}",
        result.err()
    );
    let index = result.unwrap();
    assert!(
        !index.is_empty(),
        "Should find documents for 'graph' query without KG scorer"
    );
}

#[tokio::test]
#[serial]
async fn test_search_haystacks_empty_thesaurus_no_scorer() {
    use terraphim_config::ConfigState;
    use terraphim_middleware::search_haystacks;
    use terraphim_types::SearchQuery;

    let mut config = ConfigBuilder::new()
        .global_shortcut("Ctrl+T")
        .add_role("KGTest", create_terraphim_graph_role())
        .build()
        .unwrap();

    let config_state = ConfigState::new(&mut config).await.unwrap();

    let query = SearchQuery {
        search_term: "graph".into(),
        search_terms: None,
        operator: None,
        skip: None,
        limit: None,
        role: Some(RoleName::new("KGTest")),
        layer: terraphim_types::Layer::default(),
        include_pinned: false,
        min_quality: None,
    };

    let result = search_haystacks(config_state, query).await;
    assert!(
        result.is_ok(),
        "search_haystacks should succeed even with empty thesaurus"
    );
}

#[tokio::test]
#[serial]
async fn test_search_haystacks_kg_scorer_preserves_thesaurus_data() {
    use std::sync::Arc;
    use terraphim_file_search::kg_scorer::KgPathScorer;
    use terraphim_middleware::FffIndexer;
    use terraphim_types::{NormalizedTerm, NormalizedTermValue, Thesaurus};

    let mut thesaurus = Thesaurus::new("verify".to_string());
    let key = NormalizedTermValue::from("machine".to_string());
    let term = NormalizedTerm {
        id: 42,
        value: NormalizedTermValue::from("machine".to_string()),
        display_value: None,
        url: None,
        action: None,
        priority: None,
        trigger: None,
        pinned: false,
    };
    thesaurus.insert(key, term);

    let scorer = Arc::new(KgPathScorer::new(thesaurus.clone()));
    let indexer = FffIndexer::default().with_kg_scorer(scorer);

    let haystack = Haystack {
        location: "../../terraphim_server/fixtures/haystack".to_string(),
        service: ServiceType::Ripgrep,
        read_only: true,
        fetch_content: false,
        atomic_server_secret: None,
        extra_parameters: std::collections::HashMap::new(),
    };

    let index = indexer.index("graph", &haystack).await.unwrap();
    assert!(
        !index.is_empty(),
        "Scorer with 'machine' thesaurus should produce results"
    );
}

#[tokio::test]
#[serial]
async fn test_fff_multiple_extensions_configured() {
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let rs_file = temp_dir.path().join("example.rs");
    tokio::fs::write(&rs_file, "pub fn hello() {}")
        .await
        .unwrap();
    let md_file = temp_dir.path().join("example.md");
    tokio::fs::write(&md_file, "# Hello\n\nContent.")
        .await
        .unwrap();

    let mut extra_params = std::collections::HashMap::new();
    extra_params.insert("extensions".to_string(), "rs,md".to_string());

    let haystack = Haystack {
        location: temp_dir.path().to_string_lossy().to_string(),
        service: ServiceType::Ripgrep,
        read_only: true,
        fetch_content: false,
        atomic_server_secret: None,
        extra_parameters: extra_params,
    };

    let indexer = FffIndexer::default();
    let index = indexer.index("fn", &haystack).await.unwrap();
    assert_eq!(index.len(), 1, "Should find the .rs file");

    let index2 = indexer.index("Hello", &haystack).await.unwrap();
    assert_eq!(index2.len(), 1, "Should find the .md file");
}

#[tokio::test]
#[serial]
async fn test_search_haystacks_kg_scorer_boosts_priority_file_above_page_limit() {
    use terraphim_config::ConfigState;
    use terraphim_middleware::search_haystacks;
    use terraphim_rolegraph::RoleGraphSync;
    use terraphim_types::{NormalizedTerm, NormalizedTermValue, SearchQuery, Thesaurus};

    let temp_dir = TempDir::new().unwrap();
    let haystack_path = temp_dir.path();

    for i in 0..200 {
        let file_path = haystack_path.join(format!("neutral-{:03}.md", i));
        tokio::fs::write(
            &file_path,
            format!("# File {}\n\nContent with needle term.\n", i),
        )
        .await
        .unwrap();
    }

    let priority_file = haystack_path.join("kg-priority/special-concept.md");
    tokio::fs::create_dir_all(priority_file.parent().unwrap())
        .await
        .unwrap();
    tokio::fs::write(
        &priority_file,
        "# Special Concept\n\nContent with needle term.\n",
    )
    .await
    .unwrap();

    let mut config = ConfigBuilder::new()
        .global_shortcut("Ctrl+T")
        .add_role(
            "KGTest",
            create_terraphim_graph_role_with_haystack(haystack_path.to_string_lossy().to_string()),
        )
        .build()
        .unwrap();

    let mut config_state = ConfigState::new(&mut config).await.unwrap();

    let mut thesaurus = Thesaurus::new("kg_pagination_test".to_string());
    let key = NormalizedTermValue::from("special-concept".to_string());
    let term = NormalizedTerm {
        id: 1,
        value: NormalizedTermValue::from("special-concept".to_string()),
        display_value: None,
        url: None,
        action: None,
        priority: None,
        trigger: None,
        pinned: false,
    };
    thesaurus.insert(key, term);

    let rolegraph = terraphim_rolegraph::RoleGraph::new(RoleName::new("KGTest"), thesaurus)
        .await
        .unwrap();
    config_state
        .roles
        .insert(RoleName::new("KGTest"), RoleGraphSync::from(rolegraph));

    let query = SearchQuery {
        search_term: "needle".into(),
        search_terms: None,
        operator: None,
        skip: None,
        limit: None,
        role: Some(RoleName::new("KGTest")),
        layer: terraphim_types::Layer::default(),
        include_pinned: false,
        min_quality: None,
    };

    let result = search_haystacks(config_state, query).await.unwrap();
    let priority_found = result
        .values()
        .any(|doc| doc.url.ends_with("kg-priority/special-concept.md"));
    assert!(
        priority_found,
        "KG scorer should boost priority file into top 200 results, but it was not found"
    );
}
