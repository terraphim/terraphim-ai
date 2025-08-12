use crate::indexer::IndexMiddleware;
use crate::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use terraphim_config::Haystack;
use terraphim_types::{Document, Index};

#[derive(Debug, Clone)]
pub struct ClickUpHaystackIndexer {
    client: Client,
}

impl Default for ClickUpHaystackIndexer {
    fn default() -> Self {
        Self {
            client: Client::new(),
        }
    }
}

#[async_trait]
impl IndexMiddleware for ClickUpHaystackIndexer {
    fn index(
        &self,
        needle: &str,
        haystack: &Haystack,
    ) -> impl std::future::Future<Output = Result<Index>> + Send {
        let client = self.client.clone();
        let query = needle.to_string();
        let extras = haystack.get_extra_parameters().clone();
        async move {
            let token = std::env::var("CLICKUP_API_TOKEN").unwrap_or_default();
            if token.is_empty() {
                log::warn!("CLICKUP_API_TOKEN not set; returning empty index");
                return Ok(Index::default());
            }

            // Resolve search scope
            let team_id = extras
                .get("team_id")
                .cloned()
                .or_else(|| std::env::var("CLICKUP_TEAM_ID").ok());
            let list_id = extras.get("list_id").cloned();

            // Optional flags
            let include_closed = parse_bool_param(extras.get("include_closed"), false);
            let subtasks = parse_bool_param(extras.get("subtasks"), true);
            let page = extras
                .get("page")
                .cloned()
                .unwrap_or_else(|| "0".to_string());

            // Prefer universal search if available, otherwise fallback to list-based search via task endpoint
            let mut documents: Vec<Document> = Vec::new();

            if let Some(list) = list_id.clone() {
                if let Ok(mut docs) =
                    search_clickup_list(&client, &token, &list, &query, include_closed, subtasks)
                        .await
                {
                    documents.append(&mut docs);
                }
            } else if let Some(team) = team_id.clone() {
                if let Ok(mut docs) = search_clickup_universal(
                    &client,
                    &token,
                    &team,
                    &query,
                    &page,
                    include_closed,
                    subtasks,
                )
                .await
                {
                    documents.append(&mut docs);
                }
            }

            // Deduplicate by id
            let mut index = Index::new();
            for doc in documents {
                index.insert(doc.id.clone(), doc);
            }
            Ok(index)
        }
    }
}

#[derive(Debug, Deserialize)]
struct UniversalSearchResponse {
    tasks: Option<Vec<serde_json::Value>>,
}

async fn search_clickup_universal(
    client: &Client,
    token: &str,
    team_id: &str,
    query: &str,
    page: &str,
    include_closed: bool,
    subtasks: bool,
) -> Result<Vec<Document>> {
    // GET /api/v2/team/{team_id}/task
    let url = format!("https://api.clickup.com/api/v2/team/{}/task", team_id);
    let params: Vec<(&str, String)> = vec![
        ("page", page.to_string()),
        ("order_by", "relevance".to_string()),
        ("reverse", "false".to_string()),
        ("subtasks", subtasks.to_string()),
        ("include_closed", include_closed.to_string()),
        ("query", query.to_string()),
    ];
    let resp = client
        .get(&url)
        .query(&params)
        .header("Authorization", token)
        .send()
        .await?;

    if !resp.status().is_success() {
        log::warn!("ClickUp search failed: {}", resp.status());
        return Ok(Vec::new());
    }

    // The response for tasks is an object with "tasks": [...]
    let json_value = resp.json::<serde_json::Value>().await?;
    let mut results: Vec<Document> = Vec::new();
    if let Some(tasks) = json_value.get("tasks").and_then(|v| v.as_array()) {
        for t in tasks {
            if let Some(doc) = map_task_value_to_document(t) {
                results.push(doc);
            }
        }
    }

    Ok(results)
}

async fn search_clickup_list(
    client: &Client,
    token: &str,
    list_id: &str,
    query: &str,
    include_closed: bool,
    subtasks: bool,
) -> Result<Vec<Document>> {
    // GET /api/v2/list/{list_id}/task
    let url = format!("https://api.clickup.com/api/v2/list/{}/task", list_id);
    let params: Vec<(&str, String)> = vec![
        ("archived", include_closed.to_string()),
        ("subtasks", subtasks.to_string()),
        ("order_by", "relevance".to_string()),
        ("reverse", "false".to_string()),
        ("search", query.to_string()),
    ];
    let resp = client
        .get(&url)
        .query(&params)
        .header("Authorization", token)
        .send()
        .await?;

    if !resp.status().is_success() {
        log::warn!("ClickUp list search failed: {}", resp.status());
        return Ok(Vec::new());
    }

    let json_value = resp.json::<serde_json::Value>().await?;
    let mut results: Vec<Document> = Vec::new();
    if let Some(tasks) = json_value.get("tasks").and_then(|v| v.as_array()) {
        for t in tasks {
            if let Some(doc) = map_task_value_to_document(t) {
                results.push(doc);
            }
        }
    }

    Ok(results)
}

fn map_task_value_to_document(t: &serde_json::Value) -> Option<Document> {
    let id = t.get("id").and_then(|v| v.as_str())?.to_string();
    let title = t.get("name").and_then(|v| v.as_str())?.to_string();
    let description = t
        .get("text_content")
        .or_else(|| t.get("description"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let url = format!("https://app.clickup.com/t/{}", id);
    Some(Document {
        id: format!("clickup-task-{}", id),
        url,
        title,
        body: description.clone().unwrap_or_default(),
        description,
        stub: None,
        tags: Some(vec!["clickup".to_string(), "task".to_string()]),
        rank: None,
    })
}

fn parse_bool_param(val: Option<&String>, default_value: bool) -> bool {
    val.and_then(|s| s.parse::<bool>().ok())
        .unwrap_or(default_value)
}
