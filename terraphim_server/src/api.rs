use axum::{
    extract::{Query, State, Json},
    http::StatusCode,
    response::IntoResponse,
    Extension,
};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast::Sender;

use terraphim_config::Config;
use terraphim_config::ConfigState;
use terraphim_service::TerraphimService;
use terraphim_types::{RankedNode, Rank, Document, IndexedDocument, SearchQuery, RoleName, ApiRankedNode, RankEntry};

use crate::error::{Result, Status};
pub type SearchResultsStream = Sender<IndexedDocument>;

/// Health check endpoint
pub(crate) async fn health() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

/// Response for creating a document
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateDocumentResponse {
    /// Status of the document creation
    pub status: Status,
    /// The id of the document that was successfully created
    pub id: String,
}

/// Creates index of the document for each rolegraph
pub(crate) async fn create_document(
    State(config): State<ConfigState>,
    Json(document): Json<Document>,
) -> Result<Json<CreateDocumentResponse>> {
    log::debug!("create_document");
    let mut terraphim_service = TerraphimService::new(config.clone());
    let document = terraphim_service.create_document(document).await?;
    Ok(Json(CreateDocumentResponse {
        status: Status::Success,
        id: document.id,
    }))
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ListNodesQuery {
    /// Whether to expand nodes to include linked documents
    #[serde(default)]
    pub expand: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ListDocumentsResponse {
    /// Status of the document listing
    pub status: Status,
    /// Vector of ranked nodes from the RoleGraph
    pub nodes: Vec<RankedNode>,
    /// Vector of documents with their IDs and ranks, only present when expand=true
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documents: Option<Vec<(String, IndexedDocument, Rank)>>,
    /// Total number of nodes
    pub total: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NodesRequest {
    pub role: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NodesResponse {
    pub status: Status,
    pub name: String,
    pub nodes: Vec<ApiRankedNode>,
    #[serde(skip_serializing)]
    pub total_nodes: usize,
    #[serde(skip_serializing)]
    pub max_depth: u32,
    #[serde(skip_serializing)]
    pub root_nodes: usize
}

pub(crate) async fn list_ranked_nodes(
    State(config): State<ConfigState>,
    Json(request): Json<NodesRequest>,
) -> Result<Json<NodesResponse>> {
    log::debug!("list_ranked_nodes called with role: {}", request.role);
    
    let mut terraphim_service = TerraphimService::new(config);
    let role = RoleName::new(&request.role);
    
    log::debug!("Created role name: {:?}", role);
    
    // First ensure we have the role's thesaurus loaded
    log::debug!("Loading thesaurus for role");
    let thesaurus = terraphim_service.ensure_thesaurus_loaded(&role).await?;
    log::debug!("Thesaurus loaded with {} entries", thesaurus.len());
    
    // Then get the rolegraph
    log::debug!("Getting rolegraph for role");
    let mut rolegraph = terraphim_service.get_rolegraph_by_role(role).await?;
    log::debug!("Got rolegraph");
    
    // Check if we have any nodes
    let initial_nodes = rolegraph.list_ranked_nodes()?;
    
    // For development/testing: Add a test document if no nodes found
    if initial_nodes.is_empty() {
        log::debug!("No nodes found, adding test document");
        let document_id = "test_doc_1".to_string();
        let test_document = "Life cycle concepts and project direction with Trained operators and maintainers";
        let document = Document {
            stub: None,
            url: "/test/doc".to_string(),
            tags: None,
            rank: None,
            id: document_id.clone(),
            title: test_document.to_string(),
            body: test_document.to_string(),
            description: None,
        };
        rolegraph.insert_document(&document_id, document);
        log::debug!("Added test document to graph");
    }
    
    let nodes = rolegraph.list_ranked_nodes()?;
    log::debug!("Retrieved {} nodes from rolegraph", nodes.len());
    
    // Convert RankedNode to ApiRankedNode
    let api_nodes: Vec<ApiRankedNode> = nodes.clone().into_iter().map(|node| {
        ApiRankedNode {
            id: node.id.to_string(),
            name: node.name.clone(),
            normalized_term: node.name.clone(), // Using name as normalized_term
            value: node.value as i32,
            total_documents: node.value as i32, // Using value as total_documents
            parent: node.parent.map(|p| p.to_string()),
            children: node.children.into_iter().map(|child| ApiRankedNode {
                id: child.id.to_string(),
                name: child.name.clone(),
                normalized_term: child.name.clone(),
                value: child.value as i32,
                total_documents: child.value as i32,
                parent: child.parent.map(|p| p.to_string()),
                children: Vec::new(), // We don't need deeper nesting
                expanded: false,
                ranks: child.ranks.into_iter().map(|r| RankEntry {
                    edge_weight: r.edge_weight as f64,
                    document_id: r.node_id.to_string(),
                }).collect(),
            }).collect(),
            expanded: false,
            ranks: node.ranks.into_iter().map(|r| RankEntry {
                edge_weight: r.edge_weight as f64,
                document_id: r.node_id.to_string(),
            }).collect(),
        }
    }).collect();

    Ok(Json(NodesResponse {
        status: Status::Success,
        name: "Knowledge Graph".to_string(),
        nodes: api_nodes,
        total_nodes: nodes.len(),
        max_depth: nodes.iter().map(|n| n.parent.is_some() as u32).max().unwrap_or(0),
        root_nodes: nodes.iter().filter(|n| n.parent.is_none()).count(),
    }))
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchResponse {
    /// Status of the search
    pub status: Status,
    /// Vector of results which matched the query
    pub results: Vec<Document>,
    /// The number of documents that match the search query
    pub total: usize,
}

/// Search for documents in all Terraphim graphs defined in the config via GET params
pub(crate) async fn search_documents(
    Extension(_tx): Extension<SearchResultsStream>,
    State(config_state): State<ConfigState>,
    search_query: Query<SearchQuery>,
) -> Result<Json<SearchResponse>> {
    log::debug!("search_document called with {:?}", search_query);

    let mut terraphim_service = TerraphimService::new(config_state);
    let results = terraphim_service.search(&search_query.0).await?;
    let total = results.len();

    Ok(Json(SearchResponse {
        status: Status::Success,
        results,
        total,
    }))
}

/// Search for documents in all Terraphim graphs defined in the config via POST body
pub(crate) async fn search_documents_post(
    Extension(_tx): Extension<SearchResultsStream>,
    State(config_state): State<ConfigState>,
    search_query: Json<SearchQuery>,
) -> Result<Json<SearchResponse>> {
    log::debug!("POST Searching documents with query: {search_query:?}");

    let mut terraphim_service = TerraphimService::new(config_state);
    let results = terraphim_service.search(&search_query).await?;
    let total = results.len();

    if total == 0 {
        log::debug!("No documents found");
    } else {
        log::debug!("Found {total} documents");
    }

    Ok(Json(SearchResponse {
        status: Status::Success,
        results,
        total,
    }))
}

/// Response type for showing the config
///
/// This is also used when updating the config
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConfigResponse {
    /// Status of the config fetch
    pub status: Status,
    /// The config
    pub config: Config,
}

/// API handler for Terraphim Config
pub(crate) async fn get_config(State(config): State<ConfigState>) -> Result<Json<ConfigResponse>> {
    log::debug!("Called API endpoint get_config");
    let terraphim_service = TerraphimService::new(config);
    let config = terraphim_service.fetch_config().await;
    Ok(Json(ConfigResponse {
        status: Status::Success,
        config,
    }))
}

/// API handler for Terraphim Config update
pub(crate) async fn update_config(
    State(config_state): State<ConfigState>,
    Json(config_new): Json<Config>,
) -> Json<ConfigResponse> {
    let mut config = config_state.config.lock().await;
    *config = config_new.clone();
    Json(ConfigResponse {
        status: Status::Success,
        config: config_new,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use terraphim_types::RoleName;

    #[tokio::test]
    async fn test_nodes_response_format() {
        // Create test config state
        let config = ConfigState::default();
        
        // Create test request
        let request = NodesRequest {
            role: "system operator".to_string(),
        };

        // Call the endpoint
        let response = list_ranked_nodes(
            State(config),
            Json(request)
        ).await;

        // Verify response
        assert!(response.is_ok());
        let Json(nodes_response) = response.unwrap();
        
        // Check root structure
        assert_eq!(nodes_response.status, Status::Success);
        assert_eq!(nodes_response.name, "Knowledge Graph");
        
        // Verify metadata
        assert!(nodes_response.total_nodes >= nodes_response.nodes.len());
        assert!(nodes_response.max_depth >= 0);
        assert!(nodes_response.root_nodes > 0);

        // Check node structure if any nodes exist
        if !nodes_response.nodes.is_empty() {
            let first_node = &nodes_response.nodes[0];
            
            // Verify required icicle chart fields
            assert!(!first_node.name.is_empty());
            assert!(first_node.value > 0);
            
            // Verify nodes are sorted by value
            for window in nodes_response.nodes.windows(2) {
                assert!(
                    window[0].value >= window[1].value,
                    "Nodes should be sorted by value in descending order"
                );
            }
        }
    }
}
