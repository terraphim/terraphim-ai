use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use haystack_core::HaystackProvider;
use terraphim_types::{Document, SearchQuery};

/// Represents a JMAP session.
#[derive(Debug, Serialize, Deserialize)]
pub struct Session {
    /// The primary accounts associated with the session.
    #[serde(rename = "primaryAccounts")]
    pub primary_accounts: HashMap<String, String>,

    /// The URL of the JMAP API.
    #[serde(rename = "apiUrl")]
    pub api_url: String,

    /// The capabilities of the JMAP server.
    pub capabilities: HashMap<String, serde_json::Value>,

    /// The URL for downloading attachments.
    #[serde(rename = "downloadUrl")]
    pub download_url: String,

    /// The URL for uploading attachments.
    #[serde(rename = "uploadUrl")]
    pub upload_url: String,

    /// The current state of the session.
    pub state: String,

    /// The username associated with the session.
    pub username: String,
}

/// Represents a JMAP request.
#[derive(Debug, Serialize, Deserialize)]
struct JMAPRequest {
    /// The set of capabilities being used in the request.
    using: Vec<String>,

    /// The method calls included in the request.
    #[serde(rename = "methodCalls")]
    method_calls: Vec<MethodCall>,
}

/// Represents a JMAP method call.
#[derive(Debug, Serialize, Deserialize)]
struct MethodCall(
    /// The name of the method being called.
    String,
    /// The arguments for the method call.
    HashMap<String, serde_json::Value>,
    /// The client-specified method call ID.
    String,
);

/// Represents an email.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Email {
    /// The ID of the email.
    pub id: String,

    /// The subject of the email.
    #[serde(default)]
    pub subject: Option<String>,

    /// The sender(s) of the email.
    #[serde(default)]
    pub from: Option<Vec<EmailAddress>>,

    /// The recipient(s) of the email.
    #[serde(default)]
    pub to: Option<Vec<EmailAddress>>,

    /// The body values of the email, keyed by part ID.
    #[serde(rename = "bodyValues", default)]
    pub body_values: HashMap<String, BodyValue>,

    /// The text body parts of the email.
    #[serde(rename = "textBody", default)]
    pub text_body: Vec<BodyPart>,

    /// The date and time the email was received.
    #[serde(rename = "receivedAt")]
    pub received_at: Option<String>,
}

/// Represents an email address.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EmailAddress {
    /// The name associated with the email address.
    pub name: Option<String>,

    /// The email address itself.
    pub email: String,
}

/// Represents the value of an email body part.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BodyValue {
    /// The content of the body part.
    pub value: String,

    /// Whether the content is truncated.
    #[serde(rename = "isTruncated")]
    pub is_truncated: Option<bool>,
}

/// Represents an email body part.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BodyPart {
    /// The ID of the body part.
    #[serde(rename = "partId")]
    pub part_id: String,

    /// The content type of the body part.
    #[serde(default)]
    pub type_: Option<String>,
}

/// A client for interacting with a JMAP server.
pub struct JMAPClient {
    /// The JMAP session associated with the client.
    session: Session,

    /// The HTTP client used for making requests.
    client: reqwest::Client,

    /// The access token used for authentication.
    access_token: String,
}

/// Represents a JMAP response.
#[derive(Debug, Serialize, Deserialize, Clone)]
struct JMAPResponse {
    /// The method responses included in the response.
    #[serde(rename = "methodResponses")]
    method_responses: Vec<MethodResponse>,

    /// The updated state string for the session.
    #[serde(rename = "sessionState")]
    session_state: String,
}

/// Represents a JMAP method response.
#[derive(Debug, Serialize, Deserialize, Clone)]
struct MethodResponse(
    /// The name of the method that was called.
    String,
    /// The result of the method call.
    ResponseResult,
    /// The client-specified method call ID.
    String,
);

/// Represents the result of a JMAP method call.
#[derive(Debug, Serialize, Deserialize, Clone)]
struct ResponseResult {
    /// The IDs of the relevant records.
    #[serde(default)]
    ids: Vec<String>,

    /// The list of records returned by the method.
    #[serde(default)]
    list: Vec<Email>,

    /// The total number of records matching the method criteria.
    #[serde(default)]
    total: u32,
}

impl JMAPClient {
    /// Creates a new `JMAPClient` with the given access token and session URL.
    pub async fn new(access_token: String, session_url: &str) -> Result<Self> {
        let client = reqwest::Client::new();

        log::info!("Connecting to JMAP session: {}", session_url);

        let session_response = client
            .get(session_url)
            .header("Authorization", format!("Bearer {}", &access_token))
            .send()
            .await
            .context("Failed to connect to JMAP server")?;

        let status = session_response.status();
        log::debug!("JMAP session status: {}", status);

        let response_text = session_response.text().await?;

        if !status.is_success() {
            return Err(anyhow::anyhow!(
                "Failed to authenticate: {} - {}",
                status,
                response_text
            ));
        }

        log::debug!("JMAP session body length: {} bytes", response_text.len());

        let session: Session =
            serde_json::from_str(&response_text).context("Failed to parse session response")?;

        log::info!("JMAP API URL: {}", session.api_url);

        Ok(Self {
            session,
            client,
            access_token,
        })
    }

