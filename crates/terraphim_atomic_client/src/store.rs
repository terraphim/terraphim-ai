use crate::{
    error::AtomicError,
    types::{Commit, Resource, Config},
    Result,
};
#[cfg(feature = "native")]
use crate::http;
use std::sync::Arc;
use std::collections::HashMap;
use url::Url;
use serde::{Serialize, Deserialize};
#[cfg(not(feature = "native"))]
use wasm_bindgen::prelude::*;
use serde_json::Value;

/// The main entry point for interacting with an Atomic Server.
///
/// `Store` provides methods for fetching, creating, updating, and deleting resources.
/// It uses the provided `Config` for server URL and authentication.
#[derive(Clone)]
pub struct Store {
    /// Configuration for the store, including server URL and optional agent for authentication
    pub config: Config,
    #[cfg(feature = "native")]
    client: reqwest::Client,
    #[cfg(not(feature = "native"))]
    _marker: std::marker::PhantomData<u8>,
}

impl Store {
    /// Creates a new `Store` with the given configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration including server URL and optional agent for authentication
    ///
    /// # Returns
    ///
    /// A new `Store` instance or an error if initialization fails
    pub fn new(config: Config) -> Result<Self> {
        #[cfg(feature = "native")]
        {
            let client = reqwest::Client::new();
            Ok(Self { config, client })
        }

        #[cfg(not(feature = "native"))]
        {
            Ok(Self { config, _marker: std::marker::PhantomData })
        }
    }

    /// Gets a resource from the server.
    ///
    /// # Arguments
    ///
    /// * `subject` - The subject URL of the resource to get
    ///
    /// # Returns
    ///
    /// A Result containing the resource or an error if retrieval fails
    #[cfg(feature = "native")]
    pub async fn get_resource(&self, subject: &str) -> Result<Resource> {
        use reqwest::header::ACCEPT;
        let mut request = self.client.get(subject).header(ACCEPT, "application/ad+json");
        if let Some(agent) = &self.config.agent {
            let auth_headers = crate::auth::get_authentication_headers(agent, subject, "GET")?;
            for (k, v) in auth_headers.iter() {
                request = request.header(k.as_str(), v.to_str().map_err(|e| AtomicError::Parse(e.to_string()))?);
            }
        }
        let resp = request.send().await?;
        if !resp.status().is_success() {
            return Err(AtomicError::Api(format!("Failed to get resource: {} {}", resp.status(), resp.text().await?)));
        }
        Ok(resp.json::<Resource>().await?)
    }

    #[cfg(not(feature = "native"))]
    pub async fn get_resource(&self, subject: &str) -> Result<Resource> {
        let val = crate::http::wasm::get_resource(subject, &self.config).await?;
        let res: Resource = serde_json::from_value(val)?;
        Ok(res)
    }

    /// Creates a new resource on the server.
    ///
    /// # Arguments
    ///
    /// * `resource` - The resource to create
    ///
    /// # Returns
    ///
    /// A Result containing the created resource or an error if creation fails
    #[cfg(feature = "native")]
    pub async fn create_resource(&self, resource: Resource) -> Result<Resource> {
        // According to the OpenAPI spec, we should POST directly to the resource URL
        let url = resource.subject.clone();
        
        let mut request = self.client.post(&url)
            .json(&resource)
            .header("Content-Type", "application/json")
            .header("Accept", "application/ad+json");
        
        // Add authentication headers if an agent is available
        if let Some(agent) = &self.config.agent {
            let auth_headers = crate::auth::get_authentication_headers(agent, &resource.subject, "POST")?;
            for (key, value) in auth_headers.iter() {
                request = request.header(key.as_str(), value.to_str().map_err(|e| AtomicError::Parse(e.to_string()))?);
            }
        }
        
        let response = request.send().await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await?;
            return Err(AtomicError::Api(format!(
                "Failed to create resource: {} {}",
                status, text
            )));
        }
        
