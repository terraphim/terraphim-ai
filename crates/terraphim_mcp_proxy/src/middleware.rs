use async_trait::async_trait;
use std::sync::Arc;
use std::time::Instant;

use crate::{ContentItem, Result, Tool, ToolCallRequest, ToolCallResponse};

#[cfg(feature = "audit")]
use terraphim_persistence::mcp::{McpAuditRecord, McpPersistence};

#[async_trait]
pub trait McpMiddleware: Send + Sync {
    async fn before_tool_call(&self, request: &ToolCallRequest) -> Result<Option<ToolCallRequest>> {
        Ok(Some(request.clone()))
    }

    async fn after_tool_call(
        &self,
        request: &ToolCallRequest,
        response: ToolCallResponse,
    ) -> Result<ToolCallResponse> {
        Ok(response)
    }

    async fn filter_tools(&self, tools: Vec<Tool>) -> Result<Vec<Tool>> {
        Ok(tools)
    }
}

pub struct MiddlewareChain {
    middlewares: Vec<Arc<dyn McpMiddleware>>,
}

impl MiddlewareChain {
    pub fn new() -> Self {
        Self {
            middlewares: Vec::new(),
        }
    }

    pub fn add<M: McpMiddleware + 'static>(mut self, middleware: M) -> Self {
        self.middlewares.push(Arc::new(middleware));
        self
    }

    pub async fn before_tool_call(
        &self,
        mut request: ToolCallRequest,
    ) -> Result<Option<ToolCallRequest>> {
        for middleware in &self.middlewares {
            match middleware.before_tool_call(&request).await? {
                Some(modified_request) => request = modified_request,
                None => return Ok(None),
            }
        }
        Ok(Some(request))
    }

    pub async fn after_tool_call(
        &self,
        request: &ToolCallRequest,
        mut response: ToolCallResponse,
    ) -> Result<ToolCallResponse> {
        for middleware in self.middlewares.iter().rev() {
            response = middleware.after_tool_call(request, response).await?;
        }
        Ok(response)
    }

    pub async fn filter_tools(&self, mut tools: Vec<Tool>) -> Result<Vec<Tool>> {
        for middleware in &self.middlewares {
            tools = middleware.filter_tools(tools).await?;
        }
        Ok(tools)
    }
}

impl Default for MiddlewareChain {
    fn default() -> Self {
        Self::new()
    }
}

pub struct LoggingMiddleware {
    name: String,
}

impl LoggingMiddleware {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

#[async_trait]
impl McpMiddleware for LoggingMiddleware {
    async fn before_tool_call(&self, request: &ToolCallRequest) -> Result<Option<ToolCallRequest>> {
        log::info!(
            "[{}] Tool call starting: {} with {} args",
            self.name,
            request.name,
            request
                .arguments
                .as_ref()
                .map(|a| a.to_string().len())
                .unwrap_or(0)
        );
        Ok(Some(request.clone()))
    }

    async fn after_tool_call(
        &self,
        request: &ToolCallRequest,
        response: ToolCallResponse,
    ) -> Result<ToolCallResponse> {
        let response_size: usize = response
            .content
            .iter()
            .map(|item: &ContentItem| serde_json::to_string(item).map(|s| s.len()).unwrap_or(0))
            .sum();

        log::info!(
            "[{}] Tool call completed: {} -> {} chars response",
            self.name,
            request.name,
            response_size
        );
        Ok(response)
    }
}

pub struct MetricsMiddleware {
    name: String,
}

impl MetricsMiddleware {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

#[async_trait]
impl McpMiddleware for MetricsMiddleware {
    async fn before_tool_call(&self, request: &ToolCallRequest) -> Result<Option<ToolCallRequest>> {
        let start = Instant::now();
        let mut req = request.clone();

        req.arguments = req.arguments.map(|mut args| {
            if let Some(obj) = args.as_object_mut() {
                obj.insert(
                    "_metrics_start".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(
                        start.elapsed().as_millis() as u64,
                    )),
                );
            }
            args
        });

        Ok(Some(req))
    }

