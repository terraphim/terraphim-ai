//! Security testing utilities for server API
//!
//! This module provides security-focused tests including input validation,
//! XSS prevention, SQL injection protection, and rate limiting verification.
//!
//! Note: These tests require the `server-api-tests` feature to compile,
//! as they depend on internal terraphim_server types.

#![allow(unused_imports)]

#[cfg(feature = "server-api-tests")]
use crate::testing::server_api::{TestFixtures, TestServer};
#[cfg(feature = "server-api-tests")]
use reqwest::StatusCode;

/// SQL injection prevention tests
#[cfg(feature = "server-api-tests")]
pub mod sql_injection_tests {
    use super::*;

    #[tokio::test]
    async fn test_sql_injection_prevention_search() {
        let server = TestServer::new()
            .await
            .expect("Failed to create test server");

        let malicious_queries = vec![
            "'; DROP TABLE documents; --",
            "' OR '1'='1",
            "'; SELECT * FROM users; --",
            "1' UNION SELECT password FROM admin--",
        ];

        for query in malicious_queries {
            let response = server
                .get(&format!(
                    "/documents/search?query={}",
                    urlencoding::encode(query)
                ))
                .await
                .expect("Search request failed");

            // Should handle malicious input safely and return success
            response.validate_status(StatusCode::OK);

            let search_response: terraphim_server::api::SearchResponse =
                response.validate_json().expect("JSON validation failed");

            assert_eq!(
                search_response.status,
                terraphim_server::error::Status::Success
            );
        }
    }

    #[tokio::test]
    async fn test_sql_injection_prevention_chat() {
        let server = TestServer::new()
            .await
            .expect("Failed to create test server");

        let malicious_message =
            terraphim_types::ChatMessage::user("'; DROP TABLE conversations; --".to_string());

        let chat_request = terraphim_server::api::ChatRequest {
            role: "TestRole".to_string(),
            messages: vec![malicious_message],
            model: None,
            conversation_id: None,
            max_tokens: Some(100),
            temperature: Some(0.7),
        };

        let response = server
            .post("/chat", &chat_request)
            .await
            .expect("Chat request failed");

        // Should handle malicious input safely
        response.validate_status(StatusCode::OK);

        let chat_response: terraphim_server::api::ChatResponse =
            response.validate_json().expect("JSON validation failed");

        // Response may be successful or error depending on LLM configuration
        match chat_response.status {
            terraphim_server::error::Status::Success => {
                assert!(chat_response.message.is_some());
                // Check that the malicious content didn't cause issues
                assert!(!chat_response.message.unwrap().contains("DROP TABLE"));
            }
            terraphim_server::error::Status::Error => {
                assert!(chat_response.error.is_some());
            }
            _ => {} // Other statuses are acceptable
        }
    }
}

/// XSS (Cross-Site Scripting) prevention tests
#[cfg(feature = "server-api-tests")]
pub mod xss_tests {
    use super::*;

    #[tokio::test]
    async fn test_xss_prevention_document_creation() {
        let server = TestServer::new()
            .await
            .expect("Failed to create test server");

        let malicious_document = TestFixtures::malicious_document();

        let response = server
            .post("/documents", &malicious_document)
            .await
            .expect("Document creation request failed");

        response.validate_status(StatusCode::OK);

        let create_response: terraphim_server::api::CreateDocumentResponse =
            response.validate_json().expect("JSON validation failed");

        assert_eq!(
            create_response.status,
            terraphim_server::error::Status::Success
        );

        // Search for the document and verify XSS is sanitized
        let search_response = server
            .get(&format!(
                "/documents/search?query={}",
                urlencoding::encode(&malicious_document.title)
            ))
            .await
            .expect("Search request failed");

        search_response.validate_status(StatusCode::OK);

        let search_result: terraphim_server::api::SearchResponse = search_response
            .validate_json()
            .expect("JSON validation failed");

        if let Some(found_doc) = search_result.results.first() {
            // Check that script tags are properly escaped or removed
            assert!(!found_doc.title.contains("<script>"));
            assert!(!found_doc.body.contains("<script>"));
        }
    }

