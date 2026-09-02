//! Integration tests for `linear_haystack` using a mock HTTP server.
//!
//! Patterned after `haystack_discourse/tests/wiremock.rs` and
//! `terraphim-linear-cli/tests/backend_smoke.rs`. We spin up a wiremock
//! server, point `LinearClient` at it, and assert that the GraphQL
//! round-trip produces the expected `Document` shape.

use haystack_core::HaystackProvider;
use linear_haystack::{LinearError, LinearHaystack};
use serde_json::json;
use terraphim_types::{Document, SearchQuery};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const TEST_API_KEY: &str = "lin_api_test_key";

fn mock_issues_response() -> serde_json::Value {
    json!({
        "data": {
            "issues": {
                "nodes": [
                    {
                        "id": "uuid-1",
                        "identifier": "ODITECH-580",
                        "title": "fix(docker): mentor-service build fails",
                        "description": "## Problem\n\nThe CI build fails at compile time.",
                        "url": "https://linear.app/zestic-ai/issue/ODITECH-580",
                        "priority": 2,
                        "state": { "name": "Done", "type": "completed" },
                        "assignee": { "name": "Alex" },
                        "team": { "key": "ODITECH", "name": "Odilo-Tech" },
                        "labels": { "nodes": [ { "name": "bug" }, { "name": "infra" } ] },
                        "comments": { "nodes": [
                            { "id": "c1", "body": "Fixed in PR #1234", "createdAt": "2026-07-29T00:00:00Z", "user": { "name": "Alex" } }
                        ]}
                    }
                ]
            }
        }
    })
}

#[tokio::test]
async fn search_returns_documents() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .and(header("Authorization", TEST_API_KEY))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_issues_response()))
        .mount(&server)
        .await;

    let haystack = LinearHaystack::with_endpoint(TEST_API_KEY, format!("{}/graphql", server.uri()))
        .expect("client should construct");

    let docs: Vec<Document> = haystack
        .search(&SearchQuery {
            search_term: "ODITECH-580".into(),
            limit: Some(25),
            ..Default::default()
        })
        .await
        .expect("search should succeed");

    assert_eq!(docs.len(), 1, "should produce one document per issue");
    let doc = &docs[0];
    assert_eq!(doc.id, "ODITECH-580");
    assert_eq!(
        doc.title,
        "[ODITECH-580] fix(docker): mentor-service build fails"
    );
    assert!(doc.body.contains("CI build fails"));
    assert!(doc.body.contains("Fixed in PR #1234"));
    let tags = doc.tags.as_ref().expect("tags");
    assert!(tags.contains(&"ODITECH".to_string()));
    assert!(tags.contains(&"Done".to_string()));
    assert!(tags.contains(&"P2-high".to_string()));
    assert!(tags.contains(&"bug".to_string()));
    assert_eq!(doc.source_haystack.as_deref(), Some("linear:ODITECH"));
}

#[tokio::test]
async fn search_handles_graphql_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "errors": [ { "message": "Bad query" } ],
            "data": null
        })))
        .mount(&server)
        .await;

    let haystack = LinearHaystack::with_endpoint(TEST_API_KEY, format!("{}/graphql", server.uri()))
        .expect("client should construct");

    let result = haystack
        .search(&SearchQuery {
            search_term: "anything".into(),
            ..Default::default()
        })
        .await;
    assert!(matches!(result, Err(LinearError::GraphQL(_))));
}

#[tokio::test]
async fn search_handles_rate_limit() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("Retry-After", "60")
                .set_body_string("rate limited"),
        )
        .mount(&server)
        .await;

    let haystack = LinearHaystack::with_endpoint(TEST_API_KEY, format!("{}/graphql", server.uri()))
        .expect("client should construct");

    let result = haystack
        .search(&SearchQuery {
            search_term: "anything".into(),
            ..Default::default()
        })
        .await;
    match result {
        Err(LinearError::RateLimited { retry_after_secs }) => {
            assert_eq!(retry_after_secs, 60);
        }
        other => panic!("expected RateLimited, got {:?}", other),
    }
}

#[tokio::test]
async fn search_uses_searchable_content_filter() {
    // Verify the request body contains `searchableContent: { contains: "..." }`
    // by asserting the server received the expected variables.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .and(wiremock::matchers::body_string_contains(
            "searchableContent",
        ))
        .and(wiremock::matchers::body_string_contains("pgvector"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": { "issues": { "nodes": [] } }
        })))
        .mount(&server)
        .await;

    let haystack = LinearHaystack::with_endpoint(TEST_API_KEY, format!("{}/graphql", server.uri()))
        .expect("client should construct");

    let docs: Vec<Document> = haystack
        .search(&SearchQuery {
            search_term: "pgvector".into(),
            ..Default::default()
        })
        .await
        .expect("search should succeed");

    assert_eq!(docs.len(), 0);
}
