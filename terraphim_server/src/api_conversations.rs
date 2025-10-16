// New persistent conversation endpoints using ConversationService
use axum::{
    extract::{Path, Query},
    Json,
};
use serde::{Deserialize, Serialize};
use terraphim_service::conversation_service::{
    ConversationFilter, ConversationService, ConversationStatistics,
};
use terraphim_types::{Conversation, ConversationId, ConversationSummary, RoleName};

use crate::error::{Result, Status};

// Request/Response types for persistent conversation endpoints

#[derive(Debug, Serialize, Deserialize)]
pub struct ListPersistentConversationsQuery {
    pub skip: Option<usize>,
    pub limit: Option<usize>,
    pub role: Option<String>,
    pub search: Option<String>,
    pub show_archived: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListPersistentConversationsResponse {
    pub status: Status,
    pub conversations: Vec<ConversationSummary>,
    pub total: usize,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetPersistentConversationResponse {
    pub status: Status,
    pub conversation: Option<Conversation>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreatePersistentConversationRequest {
    pub title: String,
    pub role: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreatePersistentConversationResponse {
    pub status: Status,
    pub conversation: Option<Conversation>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdatePersistentConversationRequest {
    pub title: Option<String>,
    pub metadata: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdatePersistentConversationResponse {
    pub status: Status,
    pub conversation: Option<Conversation>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeletePersistentConversationResponse {
    pub status: Status,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchPersistentConversationsQuery {
    pub query: String,
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchPersistentConversationsResponse {
    pub status: Status,
    pub conversations: Vec<ConversationSummary>,
    pub total: usize,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportConversationResponse {
    pub status: Status,
    pub json_data: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImportConversationRequest {
    pub json_data: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImportConversationResponse {
    pub status: Status,
    pub conversation: Option<Conversation>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConversationStatisticsResponse {
    pub status: Status,
    pub statistics: Option<ConversationStatistics>,
    pub error: Option<String>,
}

// API Endpoints

/// List all persistent conversations with optional filtering
pub(crate) async fn list_persistent_conversations(
    Query(params): Query<ListPersistentConversationsQuery>,
) -> Result<Json<ListPersistentConversationsResponse>> {
    log::debug!(
        "list_persistent_conversations called with params: {:?}",
        params
    );

    let service = ConversationService::new();

    let filter = ConversationFilter {
        role: params.role.map(|r| RoleName::new(&r)),
        search_query: params.search,
        show_archived: params.show_archived.unwrap_or(false),
        ..Default::default()
    };

    match service.list_conversations(filter).await {
        Ok(mut conversations) => {
            let total = conversations.len();

            // Apply pagination
            let skip = params.skip.unwrap_or(0);
            let limit = params.limit.unwrap_or(100);

            if skip < conversations.len() {
                conversations = conversations.into_iter().skip(skip).take(limit).collect();
            } else {
                conversations.clear();
            }

            Ok(Json(ListPersistentConversationsResponse {
                status: Status::Success,
                conversations,
                total,
                error: None,
            }))
        }
        Err(e) => Ok(Json(ListPersistentConversationsResponse {
            status: Status::Error,
            conversations: vec![],
            total: 0,
            error: Some(format!("Failed to list conversations: {}", e)),
        })),
    }
}

/// Get a specific persistent conversation by ID
pub(crate) async fn get_persistent_conversation(
    Path(conversation_id): Path<String>,
) -> Result<Json<GetPersistentConversationResponse>> {
    log::debug!(
        "get_persistent_conversation called for ID: {}",
        conversation_id
    );

    let service = ConversationService::new();
    let conv_id = ConversationId::from_string(conversation_id);

    match service.get_conversation(&conv_id).await {
        Ok(conversation) => Ok(Json(GetPersistentConversationResponse {
            status: Status::Success,
            conversation: Some(conversation),
            error: None,
        })),
        Err(e) => Ok(Json(GetPersistentConversationResponse {
            status: Status::Error,
            conversation: None,
            error: Some(format!("Failed to get conversation: {}", e)),
        })),
    }
}

/// Create a new persistent conversation
pub(crate) async fn create_persistent_conversation(
    Json(request): Json<CreatePersistentConversationRequest>,
) -> Result<Json<CreatePersistentConversationResponse>> {
    log::info!(
        "create_persistent_conversation called: title='{}', role='{}'",
        request.title,
        request.role
    );

    let service = ConversationService::new();
    let role = RoleName::new(&request.role);

    match service.create_conversation(request.title, role).await {
        Ok(conversation) => Ok(Json(CreatePersistentConversationResponse {
            status: Status::Success,
            conversation: Some(conversation),
            error: None,
        })),
        Err(e) => Ok(Json(CreatePersistentConversationResponse {
            status: Status::Error,
            conversation: None,
            error: Some(format!("Failed to create conversation: {}", e)),
        })),
    }
}

/// Update a persistent conversation's metadata
pub(crate) async fn update_persistent_conversation(
    Path(conversation_id): Path<String>,
    Json(request): Json<UpdatePersistentConversationRequest>,
) -> Result<Json<UpdatePersistentConversationResponse>> {
    log::debug!(
        "update_persistent_conversation called for ID: {}",
        conversation_id
    );

    let service = ConversationService::new();
    let conv_id = ConversationId::from_string(conversation_id);

    // Get existing conversation
    let mut conversation = match service.get_conversation(&conv_id).await {
        Ok(conv) => conv,
        Err(e) => {
            return Ok(Json(UpdatePersistentConversationResponse {
                status: Status::Error,
                conversation: None,
                error: Some(format!("Conversation not found: {}", e)),
            }))
        }
    };

    // Update fields
    if let Some(title) = request.title {
        conversation.title = title;
    }
    if let Some(metadata) = request.metadata {
        for (key, value) in metadata {
            conversation.metadata.insert(key, value);
        }
    }

    // Save updated conversation
    match service.update_conversation(conversation).await {
        Ok(updated) => Ok(Json(UpdatePersistentConversationResponse {
            status: Status::Success,
            conversation: Some(updated),
            error: None,
        })),
        Err(e) => Ok(Json(UpdatePersistentConversationResponse {
            status: Status::Error,
            conversation: None,
            error: Some(format!("Failed to update conversation: {}", e)),
        })),
    }
}

/// Delete a persistent conversation
pub(crate) async fn delete_persistent_conversation(
    Path(conversation_id): Path<String>,
) -> Result<Json<DeletePersistentConversationResponse>> {
    log::info!(
        "delete_persistent_conversation called for ID: {}",
        conversation_id
    );

    let service = ConversationService::new();
    let conv_id = ConversationId::from_string(conversation_id);

    match service.delete_conversation(&conv_id).await {
        Ok(()) => Ok(Json(DeletePersistentConversationResponse {
            status: Status::Success,
            error: None,
        })),
        Err(e) => Ok(Json(DeletePersistentConversationResponse {
            status: Status::Error,
            error: Some(format!("Failed to delete conversation: {}", e)),
        })),
    }
}

/// Search persistent conversations by content
pub(crate) async fn search_persistent_conversations(
    Query(params): Query<SearchPersistentConversationsQuery>,
) -> Result<Json<SearchPersistentConversationsResponse>> {
    log::debug!(
        "search_persistent_conversations called with query: '{}'",
        params.query
    );

    let service = ConversationService::new();

    match service.search_conversations(&params.query).await {
        Ok(conversations) => {
            let total = conversations.len();
            Ok(Json(SearchPersistentConversationsResponse {
                status: Status::Success,
                conversations,
                total,
                error: None,
            }))
        }
        Err(e) => Ok(Json(SearchPersistentConversationsResponse {
            status: Status::Error,
            conversations: vec![],
            total: 0,
            error: Some(format!("Failed to search conversations: {}", e)),
        })),
    }
}

/// Export a conversation to JSON
pub(crate) async fn export_persistent_conversation(
    Path(conversation_id): Path<String>,
) -> Result<Json<ExportConversationResponse>> {
    log::info!(
        "export_persistent_conversation called for ID: {}",
        conversation_id
    );

    let service = ConversationService::new();
    let conv_id = ConversationId::from_string(conversation_id);

    match service.export_conversation(&conv_id).await {
        Ok(json_data) => Ok(Json(ExportConversationResponse {
            status: Status::Success,
            json_data: Some(json_data),
            error: None,
        })),
        Err(e) => Ok(Json(ExportConversationResponse {
            status: Status::Error,
            json_data: None,
            error: Some(format!("Failed to export conversation: {}", e)),
        })),
    }
}

/// Import a conversation from JSON
pub(crate) async fn import_persistent_conversation(
    Json(request): Json<ImportConversationRequest>,
) -> Result<Json<ImportConversationResponse>> {
    log::info!("import_persistent_conversation called");

    let service = ConversationService::new();

    match service.import_conversation(&request.json_data).await {
        Ok(conversation) => Ok(Json(ImportConversationResponse {
            status: Status::Success,
            conversation: Some(conversation),
            error: None,
        })),
        Err(e) => Ok(Json(ImportConversationResponse {
            status: Status::Error,
            conversation: None,
            error: Some(format!("Failed to import conversation: {}", e)),
        })),
    }
}

/// Get conversation statistics
pub(crate) async fn get_conversation_statistics() -> Result<Json<ConversationStatisticsResponse>> {
    log::debug!("get_conversation_statistics called");

    let service = ConversationService::new();

    match service.get_statistics().await {
        Ok(statistics) => Ok(Json(ConversationStatisticsResponse {
            status: Status::Success,
            statistics: Some(statistics),
            error: None,
        })),
        Err(e) => Ok(Json(ConversationStatisticsResponse {
            status: Status::Error,
            statistics: None,
            error: Some(format!("Failed to get statistics: {}", e)),
        })),
    }
}