        let created_resource: Resource = response.json().await?;
        Ok(created_resource)
    }

    /// Updates a resource on the server.
    ///
    /// # Arguments
    ///
    /// * `resource` - The resource to update
    ///
    /// # Returns
    ///
    /// A Result containing the updated resource or an error if update fails
    #[cfg(feature = "native")]
    pub async fn update_resource(&self, resource: Resource) -> Result<Resource> {
        // According to the OpenAPI spec, we should POST directly to the resource URL
        let url = resource.subject.clone();
        
        let mut request = self.client.post(&url)
            .json(&resource)
            .header("Content-Type", "application/json")
            .header("Accept", "application/ad+json");
        
        // Add authentication headers if an agent is available
        if let Some(agent) = &self.config.agent {
            let auth_headers = crate::auth::get_authentication_headers(agent, &resource.subject, "POST")?;
            for (key, value) in auth_headers.iter() {
                request = request.header(key.as_str(), value.to_str().map_err(|e| AtomicError::Parse(e.to_string()))?);
            }
        }
        
        let response = request.send().await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await?;
            return Err(AtomicError::Api(format!(
                "Failed to update resource: {} {}",
                status, text
            )));
        }
        
        let updated_resource: Resource = response.json().await?;
        Ok(updated_resource)
    }

    /// Deletes a resource from the server.
    ///
    /// # Arguments
    ///
    /// * `subject` - The subject URL of the resource to delete
    ///
    /// # Returns
    ///
    /// A Result containing () or an error if deletion fails
    #[cfg(feature = "native")]
    pub async fn delete_resource(&self, subject: &str) -> Result<()> {
        // According to the OpenAPI spec, we DELETE directly from the resource URL
        let mut request = self.client.delete(subject);
        
        // Add authentication headers if an agent is available
        if let Some(agent) = &self.config.agent {
            let auth_headers = crate::auth::get_authentication_headers(agent, subject, "DELETE")?;
            for (key, value) in auth_headers.iter() {
                request = request.header(key.as_str(), value.to_str().map_err(|e| AtomicError::Parse(e.to_string()))?);
            }
        }
        
        let response = request.send().await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await?;
            return Err(AtomicError::Api(format!(
                "Failed to delete resource: {} {}",
                status, text
            )));
        }
        
        Ok(())
    }

    /// Sends a commit to the server.
    ///
    /// # Arguments
    ///
    /// * `commit` - The commit to send
    ///
    /// # Returns
    ///
    /// A Result containing the response as JSON or an error if the commit fails
    #[cfg(feature = "native")]
    pub async fn send_commit(&self, commit: Commit) -> Result<Value> {
        // The commit endpoint is at /commit
        // Ensure proper URL formatting by removing trailing slashes
        let server_url = self.config.server_url.trim_end_matches('/');
        let url = format!("{}/commit", server_url);
        
        let mut request = self.client.post(&url)
            .json(&commit)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json");
        
        // Add authentication headers if an agent is available
        if let Some(agent) = &self.config.agent {
            let auth_headers = crate::auth::get_authentication_headers(agent, &url, "POST")?;
            for (key, value) in auth_headers.iter() {
                request = request.header(key.as_str(), value.to_str().map_err(|e| AtomicError::Parse(e.to_string()))?);
            }
        }
        
        let response = request.send().await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await?;
            return Err(AtomicError::Api(format!(
                "Failed to send commit: {} {}",
                status, text
            )));
        }
        
        let result = response.json::<Value>().await?;
        Ok(result)
    }

    #[cfg(not(feature = "native"))]
    pub async fn send_commit(&self, commit: Commit) -> Result<Value> {
        crate::http::wasm::send_commit(&commit, &self.config).await?;
        Ok(Value::Null)
    }

    /// Creates a resource using a commit.
    ///
    /// # Arguments
    ///
    /// * `subject` - The subject URL of the resource to create
    /// * `properties` - The properties of the resource
    ///
    /// # Returns
    ///
    /// A Result containing the response as JSON or an error if the creation fails
    pub async fn create_with_commit(&self, subject: &str, properties: HashMap<String, Value>) -> Result<Value> {
        // Ensure we have an agent
        let agent = self.config.agent.as_ref()
            .ok_or_else(|| AtomicError::Authentication("No agent configured for authentication".to_string()))?;
        
        // Create a commit
        let commit = Commit::new_create_or_update(subject.to_string(), properties, agent)?
            .sign(agent)?;
        
        // Send the commit
        self.send_commit(commit).await
    }

    /// Updates a resource using a commit.
    ///
    /// # Arguments
    ///
    /// * `subject` - The subject URL of the resource to update
    /// * `properties` - The properties to update
    ///
    /// # Returns
    ///
    /// A Result containing the response as JSON or an error if the update fails
    pub async fn update_with_commit(&self, subject: &str, properties: HashMap<String, Value>) -> Result<Value> {
        // Ensure we have an agent
        let agent = self.config.agent.as_ref()
            .ok_or_else(|| AtomicError::Authentication("No agent configured for authentication".to_string()))?;
        
        // Create a commit
        let commit = Commit::new_create_or_update(subject.to_string(), properties, agent)?
            .sign(agent)?;
        
        // Send the commit
        self.send_commit(commit).await
    }

    /// Deletes a resource using a commit.
    ///
    /// # Arguments
    ///
    /// * `subject` - The subject URL of the resource to delete
    ///
    /// # Returns
    ///
    /// A Result containing the response as JSON or an error if the deletion fails
    pub async fn delete_with_commit(&self, subject: &str) -> Result<Value> {
        // Ensure we have an agent
        let agent = self.config.agent.as_ref()
            .ok_or_else(|| AtomicError::Authentication("No agent configured for authentication".to_string()))?;
        
        // Create a commit
        let commit = Commit::new_delete(subject.to_string(), agent)?
            .sign(agent)?;
        
        // Send the commit
        self.send_commit(commit).await
    }

    /// Searches for resources matching the given query.
    ///
    /// # Arguments
    ///
    /// * `query` - The search query
    ///
    /// # Returns
    ///
    /// A Result containing the search results or an error if the search fails
    #[cfg(feature = "native")]
    pub async fn search(&self, query: &str) -> Result<Value> {
        // According to the OpenAPI spec, search is at /search
        let server_url = self.config.server_url.trim_end_matches('/');
        let url = format!("{}/search", server_url);
        
        let mut request = self.client.get(&url)
            .query(&[("q", query)])
            .header("Accept", "application/json");
        
        let response = request.send().await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await?;
            return Err(AtomicError::Api(format!(
                "Failed to search: {} {}",
                status, text
            )));
        }
        
        let results = response.json::<Value>().await?;
        Ok(results)
    }

    #[cfg(not(feature = "native"))]
    pub async fn search(&self, query: &str) -> Result<Value> {
        let server_url = self.config.server_url.trim_end_matches('/');
        let url = format!("{}/search?q={}", server_url, query);
        let val = crate::http::wasm::get_resource(&url, &self.config).await?;
        Ok(val)
    }

    /// Queries resources using a collection.
    ///
    /// # Arguments
    ///
    /// * `collection_url` - The URL of the collection to query
    /// * `params` - The query parameters
    ///
    /// # Returns
    ///
    /// A Result containing the query results or an error if the query fails
    #[cfg(feature = "native")]
    pub async fn query(&self, collection_url: &str, params: &[(&str, &str)]) -> Result<Value> {
        // According to the OpenAPI spec, collections accept POST requests with query parameters
        let mut request = self.client.post(collection_url)
            .query(params)
            .header("Accept", "application/json");
        
        // Add authentication headers if an agent is available
        if let Some(agent) = &self.config.agent {
            let auth_headers = crate::auth::get_authentication_headers(agent, collection_url, "POST")?;
            for (key, value) in auth_headers.iter() {
                request = request.header(key.as_str(), value.to_str().map_err(|e| AtomicError::Parse(e.to_string()))?);
            }
        }
        
        let response = request.send().await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await?;
            return Err(AtomicError::Api(format!(
                "Failed to query collection: {} {}",
                status, text
            )));
        }
        
        let results = response.json::<Value>().await?;
        Ok(results)
    }

    /// Convenience helper that queries the root `/collections` endpoint for
    /// all resources having `isA = class_url`, sorted by `sort_property_url`.
    ///
    /// * `class_url` – full URL of the class to filter on.
    /// * `sort_property_url` – property used for sorting.
    /// * `desc` – descending if true, ascending otherwise.
    /// * `page_size` – optional maximum number of results.
    #[cfg(feature = "native")]
    pub async fn collection_by_class(
        &self,
        class_url: &str,
        sort_property_url: &str,
        desc: bool,
        page_size: Option<u32>,
    ) -> Result<Value> {
        let server_url = self.config.server_url.trim_end_matches('/');
        let collection_url = format!("{}/collections", server_url);

        // build params in Vec<(String,String)> then slice convert
        let mut params: Vec<(String, String)> = Vec::new();
        params.push((
            format!("property_value[{}]", "https://atomicdata.dev/properties/isA"),
            class_url.to_string(),
        ));
        params.push(("include".into(), "true".into()));
        params.push(("sort_by".into(), sort_property_url.to_string()));
        params.push(("sort_desc".into(), desc.to_string()));
        if let Some(size) = page_size {
            params.push(("page_size".into(), size.to_string()));
        }

        let param_refs: Vec<(&str, &str)> =
            params.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();

        self.query(&collection_url, &param_refs).await
    }

    /// Recursively gathers all resources that belong to a given ontology.
    ///
    /// The algorithm mimics the behaviour of the Deno `extract-ontology` tool:
    /// 1. Start at `ontology_subject`.
    /// 2. Follow the `classes`, `properties`, and `instances` arrays found on the
    ///    ontology resource, converting relative subjects (e.g. `defaultOntology/class/card`)
    ///    into absolute URLs belonging to the same server.
    /// 3. Perform a breadth-first crawl, following all AtomicURL links that still
    ///    point to the same server, until no new subjects are found.
    ///
    /// Returns **all** fetched resources in arbitrary order. Errors in fetching
    /// individual resources are logged and skipped so that a partially broken
    /// ontology still yields the remaining valid items.
    pub async fn gather_ontology_resources(&self, ontology_subject: &str) -> Result<Vec<Resource>> {
        use std::collections::{HashSet, VecDeque};

        // Helper: convert a possibly relative path to an absolute URL for this server.
        fn absolutize(server_prefix: &str, val: &str) -> String {
            if val.starts_with("http://") || val.starts_with("https://") {
                val.to_string()
            } else {
                // Ensure no leading slashes duplicate
                let trimmed = val.trim_start_matches('/');
                format!("{}/{}", server_prefix, trimmed)
            }
        }

        let server_prefix = self.config.server_url.trim_end_matches('/');
        // Anything under this absolute prefix belongs to the ontology we want to export.
        // e.g. ontology_subject = http://server/defaultOntology  ⇒ ontology_scope_prefix = http://server/defaultOntology/
        let ontology_scope_prefix = {
            let mut p = ontology_subject.trim_end_matches('/').to_string();
            p.push('/');
            p
        };

        let mut visited: HashSet<String> = HashSet::new();
        let mut queue: VecDeque<String> = VecDeque::new();
        let mut collected: Vec<Resource> = Vec::new();

        queue.push_back(ontology_subject.to_string());

        while let Some(subject) = queue.pop_front() {
            // Skip subjects that don't belong to this server (defence-in-depth).
            if !subject.starts_with(server_prefix) {
                continue;
            }

            // Skip resources that are *outside* the ontology scope (unless it is the root itself).
            if subject != ontology_subject && !subject.starts_with(&ontology_scope_prefix) {
                // We *do* still want to follow the link values inside ontology resources (so that
                // references keep pointing to the external resources) – we just don't include
                // such external resources in the export to avoid overwrite errors on import.
                continue;
            }

            if visited.contains(&subject) {
                continue;
            }
            visited.insert(subject.clone());

            let res = match self.get_resource(&subject).await {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("Warning: failed to fetch {}: {}", subject, e);
                    continue;
                }
            };

            // If this is the ontology root, enqueue its classes / properties / instances
            if subject == ontology_subject {
                let enqueue_array = |key: &str, properties: &std::collections::HashMap<String, serde_json::Value>, queue: &mut VecDeque<String>| {
                    if let Some(arr) = properties.get(key).and_then(|v| v.as_array()) {
                        for val in arr {
                            // Strings are either absolute or relative to server root
                            if let Some(s) = val.as_str() {
                                queue.push_back(absolutize(server_prefix, s));
                                continue;
                            }
                            // Objects with @id
                            if let Some(s) = val.get("@id").and_then(|v| v.as_str()) {
                                queue.push_back(absolutize(server_prefix, s));
                            }
                        }
                    }
                };
                enqueue_array("https://atomicdata.dev/properties/classes", &res.properties, &mut queue);
                enqueue_array("https://atomicdata.dev/properties/properties", &res.properties, &mut queue);
                enqueue_array("https://atomicdata.dev/properties/instances", &res.properties, &mut queue);
            }

            // Add resource to results now, after potential enqueue
            collected.push(res.clone());

            // Local helper to recursively extract AtomicURL links from JSON values.
            fn collect_links_local(queue: &mut VecDeque<String>, value: &serde_json::Value, server_prefix: &str) {
                if let Some(arr) = value.as_array() {
                    for val in arr {
                        collect_links_local(queue, val, server_prefix);
                    }
                } else if let Some(obj) = value.as_object() {
                    if let Some(id_val) = obj.get("@id").and_then(|v| v.as_str()) {
                        if id_val.starts_with(server_prefix) {
                            queue.push_back(id_val.to_string());
                        }
                    }
                    for (_k, v) in obj {
                        collect_links_local(queue, v, server_prefix);
                    }
                } else if let Some(str_val) = value.as_str() {
                    if str_val.starts_with(server_prefix) {
                        queue.push_back(str_val.to_string());
                    }
                }
            }

            // Walk over all property values and enqueue in-server AtomicURLs
            for (_prop, value) in &res.properties {
                collect_links_local(&mut queue, value, server_prefix);
            }
        }

        Ok(collected)
    }
}

