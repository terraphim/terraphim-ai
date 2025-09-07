use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::time::timeout;

use axum::{routing::get, Router};
use reqwest::Client;
use serde_json::{json, Value};
use terraphim_config::{ConfigState, Role};
use terraphim_server::{
    api::{
        add_context_to_conversation, create_conversation, delete_context_from_conversation,
        get_conversation, update_context_in_conversation,
    },
    AddContextRequest, CreateConversationRequest, DeleteContextResponse, UpdateContextRequest,
    UpdateContextResponse,
};
use terraphim_types::{ConversationId, RoleName};

/// Integration tests for context API endpoints using a real server instance
/// These tests verify that the new delete and update context endpoints work correctly

struct TestServer {
    port: u16,
    base_url: String,
    #[allow(dead_code)]
    handle: tokio::task::JoinHandle<()>,
}

impl TestServer {
    async fn start() -> Self {
        // Find an available port
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let base_url = format!("http://127.0.0.1:{}", port);

        // Create a minimal config for testing
        let mut test_role = Role::default();
        test_role.name = "Test Role".to_string();

        let mut roles = HashMap::new();
        roles.insert(RoleName::new("test"), test_role);

        let config = ConfigState { roles };
        let config_state = Arc::new(config);

        // Create the router with our endpoints
        let app = Router::new()
            .route("/health", get(|| async { "OK" }))
            .route("/conversations", axum::routing::post(create_conversation))
            .route("/conversations/:id", get(get_conversation))
            .route(
                "/conversations/:id/context",
                axum::routing::post(add_context_to_conversation),
            )
            .route(
                "/conversations/:id/context/:context_id",
                axum::routing::delete(delete_context_from_conversation)
                    .put(update_context_in_conversation),
            )
            .with_state(config_state);

        // Start the server
        let handle = tokio::spawn(async move {
            let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port))
                .await
                .unwrap();
            axum::serve(listener, app).await.unwrap();
        });

        // Wait for server to be ready
        let client = Client::new();
        let health_url = format!("{}/health", base_url);

        for _ in 0..10 {
            if let Ok(response) = client.get(&health_url).send().await {
                if response.status().is_success() {
                    break;
                }
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        TestServer {
            port,
            base_url,
            handle,
        }
    }

    async fn create_test_conversation(&self) -> String {
        let client = Client::new();
        let response = client
            .post(&format!("{}/conversations", self.base_url))
            .json(&json!({
                "title": "Test Conversation",
                "role": "test"
            }))
            .send()
            .await
            .unwrap();

        assert!(response.status().is_success());
        let data: Value = response.json().await.unwrap();
        data["conversation_id"].as_str().unwrap().to_string()
    }

    async fn add_test_context(&self, conversation_id: &str) -> String {
        let client = Client::new();
        let response = client
            .post(&format!(
                "{}/conversations/{}/context",
                self.base_url, conversation_id
            ))
            .json(&json!({
                "context_type": "UserInput",
                "title": "Test Context Item",
                "summary": "This is a test summary",
                "content": "This is test content for context API testing.",
                "metadata": {"test": "value"}
            }))
            .send()
            .await
            .unwrap();

        assert!(response.status().is_success());

        // Get the conversation to find the context ID
        let conv_response = client
            .get(&format!(
                "{}/conversations/{}",
                self.base_url, conversation_id
            ))
            .send()
            .await
            .unwrap();

        assert!(conv_response.status().is_success());
        let conv_data: Value = conv_response.json().await.unwrap();
        conv_data["conversation"]["global_context"][0]["id"]
            .as_str()
            .unwrap()
            .to_string()
    }
}

