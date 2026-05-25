use serial_test::serial;
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
    assert!(!index.is_empty(), "Expected at least one document for 'test' needle");
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
    assert!(!index.is_empty(), "Expected at least one document for 'graph' needle");
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
    println!("FffIndexer machine learning search: indexed {} documents", index.len());
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
