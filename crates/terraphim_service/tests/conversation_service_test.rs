use terraphim_persistence::DeviceStorage;
use terraphim_service::conversation_service::{ConversationFilter, ConversationService};
use terraphim_types::{ChatMessage, RoleName};

#[tokio::test]
async fn test_create_and_get_conversation() {
    // Initialize memory-only storage for testing
    let _ = DeviceStorage::init_memory_only().await.unwrap();

    let service = ConversationService::new();
    let conversation = service
        .create_conversation("Test Conversation".to_string(), RoleName::new("Test Role"))
        .await
        .unwrap();

    let loaded = service.get_conversation(&conversation.id).await.unwrap();
    assert_eq!(loaded.id, conversation.id);
    assert_eq!(loaded.title, "Test Conversation");
    assert_eq!(loaded.role, RoleName::new("Test Role"));
}

#[tokio::test]
async fn test_update_conversation() {
    let _ = DeviceStorage::init_memory_only().await.unwrap();

    let service = ConversationService::new();
    let mut conversation = service
        .create_conversation("Original Title".to_string(), RoleName::new("Test Role"))
        .await
        .unwrap();

    // Add messages
    conversation.add_message(ChatMessage::user("Hello".to_string()));
    conversation.add_message(ChatMessage::assistant(
        "Hi there!".to_string(),
        Some("gpt-4".to_string()),
    ));

    let conversation = service.update_conversation(conversation).await.unwrap();

    let loaded = service.get_conversation(&conversation.id).await.unwrap();
    assert_eq!(loaded.messages.len(), 2);
    assert_eq!(loaded.messages[0].content, "Hello");
    assert_eq!(loaded.messages[1].content, "Hi there!");
}

#[tokio::test]
async fn test_list_conversations() {
    let _ = DeviceStorage::init_memory_only().await.unwrap();

    let service = ConversationService::new();

    // Create multiple conversations
    service
        .create_conversation("Conv 1".to_string(), RoleName::new("Role A"))
        .await
        .unwrap();
    service
        .create_conversation("Conv 2".to_string(), RoleName::new("Role B"))
        .await
        .unwrap();
    service
        .create_conversation("Conv 3".to_string(), RoleName::new("Role A"))
        .await
        .unwrap();

    // List all
    let all = service
        .list_conversations(ConversationFilter::default())
        .await
        .unwrap();
    assert_eq!(all.len(), 3);

    // Filter by role
    let filtered = service
        .list_conversations(ConversationFilter {
            role: Some(RoleName::new("Role A")),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(filtered.len(), 2);
}

#[tokio::test]
async fn test_search_conversations() {
    let _ = DeviceStorage::init_memory_only().await.unwrap();

    let service = ConversationService::new();

    service
        .create_conversation(
            "Machine Learning Discussion".to_string(),
            RoleName::new("AI"),
        )
        .await
        .unwrap();
    service
        .create_conversation("Rust Programming Tips".to_string(), RoleName::new("Dev"))
        .await
        .unwrap();
    service
        .create_conversation("Python Data Science".to_string(), RoleName::new("AI"))
        .await
        .unwrap();

    // Search by title
    let results = service.search_conversations("machine").await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title, "Machine Learning Discussion");

    // Search with multiple results
    let results = service.search_conversations("rust").await.unwrap();
    assert_eq!(results.len(), 1);
}

#[tokio::test]
async fn test_delete_conversation() {
    let _ = DeviceStorage::init_memory_only().await.unwrap();

    let service = ConversationService::new();
    let conversation = service
        .create_conversation("To Delete".to_string(), RoleName::new("Test"))
        .await
        .unwrap();

    // Verify it exists
    let loaded = service.get_conversation(&conversation.id).await;
    assert!(loaded.is_ok());

    // Delete
    service.delete_conversation(&conversation.id).await.unwrap();

    // Verify it's gone
    let loaded = service.get_conversation(&conversation.id).await;
    assert!(loaded.is_err());
}

#[tokio::test]
async fn test_export_import_conversation() {
    let _ = DeviceStorage::init_memory_only().await.unwrap();

    let service = ConversationService::new();
    let mut conversation = service
        .create_conversation("Export Test".to_string(), RoleName::new("Test"))
        .await
        .unwrap();

    conversation.add_message(ChatMessage::user("Test message".to_string()));
    conversation.add_message(ChatMessage::assistant(
        "Test response".to_string(),
        Some("gpt-4".to_string()),
    ));
    let conversation = service.update_conversation(conversation).await.unwrap();

    // Export
    let json = service.export_conversation(&conversation.id).await.unwrap();
    assert!(json.contains("Export Test"));
    assert!(json.contains("Test message"));
    assert!(json.contains("Test response"));

    // Delete original
    service.delete_conversation(&conversation.id).await.unwrap();

    // Import
    let imported = service.import_conversation(&json).await.unwrap();
    assert_eq!(imported.title, "Export Test");
    assert_eq!(imported.messages.len(), 2);
    assert_eq!(imported.messages[0].content, "Test message");
    assert_eq!(imported.messages[1].content, "Test response");
}

#[tokio::test]
async fn test_get_statistics() {
    let _ = DeviceStorage::init_memory_only().await.unwrap();

    let service = ConversationService::new();

    // Create conversations with messages
    let mut conv1 = service
        .create_conversation("Conv 1".to_string(), RoleName::new("Role A"))
        .await
        .unwrap();
    conv1.add_message(ChatMessage::user("Message 1".to_string()));
    conv1.add_message(ChatMessage::assistant("Response 1".to_string(), None));
    service.update_conversation(conv1).await.unwrap();

    let mut conv2 = service
        .create_conversation("Conv 2".to_string(), RoleName::new("Role B"))
        .await
        .unwrap();
    conv2.add_message(ChatMessage::user("Message 2".to_string()));
    service.update_conversation(conv2).await.unwrap();

    let mut conv3 = service
        .create_conversation("Conv 3".to_string(), RoleName::new("Role A"))
        .await
        .unwrap();
    conv3.add_message(ChatMessage::user("Message 3".to_string()));
    conv3.add_message(ChatMessage::assistant("Response 3".to_string(), None));
    conv3.add_message(ChatMessage::user("Message 4".to_string()));
    service.update_conversation(conv3).await.unwrap();

    let stats = service.get_statistics().await.unwrap();
    assert_eq!(stats.total_conversations, 3);
    assert_eq!(stats.total_messages, 6);
    assert_eq!(stats.conversations_by_role.len(), 2);
    assert_eq!(*stats.conversations_by_role.get("Role A").unwrap(), 2);
    assert_eq!(*stats.conversations_by_role.get("Role B").unwrap(), 1);
    assert_eq!(stats.average_messages_per_conversation, 2.0);
}

#[tokio::test]
async fn test_conversation_ordering() {
    let _ = DeviceStorage::init_memory_only().await.unwrap();

    let service = ConversationService::new();

    // Create conversations with delays to ensure different timestamps
    service
        .create_conversation("First".to_string(), RoleName::new("Test"))
        .await
        .unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    service
        .create_conversation("Second".to_string(), RoleName::new("Test"))
        .await
        .unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    service
        .create_conversation("Third".to_string(), RoleName::new("Test"))
        .await
        .unwrap();

    // List should be ordered by updated_at descending (most recent first)
    let conversations = service
        .list_conversations(ConversationFilter::default())
        .await
        .unwrap();
    assert_eq!(conversations[0].title, "Third");
    assert_eq!(conversations[1].title, "Second");
    assert_eq!(conversations[2].title, "First");
}
