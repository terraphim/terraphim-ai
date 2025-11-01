use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{AppState, Result, Status};
use terraphim_persistence::mcp::{
    McpApiKeyRecord, McpEndpointRecord, McpNamespaceRecord, McpPersistence, McpPersistenceImpl,
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct McpNamespaceResponse {
    pub status: Status,
    pub namespace: Option<McpNamespaceRecord>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct McpNamespaceListResponse {
    pub status: Status,
    pub namespaces: Vec<McpNamespaceRecord>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct McpEndpointResponse {
    pub status: Status,
    pub endpoint: Option<McpEndpointRecord>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct McpEndpointListResponse {
    pub status: Status,
    pub endpoints: Vec<McpEndpointRecord>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct McpApiKeyResponse {
    pub status: Status,
    pub api_key: Option<McpApiKeyRecord>,
    pub key_value: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateNamespaceRequest {
    pub name: String,
    pub description: Option<String>,
    pub user_id: Option<String>,
    pub config_json: String,
    pub enabled: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateEndpointRequest {
    pub name: String,
    pub namespace_uuid: String,
    pub auth_type: String,
    pub user_id: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateApiKeyRequest {
    pub endpoint_uuid: String,
    pub user_id: Option<String>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub enabled: bool,
}

pub async fn list_namespaces(
    State(_app_state): State<AppState>,
) -> Result<Json<McpNamespaceListResponse>> {
    let persistence = get_mcp_persistence()?;

    match persistence.list_namespaces(None).await {
        Ok(namespaces) => Ok(Json(McpNamespaceListResponse {
            status: Status::Success,
            namespaces,
            error: None,
        })),
        Err(e) => Ok(Json(McpNamespaceListResponse {
            status: Status::Error,
            namespaces: vec![],
            error: Some(format!("Failed to list namespaces: {}", e)),
        })),
    }
}

pub async fn get_namespace(
    State(_app_state): State<AppState>,
    Path(uuid): Path<String>,
) -> Result<Json<McpNamespaceResponse>> {
    let persistence = get_mcp_persistence()?;

    match persistence.get_namespace(&uuid).await {
        Ok(Some(namespace)) => Ok(Json(McpNamespaceResponse {
            status: Status::Success,
            namespace: Some(namespace),
            error: None,
        })),
        Ok(None) => Ok(Json(McpNamespaceResponse {
            status: Status::Error,
            namespace: None,
            error: Some("Namespace not found".to_string()),
        })),
        Err(e) => Ok(Json(McpNamespaceResponse {
            status: Status::Error,
            namespace: None,
            error: Some(format!("Failed to get namespace: {}", e)),
        })),
    }
}

pub async fn create_namespace(
    State(_app_state): State<AppState>,
    Json(request): Json<CreateNamespaceRequest>,
) -> Result<Json<McpNamespaceResponse>> {
    let persistence = get_mcp_persistence()?;

    let record = McpNamespaceRecord {
        uuid: uuid::Uuid::new_v4().to_string(),
        name: request.name,
        description: request.description,
        user_id: request.user_id,
        config_json: request.config_json,
        created_at: chrono::Utc::now(),
        enabled: request.enabled,
    };

    match persistence.save_namespace(&record).await {
        Ok(()) => Ok(Json(McpNamespaceResponse {
            status: Status::Success,
            namespace: Some(record),
            error: None,
        })),
        Err(e) => Ok(Json(McpNamespaceResponse {
            status: Status::Error,
            namespace: None,
            error: Some(format!("Failed to create namespace: {}", e)),
        })),
    }
}

pub async fn delete_namespace(
    State(_app_state): State<AppState>,
    Path(uuid): Path<String>,
) -> Result<impl IntoResponse> {
    let persistence = get_mcp_persistence()?;

    match persistence.delete_namespace(&uuid).await {
        Ok(()) => Ok((StatusCode::NO_CONTENT, "")),
        Err(e) => {
            log::error!("Failed to delete namespace {}: {}", uuid, e);
            Err(crate::error::ApiError(
                StatusCode::INTERNAL_SERVER_ERROR,
                anyhow::anyhow!("Failed to delete namespace"),
            ))
        }
    }
}

pub async fn list_endpoints(
    State(_app_state): State<AppState>,
) -> Result<Json<McpEndpointListResponse>> {
    let persistence = get_mcp_persistence()?;

    match persistence.list_endpoints(None).await {
        Ok(endpoints) => Ok(Json(McpEndpointListResponse {
            status: Status::Success,
            endpoints,
            error: None,
        })),
        Err(e) => Ok(Json(McpEndpointListResponse {
            status: Status::Error,
            endpoints: vec![],
            error: Some(format!("Failed to list endpoints: {}", e)),
        })),
    }
}

pub async fn get_endpoint(
    State(_app_state): State<AppState>,
    Path(uuid): Path<String>,
) -> Result<Json<McpEndpointResponse>> {
    let persistence = get_mcp_persistence()?;

    match persistence.get_endpoint(&uuid).await {
        Ok(Some(endpoint)) => Ok(Json(McpEndpointResponse {
            status: Status::Success,
            endpoint: Some(endpoint),
            error: None,
        })),
        Ok(None) => Ok(Json(McpEndpointResponse {
            status: Status::Error,
            endpoint: None,
            error: Some("Endpoint not found".to_string()),
        })),
        Err(e) => Ok(Json(McpEndpointResponse {
            status: Status::Error,
            endpoint: None,
            error: Some(format!("Failed to get endpoint: {}", e)),
        })),
    }
}

pub async fn create_endpoint(
    State(_app_state): State<AppState>,
    Json(request): Json<CreateEndpointRequest>,
) -> Result<Json<McpEndpointResponse>> {
    let persistence = get_mcp_persistence()?;

    let record = McpEndpointRecord {
        uuid: uuid::Uuid::new_v4().to_string(),
        name: request.name,
        namespace_uuid: request.namespace_uuid,
        auth_type: request.auth_type,
        user_id: request.user_id,
        created_at: chrono::Utc::now(),
        enabled: request.enabled,
    };

    match persistence.save_endpoint(&record).await {
        Ok(()) => Ok(Json(McpEndpointResponse {
            status: Status::Success,
            endpoint: Some(record),
            error: None,
        })),
        Err(e) => Ok(Json(McpEndpointResponse {
            status: Status::Error,
            endpoint: None,
            error: Some(format!("Failed to create endpoint: {}", e)),
        })),
    }
}

pub async fn delete_endpoint(
    State(_app_state): State<AppState>,
    Path(uuid): Path<String>,
) -> Result<impl IntoResponse> {
    let persistence = get_mcp_persistence()?;

    match persistence.delete_endpoint(&uuid).await {
        Ok(()) => Ok((StatusCode::NO_CONTENT, "")),
        Err(e) => {
            log::error!("Failed to delete endpoint {}: {}", uuid, e);
            Err(crate::error::ApiError(
                StatusCode::INTERNAL_SERVER_ERROR,
                anyhow::anyhow!("Failed to delete endpoint"),
            ))
        }
    }
}

pub async fn create_api_key(
    State(_app_state): State<AppState>,
    Json(request): Json<CreateApiKeyRequest>,
) -> Result<Json<McpApiKeyResponse>> {
    let persistence = get_mcp_persistence()?;

    let key_value = format!("tpai_{}", uuid::Uuid::new_v4().to_string().replace('-', ""));
    let key_hash = hash_api_key(&key_value);

    let record = McpApiKeyRecord {
        uuid: uuid::Uuid::new_v4().to_string(),
        key_hash,
        endpoint_uuid: request.endpoint_uuid,
        user_id: request.user_id,
        created_at: chrono::Utc::now(),
        expires_at: request.expires_at,
        enabled: request.enabled,
    };

    match persistence.save_api_key(&record).await {
        Ok(()) => Ok(Json(McpApiKeyResponse {
            status: Status::Success,
            api_key: Some(record),
            key_value: Some(key_value),
            error: None,
        })),
        Err(e) => Ok(Json(McpApiKeyResponse {
            status: Status::Error,
            api_key: None,
            key_value: None,
            error: Some(format!("Failed to create API key: {}", e)),
        })),
    }
}

fn get_mcp_persistence() -> Result<Arc<McpPersistenceImpl>> {
    use opendal::services::Memory;
    use opendal::Operator;

    let builder = Memory::default();
    let op = Operator::new(builder)
        .map_err(|e| {
            crate::error::ApiError(
                StatusCode::INTERNAL_SERVER_ERROR,
                anyhow::anyhow!("Failed to create operator: {}", e),
            )
        })?
        .finish();

    Ok(Arc::new(McpPersistenceImpl::new(op)))
}

fn hash_api_key(key: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    format!("{:x}", hasher.finalize())
}
