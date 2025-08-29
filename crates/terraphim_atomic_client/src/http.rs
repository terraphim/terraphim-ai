//! HTTP client implementation for Atomic Server.
//!
//! This module provides HTTP client implementations for both native and WASM targets.

#[cfg(feature = "native")]
pub mod native {
    use crate::{
        auth::get_authentication_headers,
        error::AtomicError,
        types::{Commit, Config},
        Result,
    };
    use reqwest::Client;
    use serde_json::Value;

    /// Gets a resource from the server.
    ///
    /// # Arguments
    ///
    /// * `subject` - The subject URL of the resource to get
    /// * `config` - The configuration for the client
    ///
    /// # Returns
    ///
    /// A Result containing the resource as a JSON value or an error if retrieval fails
    pub async fn get_resource(subject: &str, config: &Config) -> Result<Value> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("Terraphim-Atomic-Client/1.0")
            .build()
            .unwrap_or_else(|_| Client::new());
        let mut request = client.get(subject).header("Accept", "application/json");

        // Add authentication headers if an agent is available
        if let Some(agent) = &config.agent {
            let auth_headers = get_authentication_headers(agent, subject, "GET")?;
            for (key, value) in auth_headers.iter() {
                request = request.header(
                    key.as_str(),
                    value.to_str().map_err(AtomicError::HeaderToStr)?,
                );
            }
        }

        let resp = request.send().await?;

        if !resp.status().is_success() {
            return Err(AtomicError::Api(format!(
                "Failed to get resource: {} {}",
                resp.status(),
                resp.text().await.unwrap_or_default()
            )));
        }

        let json = resp.json::<Value>().await?;
        Ok(json)
    }

    /// Sends a commit to the server.
    ///
    /// # Arguments
    ///
    /// * `commit` - The commit to send
    /// * `config` - The configuration for the client
    ///
    /// # Returns
    ///
    /// A Result containing () or an error if the commit fails
    pub async fn send_commit(commit: &Commit, config: &Config) -> Result<()> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("Terraphim-Atomic-Client/1.0")
            .build()
            .unwrap_or_else(|_| Client::new());
        let url = format!("{}/commit", config.server_url);
        let request = client
            .post(&url)
            .json(commit)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json");

        let resp = request.send().await?;

        if !resp.status().is_success() {
            return Err(AtomicError::Api(format!(
                "Failed to send commit: {} {}",
                resp.status(),
                resp.text().await.unwrap_or_default()
            )));
        }

        Ok(())
    }
}

#[cfg(feature = "wasm")]
pub mod wasm {
    use crate::{
        auth::get_authentication_headers,
        error::AtomicError,
        types::{Commit, Config},
        Result,
    };
    use serde_json::Value;
    use wasm_bindgen::{JsCast, JsValue};
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    /// Maximum size for JSON responses (10MB)
    const MAX_RESPONSE_SIZE: usize = 10 * 1024 * 1024;
    /// Maximum size for commit data (5MB)
    const MAX_COMMIT_SIZE: usize = 5 * 1024 * 1024;