/// Options for search queries.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SearchOpts {
    /// Whether to include full resources in the response
    pub include: Option<bool>,
    /// Maximum number of results to return
    pub limit: Option<u32>,
    /// Only include resources that have one of these resources as its ancestor
    pub parents: Option<Vec<String>>,
    /// Filter based on props, using tantivy QueryParser syntax
    pub filters: Option<HashMap<String, String>>,
}

/// Options for collection queries.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct QueryOpts {
    /// Whether to include full resources in the response
    pub include: Option<bool>,
    /// Maximum number of results per page
    pub page_size: Option<u32>,
    /// Page number (0-indexed)
    pub page: Option<u32>,
    /// Property to sort by
    pub sort_by: Option<String>,
    /// Sort direction (true for ascending, false for descending)
    pub sort_desc: Option<bool>,
    /// Property-value filters
    pub property_values: Option<HashMap<String, String>>,
}

/// Builds a search URL with the given parameters.
fn build_search_url(server_url: &str, query: &str, opts: &SearchOpts) -> Result<String> {
    let mut url = Url::parse(server_url).map_err(|e| AtomicError::Parse(e.to_string()))?;
    url.set_path("search");
    
    let mut query_pairs = url.query_pairs_mut();
    query_pairs.append_pair("q", query);
    
    if let Some(include) = opts.include {
        query_pairs.append_pair("include", &include.to_string());
    }
    
    if let Some(limit) = opts.limit {
        query_pairs.append_pair("limit", &limit.to_string());
    }
    
    if let Some(parents) = &opts.parents {
        if !parents.is_empty() {
            let parents_string = parents.join(",");
            query_pairs.append_pair("parents", &parents_string);
        }
    }
    
    if let Some(filters) = &opts.filters {
        if !filters.is_empty() {
            let filter_string = build_filter_string(filters);
            query_pairs.append_pair("filters", &filter_string);
        }
    }
    
    drop(query_pairs);
    Ok(url.to_string())
}

