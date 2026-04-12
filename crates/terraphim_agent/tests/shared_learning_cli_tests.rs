//! Integration tests for shared learning CLI functionality.
//!
//! Tests the SharedLearningStore operations that back the `learn shared` subcommands.
//! Feature-gated: requires `shared-learning`.

#![cfg(feature = "shared-learning")]

use terraphim_agent::shared_learning::{
    SharedLearning, SharedLearningSource, SharedLearningStore, StoreConfig, TrustLevel,
};

async fn create_store() -> SharedLearningStore {
    SharedLearningStore::open(StoreConfig::default())
        .await
        .expect("should open in-memory store")
}

#[tokio::test]
async fn shared_list_empty_store() {
    let store = create_store().await;
    let all = store.list_all().await.expect("list_all should succeed");
    assert!(all.is_empty(), "empty store should return no learnings");
}

#[tokio::test]
async fn shared_list_with_trust_level_filter() {
    let store = create_store().await;

    let l1 = SharedLearning::new(
        "L1 learning".to_string(),
        "content".to_string(),
        SharedLearningSource::Manual,
        "test-agent".to_string(),
    );
    store.insert(l1).await.expect("insert l1");

    let mut l2 = SharedLearning::new(
        "L2 learning".to_string(),
        "content".to_string(),
        SharedLearningSource::Manual,
        "test-agent".to_string(),
    );
    l2.promote_to_l2();
    store.insert(l2).await.expect("insert l2");

    let all = store.list_all().await.expect("list_all");
    assert_eq!(all.len(), 2);

    let l1_only = store
        .list_by_trust_level(TrustLevel::L1)
        .await
        .expect("list l1");
    assert_eq!(l1_only.len(), 1);
    assert_eq!(l1_only[0].title, "L1 learning");

    let l2_only = store
        .list_by_trust_level(TrustLevel::L2)
        .await
        .expect("list l2");
    assert_eq!(l2_only.len(), 1);
    assert_eq!(l2_only[0].title, "L2 learning");

    let l3_only = store
        .list_by_trust_level(TrustLevel::L3)
        .await
        .expect("list l3");
    assert!(l3_only.is_empty());
}

#[tokio::test]
async fn shared_promote_l1_to_l2() {
    let store = create_store().await;

    let learning = SharedLearning::new(
        "Promotable learning".to_string(),
        "content".to_string(),
        SharedLearningSource::BashHook,
        "test-agent".to_string(),
    );
    let id = learning.id.clone();
    store.insert(learning).await.expect("insert");

    store.promote_to_l2(&id).await.expect("promote to l2");

    let fetched = store.get(&id).await.expect("get after promote");
    assert_eq!(fetched.trust_level, TrustLevel::L2);
    assert!(fetched.promoted_at.is_some());
}

#[tokio::test]
async fn shared_promote_to_l3() {
    let store = create_store().await;

    let learning = SharedLearning::new(
        "L3 candidate".to_string(),
        "content".to_string(),
        SharedLearningSource::Manual,
        "test-agent".to_string(),
    );
    let id = learning.id.clone();
    store.insert(learning).await.expect("insert");

    store.promote_to_l3(&id).await.expect("promote to l3");

    let fetched = store.get(&id).await.expect("get after promote");
    assert_eq!(fetched.trust_level, TrustLevel::L3);
}

#[tokio::test]
async fn shared_stats_counts() {
    let store = create_store().await;

    // Insert 2 L1, 1 L2
    for i in 0..2 {
        let l = SharedLearning::new(
            format!("L1 item {}", i),
            "content".to_string(),
            SharedLearningSource::Manual,
            "agent".to_string(),
        );
        store.insert(l).await.expect("insert l1");
    }

    let mut l2 = SharedLearning::new(
        "L2 item".to_string(),
        "content".to_string(),
        SharedLearningSource::Manual,
        "agent".to_string(),
    );
    l2.promote_to_l2();
    store.insert(l2).await.expect("insert l2");

    let all = store.list_all().await.expect("list_all");
    assert_eq!(all.len(), 3);

    let l1_count = all
        .iter()
        .filter(|l| l.trust_level == TrustLevel::L1)
        .count();
    let l2_count = all
        .iter()
        .filter(|l| l.trust_level == TrustLevel::L2)
        .count();
    let l3_count = all
        .iter()
        .filter(|l| l.trust_level == TrustLevel::L3)
        .count();

    assert_eq!(l1_count, 2);
    assert_eq!(l2_count, 1);
    assert_eq!(l3_count, 0);
}

#[tokio::test]
async fn shared_import_creates_l1_entries() {
    // Simulate the import flow: create SharedLearning from local learning data
    let store = create_store().await;

    // Simulate what the Import handler does
    let command = "git push --force".to_string();
    let error = "remote: error: denied".to_string();
    let tags = vec!["git".to_string(), "push".to_string()];

    let shared = SharedLearning::new(
        command.clone(),
        error.clone(),
        SharedLearningSource::BashHook,
        "cli-import".to_string(),
    )
    .with_original_command(command)
    .with_error_context(error)
    .with_keywords(tags);

    store
        .insert(shared)
        .await
        .expect("insert imported learning");

    let all = store.list_all().await.expect("list_all");
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].trust_level, TrustLevel::L1);
    assert_eq!(all[0].source_agent, "cli-import");
    assert!(all[0].original_command.is_some());
    assert!(all[0].error_context.is_some());
    assert_eq!(all[0].keywords.len(), 2);
}

#[tokio::test]
async fn shared_trust_level_parse() {
    assert_eq!("l1".parse::<TrustLevel>().unwrap(), TrustLevel::L1);
    assert_eq!("L2".parse::<TrustLevel>().unwrap(), TrustLevel::L2);
    assert_eq!("l3".parse::<TrustLevel>().unwrap(), TrustLevel::L3);
    assert!("invalid".parse::<TrustLevel>().is_err());
}