    /// Gets a resource from the server.
    ///
    /// # Arguments
    ///
    /// * `subject` - The subject URL of the resource to get
    /// * `config` - The configuration for the client
    ///
    /// # Returns
    ///
    /// A Result containing the resource as a JSON value or an error if retrieval fails
    pub async fn get_resource(subject: &str, config: &Config) -> Result<Value> {
        let mut opts = RequestInit::new();
        opts.method("GET");
        opts.mode(RequestMode::Cors);

        let mut headers = web_sys::Headers::new()
            .map_err(|e| AtomicError::Api(format!("Failed to create headers: {:?}", e)))?;
        headers
            .append("Accept", "application/ad+json")
            .map_err(|e| AtomicError::Api(format!("Failed to append header: {:?}", e)))?;

        // Add authentication headers if an agent is available
        if let Some(agent) = &config.agent {
            let auth_headers = get_authentication_headers(agent, subject, "GET")?;
            for (key, value) in auth_headers.iter() {
                headers
                    .append(key, value)
                    .map_err(|e| AtomicError::Api(format!("Failed to append header: {:?}", e)))?;
            }
        }

        opts.headers(&headers);

        let request = Request::new_with_str_and_init(subject, &opts)
            .map_err(|e| AtomicError::Api(format!("Failed to create request: {:?}", e)))?;
        let window =
            web_sys::window().ok_or_else(|| AtomicError::Api("No window found".to_string()))?;
        let resp_value = JsFuture::from(window.fetch_with_request(&request))
            .await
            .map_err(|e| AtomicError::Api(format!("Failed to fetch: {:?}", e)))?;
        let resp: Response = resp_value
            .dyn_into()
            .map_err(|_| AtomicError::Api("Failed to convert response".to_string()))?;

        if !resp.ok() {
            return Err(AtomicError::Api(format!(
                "Failed to get resource: {}",
                resp.status()
            )));
        }

        // Check content length if available
        if let Some(content_length) = resp.headers().get("content-length").ok().flatten() {
            if let Ok(size) = content_length.parse::<usize>() {
                if size > MAX_RESPONSE_SIZE {
                    return Err(AtomicError::Api(format!(
                        "Response too large: {} bytes (max {})", 
                        size, MAX_RESPONSE_SIZE
                    )));
                }
            }
        }

        // Get the response as text first to check size
        let text_promise = resp.text()
            .map_err(|e| AtomicError::Api(format!("Failed to get response text: {:?}", e)))?;
        let text_js = JsFuture::from(text_promise)
            .await
            .map_err(|e| AtomicError::Api(format!("Failed to get response text: {:?}", e)))?;
        let text = text_js.as_string()
            .ok_or_else(|| AtomicError::Api("Response is not a string".to_string()))?;
        
        // Check text size before parsing
        if text.len() > MAX_RESPONSE_SIZE {
            return Err(AtomicError::Api(format!(
                "Response too large: {} bytes (max {})", 
                text.len(), MAX_RESPONSE_SIZE
            )));
        }
        
        // Parse JSON from the text
        let value: Value = serde_json::from_str(&text)
            .map_err(|e| AtomicError::Api(format!("Failed to parse JSON: {}", e)))?;
        Ok(value)
    }

    /// Sends a commit to the server.
    ///
    /// # Arguments
    ///
    /// * `commit` - The commit to send
    /// * `config` - The configuration for the client
    ///
    /// # Returns
    ///
    /// A Result containing () or an error if the commit fails
    pub async fn send_commit(commit: &Commit, config: &Config) -> Result<()> {
        let mut opts = RequestInit::new();
        opts.method("POST");
        opts.mode(RequestMode::Cors);

        let url = format!("{}/commit", config.server_url);

        let commit_json_str = serde_json::to_string(commit)?;
        
        // Check commit size before sending
        if commit_json_str.len() > MAX_COMMIT_SIZE {
            return Err(AtomicError::Api(format!(
                "Commit too large: {} bytes (max {})", 
                commit_json_str.len(), MAX_COMMIT_SIZE
            )));
        }
        
        let commit_json = JsValue::from_str(&commit_json_str);
        opts.body(Some(&commit_json));

        let mut headers = web_sys::Headers::new()
            .map_err(|e| AtomicError::Api(format!("Failed to create headers: {:?}", e)))?;
        headers
            .append("Content-Type", "application/json")
            .map_err(|e| AtomicError::Api(format!("Failed to append header: {:?}", e)))?;
        headers
            .append("Accept", "application/ad+json")
            .map_err(|e| AtomicError::Api(format!("Failed to append header: {:?}", e)))?;
        opts.headers(&headers);

        let request = Request::new_with_str_and_init(&url, &opts)
            .map_err(|e| AtomicError::Api(format!("Failed to create request: {:?}", e)))?;
        let window =
            web_sys::window().ok_or_else(|| AtomicError::Api("No window found".to_string()))?;
        let resp_value = JsFuture::from(window.fetch_with_request(&request))
            .await
            .map_err(|e| AtomicError::Api(format!("Failed to fetch: {:?}", e)))?;
        let resp: Response = resp_value
            .dyn_into()
            .map_err(|_| AtomicError::Api("Failed to convert response".to_string()))?;

        if !resp.ok() {
            return Err(AtomicError::Api(format!(
                "Failed to send commit: {}",
                resp.status()
            )));
        }

        Ok(())
    }
}
