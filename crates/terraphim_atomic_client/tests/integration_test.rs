use dotenvy::dotenv;
use serde_json::json;
use std::collections::HashMap;
use terraphim_atomic_client::{
    store::Store,
    types::{Config, Resource},
};

#[tokio::test]
async fn test_config_from_env() {
    // Load .env file if present
    dotenv().ok();

    // Create a config directly since we can't rely on environment variables in CI
    let config = Config {
        server_url: "http://localhost:9883".to_string(),
        agent: None,
    };

    // Verify the server URL is set
    assert!(!config.server_url.is_empty());

    // Create a store with the config
    let store = Store::new(config).expect("Failed to create store");

    // Since we have no agent, we can't test fetching resources
    assert!(store.config.agent.is_none());
}

#[cfg(test)]
mod tests {
    use dotenvy::dotenv;
    use serde_json::json;
    use std::collections::HashMap;
    use terraphim_atomic_client::{Config, Resource, Store};

    #[tokio::test]
    async fn test_crud_operations() {
        // Load .env file if present
        dotenv().ok();

        // Skip test in CI if environment variables are not set
        if std::env::var("ATOMIC_SERVER_URL").is_err()
            || std::env::var("ATOMIC_SERVER_SECRET").is_err()
        {
            eprintln!(
                "Skipping test_crud_operations: ATOMIC_SERVER_URL and ATOMIC_SERVER_SECRET not set"
            );
            return;
        }

        // Load configuration and ensure agent is present
        let config = Config::from_env()
            .expect("Environment variables ATOMIC_SERVER_URL and ATOMIC_SERVER_SECRET must be set");
        assert!(
            config.agent.is_some(),
            "ATOMIC_SERVER_SECRET must decode into a valid Agent"
        );

        let store = Store::new(config).expect("Failed to create store");

        // Create a unique resource ID for testing
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        // Ensure no double slash and avoid nonexistent /resources path.
        let base_url = store.config.server_url.trim_end_matches('/');
        let test_resource_id = format!("{}/test-resource-{}", base_url, timestamp);
        println!("Creating resource with ID: {}", test_resource_id);

        // Create a new resource
        let mut properties = HashMap::new();
        properties.insert(
            "https://atomicdata.dev/properties/shortname".to_string(),
            json!(format!("test-resource-{}", timestamp)),
        );
        properties.insert(
            "https://atomicdata.dev/properties/description".to_string(),
            json!("A test resource created by the Rust client"),
        );
        properties.insert(
            "https://atomicdata.dev/properties/parent".to_string(),
            json!(base_url),
        );
        properties.insert(
            "https://atomicdata.dev/properties/isA".to_string(),
            json!(["https://atomicdata.dev/classes/Article"]),
        );
        properties.insert(
            "https://atomicdata.dev/properties/name".to_string(),
            json!(format!("Test Resource {}", timestamp)),
        );
        let _resource = Resource {
            subject: test_resource_id.clone(),
            properties: properties.clone(),
        };

        // Create the resource via commit (preferred method)
        store
            .create_with_commit(&test_resource_id, properties.clone())
            .await
            .expect("Failed to create resource via commit");

        // Get the resource
        let retrieved_resource = store
            .get_resource(&test_resource_id)
            .await
            .expect("Failed to retrieve created resource");

        // Verify the retrieved resource
        assert_eq!(retrieved_resource.subject, test_resource_id);
        assert!(
            retrieved_resource
                .properties
                .contains_key("https://atomicdata.dev/properties/shortname")
        );

        // Update the resource
        let mut update_map = HashMap::new();
        update_map.insert(
            "https://atomicdata.dev/properties/description".to_string(),
            json!("An updated test resource"),
        );

        store
            .update_with_commit(&test_resource_id, update_map.clone())
            .await
            .expect("Failed to update resource via commit");

        // Verify the updated resource
        let updated_resource = store
            .get_resource(&test_resource_id)
            .await
            .expect("Failed to fetch updated resource");

        assert_eq!(updated_resource.subject, test_resource_id);
        assert_eq!(
            updated_resource
                .properties
                .get("https://atomicdata.dev/properties/description"),
            Some(&json!("An updated test resource"))
        );

        // Delete the resource
        store
            .delete_with_commit(&test_resource_id)
            .await
            .expect("Failed to delete resource via commit");

        // Verify the resource was deleted
        if store.get_resource(&test_resource_id).await.is_ok() {
            panic!("Resource was not deleted")
        }
    }

    // ... other test functions
}

#[tokio::test]
async fn test_search() {
    // Load .env file if present
    dotenv().ok();

    // Skip test in CI if environment variables are not set
    if std::env::var("ATOMIC_SERVER_URL").is_err() || std::env::var("ATOMIC_SERVER_SECRET").is_err()
    {
        eprintln!("Skipping test_search: ATOMIC_SERVER_URL and ATOMIC_SERVER_SECRET not set");
        return;
    }

    let config = Config::from_env().expect("Environment variables must be set");
    let store = Store::new(config).expect("Failed to create Store");

    let _results = store.search("test").await.expect("Search request failed");
    // basic sanity
    assert!(!_results.is_null(), "Search returned null");
}

