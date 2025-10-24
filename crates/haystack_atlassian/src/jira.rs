use anyhow::Result;
use base64::Engine;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct Issue {
    pub key: String,
    #[serde(rename = "fields")]
    pub fields: Fields,
}

#[derive(Debug, Deserialize)]
pub struct Fields {
    pub summary: String,
    pub description: Option<String>,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
    pub status: Status,
    pub priority: Option<Priority>,
    pub assignee: Option<User>,
    pub reporter: Option<User>,
    #[serde(rename = "issuetype")]
    pub issue_type: IssueType,
    pub labels: Option<Vec<String>>,
    pub components: Option<Vec<Component>>,
    #[serde(rename = "fixVersions")]
    pub fix_versions: Option<Vec<Version>>,
    #[serde(rename = "duedate")]
    pub due_date: Option<String>,
    pub resolution: Option<Resolution>,
}

#[derive(Debug, Deserialize)]
pub struct IssueType {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct Component {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct Version {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct Resolution {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct Status {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct Priority {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct User {
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "emailAddress")]
    pub email_address: Option<String>,
}

pub async fn get_issue(
    base_url: &str,
    username: &str,
    token: &str,
    issue_key: &str,
    expand: Option<&str>,
) -> Result<Issue> {
    let client = reqwest::Client::new();
    let base_url = base_url.trim_end_matches('/');
    let url = format!("{}/rest/api/2/issue/{}", base_url, issue_key);

    let auth = format!("{}:{}", username, token);
    let auth = format!(
        "Basic {}",
        base64::engine::general_purpose::STANDARD.encode(auth)
    );

    let mut params = vec![("fields", "*all")];
    if let Some(expand) = expand {
        params.push(("expand", expand));
    }

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
            "Jira API error: {} - {}",
            status,
            response_text
        ));
    }

    let issue: Issue = serde_json::from_str(&response_text)?;
    Ok(issue)
}

pub async fn search(
    base_url: &str,
    username: &str,
    token: &str,
    jql: &str,
    fields: &str,
    limit: u32,
) -> Result<Vec<Issue>> {
    let client = reqwest::Client::new();
    let base_url = base_url.trim_end_matches('/');
    let url = format!("{}/rest/api/2/search", base_url);

    let auth = format!("{}:{}", username, token);
    let auth = format!(
        "Basic {}",
        base64::engine::general_purpose::STANDARD.encode(auth)
    );

    #[derive(Serialize, Debug)]
    struct SearchRequest<'a> {
        jql: &'a str,
        fields: Vec<&'a str>,
        #[serde(rename = "maxResults")]
        max_results: u32,
    }

    let fields: Vec<&str> = fields.split(',').collect();
    let search_request = SearchRequest {
        jql,
        fields,
        max_results: limit,
    };

    eprintln!("DEBUG: Request URL: {}", url);
    eprintln!("DEBUG: Request body: {:?}", search_request);

    let response = client
        .post(&url)
        .header("Authorization", auth)
        .json(&search_request)
        .send()
        .await?;

    let status = response.status();
    let response_text = response.text().await?;
    eprintln!("DEBUG: Response status: {}", status);
    eprintln!("DEBUG: Response body: {}", response_text);

    if !status.is_success() {
        return Err(anyhow::anyhow!(
            "Jira API error: {} - {}",
            status,
            response_text
        ));
    }

    #[derive(Deserialize)]
    struct SearchResponse {
        issues: Vec<Issue>,
    }

    let search_response: SearchResponse = serde_json::from_str(&response_text)?;
    Ok(search_response.issues)
}

pub async fn get_project_issues(
    base_url: &str,
    username: &str,
    token: &str,
    project_key: &str,
    limit: u32,
) -> Result<Vec<Issue>> {
    let jql = format!("project = {} ORDER BY created DESC", project_key);
    search(base_url, username, token, &jql, "*all", limit).await
}
