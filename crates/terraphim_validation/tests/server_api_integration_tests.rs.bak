#![cfg(feature = "server-api-tests")]
//! Server API integration tests
//!
//! This module contains integration tests that exercise the full terraphim server API
//! using the test harness and fixtures defined in the server_api module.

use std::time::Duration;
use terraphim_validation::testing::server_api::*;

#[cfg(test)]
mod api_integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_full_api_workflow() {
        let server = TestServer::new()
            .await
            .expect("Failed to create test server");

        // 1. Health check
        let response = server.get("/health").await;
        response.validate_status(reqwest::StatusCode::OK);
        let body = response
            .text()
            .await
            .expect("Failed to read health response");
        assert_eq!(body, "OK");

        // 2. Create documents
        let documents = TestFixtures::sample_documents(3);
        let mut created_ids = Vec::new();

        for doc in documents {
            let response = server
                .post("/documents", &doc)
                .await
                .expect("Document creation failed");
            response.validate_status(reqwest::StatusCode::OK);

            let create_response: terraphim_server::api::CreateDocumentResponse =
                response.validate_json().expect("JSON validation failed");
            assert_eq!(
                create_response.status,
                terraphim_server::error::Status::Success
            );
            created_ids.push(create_response.id);
        }

        // 3. Search documents
        let search_query = TestFixtures::search_query("test");
        let response = server
            .post("/documents/search", &search_query)
            .await
            .expect("Search failed");
        response.validate_status(reqwest::StatusCode::OK);

        let search_response: terraphim_server::api::SearchResponse =
            response.validate_json().expect("JSON validation failed");
        assert_eq!(
            search_response.status,
            terraphim_server::error::Status::Success
        );
        assert!(search_response.total >= 3);

        // 4. Get configuration
        let response = server.get("/config").await;
        response.validate_status(reqwest::StatusCode::OK);

        let config_response: terraphim_server::api::ConfigResponse =
            response.validate_json().expect("JSON validation failed");
        assert_eq!(
            config_response.status,
            terraphim_server::error::Status::Success
        );

        // 5. Update configuration
        let mut updated_config = config_response.config;
        updated_config.global_shortcut = "Ctrl+Shift+X".to_string();

        let response = server
            .post("/config", &updated_config)
            .await
            .expect("Config update failed");
        response.validate_status(reqwest::StatusCode::OK);

        let update_response: terraphim_server::api::ConfigResponse =
            response.validate_json().expect("JSON validation failed");
        assert_eq!(
            update_response.status,
            terraphim_server::error::Status::Success
        );
        assert_eq!(update_response.config.global_shortcut, "Ctrl+Shift+X");

        // 6. Test rolegraph visualization
        let response = server
            .get("/rolegraph")
            .await
            .expect("Rolegraph fetch failed");
        response.validate_status(reqwest::StatusCode::OK);

        let rolegraph_response: terraphim_server::api::RoleGraphResponseDto =
            response.validate_json().expect("JSON validation failed");
        assert_eq!(
            rolegraph_response.status,
            terraphim_server::error::Status::Success
        );