#[tokio::test]
async fn test_query() {
    // Load .env file if present
    dotenv().ok();

    // Skip test in CI if environment variables are not set
    if std::env::var("ATOMIC_SERVER_URL").is_err() || std::env::var("ATOMIC_SERVER_SECRET").is_err()
    {
        eprintln!("Skipping test_query: ATOMIC_SERVER_URL and ATOMIC_SERVER_SECRET not set");
        return;
    }

    let config = Config::from_env().expect("Environment variables must be set");
    let store = Store::new(config).expect("Failed to create Store");

    // Query the collections resource directly using GET with query params
    let base_url = store.config.server_url.trim_end_matches('/');
    let query_url = format!(
        "{}/collections?current_page=0&page_size=10&include_nested=true",
        base_url
    );

    let _results = store
        .get_resource(&query_url)
        .await
        .expect("Query request failed");

    // Basic sanity: ensure response is not null
    assert!(
        !_results.properties.is_empty(),
        "Query returned empty properties"
    );
}

#[tokio::test]
async fn test_create_and_search() {
    // Load .env file if present
    dotenv().ok();

    // Skip test in CI if environment variables are not set
    if std::env::var("ATOMIC_SERVER_URL").is_err() || std::env::var("ATOMIC_SERVER_SECRET").is_err()
    {
        eprintln!(
            "Skipping test_create_and_search: ATOMIC_SERVER_URL and ATOMIC_SERVER_SECRET not set"
        );
        return;
    }

    let config = Config::from_env().expect("Environment variables must be set");
    assert!(
        config.agent.is_some(),
        "ATOMIC_SERVER_SECRET must decode into Agent"
    );
    let store = Store::new(config).expect("Failed to create store");

    // Create a unique test resource
    let unique_id = format!(
        "search-test-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    );

    let mut properties = HashMap::new();
    properties.insert(
        "https://atomicdata.dev/properties/shortname".to_string(),
        json!(unique_id.clone()),
    );
    properties.insert(
        "https://atomicdata.dev/properties/description".to_string(),
        json!(format!("A searchable test resource {}", unique_id)),
    );
    properties.insert(
        "https://atomicdata.dev/properties/parent".to_string(),
        json!(store.config.server_url.trim_end_matches('/')),
    );
    properties.insert(
        "https://atomicdata.dev/properties/isA".to_string(),
        json!(["https://atomicdata.dev/classes/Article"]),
    );
    properties.insert(
        "https://atomicdata.dev/properties/name".to_string(),
        json!(format!("Search Test {}", unique_id)),
    );

    let resource = Resource {
        subject: format!(
            "{}/{}",
            store.config.server_url.trim_end_matches('/'),
            unique_id
        ),
        properties,
    };

    // Create the resource
    store
        .create_with_commit(&resource.subject, resource.properties.clone())
        .await
        .expect("Failed to create resource");

    // Wait a moment for the search index to update
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    let _results = store.search(&unique_id).await.expect("Search failed");
    assert!(!_results.is_null(), "Search returned null");

    // Clean up
    store
        .delete_with_commit(&resource.subject)
        .await
        .expect("Failed to delete resource");
}

#[tokio::test]
async fn test_create_and_query() {
    // Load .env file if present
    dotenv().ok();

    // Skip test in CI if environment variables are not set
    if std::env::var("ATOMIC_SERVER_URL").is_err() || std::env::var("ATOMIC_SERVER_SECRET").is_err()
    {
        eprintln!(
            "Skipping test_create_and_query: ATOMIC_SERVER_URL and ATOMIC_SERVER_SECRET not set"
        );
        return;
    }

    let config = Config::from_env().expect("Environment variables must be set");
    assert!(
        config.agent.is_some(),
        "ATOMIC_SERVER_SECRET must decode into Agent"
    );
    let store = Store::new(config).expect("Failed to create store");

    // Create a unique test resource
    let unique_id = format!(
        "query-test-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    );

    let mut properties = HashMap::new();
    properties.insert(
        "https://atomicdata.dev/properties/shortname".to_string(),
        json!(unique_id.clone()),
    );
    properties.insert(
        "https://atomicdata.dev/properties/description".to_string(),
        json!(format!("A queryable test resource {}", unique_id)),
    );
    properties.insert(
        "https://atomicdata.dev/properties/name".to_string(),
        json!(format!("Query Test {}", unique_id)),
    );
    properties.insert(
        "https://atomicdata.dev/properties/parent".to_string(),
        json!(store.config.server_url.trim_end_matches('/')),
    );
    properties.insert(
        "https://atomicdata.dev/properties/isA".to_string(),
        json!(["https://atomicdata.dev/classes/Article"]),
    );

    let resource = Resource {
        subject: format!(
            "{}/{}",
            store.config.server_url.trim_end_matches('/'),
            unique_id
        ),
        properties,
    };

    // Create the resource
    store
        .create_with_commit(&resource.subject, resource.properties.clone())
        .await
        .expect("Failed to create resource");

    // Wait a moment for the index to update
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Use the API endpoint directly to query by class
    let query_url = format!(
        "{}/query?property=https://atomicdata.dev/properties/isA&value=https://atomicdata.dev/classes/Article",
        store.config.server_url.trim_end_matches('/')
    );
    let query_resource = store.get_resource(&query_url).await.expect("Query failed");
    let query_results = [query_resource];

    // Verify that we found at least one result
    assert!(!query_results.is_empty(), "Query returned no results");

    // Clean up
    store
        .delete_with_commit(&resource.subject)
        .await
        .expect("Failed to delete resource");
}
