use anyhow::Result;
use base64::Engine;
use chrono::{DateTime, Utc};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SearchResult {
    pub id: String,
    #[serde(rename = "type")]
    pub content_type: String,
    pub title: String,
    pub space: Space,
    pub excerpt: Option<String>,
    #[serde(rename = "_links")]
    pub links: Links,
    #[serde(default)]
    pub version: Option<Version>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Space {
    pub key: String,
    pub name: String,
    #[serde(rename = "_links")]
    pub links: Links,
}

#[derive(Debug, Deserialize)]
pub struct Links {
    #[serde(rename = "webui")]
    pub web_ui: String,
    #[serde(default)]
    pub tinyui: Option<String>,
    #[serde(rename = "self", default)]
    pub self_link: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Page {
    pub id: String,
    pub title: String,
    pub body: Body,
    pub version: Version,
    pub space: Space,
    #[serde(rename = "_links")]
    pub links: Links,
}

#[derive(Debug, Deserialize)]
pub struct Body {
    pub storage: Storage,
}

#[derive(Debug, Deserialize)]
pub struct Storage {
    pub value: String,
}

#[derive(Debug, Deserialize)]
pub struct Version {
    pub number: i32,
    #[serde(rename = "when")]
    pub modified: DateTime<Utc>,
    #[serde(default)]
    pub by: Option<Author>,
    #[serde(rename = "minorEdit", default)]
    pub minor_edit: Option<bool>,
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Comment {
    pub body: Body,
    pub version: Version,
    pub author: Author,
    #[serde(rename = "createdDate", default)]
    pub created_date: Option<DateTime<Utc>>,
    #[serde(rename = "updateDate", default)]
    pub updated_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct Author {
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(default)]
    pub email: Option<String>,
}

pub async fn search(
    base_url: &str,
    username: &str,
    token: &str,
    query: &str,
    limit: u32,
) -> Result<Vec<SearchResult>> {
    let client = reqwest::Client::new();
    let url = format!("{}/wiki/rest/api/content/search", base_url);

    let auth = format!("{}:{}", username, token);
    let auth = format!(
        "Basic {}",
        base64::engine::general_purpose::STANDARD.encode(auth)
    );

    let params = [
        ("cql", query.to_string()),
        ("limit", limit.to_string()),
        (
            "expand",
            "space,version,history,body.view,_links".to_string(),
        ),
    ];

    let response = client
        .get(&url)
        .header("Authorization", auth)
        .query(&params)
        .send()
        .await?;

    let status = response.status();
    let response_text = response.text().await?;
    eprintln!("DEBUG: Response status: {}", status);
    eprintln!("DEBUG: Response body: {}", response_text);

    if !status.is_success() {
        return Err(anyhow::anyhow!(
            "Confluence API error: {} - {}",
            status,
            response_text
        ));
    }

    #[derive(Deserialize)]
    struct SearchResponse {
        results: Vec<SearchResult>,
    }

    let search_response: SearchResponse = serde_json::from_str(&response_text)?;
    Ok(search_response.results)
}

pub async fn get_page(
    base_url: &str,
    username: &str,
    token: &str,
    page_id: &str,
    include_metadata: bool,
) -> Result<Page> {
    let client = reqwest::Client::new();
    let url = format!("{}/wiki/rest/api/content/{}", base_url, page_id);

    let auth = format!("{}:{}", username, token);
    let auth = format!(
        "Basic {}",
        base64::engine::general_purpose::STANDARD.encode(auth)
    );

    let expand = if include_metadata {
        "body.storage,version,space,_links"
    } else {
        "body.storage"
    };

    let params = [("expand", expand)];

    let response = client
        .get(&url)
        .header("Authorization", auth)
        .query(&params)
        .send()
        .await?
        .error_for_status()?;

    let page: Page = response.json().await?;
    Ok(page)
}

pub async fn get_comments(
    base_url: &str,
    username: &str,
    token: &str,
    page_id: &str,
) -> Result<Vec<Comment>> {
    let client = reqwest::Client::new();
    let url = format!(
        "{}/wiki/rest/api/content/{}/child/comment",
        base_url, page_id
    );

    let auth = format!("{}:{}", username, token);
    let auth = format!(
        "Basic {}",
        base64::engine::general_purpose::STANDARD.encode(auth)
    );

    let params = [("expand", "body.storage,version,author"), ("limit", "50")];

    let response = client
        .get(&url)
        .header("Authorization", auth)
        .query(&params)
        .send()
        .await?
        .error_for_status()?;

    #[derive(Deserialize)]
    struct CommentsResponse {
        results: Vec<Comment>,
    }

    let comments_response: CommentsResponse = response.json().await?;
    Ok(comments_response.results)
}