        println!("Full API workflow test completed successfully");
    }

    #[tokio::test]
    async fn test_concurrent_load() {
        let server = TestServer::new()
            .await
            .expect("Failed to create test server");

        // Test concurrent search requests
        let results = performance::test_concurrent_requests(
            &server,
            "/documents/search?query=test",
            10, // concurrency
            50, // total requests
        )
        .await
        .expect("Concurrent load test failed");

        // Assert performance requirements
        performance::assertions::assert_avg_response_time(&results, 1000); // 1 second max avg
        performance::assertions::assert_p95_response_time(&results, 2000); // 2 seconds max p95
        performance::assertions::assert_failure_rate(&results, 0.1); // Max 10% failure rate

        println!(
            "Concurrent load test results: {:.2} req/sec, avg {}ms, p95 {}ms",
            results.requests_per_second,
            results.avg_response_time.as_millis(),
            results.p95_response_time.as_millis()
        );
    }

    #[tokio::test]
    async fn test_large_dataset_processing() {
        let server = TestServer::new()
            .await
            .expect("Failed to create test server");

        let results = performance::test_large_dataset_processing(&server)
            .await
            .expect("Large dataset test failed");

        // Assert that large document processing completes within reasonable time
        performance::assertions::assert_avg_response_time(&results, 10000); // 10 seconds max for large docs

        println!(
            "Large dataset processing test completed in {}ms",
            results.total_duration.as_millis()
        );
    }

    #[tokio::test]
    async fn test_security_comprehensive() {
        let server = TestServer::new()
            .await
            .expect("Failed to create test server");

        // Test various security scenarios
        let malicious_document = TestFixtures::malicious_document();
        let response = server
            .post("/documents", &malicious_document)
            .await
            .expect("Malicious document creation failed");

        response.validate_status(reqwest::StatusCode::OK);

        let create_response: terraphim_server::api::CreateDocumentResponse =
            response.validate_json().expect("JSON validation failed");

        assert_eq!(
            create_response.status,
            terraphim_server::error::Status::Success
        );

        // Verify XSS sanitization by searching
        let search_response = server
            .get("/documents/search?query=script")
            .await
            .expect("XSS search failed");

        search_response.validate_status(reqwest::StatusCode::OK);

        let search_result: terraphim_server::api::SearchResponse = search_response
            .validate_json()
            .expect("JSON validation failed");

        // Ensure no active script tags in results
        for doc in &search_result.results {
            assert!(!doc.title.contains("<script>"));
            assert!(!doc.body.contains("<script>"));
        }

        println!("Security comprehensive test passed");
    }

    #[tokio::test]
    async fn test_error_handling_comprehensive() {
        let server = TestServer::new()
            .await
            .expect("Failed to create test server");

        // Test invalid role
        let response = server
            .get("/thesaurus/NonExistentRole")
            .await
            .expect("Invalid role request failed");
        response.validate_status(reqwest::StatusCode::NOT_FOUND);

        let thesaurus_response: terraphim_server::api::ThesaurusResponse =
            response.validate_json().expect("JSON validation failed");
        assert_eq!(
            thesaurus_response.status,
            terraphim_server::error::Status::Error
        );

        // Test malformed JSON
        let client = reqwest::Client::new();
        let response = client
            .post(&format!("{}/documents", server.base_url))
            .header("Content-Type", "application/json")
            .body("{ invalid json content }")
            .send()
            .await
            .expect("Malformed JSON request failed");

        response.validate_status(reqwest::StatusCode::BAD_REQUEST);

        // Test empty search (should handle gracefully)
        let response = server
            .get("/documents/search?query=")
            .await
            .expect("Empty search failed");
        response.validate_status(reqwest::StatusCode::OK);

        let search_response: terraphim_server::api::SearchResponse =
            response.validate_json().expect("JSON validation failed");
        assert_eq!(
            search_response.status,
            terraphim_server::error::Status::Success
        );

        println!("Error handling comprehensive test passed");
    }

    #[tokio::test]
    async fn test_chat_and_conversation_workflow() {
        let server = TestServer::new()
            .await
            .expect("Failed to create test server");

        // Create a conversation
        let conversation_request = terraphim_server::api_conversations::CreateConversationRequest {
            title: Some("Test Conversation".to_string()),
            role: "TestRole".to_string(),
        };

        let response = server
            .post("/conversations", &conversation_request)
            .await
            .expect("Conversation creation failed");

        response.validate_status(reqwest::StatusCode::OK);

        let create_conv_response: terraphim_server::api_conversations::CreateConversationResponse =
            response.validate_json().expect("JSON validation failed");

        assert_eq!(
            create_conv_response.status,
            terraphim_server::error::Status::Success
        );
        let conversation_id = create_conv_response.id.clone();

        // List conversations
        let response = server
            .get("/conversations")
            .await
            .expect("List conversations failed");
        response.validate_status(reqwest::StatusCode::OK);

        let list_response: terraphim_server::api_conversations::ListConversationsResponse =
            response.validate_json().expect("JSON validation failed");

        assert_eq!(
            list_response.status,
            terraphim_server::error::Status::Success
        );
        assert!(
            list_response
                .conversations
                .iter()
                .any(|c| c.id == conversation_id)
        );

        // Get specific conversation
        let response = server
            .get(&format!("/conversations/{}", conversation_id))
            .await
            .expect("Get conversation failed");

        response.validate_status(reqwest::StatusCode::OK);

        let get_response: terraphim_server::api_conversations::GetConversationResponse =
            response.validate_json().expect("JSON validation failed");

        assert_eq!(
            get_response.status,
            terraphim_server::error::Status::Success
        );
        assert_eq!(get_response.conversation.id, conversation_id);

        // Add a message to the conversation
        let message_request = terraphim_server::api_conversations::AddMessageRequest {
            message: TestFixtures::chat_message("Hello, this is a test message"),
        };

        let response = server
            .post(
                &format!("/conversations/{}/messages", conversation_id),
                &message_request,
            )
            .await
            .expect("Add message failed");

        response.validate_status(reqwest::StatusCode::OK);

        let add_msg_response: terraphim_server::api_conversations::AddMessageResponse =
            response.validate_json().expect("JSON validation failed");

        assert_eq!(
            add_msg_response.status,
            terraphim_server::error::Status::Success
        );

        println!("Chat and conversation workflow test completed successfully");
    }
}