    /// Searches for emails matching the given query with configurable result limit.
    pub async fn search_emails(&self, query: &str, limit: u32) -> Result<Vec<Email>> {
        let account_id = self
            .session
            .primary_accounts
            .get("urn:ietf:params:jmap:mail")
            .context("No mail account found in primaryAccounts")?;

        let mut method_params = HashMap::new();
        method_params.insert("accountId".to_string(), serde_json::json!(account_id));
        method_params.insert(
            "filter".to_string(),
            serde_json::json!({
                "text": query
            }),
        );
        method_params.insert("limit".to_string(), serde_json::json!(limit));

        let request = JMAPRequest {
            using: vec![
                "urn:ietf:params:jmap:core".to_string(),
                "urn:ietf:params:jmap:mail".to_string(),
            ],
            method_calls: vec![MethodCall(
                "Email/query".to_string(),
                method_params,
                "s1".to_string(),
            )],
        };

        log::debug!("JMAP search request: {}", serde_json::to_string_pretty(&request)?);

        let response = self
            .client
            .post(&self.session.api_url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to send search request")?;

        let status = response.status();
        log::debug!("JMAP search response status: {}", status);

        let response_text = response.text().await?;

        if !status.is_success() {
            return Err(anyhow::anyhow!(
                "Search request failed: {} - {}",
                status,
                response_text
            ));
        }

        let jmap_response: JMAPResponse =
            serde_json::from_str(&response_text).context("Failed to parse JMAP response")?;

        if let Some(MethodResponse(_, result, _)) = jmap_response.method_responses.first() {
            let email_ids = result.ids.clone();
            self.get_emails(&email_ids).await
        } else {
            Ok(Vec::new())
        }
    }

    /// Fetches the full email data for the given email IDs.
    async fn get_emails(&self, email_ids: &[String]) -> Result<Vec<Email>> {
        if email_ids.is_empty() {
            return Ok(Vec::new());
        }

        let account_id = self
            .session
            .primary_accounts
            .get("urn:ietf:params:jmap:mail")
            .context("No mail account found in primaryAccounts")?;

        let mut method_params = HashMap::new();
        method_params.insert("accountId".to_string(), serde_json::json!(account_id));
        method_params.insert("ids".to_string(), serde_json::json!(email_ids));
        method_params.insert(
            "properties".to_string(),
            serde_json::json!([
                "id",
                "subject",
                "from",
                "to",
                "textBody",
                "bodyValues",
                "receivedAt",
                "bodyStructure",
                "bodyValues",
                "textBody"
            ]),
        );
        method_params.insert("fetchTextBodyValues".to_string(), serde_json::json!(true));

        let request = JMAPRequest {
            using: vec![
                "urn:ietf:params:jmap:core".to_string(),
                "urn:ietf:params:jmap:mail".to_string(),
            ],
            method_calls: vec![MethodCall(
                "Email/get".to_string(),
                method_params,
                "s2".to_string(),
            )],
        };

        let response = self
            .client
            .post(&self.session.api_url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to fetch emails")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!(
                "Email fetch failed: {} - {}",
                status,
                error_text
            ));
        }

        let jmap_response: JMAPResponse = response
            .json()
            .await
            .context("Failed to parse email response")?;

        if let Some(MethodResponse(_, result, _)) = jmap_response.method_responses.first() {
            Ok(result.list.clone())
        } else {
            Ok(Vec::new())
        }
    }
}

/// Converts an Email into a Document with enriched metadata.
pub fn email_to_document(email: &Email) -> Document {
    let sender = email
        .from
        .as_ref()
        .and_then(|addrs| addrs.first())
        .map(|a| a.email.clone())
        .unwrap_or_default();

    let recipient = email
        .to
        .as_ref()
        .and_then(|addrs| addrs.first())
        .map(|a| a.email.clone())
        .unwrap_or_default();

    let description = Some(format!("From: {} To: {}", sender, recipient));

    let body_text = email
        .body_values
        .values()
        .next()
        .map(|bv| bv.value.clone())
        .unwrap_or_default();

    let stub = if body_text.is_empty() {
        None
    } else {
        Some(body_text.chars().take(200).collect::<String>())
    };

    let mut tags = vec!["email".to_string()];
    if !sender.is_empty() {
        tags.push(format!("sender:{}", sender));
    }
    if let Some(ref date) = email.received_at {
        if let Some(date_part) = date.split('T').next() {
            tags.push(date_part.to_string());
        }
    }

    let url = format!("jmap:///email/{}", email.id);

    Document {
        id: email.id.clone(),
        title: email.subject.clone().unwrap_or_default(),
        body: body_text,
        url,
        description,
        stub,
        tags: Some(tags),
        summarization: None,
        rank: None,
        source_haystack: None,
        doc_type: terraphim_types::DocumentType::KgEntry,
        synonyms: None,
        route: None,
        priority: None,
    }
}

impl HaystackProvider for JMAPClient {
    type Error = anyhow::Error;

    async fn search(&self, query: &SearchQuery) -> Result<Vec<Document>, Self::Error> {
        let emails = self.search_emails(&query.search_term.to_string(), 50).await?;
        Ok(emails.iter().map(email_to_document).collect())
    }
}
