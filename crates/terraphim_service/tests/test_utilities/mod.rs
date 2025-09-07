use std::sync::Arc;
use tokio::net::TcpListener;
use axum::{routing::get, Router};
use terraphim_config::{ConfigState, Role};
use terraphim_server::{
    api::{
        add_context_to_conversation, create_conversation, delete_context_from_conversation,
        get_conversation, update_context_in_conversation,
    },
    AddContextRequest, CreateConversationRequest, UpdateContextRequest,
};
use terraphim_types::{ContextItem, ContextType, ConversationId, RoleName};
use ahash::AHashMap;
use std::collections::HashMap;

/// Real server test utilities for context management testing
/// Provides utilities for starting real Terraphim servers and managing test data

pub struct TestServerInstance {
    pub port: u16,
    pub base_url: String,
    handle: tokio::task::JoinHandle<()>,
    pub client: reqwest::Client,
}

impl TestServerInstance {
    /// Start a new test server instance with real Terraphim backend
    pub async fn start(config: Option<ConfigState>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // Find an available port
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let port = listener.local_addr()?.port();
        let base_url = format!("http://127.0.0.1:{}", port);

        // Use provided config or create default test config
        let config_state = config.unwrap_or_else(|| {
            let mut test_role = Role::default();
            test_role.name = "Test Role".to_string();
            
            let mut roles = HashMap::new();
            roles.insert(RoleName::new("test"), test_role);
            
            let config = terraphim_config::Config { 
                roles,
                ..Default::default()
            };
            Arc::new(config.into())
        });

        // Create the real Terraphim API router
        let app = Router::new()
            .route("/health", get(|| async { "OK" }))
            .route("/conversations", axum::routing::post(create_conversation).get(get_conversations))
            .route("/conversations/:id", get(get_conversation))
            .route("/conversations/:id/context", axum::routing::post(add_context_to_conversation))
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
                .expect("Failed to bind test server");
            axum::serve(listener, app).await.expect("Failed to start test server");
        });

        // Create HTTP client
        let client = reqwest::Client::new();

        // Wait for server to be ready
        let health_url = format!("{}/health", base_url);
        for _ in 0..50 {
            if let Ok(response) = client.get(&health_url).send().await {
                if response.status().is_success() {
                    break;
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        Ok(TestServerInstance {
            port,
            base_url,
            handle,
            client,
        })
    }

    /// Create a test conversation and return its ID
    pub async fn create_test_conversation(&self, title: &str, role: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let request = CreateConversationRequest {
            title: title.to_string(),
            role: role.to_string(),
        };

        let response = self.client
            .post(&format!("{}/conversations", self.base_url))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(format!("Failed to create conversation: {}", response.status()).into());
        }

        let data: serde_json::Value = response.json().await?;
        Ok(data["conversation_id"].as_str().unwrap().to_string())
    }

    /// Add a test context item to a conversation
    pub async fn add_test_context(
        &self, 
        conversation_id: &str, 
        title: &str, 
        content: &str,
        summary: Option<String>
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let request = AddContextRequest {
            context_type: "Document".to_string(),
            title: title.to_string(),
            summary,
            content: content.to_string(),
            metadata: Some({
                let mut metadata = HashMap::new();
                metadata.insert("test".to_string(), "true".to_string());
                metadata
            }),
        };

        let response = self.client
            .post(&format!("{}/conversations/{}/context", self.base_url, conversation_id))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(format!("Failed to add context: {}", response.status()).into());
        }

        // Get the conversation to retrieve the context ID
        let conv_response = self.client
            .get(&format!("{}/conversations/{}", self.base_url, conversation_id))
            .send()
            .await?;

        let conv_data: serde_json::Value = conv_response.json().await?;
        let context_array = conv_data["conversation"]["global_context"].as_array().unwrap();
        Ok(context_array.last().unwrap()["id"].as_str().unwrap().to_string())
    }

    /// Update a context item
    pub async fn update_context(
        &self,
        conversation_id: &str,
        context_id: &str,
        title: Option<String>,
        summary: Option<String>,
        content: Option<String>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let request = UpdateContextRequest {
            context_type: None,
            title,
            summary,
            content,
            metadata: None,
        };

        let response = self.client
            .put(&format!(
                "{}/conversations/{}/context/{}",
                self.base_url, conversation_id, context_id
            ))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(format!("Failed to update context: {}", response.status()).into());
        }

        Ok(())
    }

    /// Delete a context item
    pub async fn delete_context(
        &self,
        conversation_id: &str,
        context_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let response = self.client
            .delete(&format!(
                "{}/conversations/{}/context/{}",
                self.base_url, conversation_id, context_id
            ))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(format!("Failed to delete context: {}", response.status()).into());
        }

        Ok(())
    }

    /// Get conversation with all context items
    pub async fn get_conversation_with_context(
        &self,
        conversation_id: &str,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let response = self.client
            .get(&format!("{}/conversations/{}", self.base_url, conversation_id))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(format!("Failed to get conversation: {}", response.status()).into());
        }

        Ok(response.json().await?)
    }

    /// Verify server health
    pub async fn check_health(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let response = self.client
            .get(&format!("{}/health", self.base_url))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(format!("Health check failed: {}", response.status()).into());
        }

        Ok(())
    }

    /// Create multiple test contexts for bulk testing
    pub async fn create_bulk_contexts(
        &self,
        conversation_id: &str,
        count: usize,
        content_prefix: &str,
    ) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        let mut context_ids = Vec::new();

        for i in 0..count {
            let title = format!("{} Context {}", content_prefix, i + 1);
            let content = format!("This is {} content for bulk testing item {}", content_prefix.to_lowercase(), i + 1);
            let summary = Some(format!("Summary for {} {}", content_prefix.to_lowercase(), i + 1));

            let context_id = self.add_test_context(conversation_id, &title, &content, summary).await?;
            context_ids.push(context_id);
        }

        Ok(context_ids)
    }

    /// Verify context count in conversation
    pub async fn verify_context_count(
        &self,
        conversation_id: &str,
        expected_count: usize,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let conv_data = self.get_conversation_with_context(conversation_id).await?;
        let context_array = conv_data["conversation"]["global_context"].as_array().unwrap();
        Ok(context_array.len() == expected_count)
    }

    /// Stop the test server
    pub fn stop(self) {
        self.handle.abort();
    }
}

