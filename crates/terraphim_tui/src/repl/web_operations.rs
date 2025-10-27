//! Web operations module for Terraphim TUI
//!
//! This module provides web-based operations that execute in isolated VM environments,
//! allowing safe web scraping, API interactions, and browser automation without exposing
//! the host system to potential security risks.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Web operation types supported by the VM sandbox
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WebOperationType {
    /// HTTP GET request to fetch web content
    HttpGet {
        url: String,
        headers: Option<HashMap<String, String>>,
    },
    /// HTTP POST request with JSON payload
    HttpPost {
        url: String,
        headers: Option<HashMap<String, String>>,
        body: String,
    },
    /// Web scraping with CSS selector
    WebScrape {
        url: String,
        selector: String,
        wait_for_element: Option<String>,
    },
    /// Screenshot capture of a web page
    Screenshot {
        url: String,
        width: Option<u32>,
        height: Option<u32>,
        full_page: Option<bool>,
    },
    /// PDF generation from web content
    PdfGeneration {
        url: String,
        page_size: Option<String>,
    },
    /// Form submission automation
    FormSubmit {
        url: String,
        form_data: HashMap<String, String>,
    },
    /// API interaction with rate limiting
    ApiInteraction {
        base_url: String,
        endpoints: Vec<String>,
        rate_limit_ms: Option<u64>,
    },
}

/// Web operation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebOperationRequest {
    pub operation: WebOperationType,
    pub vm_id: Option<String>,
    pub timeout_ms: Option<u64>,
    pub user_agent: Option<String>,
    pub proxy: Option<ProxyConfig>,
}

/// Proxy configuration for web operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
}

/// Builder for web operation requests
pub struct WebOperationBuilder {
    operation: WebOperationType,
    vm_id: Option<String>,
    timeout_ms: Option<u64>,
    user_agent: Option<String>,
    proxy: Option<ProxyConfig>,
}

impl WebOperationBuilder {
    pub fn new(operation: WebOperationType) -> Self {
        Self {
            operation,
            vm_id: None,
            timeout_ms: None,
            user_agent: None,
            proxy: None,
        }
    }

    pub fn timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }

    pub fn build(self) -> WebOperationRequest {
        WebOperationRequest {
            operation: self.operation,
            vm_id: self.vm_id,
            timeout_ms: self.timeout_ms,
            user_agent: self.user_agent,
            proxy: self.proxy,
        }
    }
}

impl WebOperationType {
    /// Create HTTP GET operation with custom headers
    pub fn http_get_with_headers(url: impl Into<String>, headers: HashMap<String, String>) -> Self {
        Self::HttpGet {
            url: url.into(),
            headers: Some(headers),
        }
    }

    /// Create HTTP POST operation with custom headers
    pub fn http_post_with_headers(
        url: impl Into<String>,
        headers: HashMap<String, String>,
        body: impl Into<String>,
    ) -> Self {
        Self::HttpPost {
            url: url.into(),
            headers: Some(headers),
            body: body.into(),
        }
    }

    /// Create web scraping operation
    pub fn scrape(url: impl Into<String>, selector: impl Into<String>) -> Self {
        Self::WebScrape {
            url: url.into(),
            selector: selector.into(),
            wait_for_element: None,
        }
    }

    /// Create web scraping operation with wait condition
    pub fn scrape_with_wait(
        url: impl Into<String>,
        selector: impl Into<String>,
        wait_for_element: impl Into<String>,
    ) -> Self {
        Self::WebScrape {
            url: url.into(),
            selector: selector.into(),
            wait_for_element: Some(wait_for_element.into()),
        }
    }

    /// Create screenshot operation
    pub fn screenshot(url: impl Into<String>) -> Self {
        Self::Screenshot {
            url: url.into(),
            width: None,
            height: None,
            full_page: None,
        }
    }

    /// Create screenshot operation with custom dimensions
    pub fn screenshot_with_dimensions(url: impl Into<String>, width: u32, height: u32) -> Self {
        Self::Screenshot {
            url: url.into(),
            width: Some(width),
            height: Some(height),
            full_page: None,
        }
    }

    /// Create full page screenshot
    pub fn full_page_screenshot(url: impl Into<String>) -> Self {
        Self::Screenshot {
            url: url.into(),
            width: None,
            height: None,
            full_page: Some(true),
        }
    }

    /// Create PDF generation operation
    pub fn generate_pdf(url: impl Into<String>) -> Self {
        Self::PdfGeneration {
            url: url.into(),
            page_size: None,
        }
    }

    /// Create PDF generation with custom page size
    pub fn generate_pdf_with_page_size(
        url: impl Into<String>,
        page_size: impl Into<String>,
    ) -> Self {
        Self::PdfGeneration {
            url: url.into(),
            page_size: Some(page_size.into()),
        }
    }

    /// Create form submission operation
    pub fn submit_form(url: impl Into<String>, form_data: HashMap<String, String>) -> Self {
        Self::FormSubmit {
            url: url.into(),
            form_data,
        }
    }

    /// Create API interaction operation
    pub fn api_interaction(base_url: impl Into<String>, endpoints: Vec<impl Into<String>>) -> Self {
        Self::ApiInteraction {
            base_url: base_url.into(),
            endpoints: endpoints.into_iter().map(|e| e.into()).collect(),
            rate_limit_ms: None,
        }
    }

    /// Create API interaction with rate limiting
    pub fn api_interaction_with_rate_limit(
        base_url: impl Into<String>,
        endpoints: Vec<impl Into<String>>,
        rate_limit_ms: u64,
    ) -> Self {
        Self::ApiInteraction {
            base_url: base_url.into(),
            endpoints: endpoints.into_iter().map(|e| e.into()).collect(),
            rate_limit_ms: Some(rate_limit_ms),
        }
    }
}

/// Utility functions for web operations
pub mod utils {
    /// Generate unique operation ID
    pub fn generate_operation_id() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();

        format!("webop-{}", timestamp)
    }
}
