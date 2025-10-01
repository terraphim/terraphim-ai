use serial_test::serial;
use terraphim_config::{ConfigBuilder, Haystack, Role, ServiceType};
use terraphim_middleware::{indexer::IndexMiddleware, RipgrepIndexer};
use terraphim_types::{RelevanceFunction, RoleName};

fn create_test_role() -> Role {
    Role {
        shortname: Some("Test".to_string()),
        name: "Test".into(),
        relevance_function: RelevanceFunction::TitleScorer,
        terraphim_it: false,
        theme: "default".to_string(),
        kg: None,
        haystacks: vec![Haystack {
            location: "test_data".to_string(),
            service: ServiceType::Ripgrep,
            read_only: true,
            atomic_server_secret: None,
            extra_parameters: std::collections::HashMap::new(),
            weight: 1.0,
        }],
        extra: ahash::AHashMap::new(),
        ..Default::default()
    }
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
async fn test_indexer() {
    let _config = create_test_config();
    let haystack = Haystack {
        location: "fixtures/haystack".to_string(),
        service: ServiceType::Ripgrep,
        read_only: true,
        atomic_server_secret: None,
        extra_parameters: std::collections::HashMap::new(),
        weight: 1.0,
    };
    let indexer = RipgrepIndexer::default();
    let _index = indexer.index("test", &haystack).await.unwrap();
}

#[tokio::test]
#[serial]
async fn test_search_graph() {
    let _config = create_test_config();
    let haystack = Haystack {
        location: "fixtures/haystack".to_string(),
        service: ServiceType::Ripgrep,
        read_only: true,
        atomic_server_secret: None,
        extra_parameters: std::collections::HashMap::new(),
        weight: 1.0,
    };
    let indexer = RipgrepIndexer::default();
    let _index = indexer.index("graph", &haystack).await.unwrap();
}

#[tokio::test]
#[serial]
async fn test_search_machine_learning() {
    let _config = create_test_config();
    let haystack = Haystack {
        location: "fixtures/haystack".to_string(),
        service: ServiceType::Ripgrep,
        read_only: true,
        atomic_server_secret: None,
        extra_parameters: std::collections::HashMap::new(),
        weight: 1.0,
    };

    let indexer = RipgrepIndexer::default();
    let index = indexer.index("graph", &haystack).await.unwrap();
    println!("Indexed documents: {:#?}", index);
}

#[tokio::test]
#[serial]
async fn test_role_configuration() {
    let config = create_test_config();

    // Test that roles are configured correctly
    assert!(config.roles.contains_key(&RoleName::new("Test")));

    // Test haystack configuration
    let test_role = config.roles.get(&RoleName::new("Test")).unwrap();
    assert_eq!(test_role.haystacks.len(), 1);
    assert_eq!(test_role.haystacks[0].service, ServiceType::Ripgrep);
    assert_eq!(test_role.haystacks[0].atomic_server_secret, None);
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