/// Test data builders for context management
pub struct ContextTestData;

impl ContextTestData {
    /// Create a test context item
    pub fn create_context_item(id: &str, title: &str, content: &str) -> ContextItem {
        ContextItem {
            id: id.to_string(),
            context_type: ContextType::Document,
            title: title.to_string(),
            summary: Some(format!("Summary for {}", title)),
            content: content.to_string(),
            metadata: {
                let mut metadata = AHashMap::new();
                metadata.insert("test_data".to_string(), "true".to_string());
                metadata.insert("created_by".to_string(), "test_utilities".to_string());
                metadata
            },
            created_at: chrono::Utc::now(),
            relevance_score: Some(0.85),
        }
    }

    /// Create multiple test context items
    pub fn create_multiple_contexts(count: usize, prefix: &str) -> Vec<ContextItem> {
        (0..count)
            .map(|i| {
                Self::create_context_item(
                    &format!("{}-ctx-{}", prefix, i),
                    &format!("{} Context Item {}", prefix, i + 1),
                    &format!("Content for {} context item number {}", prefix, i + 1),
                )
            })
            .collect()
    }

    /// Create context with specific metadata
    pub fn create_context_with_metadata(
        id: &str,
        title: &str,
        content: &str,
        metadata: AHashMap<String, String>,
    ) -> ContextItem {
        ContextItem {
            id: id.to_string(),
            context_type: ContextType::Document,
            title: title.to_string(),
            summary: Some(format!("Auto-generated summary for {}", title)),
            content: content.to_string(),
            metadata,
            created_at: chrono::Utc::now(),
            relevance_score: Some(0.90),
        }
    }

    /// Create context items of different types
    pub fn create_varied_context_types() -> Vec<ContextItem> {
        vec![
            ContextItem {
                id: "doc-ctx".to_string(),
                context_type: ContextType::Document,
                title: "Document Context".to_string(),
                summary: Some("Document-type context item".to_string()),
                content: "This is a document context item".to_string(),
                metadata: AHashMap::new(),
                created_at: chrono::Utc::now(),
                relevance_score: Some(0.95),
            },
            ContextItem {
                id: "search-ctx".to_string(),
                context_type: ContextType::SearchResult,
                title: "Search Result Context".to_string(),
                summary: Some("Search result context item".to_string()),
                content: "This context came from a search result".to_string(),
                metadata: AHashMap::new(),
                created_at: chrono::Utc::now(),
                relevance_score: Some(0.88),
            },
            ContextItem {
                id: "user-ctx".to_string(),
                context_type: ContextType::UserInput,
                title: "User Input Context".to_string(),
                summary: Some("User-provided context item".to_string()),
                content: "This context was provided by the user".to_string(),
                metadata: AHashMap::new(),
                created_at: chrono::Utc::now(),
                relevance_score: None,
            },
        ]
    }
}

/// Configuration utilities for testing
pub struct TestConfiguration;

impl TestConfiguration {
    /// Create a basic test role configuration
    pub fn create_basic_role(name: &str, shortname: &str) -> Role {
        Role {
            name: name.to_string(),
            shortname: Some(shortname.to_string()),
            relevance_function: terraphim_types::RelevanceFunction::TitleScorer,
            theme: "default".to_string(),
            haystacks: vec![],
            extra: AHashMap::new(),
            terraphim_it: false,
            ..Default::default()
        }
    }

    /// Create a role with LLM configuration
    pub fn create_llm_enabled_role(name: &str, shortname: &str, base_url: &str) -> Role {
        let mut role = Self::create_basic_role(name, shortname);
        role.extra.insert("llm_provider".to_string(), serde_json::json!("ollama"));
        role.extra.insert("llm_model".to_string(), serde_json::json!("llama3.2:3b"));
        role.extra.insert("llm_base_url".to_string(), serde_json::json!(base_url));
        role.extra.insert("llm_auto_summarize".to_string(), serde_json::json!(true));
        role
    }

