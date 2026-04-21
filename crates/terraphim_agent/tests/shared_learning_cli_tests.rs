//! Integration tests for shared learning CLI functionality.
//!
//! Tests the SharedLearningStore operations that back the `learn shared` subcommands.
//! Feature-gated: requires `shared-learning`.

#![cfg(feature = "shared-learning")]

use terraphim_agent::shared_learning::{
    MarkdownStoreConfig, SharedLearning, SharedLearningSource, SharedLearningStore, StoreConfig,
    TrustLevel,
};

async fn create_store() -> SharedLearningStore {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.keep();
    let markdown_config = MarkdownStoreConfig {
        learnings_dir: path,
        shared_dir_name: "shared".to_string(),
    };
    let store_config = StoreConfig {
        similarity_threshold: 0.8,
        auto_promote_l2: true,
        markdown: markdown_config,
    };
    SharedLearningStore::open(store_config)
        .await
        .expect("should open store")
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

#[tokio::test]
async fn shared_store_survives_restart() {
    let temp_dir = tempfile::tempdir().unwrap();
    let markdown_config = MarkdownStoreConfig {
        learnings_dir: temp_dir.path().to_path_buf(),
        shared_dir_name: "shared".to_string(),
    };

    // 1. Create a store with temp dir config
    let store_config = StoreConfig {
        similarity_threshold: 0.8,
        auto_promote_l2: true,
        markdown: markdown_config.clone(),
    };
    let store = SharedLearningStore::open(store_config)
        .await
        .expect("should open store");

    // 2. Insert a learning with quality metrics
    let mut learning = SharedLearning::new(
        "Restart Test".to_string(),
        "Testing restart persistence.".to_string(),
        SharedLearningSource::Manual,
        "test-agent".to_string(),
    );
    learning.quality.applied_count = 3;
    learning.quality.effective_count = 3;
    learning.quality.agent_names = vec!["agent1".to_string(), "agent2".to_string()];
    let id = learning.id.clone();
    store.insert(learning).await.expect("insert");

    // 3. Promote it to L2
    store.promote_to_l2(&id).await.expect("promote to l2");

    // 4. Drop the store (simulating process exit)
    drop(store);

    // 5. Create a NEW store with the SAME temp dir config
    let store_config2 = StoreConfig {
        similarity_threshold: 0.8,
        auto_promote_l2: true,
        markdown: markdown_config,
    };
    let reopened = SharedLearningStore::open(store_config2)
        .await
        .expect("should reopen store");

    // 6. Verify the learning is still there with correct trust level
    let retrieved = reopened.get(&id).await.expect("get after reopen");
    assert_eq!(retrieved.trust_level, TrustLevel::L2);
    assert_eq!(retrieved.title, "Restart Test");
    assert!(retrieved.promoted_at.is_some());

    // 7. Verify quality metrics were preserved
    assert_eq!(retrieved.quality.applied_count, 3);
    assert_eq!(retrieved.quality.effective_count, 3);
    assert_eq!(retrieved.quality.agent_names.len(), 2);
}

#[tokio::test]
async fn shared_store_dedups_on_restart() {
    let temp_dir = tempfile::tempdir().unwrap();
    let markdown_config = MarkdownStoreConfig {
        learnings_dir: temp_dir.path().to_path_buf(),
        shared_dir_name: "shared".to_string(),
    };

    // 1. Create store with temp dir
    let store_config = StoreConfig {
        similarity_threshold: 0.8,
        auto_promote_l2: true,
        markdown: markdown_config.clone(),
    };
    let store = SharedLearningStore::open(store_config)
        .await
        .expect("should open store");

    // 2. Insert a learning
    let learning = SharedLearning::new(
        "Dedup Test".to_string(),
        "Testing deduplication on restart.".to_string(),
        SharedLearningSource::Manual,
        "test-agent".to_string(),
    );
    let id = learning.id.clone();
    store.insert(learning).await.expect("insert");

    // 3. Manually copy the markdown file to the shared directory
    let canonical_path = temp_dir
        .path()
        .join("test-agent")
        .join(format!("{}.md", id));
    let shared_dir = temp_dir.path().join("shared");
    tokio::fs::create_dir_all(&shared_dir).await.unwrap();
    let shared_path = shared_dir.join(format!("test-agent-{}.md", id));
    tokio::fs::copy(&canonical_path, &shared_path)
        .await
        .expect("copy to shared dir");

    // 4. Drop and reopen store
    drop(store);

    let store_config2 = StoreConfig {
        similarity_threshold: 0.8,
        auto_promote_l2: true,
        markdown: markdown_config,
    };
    let reopened = SharedLearningStore::open(store_config2)
        .await
        .expect("should reopen store");

    // 5. Verify only one copy exists in the store index
    let all = reopened.list_all().await.expect("list_all");
    assert_eq!(
        all.len(),
        1,
        "should deduplicate canonical and shared copies"
    );
    assert_eq!(all[0].id, id);
}