    async fn after_tool_call(
        &self,
        request: &ToolCallRequest,
        response: ToolCallResponse,
    ) -> Result<ToolCallResponse> {
        if let Some(args) = &request.arguments {
            if let Some(obj) = args.as_object() {
                if let Some(start_ms) = obj.get("_metrics_start").and_then(|v| v.as_u64()) {
                    let now_ms = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64;
                    let elapsed_ms = now_ms.saturating_sub(start_ms);

                    log::info!(
                        "[{}] Tool {} latency: {}ms",
                        self.name,
                        request.name,
                        elapsed_ms
                    );
                }
            }
        }
        Ok(response)
    }
}

pub struct ToolFilterMiddleware {
    allowed_tools: Vec<String>,
}

impl ToolFilterMiddleware {
    pub fn new(allowed_tools: Vec<String>) -> Self {
        Self { allowed_tools }
    }
}

#[cfg(feature = "audit")]
pub struct AuditMiddleware<P: McpPersistence> {
    persistence: Arc<P>,
    endpoint_uuid: String,
    namespace_uuid: String,
}

#[cfg(feature = "audit")]
impl<P: McpPersistence> AuditMiddleware<P> {
    pub fn new(persistence: Arc<P>, endpoint_uuid: String, namespace_uuid: String) -> Self {
        Self {
            persistence,
            endpoint_uuid,
            namespace_uuid,
        }
    }
}

#[cfg(feature = "audit")]
#[async_trait]
impl<P: McpPersistence> McpMiddleware for AuditMiddleware<P> {
    async fn after_tool_call(
        &self,
        request: &ToolCallRequest,
        response: ToolCallResponse,
    ) -> Result<ToolCallResponse> {
        let start = Instant::now();

        let audit_record = McpAuditRecord {
            uuid: uuid::Uuid::new_v4().to_string(),
            user_id: None,
            endpoint_uuid: self.endpoint_uuid.clone(),
            namespace_uuid: self.namespace_uuid.clone(),
            tool_name: request.name.clone(),
            arguments: request
                .arguments
                .as_ref()
                .and_then(|a| serde_json::to_string(a).ok()),
            response: serde_json::to_string(&response.content).ok(),
            is_error: response.is_error,
            latency_ms: start.elapsed().as_millis() as u64,
            created_at: chrono::Utc::now(),
        };

        if let Err(e) = self.persistence.save_audit(&audit_record).await {
            log::warn!("Failed to save audit record: {}", e);
        }

        Ok(response)
    }
}

#[async_trait]
impl McpMiddleware for ToolFilterMiddleware {
    async fn filter_tools(&self, tools: Vec<Tool>) -> Result<Vec<Tool>> {
        if self.allowed_tools.is_empty() {
            return Ok(tools);
        }

        Ok(tools
            .into_iter()
            .filter(|tool| self.allowed_tools.contains(&tool.name))
            .collect())
    }

    async fn before_tool_call(&self, request: &ToolCallRequest) -> Result<Option<ToolCallRequest>> {
        if !self.allowed_tools.is_empty() && !self.allowed_tools.contains(&request.name) {
            log::warn!(
                "Tool {} is not in allowed list, blocking call",
                request.name
            );
            return Ok(None);
        }
        Ok(Some(request.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_middleware_chain() {
        let chain = MiddlewareChain::new()
            .add(LoggingMiddleware::new("test"))
            .add(MetricsMiddleware::new("test"));

        let request = ToolCallRequest {
            name: "test_tool".to_string(),
            arguments: Some(json!({"key": "value"})),
        };

        let result = chain.before_tool_call(request.clone()).await.unwrap();
        assert!(result.is_some());

        let response = ToolCallResponse {
            content: vec![ContentItem::Text {
                text: "success".to_string(),
            }],
            is_error: false,
        };

        let result = chain.after_tool_call(&request, response).await.unwrap();
        assert_eq!(result.content.len(), 1);
    }

    #[tokio::test]
    async fn test_tool_filter_middleware() {
        let middleware = ToolFilterMiddleware::new(vec!["allowed_tool".to_string()]);

        let tools = vec![
            Tool {
                name: "allowed_tool".to_string(),
                description: "Allowed".to_string(),
                input_schema: Some(json!({})),
            },
            Tool {
                name: "blocked_tool".to_string(),
                description: "Blocked".to_string(),
                input_schema: Some(json!({})),
            },
        ];

        let filtered = middleware.filter_tools(tools).await.unwrap();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "allowed_tool");

        let allowed_request = ToolCallRequest {
            name: "allowed_tool".to_string(),
            arguments: None,
        };
        assert!(middleware
            .before_tool_call(&allowed_request)
            .await
            .unwrap()
            .is_some());

        let blocked_request = ToolCallRequest {
            name: "blocked_tool".to_string(),
            arguments: None,
        };
        assert!(middleware
            .before_tool_call(&blocked_request)
            .await
            .unwrap()
            .is_none());
    }
}