    #[tokio::test]
    async fn test_xss_prevention_chat_messages() {
        let server = TestServer::new()
            .await
            .expect("Failed to create test server");

        let malicious_message = terraphim_types::ChatMessage::user(
            "<script>alert('xss')</script>Hello world".to_string(),
        );

        let chat_request = terraphim_server::api::ChatRequest {
            role: "TestRole".to_string(),
            messages: vec![malicious_message],
            model: None,
            conversation_id: None,
            max_tokens: Some(100),
            temperature: Some(0.7),
        };

        let response = server
            .post("/chat", &chat_request)
            .await
            .expect("Chat request failed");

        response.validate_status(StatusCode::OK);

        let chat_response: terraphim_server::api::ChatResponse =
            response.validate_json().expect("JSON validation failed");

        if let Some(message) = chat_response.message {
            // Response should not contain active script tags
            assert!(!message.contains("<script>"));
            // But should contain the text content
            assert!(
                message.to_lowercase().contains("hello world")
                    || chat_response.status == terraphim_server::error::Status::Error
            );
        }
    }
}

/// Path traversal prevention tests
#[cfg(feature = "server-api-tests")]
pub mod path_traversal_tests {
    use super::*;

    #[tokio::test]
    async fn test_path_traversal_prevention() {
        let server = TestServer::new()
            .await
            .expect("Failed to create test server");

        let malicious_paths = vec![
            "../../../etc/passwd",
            "..\\..\\..\\windows\\system32\\config\\sam",
            "/etc/passwd",
            "C:\\Windows\\System32\\config\\sam",
        ];

        for path in malicious_paths {
            let malicious_document = terraphim_types::Document {
                id: "malicious-doc".to_string(),
                url: format!("file://{}", path),
                title: "Path Traversal Test".to_string(),
                body: "Test content".to_string(),
                description: None,
                summarization: None,
                stub: None,
                tags: None,
                rank: None,
                source_haystack: None,
            };

            let response = server
                .post("/documents", &malicious_document)
                .await
                .expect("Document creation request failed");

            // Should either succeed (if path traversal is allowed for file:// URLs)
            // or fail gracefully, but not expose sensitive information
            match response.status() {
                StatusCode::OK => {
                    let create_response: terraphim_server::api::CreateDocumentResponse =
                        response.validate_json().expect("JSON validation failed");
                    assert_eq!(
                        create_response.status,
                        terraphim_server::error::Status::Success
                    );
                }
                StatusCode::BAD_REQUEST => {
                    // This is acceptable - server may reject suspicious paths
                }
                _ => {
                    // Ensure no sensitive information is leaked in error responses
                    let error_text = response.text().await.unwrap_or_default();
                    assert!(!error_text.contains("root:"));
                    assert!(!error_text.contains("admin:"));
                }
            }
        }
    }
}

/// Rate limiting tests
#[cfg(feature = "server-api-tests")]
pub mod rate_limiting_tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_rate_limiting_burst_requests() {
        let server = TestServer::new()
            .await
            .expect("Failed to create test server");
        let client = reqwest::Client::new();

        let mut responses = Vec::new();

        // Send many requests rapidly
        for i in 0..50 {
            let response = client
                .get(&format!(
                    "{}/documents/search?query=test{}",
                    server.base_url, i
                ))
                .send()
                .await;

            match response {
                Ok(resp) => responses.push(resp.status()),
                Err(_) => responses.push(StatusCode::INTERNAL_SERVER_ERROR),
            }

            // Small delay to avoid overwhelming the test environment
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        let success_count = responses
            .iter()
            .filter(|&&status| status.is_success())
            .count();
        let rate_limited_count = responses
            .iter()
            .filter(|&&status| status == StatusCode::TOO_MANY_REQUESTS)
            .count();

        // Either all requests succeed (no rate limiting) or some are rate limited
        assert!(
            success_count + rate_limited_count == responses.len(),
            "Unexpected status codes in responses: {:?}",
            responses
        );

        println!(
            "Rate limiting test: {}/{} requests succeeded, {}/{} rate limited",
            success_count,
            responses.len(),
            rate_limited_count,
            responses.len()
        );
    }
}

/// Input validation tests
#[cfg(feature = "server-api-tests")]
pub mod input_validation_tests {
    use super::*;

