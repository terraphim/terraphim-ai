use crate::{confluence, jira};
use anyhow::Result;
use mockito::Server;
use serde_json::json;

mod confluence_tests {
    use super::*;

    #[tokio::test]
    async fn test_confluence_search() -> Result<()> {
        let mut server = Server::new_async().await;
        let mock_response = json!({
            "results": [
                {
                    "id": "123",
                    "type": "page",
                    "status": "current",
                    "title": "Test Page",
                    "space": {
                        "key": "TEAM",
                        "name": "Team Space",
                        "_links": {
                            "webui": "/spaces/TEAM"
                        }
                    },
                    "excerpt": "This is a test page",
                    "_links": {
                        "webui": "/pages/123"
                    }
                }
            ]
        });

        server
            .mock("GET", "/wiki/rest/api/content/search")
            .match_query(mockito::Matcher::AllOf(vec![
                mockito::Matcher::UrlEncoded("cql".into(), "text ~ \"test\"".into()),
                mockito::Matcher::UrlEncoded("limit".into(), "10".into()),
            ]))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(mock_response.to_string())
            .create();

        let results = confluence::search(
            &server.url(),
            "test-user",
            "test-token",
            "text ~ \"test\"",
            10,
        )
        .await?;

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Test Page");
        assert_eq!(results[0].content_type, "page");
        Ok(())
    }

    #[tokio::test]
    async fn test_confluence_get_page() -> Result<()> {
        let mut server = Server::new_async().await;
        let mock_response = json!({
            "id": "123",
            "title": "Test Page",
            "body": {
                "storage": {
                    "value": "Page content"
                }
            },
            "version": {
                "number": 1,
                "when": "2023-01-01T00:00:00.000Z"
            },
            "space": {
                "key": "TEAM",
                "name": "Team Space",
                "_links": {
                    "webui": "/spaces/TEAM"
                }
            },
            "_links": {
                "webui": "/pages/123"
            }
        });

        server
            .mock("GET", "/wiki/rest/api/content/123")
            .match_query(mockito::Matcher::UrlEncoded(
                "expand".into(),
                "body.storage,version,space,_links".into(),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(mock_response.to_string())
            .create();

        let page =
            confluence::get_page(&server.url(), "test-user", "test-token", "123", true).await?;

        assert_eq!(page.id, "123");
        assert_eq!(page.title, "Test Page");
        Ok(())
    }

    #[tokio::test]
    async fn test_confluence_get_comments() -> Result<()> {
        let mut server = Server::new_async().await;
        let mock_response = json!({
            "results": [
                {
                    "body": {
                        "storage": {
                            "value": "Test comment"
                        }
                    },
                    "version": {
                        "number": 1,
                        "when": "2023-01-01T00:00:00.000Z"
                    },
                    "author": {
                        "displayName": "Test User",
                        "email": "test@example.com"
                    }
                }
            ]
        });

        server
            .mock("GET", "/wiki/rest/api/content/123/child/comment")
            .match_query(mockito::Matcher::AllOf(vec![
                mockito::Matcher::UrlEncoded("expand".into(), "body.storage,version,author".into()),
                mockito::Matcher::UrlEncoded("limit".into(), "50".into()),
            ]))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(mock_response.to_string())
            .create();

        let comments =
            confluence::get_comments(&server.url(), "test-user", "test-token", "123").await?;

        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].author.display_name, "Test User");
        Ok(())
    }
}

mod jira_tests {
    use super::*;

    #[tokio::test]
    async fn test_jira_search() -> Result<()> {
        let mut server = Server::new_async().await;
        let mock_response = json!({
            "issues": [
                {
                    "key": "PROJ-123",
                    "fields": {
                        "summary": "Test Issue",
                        "description": "Test Description",
                        "created": "2023-01-01T00:00:00.000Z",
                        "updated": "2023-01-01T00:00:00.000Z",
                        "status": {
                            "name": "In Progress"
                        },
                        "issuetype": {
                            "name": "Bug"
                        },
                        "priority": {
                            "name": "High"
                        }
                    }
                }
            ]
        });

        server
            .mock("POST", "/rest/api/2/search")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(mock_response.to_string())
            .create();

        let results = jira::search(
            &server.url(),
            "test-user",
            "test-token",
            "project = PROJ",
            "*all",
            10,
        )
        .await?;

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].key, "PROJ-123");
        Ok(())
    }

    #[tokio::test]
    async fn test_jira_get_issue() -> Result<()> {
        let mut server = Server::new_async().await;
        let mock_response = json!({
            "key": "PROJ-123",
            "fields": {
                "summary": "Test Issue",
                "description": "Test Description",
                "created": "2023-01-01T00:00:00.000Z",
                "updated": "2023-01-01T00:00:00.000Z",
                "status": {
                    "name": "In Progress"
                },
                "issuetype": {
                    "name": "Bug"
                },
                "priority": {
                    "name": "High"
                }
            }
        });

        server
            .mock("GET", "/rest/api/2/issue/PROJ-123")
            .match_query(mockito::Matcher::UrlEncoded("fields".into(), "*all".into()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(mock_response.to_string())
            .create();

        let issue =
            jira::get_issue(&server.url(), "test-user", "test-token", "PROJ-123", None).await?;

        assert_eq!(issue.key, "PROJ-123");
        assert_eq!(issue.fields.status.name, "In Progress");
        Ok(())
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::env;

    async fn setup() -> Result<()> {
        dotenv::dotenv().ok();
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_real_confluence_search() -> Result<()> {
        setup().await?;
        let confluence_url = env::var("CONFLUENCE_URL")?;
        let username = env::var("ATLASSIAN_USERNAME")?;
        let token = env::var("ATLASSIAN_TOKEN")?;

        let results =
            confluence::search(&confluence_url, &username, &token, "text ~ \"test\"", 10).await?;

        assert!(!results.is_empty());
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_real_jira_search() -> Result<()> {
        setup().await?;
        let jira_url = env::var("JIRA_URL")?;
        let username = env::var("ATLASSIAN_USERNAME")?;
        let token = env::var("ATLASSIAN_TOKEN")?;

        let results =
            jira::search(&jira_url, &username, &token, "project = PROJ", "*all", 10).await?;

        assert!(!results.is_empty());
        Ok(())
    }
}
