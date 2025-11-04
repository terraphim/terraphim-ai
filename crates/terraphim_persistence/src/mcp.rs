use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::TryStreamExt;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::{Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpNamespaceRecord {
    pub uuid: String,
    pub name: String,
    pub description: Option<String>,
    pub user_id: Option<String>,
    pub config_json: String,
    pub created_at: DateTime<Utc>,
    pub enabled: bool,
    #[serde(default = "default_visibility")]
    pub visibility: NamespaceVisibility,
}

fn default_visibility() -> NamespaceVisibility {
    NamespaceVisibility::Private
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpEndpointRecord {
    pub uuid: String,
    pub name: String,
    pub namespace_uuid: String,
    pub auth_type: String,
    pub user_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpApiKeyRecord {
    pub uuid: String,
    pub key_hash: String,
    pub endpoint_uuid: String,
    pub user_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ToolStatus {
    Active,
    Inactive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolRecord {
    pub uuid: String,
    pub namespace_uuid: String,
    pub server_name: String,
    pub tool_name: String,
    pub original_name: String,
    pub status: ToolStatus,
    pub override_name: Option<String>,
    pub override_description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDiscoveryCache {
    pub namespace_uuid: String,
    pub tools_json: String,
    pub cached_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpAuditRecord {
    pub uuid: String,
    pub user_id: Option<String>,
    pub endpoint_uuid: String,
    pub namespace_uuid: String,
    pub tool_name: String,
    pub arguments: Option<String>,
    pub response: Option<String>,
    pub is_error: bool,
    pub latency_ms: u64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NamespaceVisibility {
    Public,
    Private,
}

#[async_trait]
pub trait McpPersistence: Send + Sync {
    async fn save_namespace(&self, record: &McpNamespaceRecord) -> Result<()>;
    async fn get_namespace(&self, uuid: &str) -> Result<Option<McpNamespaceRecord>>;
    async fn list_namespaces(&self, user_id: Option<&str>) -> Result<Vec<McpNamespaceRecord>>;
    async fn list_namespaces_with_visibility(
        &self,
        user_id: Option<&str>,
        include_public: bool,
    ) -> Result<Vec<McpNamespaceRecord>>;
    async fn delete_namespace(&self, uuid: &str) -> Result<()>;

    async fn save_endpoint(&self, record: &McpEndpointRecord) -> Result<()>;
    async fn get_endpoint(&self, uuid: &str) -> Result<Option<McpEndpointRecord>>;
    async fn list_endpoints(&self, user_id: Option<&str>) -> Result<Vec<McpEndpointRecord>>;
    async fn delete_endpoint(&self, uuid: &str) -> Result<()>;

    async fn save_api_key(&self, record: &McpApiKeyRecord) -> Result<()>;
    async fn get_api_key(&self, uuid: &str) -> Result<Option<McpApiKeyRecord>>;
    async fn verify_api_key(&self, key_hash: &str) -> Result<Option<McpApiKeyRecord>>;
    async fn list_api_keys(&self, user_id: Option<&str>) -> Result<Vec<McpApiKeyRecord>>;
    async fn delete_api_key(&self, uuid: &str) -> Result<()>;

    async fn save_tool(&self, record: &McpToolRecord) -> Result<()>;
    async fn get_tool(&self, uuid: &str) -> Result<Option<McpToolRecord>>;
    async fn list_tools(&self, namespace_uuid: Option<&str>) -> Result<Vec<McpToolRecord>>;
    async fn update_tool_status(&self, uuid: &str, status: ToolStatus) -> Result<()>;
    async fn delete_tool(&self, uuid: &str) -> Result<()>;

    async fn save_tool_cache(&self, cache: &ToolDiscoveryCache) -> Result<()>;
    async fn get_tool_cache(&self, namespace_uuid: &str) -> Result<Option<ToolDiscoveryCache>>;
    async fn delete_tool_cache(&self, namespace_uuid: &str) -> Result<()>;

    async fn save_audit(&self, record: &McpAuditRecord) -> Result<()>;
    async fn get_audit(&self, uuid: &str) -> Result<Option<McpAuditRecord>>;
    async fn list_audits(
        &self,
        user_id: Option<&str>,
        endpoint_uuid: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<McpAuditRecord>>;
    async fn delete_audit(&self, uuid: &str) -> Result<()>;
}

pub struct McpPersistenceImpl {
    operator: Arc<RwLock<opendal::Operator>>,
}

impl McpPersistenceImpl {
    pub fn new(operator: opendal::Operator) -> Self {
        Self {
            operator: Arc::new(RwLock::new(operator)),
        }
    }

    fn namespace_path(uuid: &str) -> String {
        format!("mcp/namespaces/{}.json", uuid)
    }

    fn endpoint_path(uuid: &str) -> String {
        format!("mcp/endpoints/{}.json", uuid)
    }

    fn api_key_path(uuid: &str) -> String {
        format!("mcp/api_keys/{}.json", uuid)
    }

    fn tool_path(uuid: &str) -> String {
        format!("mcp/tools/{}.json", uuid)
    }

    fn tool_cache_path(namespace_uuid: &str) -> String {
        format!("mcp/tool_cache/{}.json", namespace_uuid)
    }

    fn audit_path(uuid: &str) -> String {
        format!("mcp/audit/{}.json", uuid)
    }
}

#[async_trait]
impl McpPersistence for McpPersistenceImpl {
    async fn save_namespace(&self, record: &McpNamespaceRecord) -> Result<()> {
        let path = Self::namespace_path(&record.uuid);
        let data = serde_json::to_vec(record)?;
        self.operator.write().await.write(&path, data).await?;
        Ok(())
    }

    async fn get_namespace(&self, uuid: &str) -> Result<Option<McpNamespaceRecord>> {
        let path = Self::namespace_path(uuid);
        match self.operator.read().await.read(&path).await {
            Ok(data) => {
                let record = serde_json::from_slice(&data.to_vec())?;
                Ok(Some(record))
            }
            Err(e) if e.kind() == opendal::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(Error::OpenDal(Box::new(e))),
        }
    }

    async fn list_namespaces(&self, user_id: Option<&str>) -> Result<Vec<McpNamespaceRecord>> {
        let mut namespaces = Vec::new();
        let op = self.operator.read().await;

        let mut lister = op.lister("mcp/namespaces/").await?;
        while let Some(entry) = lister.try_next().await? {
            if let Ok(data) = op.read(entry.path()).await {
                if let Ok(record) = serde_json::from_slice::<McpNamespaceRecord>(&data.to_vec()) {
                    if user_id.is_none() || record.user_id.as_deref() == user_id {
                        namespaces.push(record);
                    }
                }
            }
        }

        Ok(namespaces)
    }

    async fn list_namespaces_with_visibility(
        &self,
        user_id: Option<&str>,
        include_public: bool,
    ) -> Result<Vec<McpNamespaceRecord>> {
        let mut namespaces = Vec::new();
        let op = self.operator.read().await;

        let mut lister = op.lister("mcp/namespaces/").await?;
        while let Some(entry) = lister.try_next().await? {
            if let Ok(data) = op.read(entry.path()).await {
                if let Ok(record) = serde_json::from_slice::<McpNamespaceRecord>(&data.to_vec()) {
                    let user_match = user_id.is_none() || record.user_id.as_deref() == user_id;
                    let visibility_match = if include_public {
                        user_match || record.visibility == NamespaceVisibility::Public
                    } else {
                        user_match
                    };

                    if visibility_match {
                        namespaces.push(record);
                    }
                }
            }
        }

        Ok(namespaces)
    }

    async fn delete_namespace(&self, uuid: &str) -> Result<()> {
        let path = Self::namespace_path(uuid);
        self.operator.write().await.delete(&path).await?;
        Ok(())
    }

    async fn save_endpoint(&self, record: &McpEndpointRecord) -> Result<()> {
        let path = Self::endpoint_path(&record.uuid);
        let data = serde_json::to_vec(record)?;
        self.operator.write().await.write(&path, data).await?;
        Ok(())
    }

    async fn get_endpoint(&self, uuid: &str) -> Result<Option<McpEndpointRecord>> {
        let path = Self::endpoint_path(uuid);
        match self.operator.read().await.read(&path).await {
            Ok(data) => {
                let record = serde_json::from_slice(&data.to_vec())?;
                Ok(Some(record))
            }
            Err(e) if e.kind() == opendal::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(Error::OpenDal(Box::new(e))),
        }
    }

    async fn list_endpoints(&self, user_id: Option<&str>) -> Result<Vec<McpEndpointRecord>> {
        let mut endpoints = Vec::new();
        let op = self.operator.read().await;

        let mut lister = op.lister("mcp/endpoints/").await?;
        while let Some(entry) = lister.try_next().await? {
            if let Ok(data) = op.read(entry.path()).await {
                if let Ok(record) = serde_json::from_slice::<McpEndpointRecord>(&data.to_vec()) {
                    if user_id.is_none() || record.user_id.as_deref() == user_id {
                        endpoints.push(record);
                    }
                }
            }
        }

        Ok(endpoints)
    }

    async fn delete_endpoint(&self, uuid: &str) -> Result<()> {
        let path = Self::endpoint_path(uuid);
        self.operator.write().await.delete(&path).await?;
        Ok(())
    }

    async fn save_api_key(&self, record: &McpApiKeyRecord) -> Result<()> {
        let path = Self::api_key_path(&record.uuid);
        let data = serde_json::to_vec(record)?;
        self.operator.write().await.write(&path, data).await?;
        Ok(())
    }

    async fn get_api_key(&self, uuid: &str) -> Result<Option<McpApiKeyRecord>> {
        let path = Self::api_key_path(uuid);
        match self.operator.read().await.read(&path).await {
            Ok(data) => {
                let record = serde_json::from_slice(&data.to_vec())?;
                Ok(Some(record))
            }
            Err(e) if e.kind() == opendal::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(Error::OpenDal(Box::new(e))),
        }
    }

    async fn verify_api_key(&self, key_hash: &str) -> Result<Option<McpApiKeyRecord>> {
        let op = self.operator.read().await;

        let mut lister = op.lister("mcp/api_keys/").await?;
        while let Some(entry) = lister.try_next().await? {
            if let Ok(data) = op.read(entry.path()).await {
                if let Ok(record) = serde_json::from_slice::<McpApiKeyRecord>(&data.to_vec()) {
                    if record.key_hash == key_hash && record.enabled {
                        if let Some(expires_at) = record.expires_at {
                            if expires_at > Utc::now() {
                                return Ok(Some(record));
                            }
                        } else {
                            return Ok(Some(record));
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    async fn list_api_keys(&self, user_id: Option<&str>) -> Result<Vec<McpApiKeyRecord>> {
        let mut api_keys = Vec::new();
        let op = self.operator.read().await;

        let mut lister = op.lister("mcp/api_keys/").await?;
        while let Some(entry) = lister.try_next().await? {
            if let Ok(data) = op.read(entry.path()).await {
                if let Ok(record) = serde_json::from_slice::<McpApiKeyRecord>(&data.to_vec()) {
                    if user_id.is_none() || record.user_id.as_deref() == user_id {
                        api_keys.push(record);
                    }
                }
            }
        }

        Ok(api_keys)
    }

    async fn delete_api_key(&self, uuid: &str) -> Result<()> {
        let path = Self::api_key_path(uuid);
        self.operator.write().await.delete(&path).await?;
        Ok(())
    }

    async fn save_tool(&self, record: &McpToolRecord) -> Result<()> {
        let path = Self::tool_path(&record.uuid);
        let data = serde_json::to_vec(record)?;
        self.operator.write().await.write(&path, data).await?;
        Ok(())
    }

    async fn get_tool(&self, uuid: &str) -> Result<Option<McpToolRecord>> {
        let path = Self::tool_path(uuid);
        match self.operator.read().await.read(&path).await {
            Ok(data) => {
                let record = serde_json::from_slice(&data.to_vec())?;
                Ok(Some(record))
            }
            Err(e) if e.kind() == opendal::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(Error::OpenDal(Box::new(e))),
        }
    }

    async fn list_tools(&self, namespace_uuid: Option<&str>) -> Result<Vec<McpToolRecord>> {
        let mut tools = Vec::new();
        let op = self.operator.read().await;

        let mut lister = op.lister("mcp/tools/").await?;
        while let Some(entry) = lister.try_next().await? {
            if let Ok(data) = op.read(entry.path()).await {
                if let Ok(record) = serde_json::from_slice::<McpToolRecord>(&data.to_vec()) {
                    if namespace_uuid.is_none()
                        || namespace_uuid == Some(record.namespace_uuid.as_str())
                    {
                        tools.push(record);
                    }
                }
            }
        }

        Ok(tools)
    }

    async fn update_tool_status(&self, uuid: &str, status: ToolStatus) -> Result<()> {
        if let Some(mut tool) = self.get_tool(uuid).await? {
            tool.status = status;
            tool.updated_at = Utc::now();
            self.save_tool(&tool).await?;
        }
        Ok(())
    }

    async fn delete_tool(&self, uuid: &str) -> Result<()> {
        let path = Self::tool_path(uuid);
        self.operator.write().await.delete(&path).await?;
        Ok(())
    }

    async fn save_tool_cache(&self, cache: &ToolDiscoveryCache) -> Result<()> {
        let path = Self::tool_cache_path(&cache.namespace_uuid);
        let data = serde_json::to_vec(cache)?;
        self.operator.write().await.write(&path, data).await?;
        Ok(())
    }

    async fn get_tool_cache(&self, namespace_uuid: &str) -> Result<Option<ToolDiscoveryCache>> {
        let path = Self::tool_cache_path(namespace_uuid);
        match self.operator.read().await.read(&path).await {
            Ok(data) => {
                let cache = serde_json::from_slice(&data.to_vec())?;
                Ok(Some(cache))
            }
            Err(e) if e.kind() == opendal::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(Error::OpenDal(Box::new(e))),
        }
    }

    async fn delete_tool_cache(&self, namespace_uuid: &str) -> Result<()> {
        let path = Self::tool_cache_path(namespace_uuid);
        self.operator.write().await.delete(&path).await?;
        Ok(())
    }

    async fn save_audit(&self, record: &McpAuditRecord) -> Result<()> {
        let path = Self::audit_path(&record.uuid);
        let data = serde_json::to_vec(record)?;
        self.operator.write().await.write(&path, data).await?;
        Ok(())
    }

    async fn get_audit(&self, uuid: &str) -> Result<Option<McpAuditRecord>> {
        let path = Self::audit_path(uuid);
        match self.operator.read().await.read(&path).await {
            Ok(data) => {
                let record = serde_json::from_slice(&data.to_vec())?;
                Ok(Some(record))
            }
            Err(e) if e.kind() == opendal::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(Error::OpenDal(Box::new(e))),
        }
    }

    async fn list_audits(
        &self,
        user_id: Option<&str>,
        endpoint_uuid: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<McpAuditRecord>> {
        let mut audits = Vec::new();
        let op = self.operator.read().await;

        let mut lister = op.lister("mcp/audit/").await?;
        while let Some(entry) = lister.try_next().await? {
            if let Ok(data) = op.read(entry.path()).await {
                if let Ok(record) = serde_json::from_slice::<McpAuditRecord>(&data.to_vec()) {
                    let user_match = user_id.is_none() || record.user_id.as_deref() == user_id;
                    let endpoint_match = endpoint_uuid.is_none()
                        || endpoint_uuid == Some(record.endpoint_uuid.as_str());

                    if user_match && endpoint_match {
                        audits.push(record);

                        if let Some(limit) = limit {
                            if audits.len() >= limit {
                                break;
                            }
                        }
                    }
                }
            }
        }

        audits.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(audits)
    }

    async fn delete_audit(&self, uuid: &str) -> Result<()> {
        let path = Self::audit_path(uuid);
        self.operator.write().await.delete(&path).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_namespace_persistence() {
        let op = opendal::Operator::via_iter(opendal::Scheme::Memory, Default::default()).unwrap();
        let persistence = McpPersistenceImpl::new(op);

        let record = McpNamespaceRecord {
            uuid: "test-uuid".to_string(),
            name: "test-namespace".to_string(),
            description: Some("Test description".to_string()),
            user_id: Some("user-123".to_string()),
            config_json: "{}".to_string(),
            created_at: Utc::now(),
            enabled: true,
            visibility: NamespaceVisibility::Private,
        };

        persistence.save_namespace(&record).await.unwrap();

        let retrieved = persistence
            .get_namespace("test-uuid")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(retrieved.name, "test-namespace");

        let list = persistence.list_namespaces(Some("user-123")).await.unwrap();
        assert_eq!(list.len(), 1);

        persistence.delete_namespace("test-uuid").await.unwrap();
        assert!(persistence
            .get_namespace("test-uuid")
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn test_api_key_verification() {
        let op = opendal::Operator::via_iter(opendal::Scheme::Memory, Default::default()).unwrap();
        let persistence = McpPersistenceImpl::new(op);

        let record = McpApiKeyRecord {
            uuid: "key-uuid".to_string(),
            key_hash: "hash123".to_string(),
            endpoint_uuid: "endpoint-uuid".to_string(),
            user_id: Some("user-123".to_string()),
            created_at: Utc::now(),
            expires_at: None,
            enabled: true,
        };

        persistence.save_api_key(&record).await.unwrap();

        let verified = persistence
            .verify_api_key("hash123")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(verified.uuid, "key-uuid");

        assert!(persistence
            .verify_api_key("invalid")
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn test_tool_management() {
        let op = opendal::Operator::via_iter(opendal::Scheme::Memory, Default::default()).unwrap();
        let persistence = McpPersistenceImpl::new(op);

        let tool = McpToolRecord {
            uuid: "tool-uuid".to_string(),
            namespace_uuid: "ns-uuid".to_string(),
            server_name: "filesystem".to_string(),
            tool_name: "filesystem__read_file".to_string(),
            original_name: "read_file".to_string(),
            status: ToolStatus::Active,
            override_name: Some("read_code".to_string()),
            override_description: Some("Read source code file".to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        persistence.save_tool(&tool).await.unwrap();

        let retrieved = persistence.get_tool("tool-uuid").await.unwrap().unwrap();
        assert_eq!(retrieved.tool_name, "filesystem__read_file");
        assert_eq!(retrieved.override_name, Some("read_code".to_string()));

        persistence
            .update_tool_status("tool-uuid", ToolStatus::Inactive)
            .await
            .unwrap();

        let updated = persistence.get_tool("tool-uuid").await.unwrap().unwrap();
        assert_eq!(updated.status, ToolStatus::Inactive);

        let tools = persistence.list_tools(Some("ns-uuid")).await.unwrap();
        assert_eq!(tools.len(), 1);

        persistence.delete_tool("tool-uuid").await.unwrap();
        assert!(persistence.get_tool("tool-uuid").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_tool_cache() {
        let op = opendal::Operator::via_iter(opendal::Scheme::Memory, Default::default()).unwrap();
        let persistence = McpPersistenceImpl::new(op);

        let cache = ToolDiscoveryCache {
            namespace_uuid: "ns-uuid".to_string(),
            tools_json: r#"[{"name": "read_file"}]"#.to_string(),
            cached_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::hours(1),
        };

        persistence.save_tool_cache(&cache).await.unwrap();

        let retrieved = persistence
            .get_tool_cache("ns-uuid")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(retrieved.namespace_uuid, "ns-uuid");

        persistence.delete_tool_cache("ns-uuid").await.unwrap();
        assert!(persistence
            .get_tool_cache("ns-uuid")
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn test_audit_trail() {
        let op = opendal::Operator::via_iter(opendal::Scheme::Memory, Default::default()).unwrap();
        let persistence = McpPersistenceImpl::new(op);

        let audit1 = McpAuditRecord {
            uuid: "audit-1".to_string(),
            user_id: Some("user-123".to_string()),
            endpoint_uuid: "endpoint-1".to_string(),
            namespace_uuid: "ns-1".to_string(),
            tool_name: "test_tool".to_string(),
            arguments: Some(r#"{"key": "value"}"#.to_string()),
            response: Some(r#"{"result": "success"}"#.to_string()),
            is_error: false,
            latency_ms: 150,
            created_at: Utc::now(),
        };

        let audit2 = McpAuditRecord {
            uuid: "audit-2".to_string(),
            user_id: Some("user-456".to_string()),
            endpoint_uuid: "endpoint-1".to_string(),
            namespace_uuid: "ns-1".to_string(),
            tool_name: "another_tool".to_string(),
            arguments: None,
            response: Some(r#"{"error": "failed"}"#.to_string()),
            is_error: true,
            latency_ms: 50,
            created_at: Utc::now(),
        };

        persistence.save_audit(&audit1).await.unwrap();
        persistence.save_audit(&audit2).await.unwrap();

        let retrieved = persistence.get_audit("audit-1").await.unwrap().unwrap();
        assert_eq!(retrieved.tool_name, "test_tool");
        assert_eq!(retrieved.latency_ms, 150);

        let user_audits = persistence
            .list_audits(Some("user-123"), None, None)
            .await
            .unwrap();
        assert_eq!(user_audits.len(), 1);
        assert_eq!(user_audits[0].uuid, "audit-1");

        let endpoint_audits = persistence
            .list_audits(None, Some("endpoint-1"), Some(10))
            .await
            .unwrap();
        assert_eq!(endpoint_audits.len(), 2);

        persistence.delete_audit("audit-1").await.unwrap();
        assert!(persistence.get_audit("audit-1").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_namespace_visibility() {
        let op = opendal::Operator::via_iter(opendal::Scheme::Memory, Default::default()).unwrap();
        let persistence = McpPersistenceImpl::new(op);

        let private_ns = McpNamespaceRecord {
            uuid: "private-ns".to_string(),
            name: "Private Namespace".to_string(),
            description: None,
            user_id: Some("user-123".to_string()),
            config_json: "{}".to_string(),
            created_at: Utc::now(),
            enabled: true,
            visibility: NamespaceVisibility::Private,
        };

        let public_ns = McpNamespaceRecord {
            uuid: "public-ns".to_string(),
            name: "Public Namespace".to_string(),
            description: None,
            user_id: Some("user-456".to_string()),
            config_json: "{}".to_string(),
            created_at: Utc::now(),
            enabled: true,
            visibility: NamespaceVisibility::Public,
        };

        persistence.save_namespace(&private_ns).await.unwrap();
        persistence.save_namespace(&public_ns).await.unwrap();

        let user_only = persistence
            .list_namespaces_with_visibility(Some("user-123"), false)
            .await
            .unwrap();
        assert_eq!(user_only.len(), 1);
        assert_eq!(user_only[0].uuid, "private-ns");

        let with_public = persistence
            .list_namespaces_with_visibility(Some("user-123"), true)
            .await
            .unwrap();
        assert_eq!(with_public.len(), 2);

        let other_user = persistence
            .list_namespaces_with_visibility(Some("user-789"), true)
            .await
            .unwrap();
        assert_eq!(other_user.len(), 1);
        assert_eq!(other_user[0].uuid, "public-ns");
    }
}
