use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use terraphim_persistence::mcp::{
    McpNamespaceRecord, McpPersistence, McpPersistenceImpl, NamespaceVisibility,
};

/// RED Phase: Test that SQLite persistence saves and retrieves data
#[tokio::test]
async fn test_sqlite_persistence_basic() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");

    // Create persistence with SQLite backend
    let persistence = create_sqlite_persistence(&db_path).await;

    // Create a namespace
    let namespace = McpNamespaceRecord {
        uuid: "test-uuid-123".to_string(),
        name: "test-namespace".to_string(),
        description: Some("Test description".to_string()),
        user_id: Some("user-123".to_string()),
        config_json: r#"{"servers":[]}"#.to_string(),
        created_at: chrono::Utc::now(),
        enabled: true,
        visibility: NamespaceVisibility::Private,
    };

    // Save it
    persistence
        .save_namespace(&namespace)
        .await
        .expect("Failed to save namespace");

    // Retrieve it
    let retrieved = persistence
        .get_namespace("test-uuid-123")
        .await
        .expect("Failed to get namespace")
        .expect("Namespace not found");

    assert_eq!(retrieved.uuid, "test-uuid-123");
    assert_eq!(retrieved.name, "test-namespace");
}

/// RED Phase: Test that data persists after recreating persistence layer
#[tokio::test]
async fn test_sqlite_data_persists_across_instances() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");

    // First instance - save data
    {
        let persistence = create_sqlite_persistence(&db_path).await;
        let namespace = McpNamespaceRecord {
            uuid: "persistent-uuid".to_string(),
            name: "persistent-namespace".to_string(),
            description: Some("Should persist".to_string()),
            user_id: None,
            config_json: r#"{"servers":[]}"#.to_string(),
            created_at: chrono::Utc::now(),
            enabled: true,
            visibility: NamespaceVisibility::Private,
        };

        persistence
            .save_namespace(&namespace)
            .await
            .expect("Failed to save");
    } // First instance dropped here

    // Second instance - retrieve data
    {
        let persistence = create_sqlite_persistence(&db_path).await;
        let retrieved = persistence
            .get_namespace("persistent-uuid")
            .await
            .expect("Failed to get")
            .expect("Data should persist!");

        assert_eq!(retrieved.uuid, "persistent-uuid");
        assert_eq!(retrieved.name, "persistent-namespace");
    }
}

/// RED Phase: Test concurrent writes don't corrupt data
#[tokio::test]
async fn test_sqlite_concurrent_writes() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let persistence = Arc::new(create_sqlite_persistence(&db_path).await);

    // Create 10 namespaces concurrently
    let mut handles = vec![];
    for i in 0..10 {
        let p = persistence.clone();
        let handle = tokio::spawn(async move {
            let namespace = McpNamespaceRecord {
                uuid: format!("uuid-{}", i),
                name: format!("namespace-{}", i),
                description: None,
                user_id: None,
                config_json: r#"{"servers":[]}"#.to_string(),
                created_at: chrono::Utc::now(),
                enabled: true,
                visibility: NamespaceVisibility::Private,
            };
            p.save_namespace(&namespace).await
        });
        handles.push(handle);
    }

    // All should succeed
    for handle in handles {
        handle.await.unwrap().expect("Concurrent write failed");
    }

    // Verify all 10 exist
    let namespaces = persistence.list_namespaces(None).await.unwrap();
    assert_eq!(namespaces.len(), 10);
}

// Helper function - GREEN phase implementation
async fn create_sqlite_persistence(db_path: &PathBuf) -> McpPersistenceImpl {
    use opendal::services::Sqlite;
    use opendal::Operator;

    // Ensure parent directory exists
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).expect("Failed to create database directory");
    }

    // Create SQLite operator (methods consume and return Self, so chain them)
    let builder = Sqlite::default()
        .connection_string(db_path.to_str().unwrap())
        .table("terraphim_mcp");

    let op = Operator::new(builder)
        .expect("Failed to create SQLite operator")
        .finish();

    McpPersistenceImpl::new(op)
}