/// Builds a query URL for a collection with the given parameters.
fn build_query_url(collection_subject: &str, opts: &QueryOpts) -> Result<String> {
    let mut url = Url::parse(collection_subject).map_err(|e| AtomicError::Parse(e.to_string()))?;
    
    let mut query_pairs = url.query_pairs_mut();
    
    if let Some(include) = opts.include {
        query_pairs.append_pair("include", &include.to_string());
    }
    
    if let Some(page_size) = opts.page_size {
        query_pairs.append_pair("page_size", &page_size.to_string());
    }
    
    if let Some(page) = opts.page {
        query_pairs.append_pair("page", &page.to_string());
    }
    
    if let Some(sort_by) = &opts.sort_by {
        query_pairs.append_pair("sort_by", sort_by);
    }
    
    if let Some(sort_desc) = opts.sort_desc {
        query_pairs.append_pair("sort_desc", &sort_desc.to_string());
    }
    
    if let Some(property_values) = &opts.property_values {
        for (property, value) in property_values {
            query_pairs.append_pair(&format!("property_value[{}]", property), value);
        }
    }
    
    drop(query_pairs);
    Ok(url.to_string())
}

/// Builds a filter string for the URL.
fn build_filter_string(filters: &HashMap<String, String>) -> String {
    filters
        .iter()
        .filter_map(|(key, value)| {
            if !value.is_empty() {
                Some(format!("{}:\"{}\"", escape_tantivy_key(key), value))
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join(" AND ")
}

/// Escapes special characters for Tantivy query syntax.
fn escape_tantivy_key(key: &str) -> String {
    const SPECIAL_CHARS: &[char] = &[
        '+', '^', '`', ':', '{', '}', '"', '[', ']', '(', ')', '!', '\\', '*', ' ', '.',
    ];
    
    key.chars()
        .map(|c| {
            if SPECIAL_CHARS.contains(&c) {
                format!("\\{}", c)
            } else {
                c.to_string()
            }
        })
        .collect()
} 