    #[tokio::test]
    async fn test_extremely_large_input() {
        let server = TestServer::new()
            .await
            .expect("Failed to create test server");

        // Create a document with extremely large content (100MB)
        let large_content = "x".repeat(100 * 1024 * 1024);
        let large_document = terraphim_types::Document {
            id: "large-input-test".to_string(),
            url: "file:///test/large.txt".to_string(),
            title: "Large Input Test".to_string(),
            body: large_content,
            description: Some("Testing large input handling".to_string()),
            summarization: None,
            stub: None,
            tags: None,
            rank: None,
            source_haystack: None,
        };

        let response = server
            .post("/documents", &large_document)
            .await
            .expect("Large document creation request failed");

        // Should either succeed or fail gracefully with appropriate error
        match response.status() {
            StatusCode::OK => {
                let create_response: terraphim_server::api::CreateDocumentResponse =
                    response.validate_json().expect("JSON validation failed");
                assert_eq!(
                    create_response.status,
                    terraphim_server::error::Status::Success
                );
            }
            StatusCode::BAD_REQUEST | StatusCode::PAYLOAD_TOO_LARGE => {
                // Acceptable - server may reject extremely large inputs
                let error_text = response.text().await.unwrap_or_default();
                assert!(!error_text.contains("panic") && !error_text.contains("stack trace"));
            }
            _ => panic!(
                "Unexpected status code for large input: {}",
                response.status()
            ),
        }
    }

    #[tokio::test]
    async fn test_null_bytes_injection() {
        let server = TestServer::new()
            .await
            .expect("Failed to create test server");

        let malicious_document = terraphim_types::Document {
            id: "null-byte-test".to_string(),
            url: "file:///test/null.txt".to_string(),
            title: "Null Byte Test\0Malicious".to_string(),
            body: "Content with null byte: \0".to_string(),
            description: None,
            summarization: None,
            stub: None,
            tags: None,
            rank: None,
            source_haystack: None,
        };

        let response = server
            .post("/documents", &malicious_document)
            .await
            .expect("Null byte document creation request failed");

        // Should handle null bytes safely
        response.validate_status(StatusCode::OK);

        let create_response: terraphim_server::api::CreateDocumentResponse =
            response.validate_json().expect("JSON validation failed");

        assert_eq!(
            create_response.status,
            terraphim_server::error::Status::Success
        );
    }

    #[tokio::test]
    async fn test_unicode_normalization_attacks() {
        let server = TestServer::new()
            .await
            .expect("Failed to create test server");

        // Test various Unicode normalization forms that could be used for bypass attacks
        let malicious_queries = vec![
            "test",         // Normal
            "t\u{0065}st",  // Decomposed e
            "t\u{00e9}st",  // Composed é
            "test\u{200b}", // Zero-width space
            "test\u{200c}", // Zero-width non-joiner
            "test\u{200d}", // Zero-width joiner
        ];

        for query in malicious_queries {
            let response = server
                .get(&format!(
                    "/documents/search?query={}",
                    urlencoding::encode(query)
                ))
                .await
                .expect("Unicode search request failed");

            response.validate_status(StatusCode::OK);

            let search_response: terraphim_server::api::SearchResponse =
                response.validate_json().expect("JSON validation failed");

            assert_eq!(
                search_response.status,
                terraphim_server::error::Status::Success
            );
        }
    }
}

/// Command injection prevention tests
#[cfg(feature = "server-api-tests")]
pub mod command_injection_tests {
    use super::*;

    #[tokio::test]
    async fn test_command_injection_prevention() {
        let server = TestServer::new()
            .await
            .expect("Failed to create test server");

        let malicious_commands = vec![
            "$(rm -rf /)",
            "`rm -rf /`",
            "; rm -rf /",
            "| rm -rf /",
            "&& rm -rf /",
            "|| rm -rf /",
        ];

        for command in malicious_commands {
            let malicious_document = terraphim_types::Document {
                id: "cmd-injection-test".to_string(),
                url: format!("file:///test/{}", command),
                title: "Command Injection Test".to_string(),
                body: "Test content".to_string(),
                description: None,
                summarization: None,
                stub: None,
                tags: None,
                rank: None,
                source_haystack: None,
            };

            let response = server
                .post("/documents", &malicious_document)
                .await
                .expect("Command injection document creation request failed");

            // Should handle command injection attempts safely
            match response.status() {
                StatusCode::OK => {
                    let create_response: terraphim_server::api::CreateDocumentResponse =
                        response.validate_json().expect("JSON validation failed");
                    assert_eq!(
                        create_response.status,
                        terraphim_server::error::Status::Success
                    );
                }
                StatusCode::BAD_REQUEST => {
                    // Acceptable - server may reject suspicious input
                }
                _ => {
                    // Ensure no command execution occurred
                    let error_text = response.text().await.unwrap_or_default();
                    assert!(!error_text.contains("rm:") && !error_text.contains("cannot remove"));
                }
            }
        }
    }
}
