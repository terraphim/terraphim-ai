use terraphim_config::Haystack;
use terraphim_middleware::{haystack::AtomicHaystackIndexer, indexer::IndexMiddleware};
use terraphim_atomic_client::{self, Store};
use serde_json::json;
use std::collections::HashMap;
use uuid::Uuid;

#[tokio::test]
#[ignore] 
async fn test_atomic_haystack_indexer() {
    // This test requires a running Atomic Server instance and a .env file
    // at the root of the workspace with the following content:
    // ATOMIC_SERVER_URL=http://localhost:9883
    // ATOMIC_SERVER_SECRET=...
    dotenvy::dotenv().ok();

    let config = terraphim_atomic_client::Config::from_env().expect("Failed to load config from env");
    let store = Store::new(config.clone()).expect("Failed to create store");

    // 1. Create a parent resource for the test articles
    let server_url = config.server_url.trim_end_matches('/');
    let parent_subject = format!("{}/test/articles", server_url);
    let mut parent_properties = HashMap::new();
    parent_properties.insert("https://atomicdata.dev/properties/isA".to_string(), json!(["https://atomicdata.dev/classes/Collection"]));
    parent_properties.insert("https://atomicdata.dev/properties/name".to_string(), json!("Test Articles"));
    parent_properties.insert("https://atomicdata.dev/properties/parent".to_string(), json!(server_url));
    store.create_with_commit(&parent_subject, parent_properties).await.unwrap();

    // 2. Create some test articles on the server
    let article1_subject = format!("{}/test/article/{}", server_url, Uuid::new_v4());
    let mut properties1 = HashMap::new();
    properties1.insert("https://atomicdata.dev/properties/isA".to_string(), json!(["https://atomicdata.dev/classes/Article"]));
    properties1.insert("https://atomicdata.dev/properties/name".to_string(), json!("Test Article 1: The Magic of Rust"));
    properties1.insert("https://atomicdata.dev/properties/description".to_string(), json!("A deep dive into Rust's ownership model and concurrency features."));
    properties1.insert("https://atomicdata.dev/properties/parent".to_string(), json!(parent_subject));
    
    store.create_with_commit(&article1_subject, properties1).await.unwrap();

    let article2_subject = format!("{}/test/article/{}", server_url, Uuid::new_v4());
    let mut properties2 = HashMap::new();
    properties2.insert("https://atomicdata.dev/properties/isA".to_string(), json!(["https://atomicdata.dev/classes/Article"]));
    properties2.insert("https://atomicdata.dev/properties/name".to_string(), json!("Test Article 2: Svelte for Beginners"));
    properties2.insert("https://atomicdata.dev/properties/description".to_string(), json!("Getting started with Svelte, the reactive UI framework."));
    properties2.insert("https://atomicdata.dev/properties/parent".to_string(), json!(parent_subject));

    store.create_with_commit(&article2_subject, properties2).await.unwrap();

    // Give the server a moment to index the new resources
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // 3. Instantiate the indexer
    let indexer = AtomicHaystackIndexer::default();

    // 4. Create a Haystack config
    let haystack = Haystack::new(
        config.server_url.clone(),
        terraphim_config::ServiceType::Atomic,
        true,
    ).with_atomic_secret(std::env::var("ATOMIC_SERVER_SECRET").ok());

    // Poll the server until the document is indexed or we time out
    let mut index = terraphim_types::Index::new();
    for _ in 0..10 {
        index = indexer.index("Rust", &haystack).await.unwrap();
        if !index.is_empty() {
            break;
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
    println!("Final search results: {:?}", index);

    assert_eq!(index.len(), 1);
    let doc = index.values().next().unwrap();
    assert_eq!(doc.title, "Test Article 1: The Magic of Rust");
    assert!(doc.description.as_ref().unwrap().contains("ownership model"));

    // Cleanup
    store.delete_with_commit(&article1_subject).await.unwrap();
    store.delete_with_commit(&article2_subject).await.unwrap();
    store.delete_with_commit(&parent_subject).await.unwrap();
} 