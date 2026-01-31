#[cfg(test)]
mod tests {
    use dotenvy::dotenv;
    use serde_json::json;
    use std::collections::HashMap;
    use std::time::{SystemTime, UNIX_EPOCH};
    use terraphim_atomic_client::{Config, Store};

    /// Tests the commit functionality for creating resources
    #[tokio::test]
    async fn test_commit_create() {
        // Load .env file if present
        dotenv().ok();

        // Skip test in CI if environment variables are not set
        if std::env::var("ATOMIC_SERVER_URL").is_err()
            || std::env::var("ATOMIC_SERVER_SECRET").is_err()
        {
            eprintln!(
                "Skipping test_commit_create: ATOMIC_SERVER_URL and ATOMIC_SERVER_SECRET not set"
            );
            return;
        }

        // Load configuration and ensure an Agent is present
        let config = Config::from_env()
            .expect("Environment variables ATOMIC_SERVER_URL and ATOMIC_SERVER_SECRET must be set");
        assert!(
            config.agent.is_some(),
            "ATOMIC_SERVER_SECRET must decode into a valid Agent"
        );

        // Create a store with the config
        let store = Store::new(config).expect("Failed to create store");

        // Create a unique resource ID for testing
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        // Build subject without double slash by trimming trailing slash from server_url
        let base_url = store.config.server_url.trim_end_matches('/');
        let test_resource_id = format!("{}/test-article-{}", base_url, timestamp);
        println!("Creating resource with ID: {}", test_resource_id);

        // Create properties for an article
        let mut properties = HashMap::new();
        properties.insert(
            "https://atomicdata.dev/properties/shortname".to_string(),
            json!(format!("test-article-{}", timestamp)),
        );
        properties.insert(
            "https://atomicdata.dev/properties/description".to_string(),
            json!("A test article created by the Rust client"),
        );
        properties.insert(
            "https://atomicdata.dev/properties/name".to_string(),
            json!(format!("Test Article {}", timestamp)),
        );
        properties.insert(
            "https://atomicdata.dev/properties/parent".to_string(),
            json!(base_url),
        );
        properties.insert(
            "https://atomicdata.dev/properties/isA".to_string(),
            json!(["https://atomicdata.dev/classes/Article"]),
        );

        // Create the resource using a commit
        let _result = store
            .create_with_commit(&test_resource_id, properties)
            .await
            .expect("Failed to create resource via commit");

        // Verify the resource was created
        let resource = store
            .get_resource(&test_resource_id)
            .await
            .expect("Failed to retrieve created resource");

        println!("Retrieved resource: {:#?}", resource);

        // Update the resource
        let mut update_properties = HashMap::new();
        update_properties.insert(
            "https://atomicdata.dev/properties/description".to_string(),
            json!("An updated test article"),
        );

        // Update the resource using a commit
        let _update_result = store
            .update_with_commit(&test_resource_id, update_properties)
            .await
            .expect("Failed to update resource via commit");

        // Verify the resource was updated
        let updated_resource = store
            .get_resource(&test_resource_id)
            .await
            .expect("Failed to fetch updated resource");

        println!("Updated resource: {:#?}", updated_resource);
        assert_eq!(
            updated_resource
                .properties
                .get("https://atomicdata.dev/properties/description")
                .unwrap(),
            &json!("An updated test article")
        );

        // Delete the resource using a commit
        store
            .delete_with_commit(&test_resource_id)
            .await
            .expect("Failed to delete resource via commit");

        // Verify the resource was deleted
        if store.get_resource(&test_resource_id).await.is_ok() {
            panic!("Resource was not deleted")
        }
    }
}