    /// Create a test config state with multiple roles
    pub fn create_multi_role_config() -> ConfigState {
        let mut roles = HashMap::new();
        
        roles.insert(
            RoleName::new("basic"),
            Self::create_basic_role("Basic Role", "basic"),
        );
        
        roles.insert(
            RoleName::new("advanced"),
            Self::create_llm_enabled_role("Advanced Role", "advanced", "http://127.0.0.1:11434"),
        );

        let config = terraphim_config::Config {
            roles,
            default_role: RoleName::new("basic"),
            selected_role: RoleName::new("basic"),
            ..Default::default()
        };

        Arc::new(config.into())
    }
}

/// Performance testing utilities
pub struct PerformanceTestUtils;

impl PerformanceTestUtils {
    /// Measure operation performance
    pub async fn measure_operation<F, Fut, T>(
        operation: F,
        operation_name: &str,
    ) -> Result<(T, std::time::Duration), Box<dyn std::error::Error + Send + Sync>>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, Box<dyn std::error::Error + Send + Sync>>>,
    {
        let start = std::time::Instant::now();
        let result = operation().await?;
        let duration = start.elapsed();

        println!("⏱️  Operation '{}' completed in {:?}", operation_name, duration);

        Ok((result, duration))
    }

    /// Run concurrent performance test
    pub async fn run_concurrent_test<F, Fut>(
        operation_factory: F,
        concurrent_count: usize,
        operation_name: &str,
    ) -> Result<Vec<std::time::Duration>, Box<dyn std::error::Error + Send + Sync>>
    where
        F: Fn(usize) -> Fut + Send + Sync + Clone + 'static,
        Fut: std::future::Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send,
    {
        let mut tasks = vec![];

        let start_time = std::time::Instant::now();

        for i in 0..concurrent_count {
            let operation = operation_factory.clone();
            let task = tokio::spawn(async move {
                let task_start = std::time::Instant::now();
                operation(i).await?;
                Ok::<std::time::Duration, Box<dyn std::error::Error + Send + Sync>>(task_start.elapsed())
            });
            tasks.push(task);
        }

        let results = futures::future::try_join_all(tasks).await?;
        let durations: Result<Vec<_>, _> = results.into_iter().collect();
        let durations = durations?;

        let total_duration = start_time.elapsed();
        let avg_duration = durations.iter().sum::<std::time::Duration>() / durations.len() as u32;

        println!("🏃 Concurrent test '{}' results:", operation_name);
        println!("  Operations: {}", concurrent_count);
        println!("  Total time: {:?}", total_duration);
        println!("  Average operation time: {:?}", avg_duration);
        println!("  Fastest operation: {:?}", durations.iter().min().unwrap());
        println!("  Slowest operation: {:?}", durations.iter().max().unwrap());

        Ok(durations)
    }
}

// Simple handler for GET /conversations (used by test server)
async fn get_conversations() -> axum::response::Json<serde_json::Value> {
    axum::response::Json(serde_json::json!({
        "status": "Success",
        "conversations": []
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_server_instance_creation() {
        let server = TestServerInstance::start(None).await.unwrap();
        
        // Test health check
        server.check_health().await.unwrap();
        
        // Test conversation creation
        let conversation_id = server.create_test_conversation("Test Conversation", "test").await.unwrap();
        assert!(!conversation_id.is_empty());
        
        server.stop();
    }

    #[tokio::test]
    async fn test_context_data_builders() {
        let context = ContextTestData::create_context_item("test-1", "Test Context", "Test content");
        assert_eq!(context.id, "test-1");
        assert_eq!(context.title, "Test Context");
        assert_eq!(context.content, "Test content");
        assert!(context.summary.is_some());

        let contexts = ContextTestData::create_multiple_contexts(3, "Bulk");
        assert_eq!(contexts.len(), 3);
        assert_eq!(contexts[0].id, "Bulk-ctx-0");
        assert_eq!(contexts[2].title, "Bulk Context Item 3");
    }

    #[tokio::test]
    async fn test_configuration_utilities() {
        let role = TestConfiguration::create_basic_role("Test Role", "test");
        assert_eq!(role.name, "Test Role");
        assert_eq!(role.shortname, Some("test".to_string()));

        let llm_role = TestConfiguration::create_llm_enabled_role("LLM Role", "llm", "http://localhost:11434");
        assert!(llm_role.extra.contains_key("llm_provider"));
        assert_eq!(llm_role.extra.get("llm_provider").unwrap(), &serde_json::json!("ollama"));
    }

    #[tokio::test]
    async fn test_performance_utils() {
        let (result, duration) = PerformanceTestUtils::measure_operation(
            || async { Ok::<String, Box<dyn std::error::Error + Send + Sync>>("test".to_string()) },
            "test operation"
        ).await.unwrap();

        assert_eq!(result, "test");
        assert!(duration < std::time::Duration::from_secs(1));
    }
}