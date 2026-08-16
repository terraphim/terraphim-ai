//! Home Assistant integration (REST API).
//!
//! Port of Hermes `tools/homeassistant_tool.py`. Four tools over the HA REST
//! API: `ha_list_entities`, `ha_get_state`, `ha_list_services`,
//! `ha_call_service`. Auth via a long-lived access token (`Bearer`).

use crate::config::HomeAssistantConfig;
use crate::tools::{Tool, ToolError};
use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;

/// HTTP client for the Home Assistant REST API.
pub struct HomeAssistantClient {
    http: reqwest::Client,
    url: String,
    token: String,
}

impl HomeAssistantClient {
    pub fn new(url: impl Into<String>, token: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .unwrap_or_default();
        Self {
            http: client,
            url: url.into().trim_end_matches('/').to_string(),
            token: token.into(),
        }
    }

    fn headers(&self) -> reqwest::header::HeaderMap {
        let mut h = reqwest::header::HeaderMap::new();
        let bearer = format!("Bearer {}", self.token);
        if let Ok(v) = reqwest::header::HeaderValue::from_str(&bearer) {
            h.insert(reqwest::header::AUTHORIZATION, v);
        }
        h.insert(
            reqwest::header::CONTENT_TYPE,
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        h
    }

    /// List entity states, optionally filtered by domain/area.
    pub async fn list_entities(
        &self,
        domain: Option<&str>,
        area: Option<&str>,
    ) -> Result<serde_json::Value, String> {
        let url = format!("{}/api/states", self.url);
        let resp = self
            .http
            .get(&url)
            .headers(self.headers())
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("HA list_entities failed: HTTP {}", resp.status()));
        }
        let states: Vec<serde_json::Value> = resp.json().await.map_err(|e| e.to_string())?;

        let filtered: Vec<serde_json::Value> = states
            .into_iter()
            .filter(|s| {
                if let Some(d) = domain {
                    let id = s.get("entity_id").and_then(|v| v.as_str()).unwrap_or("");
                    if !id.starts_with(&format!("{d}.")) {
                        return false;
                    }
                }
                if let Some(a) = area {
                    let al = a.to_lowercase();
                    let attrs = s.get("attributes").unwrap_or(&serde_json::Value::Null);
                    let friendly = attrs
                        .get("friendly_name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_lowercase();
                    let ar = attrs
                        .get("area")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_lowercase();
                    if !friendly.contains(&al) && !ar.contains(&al) {
                        return false;
                    }
                }
                true
            })
            .map(|s| {
                let friendly = s
                    .get("attributes")
                    .and_then(|a| a.get("friendly_name"))
                    .cloned()
                    .unwrap_or(serde_json::Value::String(String::new()));
                serde_json::json!({
                    "entity_id": s.get("entity_id").cloned().unwrap_or(serde_json::Value::Null),
                    "state": s.get("state").cloned().unwrap_or(serde_json::Value::Null),
                    "friendly_name": friendly,
                })
            })
            .collect();

        Ok(serde_json::json!({
            "count": filtered.len(),
            "entities": filtered,
        }))
    }

    /// Fetch detailed state of a single entity.
    pub async fn get_state(&self, entity_id: &str) -> Result<serde_json::Value, String> {
        let url = format!("{}/api/states/{}", self.url, entity_id);
        let resp = self
            .http
            .get(&url)
            .headers(self.headers())
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("HA get_state failed: HTTP {}", resp.status()));
        }
        let data: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        Ok(serde_json::json!({
            "entity_id": data.get("entity_id").cloned().unwrap_or(serde_json::Value::Null),
            "state": data.get("state").cloned().unwrap_or(serde_json::Value::Null),
            "attributes": data.get("attributes").cloned().unwrap_or(serde_json::Value::Null),
            "last_changed": data.get("last_changed").cloned().unwrap_or(serde_json::Value::Null),
            "last_updated": data.get("last_updated").cloned().unwrap_or(serde_json::Value::Null),
        }))
    }

    /// List services, optionally filtered by domain.
    pub async fn list_services(&self, domain: Option<&str>) -> Result<serde_json::Value, String> {
        let url = format!("{}/api/services", self.url);
        let resp = self
            .http
            .get(&url)
            .headers(self.headers())
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("HA list_services failed: HTTP {}", resp.status()));
        }
        let data: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        match domain {
            Some(d) => Ok(serde_json::json!({
                "domain": d,
                "services": data.get(d).cloned().unwrap_or(serde_json::Value::Null),
            })),
            None => Ok(data),
        }
    }

    /// Call a service.
    pub async fn call_service(
        &self,
        domain: &str,
        service: &str,
        entity_id: Option<&str>,
        data: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, String> {
        let url = format!("{}/api/services/{}/{}", self.url, domain, service);

        let mut payload = data.unwrap_or_else(|| serde_json::json!({}));
        if let Some(eid) = entity_id {
            payload["entity_id"] = serde_json::json!(eid);
        }

        let resp = self
            .http
            .post(&url)
            .headers(self.headers())
            .json(&payload)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("HA call_service failed: HTTP {}", resp.status()));
        }
        let result: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

        let affected: Vec<serde_json::Value> = result
            .as_array()
            .map(|arr| {
                arr.iter()
                    .map(|s| {
                        serde_json::json!({
                            "entity_id": s.get("entity_id").cloned().unwrap_or(serde_json::Value::Null),
                            "state": s.get("state").cloned().unwrap_or(serde_json::Value::Null),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(serde_json::json!({
            "success": true,
            "service": format!("{}.{}", domain, service),
            "affected_entities": affected,
        }))
    }
}

/// Shared config wrapper for the HA tools.
struct HaTools {
    client: Arc<HomeAssistantClient>,
}

/// `ha_list_entities` tool.
pub struct HaListEntitiesTool {
    inner: Arc<HaTools>,
}

/// `ha_get_state` tool.
pub struct HaGetStateTool {
    inner: Arc<HaTools>,
}

/// `ha_list_services` tool.
pub struct HaListServicesTool {
    inner: Arc<HaTools>,
}

/// `ha_call_service` tool.
pub struct HaCallServiceTool {
    inner: Arc<HaTools>,
}

/// Build the four HA tools from config.
pub fn build_tools(config: &HomeAssistantConfig) -> Vec<Box<dyn Tool>> {
    let client = Arc::new(HomeAssistantClient::new(&config.url, &config.token));
    let inner = Arc::new(HaTools { client });
    vec![
        Box::new(HaListEntitiesTool {
            inner: inner.clone(),
        }),
        Box::new(HaGetStateTool {
            inner: inner.clone(),
        }),
        Box::new(HaListServicesTool {
            inner: inner.clone(),
        }),
        Box::new(HaCallServiceTool { inner }),
    ]
}

#[async_trait]
impl Tool for HaListEntitiesTool {
    fn name(&self) -> &str {
        "ha_list_entities"
    }
    fn description(&self) -> &str {
        "List Home Assistant entities and their states, optionally filtered by \
         domain (e.g. 'light') or area (friendly name / area substring)."
    }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "domain": { "type": "string", "description": "Filter by entity domain" },
                "area": { "type": "string", "description": "Filter by area or friendly name" }
            }
        })
    }
    async fn execute(&self, args: serde_json::Value) -> Result<String, ToolError> {
        let domain = args["domain"].as_str();
        let area = args["area"].as_str();
        let out = self
            .inner
            .client
            .list_entities(domain, area)
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                tool: "ha_list_entities".to_string(),
                message: e,
            })?;
        Ok(out.to_string())
    }
}

