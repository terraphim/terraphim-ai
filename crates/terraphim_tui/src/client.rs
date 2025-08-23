use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use terraphim_types::{Document, SearchQuery};

#[derive(Clone)]
pub struct ApiClient {
    http: Client,
    base: String,
}

impl ApiClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .user_agent("Terraphim-TUI/1.0")
            .build()
            .unwrap_or_else(|_| Client::new());
            
        Self {
            http: client,
            base: base_url.into(),
        }
    }

    pub async fn health(&self) -> Result<()> {
        let url = format!("{}/health", self.base);
        let res = self.http.get(url).send().await?;
        if res.status().is_success() {
            Ok(())
        } else {
            anyhow::bail!("health check failed")
        }
    }

    pub async fn get_config(&self) -> Result<ConfigResponse> {
        let url = format!("{}/config", self.base);
        let res = self.http.get(url).send().await?;
        let body = res.error_for_status()?.json::<ConfigResponse>().await?;
        Ok(body)
    }

    pub async fn update_selected_role(&self, role: &str) -> Result<ConfigResponse> {
        let url = format!("{}/config/selected_role", self.base);
        #[derive(Serialize)]
        struct Payload<'a> {
            selected_role: &'a str,
        }
        let res = self
            .http
            .post(url)
            .json(&Payload {
                selected_role: role,
            })
            .send()
            .await?;
        let body = res.error_for_status()?.json::<ConfigResponse>().await?;
        Ok(body)
    }

    pub async fn post_config(&self, cfg: &terraphim_config::Config) -> Result<ConfigResponse> {
        let url = format!("{}/config", self.base);
        let res = self.http.post(url).json(cfg).send().await?;
        let body = res.error_for_status()?.json::<ConfigResponse>().await?;
        Ok(body)
    }

    pub async fn get_rolegraph_edges(&self, role: Option<&str>) -> Result<RoleGraphResponseDto> {
        self.rolegraph(role).await
    }

    pub async fn search(&self, query: &SearchQuery) -> Result<SearchResponse> {
        let url = format!("{}/documents/search", self.base);
        let res = self.http.post(url).json(query).send().await?;
        let body = res.error_for_status()?.json::<SearchResponse>().await?;
        Ok(body)
    }

    pub async fn rolegraph(&self, role: Option<&str>) -> Result<RoleGraphResponseDto> {
        let url = match role {
            Some(r) if !r.is_empty() => {
                format!("{}/rolegraph?role={}", self.base, urlencoding::encode(r))
            }
            _ => format!("{}/rolegraph", self.base),
        };
        let res = self.http.get(url).send().await?;
        let body = res
            .error_for_status()?
            .json::<RoleGraphResponseDto>()
            .await?;
        Ok(body)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchResponse {
    pub status: String,
    pub results: Vec<Document>,
    pub total: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConfigResponse {
    pub status: String,
    pub config: terraphim_config::Config,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GraphNodeDto {
    pub id: u64,
    pub label: String,
    pub rank: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GraphEdgeDto {
    pub source: u64,
    pub target: u64,
    pub rank: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RoleGraphResponseDto {
    pub status: String,
    pub nodes: Vec<GraphNodeDto>,
    pub edges: Vec<GraphEdgeDto>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatRequest {
    pub role: String,
    pub messages: Vec<ChatMessage>,
    pub model: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatResponse {
    pub status: String,
    pub message: Option<String>,
    pub model_used: Option<String>,
    pub error: Option<String>,
}

impl ApiClient {
    pub async fn chat(
        &self,
        role: &str,
        user_message: &str,
        model: Option<&str>,
    ) -> Result<ChatResponse> {
        let url = format!("{}/chat", self.base);
        let req = ChatRequest {
            role: role.to_string(),
            messages: vec![ChatMessage {
                role: "user".into(),
                content: user_message.into(),
            }],
            model: model.map(|s| s.to_string()),
        };
        let res = self.http.post(url).json(&req).send().await?;
        let body = res.error_for_status()?.json::<ChatResponse>().await?;
        Ok(body)
    }
}
