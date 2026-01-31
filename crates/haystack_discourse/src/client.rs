use anyhow::{Context, Result};
use reqwest::Client;
use url::Url;

use crate::models::{Post, PostDetailsResponse, SearchResponse};

pub struct DiscourseClient {
    client: Client,
    base_url: Url,
    api_key: String,
    api_username: String,
}

impl DiscourseClient {
    pub fn new(base_url: &str, api_key: &str, api_username: &str) -> Result<Self> {
        let base_url = Url::parse(base_url).context("Failed to parse base URL")?;
        println!("Initializing Discourse client for URL: {}", base_url);
        let client = Client::new();

        Ok(Self {
            client,
            base_url,
            api_key: api_key.to_string(),
            api_username: api_username.to_string(),
        })
    }

    async fn fetch_post_details(&self, post_id: u64) -> Result<String> {
        let url = self.base_url.join(&format!("posts/{}.json", post_id))?;

        println!("Fetching post details from: {}", url);

        let response = self
            .client
            .get(url)
            .header("Api-Key", &self.api_key)
            .header("Api-Username", &self.api_username)
            .send()
            .await
            .context("Failed to fetch post details")?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Failed to fetch post details: {}", error_text);
        }

        let details: PostDetailsResponse = response
            .json()
            .await
            .context("Failed to parse post details")?;

        Ok(details.post.cooked)
    }

    pub async fn search_posts(&self, query: &str, limit: u32) -> Result<Vec<Post>> {
        let mut url = self.base_url.join("search.json")?;
        url.query_pairs_mut()
            .append_pair("q", query)
            .append_pair("limit", &limit.to_string());

        println!("Making request to: {}", url);
        println!("Headers:");
        println!("  Api-Username: {}", self.api_username);
        println!("  Api-Key: {}", "*".repeat(8));

        let response = self
            .client
            .get(url)
            .header("Api-Key", &self.api_key)
            .header("Api-Username", &self.api_username)
            .send()
            .await
            .context("Failed to send request")?;

        let status = response.status();
        println!("Response status: {}", status);

        if !status.is_success() {
            let error_text = response.text().await?;
            println!("Error response: {}", error_text);
            anyhow::bail!("API request failed with status {}: {}", status, error_text);
        }

        let response_text = response.text().await?;
        println!("Response body: {}", response_text);

        let search_response: SearchResponse =
            serde_json::from_str(&response_text).context("Failed to parse response JSON")?;

        println!("Found {} posts in response", search_response.posts.len());

        let mut posts = Vec::new();
        for post in search_response.posts {
            // Find corresponding topic
            if let Some(topic) = search_response
                .topics
                .iter()
                .find(|t| t.id == post.topic_id)
            {
                let url = self
                    .base_url
                    .join(&format!(
                        "t/{}/{}/{}",
                        topic.slug, topic.id, post.post_number
                    ))
                    .context("Failed to construct post URL")?;

                // Fetch full post content
                let body = self.fetch_post_details(post.id).await.ok();

                posts.push(Post {
                    id: post.id,
                    title: topic.fancy_title.clone(),
                    url: url.to_string(),
                    excerpt: post.blurb,
                    body,
                    created_at: post.created_at,
                    username: post.username,
                });
            }
        }

        println!("Processed {} valid posts", posts.len());
        Ok(posts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{header, method, path, query_param},
    };

    fn can_bind_localhost() -> bool {
        std::net::TcpListener::bind("127.0.0.1:0").is_ok()
    }

    async fn start_mock_server() -> Option<MockServer> {
        if !can_bind_localhost() {
            eprintln!("Skipping wiremock test: cannot bind to localhost");
            return None;
        }
        Some(MockServer::start().await)
    }

    #[tokio::test]
    async fn test_search_posts() {
        let Some(mock_server) = start_mock_server().await else {
            return;
        };

        let mock_response = json!({
            "posts": [{
                "id": 1,
                "blurb": "Test post content",
                "topic_id": 100,
                "post_number": 1,
                "username": "test_user",
                "created_at": "2024-01-01T00:00:00Z"
            }],
            "topics": [{
                "id": 100,
                "title": "Test Topic",
                "slug": "test-topic",
                "posts_count": 1,
                "fancy_title": "Test Topic Title"
            }]
        });

        Mock::given(method("GET"))
            .and(path("/search.json"))
            .and(query_param("q", "test"))
            .and(query_param("limit", "10"))
            .and(header("Api-Key", "test_api_key"))
            .and(header("Api-Username", "test_user"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&mock_response))
            .mount(&mock_server)
            .await;

        let post_details_response = json!({
            "post": {
                "cooked": "<p>Test post content</p>"
            }
        });

        Mock::given(method("GET"))
            .and(path("/posts/1.json"))
            .and(header("Api-Key", "test_api_key"))
            .and(header("Api-Username", "test_user"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&post_details_response))
            .mount(&mock_server)
            .await;

        let client = DiscourseClient::new(&mock_server.uri(), "test_api_key", "test_user").unwrap();

        let posts = client.search_posts("test", 10).await.unwrap();

        assert_eq!(posts.len(), 1);
        assert_eq!(posts[0].id, 1);
        assert_eq!(posts[0].username, "test_user");
        assert_eq!(posts[0].title, "Test Topic Title");
        assert!(posts[0].body.is_some());
        assert_eq!(posts[0].body.as_ref().unwrap(), "<p>Test post content</p>");
    }

    #[tokio::test]
    async fn test_fetch_post_details() {
        let Some(mock_server) = start_mock_server().await else {
            return;
        };

        // Test successful post fetch
        let post_response = json!({
            "post": {
                "cooked": "<p>Detailed post content</p>"
            }
        });

        Mock::given(method("GET"))
            .and(path("/posts/42.json"))
            .and(header("Api-Key", "test_api_key"))
            .and(header("Api-Username", "test_user"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&post_response))
            .mount(&mock_server)
            .await;

        let client = DiscourseClient::new(&mock_server.uri(), "test_api_key", "test_user").unwrap();

        let content = client.fetch_post_details(42).await.unwrap();
        assert_eq!(content, "<p>Detailed post content</p>");

        // Test error response
        Mock::given(method("GET"))
            .and(path("/posts/404.json"))
            .and(header("Api-Key", "test_api_key"))
            .and(header("Api-Username", "test_user"))
            .respond_with(ResponseTemplate::new(404).set_body_string("Post not found"))
            .mount(&mock_server)
            .await;

        let error = client.fetch_post_details(404).await.unwrap_err();
        assert!(error.to_string().contains("Post not found"));
    }

    #[test]
    fn test_invalid_url() {
        let result = DiscourseClient::new("not a url", "key", "user");
        assert!(result.is_err());
    }
}