#[async_trait]
impl Tool for HaGetStateTool {
    fn name(&self) -> &str {
        "ha_get_state"
    }
    fn description(&self) -> &str {
        "Get the detailed state and attributes of a single Home Assistant entity."
    }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "entity_id": { "type": "string", "description": "Entity id (e.g. light.living_room)" }
            },
            "required": ["entity_id"]
        })
    }
    async fn execute(&self, args: serde_json::Value) -> Result<String, ToolError> {
        let entity_id = args["entity_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments {
                tool: "ha_get_state".to_string(),
                message: "entity_id is required".to_string(),
            })?;
        let out = self.inner.client.get_state(entity_id).await.map_err(|e| {
            ToolError::ExecutionFailed {
                tool: "ha_get_state".to_string(),
                message: e,
            }
        })?;
        Ok(out.to_string())
    }
}

#[async_trait]
impl Tool for HaListServicesTool {
    fn name(&self) -> &str {
        "ha_list_services"
    }
    fn description(&self) -> &str {
        "List Home Assistant services, optionally filtered by domain."
    }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "domain": { "type": "string", "description": "Service domain (e.g. light, switch)" }
            }
        })
    }
    async fn execute(&self, args: serde_json::Value) -> Result<String, ToolError> {
        let domain = args["domain"].as_str();
        let out = self.inner.client.list_services(domain).await.map_err(|e| {
            ToolError::ExecutionFailed {
                tool: "ha_list_services".to_string(),
                message: e,
            }
        })?;
        Ok(out.to_string())
    }
}

#[async_trait]
impl Tool for HaCallServiceTool {
    fn name(&self) -> &str {
        "ha_call_service"
    }
    fn description(&self) -> &str {
        "Call a Home Assistant service (e.g. light.turn_on)."
    }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "domain": { "type": "string", "description": "Service domain (e.g. light)" },
                "service": { "type": "string", "description": "Service name (e.g. turn_on)" },
                "entity_id": { "type": "string", "description": "Target entity id" },
                "data": { "type": "object", "description": "Service data payload" }
            },
            "required": ["domain", "service"]
        })
    }
    async fn execute(&self, args: serde_json::Value) -> Result<String, ToolError> {
        let domain = args["domain"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments {
                tool: "ha_call_service".to_string(),
                message: "domain is required".to_string(),
            })?;
        let service = args["service"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments {
                tool: "ha_call_service".to_string(),
                message: "service is required".to_string(),
            })?;
        let entity_id = args["entity_id"].as_str();
        let data = args.get("data").cloned();
        let out = self
            .inner
            .client
            .call_service(domain, service, entity_id, data)
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                tool: "ha_call_service".to_string(),
                message: e,
            })?;
        Ok(out.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_available_requires_token() {
        let cfg = HomeAssistantConfig {
            enabled: true,
            url: "http://ha".into(),
            token: "tok".into(),
        };
        assert!(cfg.available());

        let cfg2 = HomeAssistantConfig {
            enabled: true,
            token: String::new(),
            ..Default::default()
        };
        assert!(!cfg2.available());
    }

    #[test]
    fn build_tools_returns_four() {
        let cfg = HomeAssistantConfig {
            enabled: true,
            url: "http://ha".into(),
            token: "tok".into(),
        };
        let tools = build_tools(&cfg);
        assert_eq!(tools.len(), 4);
    }

    #[test]
    fn tool_names_are_distinct() {
        let cfg = HomeAssistantConfig {
            enabled: true,
            url: "http://ha".into(),
            token: "tok".into(),
        };
        let tools = build_tools(&cfg);
        let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        let mut dedup = names.clone();
        dedup.sort();
        dedup.dedup();
        assert_eq!(names.len(), dedup.len());
    }
}
