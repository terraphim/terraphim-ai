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

#[async_trait]
pub trait McpPersistence: Send + Sync {
    async fn save_namespace(&self, record: &McpNamespaceRecord) -> Result<()>;
    async fn get_namespace(&self, uuid: &str) -> Result<Option<McpNamespaceRecord>>;
    async fn list_namespaces(&self, user_id: Option<&str>) -> Result<Vec<McpNamespaceRecord>>;
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_namespace_persistence() {
        let op = opendal::Operator::via_map(opendal::Scheme::Memory, Default::default()).unwrap();
        let persistence = McpPersistenceImpl::new(op);

        let record = McpNamespaceRecord {
            uuid: "test-uuid".to_string(),
            name: "test-namespace".to_string(),
            description: Some("Test description".to_string()),
            user_id: Some("user-123".to_string()),
            config_json: "{}".to_string(),
            created_at: Utc::now(),
            enabled: true,
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
        let op = opendal::Operator::via_map(opendal::Scheme::Memory, Default::default()).unwrap();
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
}