#[tokio::test]
async fn test_delete_context_api_success() {
    let server = TestServer::start().await;
    let client = Client::new();

    // Create conversation and context
    let conversation_id = server.create_test_conversation().await;
    let context_id = server.add_test_context(&conversation_id).await;

    // Verify context exists
    let response = client
        .get(&format!(
            "{}/conversations/{}",
            server.base_url, conversation_id
        ))
        .send()
        .await
        .unwrap();
    let data: Value = response.json().await.unwrap();
    assert_eq!(
        data["conversation"]["global_context"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    // Delete the context
    let delete_response = client
        .delete(&format!(
            "{}/conversations/{}/context/{}",
            server.base_url, conversation_id, context_id
        ))
        .send()
        .await
        .unwrap();

    assert!(delete_response.status().is_success());
    let delete_data: DeleteContextResponse = delete_response.json().await.unwrap();
    assert_eq!(delete_data.status.to_string(), "Success");
    assert!(delete_data.error.is_none());

    // Verify context was removed
    let verify_response = client
        .get(&format!(
            "{}/conversations/{}",
            server.base_url, conversation_id
        ))
        .send()
        .await
        .unwrap();
    let verify_data: Value = verify_response.json().await.unwrap();
    assert_eq!(
        verify_data["conversation"]["global_context"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
}

#[tokio::test]
async fn test_delete_context_api_not_found() {
    let server = TestServer::start().await;
    let client = Client::new();

    let conversation_id = server.create_test_conversation().await;

    // Try to delete non-existent context
    let delete_response = client
        .delete(&format!(
            "{}/conversations/{}/context/non-existent",
            server.base_url, conversation_id
        ))
        .send()
        .await
        .unwrap();

    assert!(delete_response.status().is_success()); // Returns 200 with error in body
    let delete_data: DeleteContextResponse = delete_response.json().await.unwrap();
    assert_eq!(delete_data.status.to_string(), "Error");
    assert!(delete_data.error.is_some());
    assert!(delete_data.error.unwrap().contains("not found"));
}

#[tokio::test]
async fn test_update_context_api_success() {
    let server = TestServer::start().await;
    let client = Client::new();

    // Create conversation and context
    let conversation_id = server.create_test_conversation().await;
    let context_id = server.add_test_context(&conversation_id).await;

    // Update the context
    let update_request = UpdateContextRequest {
        context_type: Some("Document".to_string()),
        title: Some("Updated Title".to_string()),
        summary: Some("Updated summary with more details".to_string()),
        content: Some("Updated content with more information".to_string()),
        metadata: Some({
            let mut map = HashMap::new();
            map.insert("updated".to_string(), "true".to_string());
            map.insert("version".to_string(), "2".to_string());
            map
        }),
    };

    let update_response = client
        .put(&format!(
            "{}/conversations/{}/context/{}",
            server.base_url, conversation_id, context_id
        ))
        .json(&update_request)
        .send()
        .await
        .unwrap();

    assert!(update_response.status().is_success());
    let update_data: UpdateContextResponse = update_response.json().await.unwrap();
    assert_eq!(update_data.status.to_string(), "Success");
    assert!(update_data.context.is_some());
    assert!(update_data.error.is_none());

    let updated_context = update_data.context.unwrap();
    assert_eq!(updated_context.title, "Updated Title");
    assert_eq!(
        updated_context.summary,
        Some("Updated summary with more details".to_string())
    );
    assert_eq!(
        updated_context.content,
        "Updated content with more information"
    );
    assert_eq!(updated_context.context_type.to_string(), "Document");

    // Verify the update persisted
    let verify_response = client
        .get(&format!(
            "{}/conversations/{}",
            server.base_url, conversation_id
        ))
        .send()
        .await
        .unwrap();
    let verify_data: Value = verify_response.json().await.unwrap();
    let context_array = verify_data["conversation"]["global_context"]
        .as_array()
        .unwrap();
    assert_eq!(context_array.len(), 1);

    let persisted_context = &context_array[0];
    assert_eq!(persisted_context["title"], "Updated Title");
    assert_eq!(
        persisted_context["summary"],
        "Updated summary with more details"
    );
    assert_eq!(
        persisted_context["content"],
        "Updated content with more information"
    );
}

#[tokio::test]
async fn test_update_context_api_partial_update() {
    let server = TestServer::start().await;
    let client = Client::new();

    // Create conversation and context
    let conversation_id = server.create_test_conversation().await;
    let context_id = server.add_test_context(&conversation_id).await;

    // Partial update - only title and summary
    let partial_update_request = UpdateContextRequest {
        context_type: None,
        title: Some("Partially Updated Title".to_string()),
        summary: Some("Partially updated summary".to_string()),
        content: None,
        metadata: None,
    };

    let update_response = client
        .put(&format!(
            "{}/conversations/{}/context/{}",
            server.base_url, conversation_id, context_id
        ))
        .json(&partial_update_request)
        .send()
        .await
        .unwrap();

    assert!(update_response.status().is_success());
    let update_data: UpdateContextResponse = update_response.json().await.unwrap();
    assert_eq!(update_data.status.to_string(), "Success");

    let updated_context = update_data.context.unwrap();
    assert_eq!(updated_context.title, "Partially Updated Title");
    assert_eq!(
        updated_context.summary,
        Some("Partially updated summary".to_string())
    );
    // Original content should remain unchanged
    assert_eq!(
        updated_context.content,
        "This is test content for context API testing."
    );
    // Original context type should remain unchanged
    assert_eq!(updated_context.context_type.to_string(), "UserInput");
}

#[tokio::test]
async fn test_update_context_api_not_found() {
    let server = TestServer::start().await;
    let client = Client::new();

    let conversation_id = server.create_test_conversation().await;

    let update_request = UpdateContextRequest {
        context_type: None,
        title: Some("Updated Title".to_string()),
        summary: None,
        content: None,
        metadata: None,
    };

    // Try to update non-existent context
    let update_response = client
        .put(&format!(
            "{}/conversations/{}/context/non-existent",
            server.base_url, conversation_id
        ))
        .json(&update_request)
        .send()
        .await
        .unwrap();

    assert!(update_response.status().is_success()); // Returns 200 with error in body
    let update_data: UpdateContextResponse = update_response.json().await.unwrap();
    assert_eq!(update_data.status.to_string(), "Error");
    assert!(update_data.context.is_none());
    assert!(update_data.error.is_some());
    assert!(update_data.error.unwrap().contains("not found"));
}

#[tokio::test]
async fn test_context_api_with_summary_field() {
    let server = TestServer::start().await;
    let client = Client::new();

    let conversation_id = server.create_test_conversation().await;

    // Add context with summary field
    let context_request = AddContextRequest {
        context_type: "Document".to_string(),
        title: "Document with Summary".to_string(),
        summary: Some("This is a comprehensive summary of the document".to_string()),
        content: "This is the full content of the document with detailed information.".to_string(),
        metadata: Some({
            let mut map = HashMap::new();
            map.insert("source".to_string(), "test".to_string());
            map
        }),
    };

    let add_response = client
        .post(&format!(
            "{}/conversations/{}/context",
            server.base_url, conversation_id
        ))
        .json(&context_request)
        .send()
        .await
        .unwrap();

    assert!(add_response.status().is_success());

    // Verify context was added with summary
    let verify_response = client
        .get(&format!(
            "{}/conversations/{}",
            server.base_url, conversation_id
        ))
        .send()
        .await
        .unwrap();
    let verify_data: Value = verify_response.json().await.unwrap();
    let context_array = verify_data["conversation"]["global_context"]
        .as_array()
        .unwrap();
    assert_eq!(context_array.len(), 1);

    let added_context = &context_array[0];
    assert_eq!(added_context["title"], "Document with Summary");
    assert_eq!(
        added_context["summary"],
        "This is a comprehensive summary of the document"
    );
    assert_eq!(
        added_context["content"],
        "This is the full content of the document with detailed information."
    );
    assert_eq!(added_context["context_type"], "Document");
}

#[tokio::test]
async fn test_context_crud_workflow() {
    let server = TestServer::start().await;
    let client = Client::new();

    // Create conversation
    let conversation_id = server.create_test_conversation().await;

    // 1. Add context
    let context_request = AddContextRequest {
        context_type: "UserInput".to_string(),
        title: "CRUD Test Context".to_string(),
        summary: Some("Initial summary".to_string()),
        content: "Initial content for CRUD testing".to_string(),
        metadata: Some({
            let mut map = HashMap::new();
            map.insert("version".to_string(), "1".to_string());
            map
        }),
    };

    let add_response = client
        .post(&format!(
            "{}/conversations/{}/context",
            server.base_url, conversation_id
        ))
        .json(&context_request)
        .send()
        .await
        .unwrap();
    assert!(add_response.status().is_success());

    // Get context ID
    let conv_response = client
        .get(&format!(
            "{}/conversations/{}",
            server.base_url, conversation_id
        ))
        .send()
        .await
        .unwrap();
    let conv_data: Value = conv_response.json().await.unwrap();
    let context_id = conv_data["conversation"]["global_context"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();

    // 2. Update context
    let update_request = UpdateContextRequest {
        context_type: Some("Document".to_string()),
        title: Some("Updated CRUD Test Context".to_string()),
        summary: Some("Updated summary for CRUD testing".to_string()),
        content: Some("Updated content with more detailed information".to_string()),
        metadata: Some({
            let mut map = HashMap::new();
            map.insert("version".to_string(), "2".to_string());
            map.insert("updated".to_string(), "true".to_string());
            map
        }),
    };

    let update_response = client
        .put(&format!(
            "{}/conversations/{}/context/{}",
            server.base_url, conversation_id, context_id
        ))
        .json(&update_request)
        .send()
        .await
        .unwrap();
    assert!(update_response.status().is_success());

    // 3. Verify update
    let verify_response = client
        .get(&format!(
            "{}/conversations/{}",
            server.base_url, conversation_id
        ))
        .send()
        .await
        .unwrap();
    let verify_data: Value = verify_response.json().await.unwrap();
    let updated_context = &verify_data["conversation"]["global_context"][0];
    assert_eq!(updated_context["title"], "Updated CRUD Test Context");
    assert_eq!(
        updated_context["summary"],
        "Updated summary for CRUD testing"
    );
    assert_eq!(updated_context["context_type"], "Document");

    // 4. Delete context
    let delete_response = client
        .delete(&format!(
            "{}/conversations/{}/context/{}",
            server.base_url, conversation_id, context_id
        ))
        .send()
        .await
        .unwrap();
    assert!(delete_response.status().is_success());

    // 5. Verify deletion
    let final_response = client
        .get(&format!(
            "{}/conversations/{}",
            server.base_url, conversation_id
        ))
        .send()
        .await
        .unwrap();
    let final_data: Value = final_response.json().await.unwrap();
    assert_eq!(
        final_data["conversation"]["global_context"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
}
