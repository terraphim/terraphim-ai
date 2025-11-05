use crate::{Error, Result};
use chrono::Utc;
use std::sync::Arc;

#[cfg(feature = "mcp-proxy")]
use terraphim_config::Role;
#[cfg(feature = "mcp-proxy")]
use terraphim_mcp_proxy::Tool;
use terraphim_persistence::mcp::{McpPersistence, McpToolRecord, ToolDiscoveryCache, ToolStatus};

const CACHE_DURATION_HOURS: i64 = 1;

pub struct ToolDiscoveryService<P: McpPersistence> {
    persistence: Arc<P>,
}

impl<P: McpPersistence> ToolDiscoveryService<P> {
    pub fn new(persistence: Arc<P>) -> Self {
        Self { persistence }
    }

    #[cfg(feature = "mcp-proxy")]
    pub async fn discover_tools_with_overrides(
        &self,
        role: &Role,
        namespace_uuid: &str,
    ) -> Result<Vec<Tool>> {
        if let Some(cached) = self.get_cached_tools(namespace_uuid).await? {
            log::info!("Using cached tools for namespace {}", namespace_uuid);
            return serde_json::from_str(&cached.tools_json).map_err(|e| {
                Error::Indexation(format!("Failed to deserialize cached tools: {}", e))
            });
        }

        log::info!("Discovering fresh tools for namespace {}", namespace_uuid);
        let raw_tools = crate::mcp_namespace::list_namespace_tools(role).await?;

        let tool_records = self
            .persistence
            .list_tools(Some(namespace_uuid))
            .await
            .map_err(Error::Persistence)?;

        let mut processed_tools = Vec::new();
        for mut tool in raw_tools {
            if let Some(record) = self.find_tool_record(&tool_records, &tool.name) {
                if record.status == ToolStatus::Inactive {
                    log::debug!("Filtering inactive tool: {}", tool.name);
                    continue;
                }

                if let Some(override_name) = &record.override_name {
                    log::debug!("Applying name override: {} -> {}", tool.name, override_name);
                    tool.name = override_name.clone();
                }

                if let Some(override_desc) = &record.override_description {
                    log::debug!(
                        "Applying description override for tool: {}",
                        record.tool_name
                    );
                    tool.description = override_desc.clone();
                }
            }

            processed_tools.push(tool);
        }

        self.cache_tools(namespace_uuid, &processed_tools).await?;

        Ok(processed_tools)
    }

    async fn get_cached_tools(&self, namespace_uuid: &str) -> Result<Option<ToolDiscoveryCache>> {
        match self
            .persistence
            .get_tool_cache(namespace_uuid)
            .await
            .map_err(Error::Persistence)?
        {
            Some(cache) if cache.expires_at > Utc::now() => Ok(Some(cache)),
            Some(_cache) => {
                log::info!("Cache expired for namespace {}", namespace_uuid);
                self.persistence
                    .delete_tool_cache(namespace_uuid)
                    .await
                    .map_err(Error::Persistence)?;
                Ok(None)
            }
            None => Ok(None),
        }
    }

    #[cfg(feature = "mcp-proxy")]
    async fn cache_tools(&self, namespace_uuid: &str, tools: &[Tool]) -> Result<()> {
        let tools_json = serde_json::to_string(tools)
            .map_err(|e| Error::Indexation(format!("Failed to serialize tools: {}", e)))?;

        let cache = ToolDiscoveryCache {
            namespace_uuid: namespace_uuid.to_string(),
            tools_json,
            cached_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::hours(CACHE_DURATION_HOURS),
        };

        self.persistence
            .save_tool_cache(&cache)
            .await
            .map_err(Error::Persistence)?;

        log::info!(
            "Cached {} tools for namespace {}",
            tools.len(),
            namespace_uuid
        );
        Ok(())
    }

    fn find_tool_record<'a>(
        &self,
        records: &'a [McpToolRecord],
        tool_name: &str,
    ) -> Option<&'a McpToolRecord> {
        records.iter().find(|r| r.tool_name == tool_name)
    }

    pub async fn invalidate_cache(&self, namespace_uuid: &str) -> Result<()> {
        self.persistence
            .delete_tool_cache(namespace_uuid)
            .await
            .map_err(Error::Persistence)?;

        log::info!("Invalidated cache for namespace {}", namespace_uuid);
        Ok(())
    }

    #[cfg(feature = "mcp-proxy")]
    pub async fn register_tool(
        &self,
        namespace_uuid: &str,
        server_name: &str,
        tool: &Tool,
    ) -> Result<()> {
        let record = McpToolRecord {
            uuid: uuid::Uuid::new_v4().to_string(),
            namespace_uuid: namespace_uuid.to_string(),
            server_name: server_name.to_string(),
            tool_name: tool.name.clone(),
            original_name: tool.name.clone(),
            status: ToolStatus::Active,
            override_name: None,
            override_description: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        self.persistence
            .save_tool(&record)
            .await
            .map_err(Error::Persistence)?;

        log::info!("Registered tool: {}", tool.name);
        Ok(())
    }

    pub async fn update_tool_override(
        &self,
        tool_uuid: &str,
        override_name: Option<String>,
        override_description: Option<String>,
    ) -> Result<()> {
        let mut tool = self
            .persistence
            .get_tool(tool_uuid)
            .await
            .map_err(Error::Persistence)?
            .ok_or_else(|| Error::Indexation(format!("Tool not found: {}", tool_uuid)))?;

        tool.override_name = override_name;
        tool.override_description = override_description;
        tool.updated_at = Utc::now();

        self.persistence
            .save_tool(&tool)
            .await
            .map_err(Error::Persistence)?;

        self.invalidate_cache(&tool.namespace_uuid).await?;

        log::info!("Updated tool override for: {}", tool_uuid);
        Ok(())
    }
}
