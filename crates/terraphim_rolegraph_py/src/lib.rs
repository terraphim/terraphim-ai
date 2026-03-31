use std::collections::{HashMap, HashSet};

use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use terraphim_automata::load_thesaurus_from_json;
use terraphim_types::LogicalOperator;

// ---------------------------------------------------------------------------
// Wrapper types
// ---------------------------------------------------------------------------

/// A node in the knowledge graph representing a single concept.
#[pyclass(name = "Node")]
#[derive(Clone)]
pub struct PyNode {
    #[pyo3(get)]
    pub id: u64,
    #[pyo3(get)]
    pub rank: u64,
    #[pyo3(get)]
    pub connected_with: HashSet<u64>,
}

impl From<&::terraphim_types::Node> for PyNode {
    fn from(node: &::terraphim_types::Node) -> Self {
        PyNode {
            id: node.id,
            rank: node.rank,
            connected_with: node.connected_with.clone(),
        }
    }
}

#[pymethods]
impl PyNode {
    fn __repr__(&self) -> String {
        format!(
            "Node(id={}, rank={}, connections={})",
            self.id,
            self.rank,
            self.connected_with.len()
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

/// An edge in the knowledge graph connecting two concepts.
#[pyclass(name = "Edge")]
#[derive(Clone)]
pub struct PyEdge {
    #[pyo3(get)]
    pub id: u64,
    #[pyo3(get)]
    pub rank: u64,
    #[pyo3(get)]
    pub doc_hash: HashMap<String, u64>,
}

impl From<&::terraphim_types::Edge> for PyEdge {
    fn from(edge: &::terraphim_types::Edge) -> Self {
        PyEdge {
            id: edge.id,
            rank: edge.rank,
            doc_hash: edge.doc_hash.iter().map(|(k, v)| (k.clone(), *v)).collect(),
        }
    }
}

#[pymethods]
impl PyEdge {
    fn __repr__(&self) -> String {
        format!(
            "Edge(id={}, rank={}, documents={})",
            self.id,
            self.rank,
            self.doc_hash.len()
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

/// An indexed document with graph metadata (edges, rank, tags, nodes).
#[pyclass(name = "IndexedDocument")]
#[derive(Clone)]
pub struct PyIndexedDocument {
    #[pyo3(get)]
    pub id: String,
    #[pyo3(get)]
    pub matched_edges: Vec<PyEdge>,
    #[pyo3(get)]
    pub rank: u64,
    #[pyo3(get)]
    pub tags: Vec<String>,
    #[pyo3(get)]
    pub nodes: Vec<u64>,
}

impl From<&::terraphim_types::IndexedDocument> for PyIndexedDocument {
    fn from(doc: &::terraphim_types::IndexedDocument) -> Self {
        PyIndexedDocument {
            id: doc.id.clone(),
            matched_edges: doc.matched_edges.iter().map(PyEdge::from).collect(),
            rank: doc.rank,
            tags: doc.tags.clone(),
            nodes: doc.nodes.clone(),
        }
    }
}

#[pymethods]
impl PyIndexedDocument {
    fn __repr__(&self) -> String {
        format!(
            "IndexedDocument(id='{}', rank={}, tags={:?})",
            self.id, self.rank, self.tags
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

/// A document to insert into the knowledge graph.
#[pyclass(name = "Document")]
#[derive(Clone)]
pub struct PyDocument {
    #[pyo3(get, set)]
    pub id: String,
    #[pyo3(get, set)]
    pub url: String,
    #[pyo3(get, set)]
    pub title: String,
    #[pyo3(get, set)]
    pub body: String,
    #[pyo3(get, set)]
    pub description: Option<String>,
    #[pyo3(get, set)]
    pub tags: Option<Vec<String>>,
    #[pyo3(get, set)]
    pub rank: Option<u64>,
}

#[pymethods]
impl PyDocument {
    #[new]
    #[pyo3(signature = (id, url, title, body, description=None, tags=None, rank=None))]
    fn new(
        id: String,
        url: String,
        title: String,
        body: String,
        description: Option<String>,
        tags: Option<Vec<String>>,
        rank: Option<u64>,
    ) -> Self {
        PyDocument {
            id,
            url,
            title,
            body,
            description,
            tags,
            rank,
        }
    }

    fn __repr__(&self) -> String {
        format!("Document(id='{}', title='{}')", self.id, self.title)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

impl From<PyDocument> for ::terraphim_types::Document {
    fn from(doc: PyDocument) -> Self {
        ::terraphim_types::Document {
            id: doc.id,
            url: doc.url,
            title: doc.title,
            body: doc.body,
            description: doc.description,
            summarization: None,
            stub: None,
            tags: doc.tags,
            rank: doc.rank,
            source_haystack: None,
            doc_type: Default::default(),
            synonyms: None,
            route: None,
            priority: None,
        }
    }
}

/// Statistics about the graph structure.
#[pyclass(name = "GraphStats")]
#[derive(Clone)]
pub struct PyGraphStats {
    #[pyo3(get)]
    pub node_count: usize,
    #[pyo3(get)]
    pub edge_count: usize,
    #[pyo3(get)]
    pub document_count: usize,
    #[pyo3(get)]
    pub thesaurus_size: usize,
    #[pyo3(get)]
    pub is_populated: bool,
}

#[pymethods]
impl PyGraphStats {
    fn __repr__(&self) -> String {
        format!(
            "GraphStats(nodes={}, edges={}, documents={}, thesaurus={}, populated={})",
            self.node_count,
            self.edge_count,
            self.document_count,
            self.thesaurus_size,
            self.is_populated
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

/// Logical operator for combining search terms.
#[pyclass(name = "LogicalOperator", eq, eq_int)]
#[derive(Clone, PartialEq)]
pub enum PyLogicalOperator {
    And = 0,
    Or = 1,
}

impl From<&PyLogicalOperator> for LogicalOperator {
    fn from(op: &PyLogicalOperator) -> Self {
        match op {
            PyLogicalOperator::And => LogicalOperator::And,
            PyLogicalOperator::Or => LogicalOperator::Or,
        }
    }
}

// ---------------------------------------------------------------------------
// Main class: PyRoleGraph
// ---------------------------------------------------------------------------

/// A knowledge graph for a specific role, supporting document insertion,
/// graph queries, path connectivity checks, and serialization.
#[pyclass(name = "RoleGraph")]
pub struct PyRoleGraph {
    inner: ::terraphim_rolegraph::RoleGraph,
}

#[pymethods]
impl PyRoleGraph {
    /// Create a new RoleGraph with a role name and thesaurus JSON.
    ///
    /// Args:
    ///     role_name: Name of the role (e.g. "engineer", "data_scientist")
    ///     thesaurus_json: JSON string with thesaurus data
    #[new]
    fn new(role_name: &str, thesaurus_json: &str) -> PyResult<Self> {
        let thesaurus = load_thesaurus_from_json(thesaurus_json)
            .map_err(|e| PyValueError::new_err(format!("Failed to load thesaurus: {}", e)))?;
        let inner = ::terraphim_rolegraph::RoleGraph::new(role_name.into(), thesaurus)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to create RoleGraph: {}", e)))?;
        Ok(PyRoleGraph { inner })
    }

    // -- Document methods ---------------------------------------------------

    /// Insert a document into the knowledge graph.
    fn insert_document(&mut self, document_id: &str, document: PyDocument) {
        self.inner.insert_document(document_id, document.into());
    }

    /// Check if a document is already indexed.
    fn has_document(&self, document_id: &str) -> bool {
        self.inner.has_document(document_id)
    }

    /// Get an indexed document by ID, or None if not found.
    fn get_document(&self, document_id: &str) -> Option<PyIndexedDocument> {
        self.inner
            .get_document(document_id)
            .map(PyIndexedDocument::from)
    }

    /// Get all indexed documents as a list of (id, IndexedDocument) tuples.
    fn get_all_documents(&self) -> Vec<(String, PyIndexedDocument)> {
        self.inner
            .get_all_documents()
            .map(|(k, v)| (k.clone(), PyIndexedDocument::from(v)))
            .collect()
    }

    /// Get the number of indexed documents.
    fn document_count(&self) -> usize {
        self.inner.document_count()
    }

    /// Find all document IDs that contain a specific term.
    fn find_document_ids_for_term(&self, term: &str) -> Vec<String> {
        self.inner.find_document_ids_for_term(term)
    }

    // -- Query methods ------------------------------------------------------

    /// Find matching node IDs for the given text.
    fn find_matching_node_ids(&self, text: &str) -> Vec<u64> {
        self.inner.find_matching_node_ids(text)
    }

    /// Check if all matched terms in the text are connected by a single path.
    fn is_all_terms_connected_by_path(&self, text: &str) -> bool {
        self.inner.is_all_terms_connected_by_path(text)
    }

    /// Query the graph with a search string, returning ranked documents.
    ///
    /// Args:
    ///     query_string: The search query
    ///     offset: Number of results to skip (default: None)
    ///     limit: Maximum number of results (default: None)
    ///
    /// Returns:
    ///     List of (document_id, IndexedDocument) tuples sorted by rank
    #[pyo3(signature = (query_string, offset=None, limit=None))]
    fn query_graph(
        &self,
        query_string: &str,
        offset: Option<usize>,
        limit: Option<usize>,
    ) -> PyResult<Vec<(String, PyIndexedDocument)>> {
        self.inner
            .query_graph(query_string, offset, limit)
            .map(|results| {
                results
                    .into_iter()
                    .map(|(id, doc)| (id, PyIndexedDocument::from(&doc)))
                    .collect()
            })
            .map_err(|e| PyRuntimeError::new_err(format!("Query failed: {}", e)))
    }

    /// Query the graph with multiple terms and a logical operator (AND/OR).
    ///
    /// Args:
    ///     search_terms: List of search terms
    ///     operator: LogicalOperator.And or LogicalOperator.Or
    ///     offset: Number of results to skip (default: None)
    ///     limit: Maximum number of results (default: None)
    #[pyo3(signature = (search_terms, operator, offset=None, limit=None))]
    fn query_graph_with_operators(
        &self,
        search_terms: Vec<String>,
        operator: &PyLogicalOperator,
        offset: Option<usize>,
        limit: Option<usize>,
    ) -> PyResult<Vec<(String, PyIndexedDocument)>> {
        let terms_refs: Vec<&str> = search_terms.iter().map(String::as_str).collect();
        let op: LogicalOperator = operator.into();
        self.inner
            .query_graph_with_operators(&terms_refs, &op, offset, limit)
            .map(|results| {
                results
                    .into_iter()
                    .map(|(id, doc)| (id, PyIndexedDocument::from(&doc)))
                    .collect()
            })
            .map_err(|e| PyRuntimeError::new_err(format!("Query failed: {}", e)))
    }

    // -- Stats methods ------------------------------------------------------

    /// Get the number of nodes in the graph.
    fn get_node_count(&self) -> usize {
        self.inner.get_node_count()
    }

    /// Get the number of edges in the graph.
    fn get_edge_count(&self) -> usize {
        self.inner.get_edge_count()
    }

    /// Get the number of documents in the graph.
    fn get_document_count(&self) -> usize {
        self.inner.get_document_count()
    }

    /// Check if the graph has been properly populated with nodes, edges, and documents.
    fn is_graph_populated(&self) -> bool {
        self.inner.is_graph_populated()
    }

    /// Get comprehensive graph statistics.
    fn get_graph_stats(&self) -> PyGraphStats {
        let stats = self.inner.get_graph_stats();
        PyGraphStats {
            node_count: stats.node_count,
            edge_count: stats.edge_count,
            document_count: stats.document_count,
            thesaurus_size: stats.thesaurus_size,
            is_populated: stats.is_populated,
        }
    }

    /// Validate documents and return a list of warning messages.
    fn validate_documents(&self) -> Vec<String> {
        self.inner.validate_documents()
    }

    // -- Inspection methods -------------------------------------------------

    /// Get all nodes as a dict mapping node_id -> Node.
    fn nodes_map(&self) -> HashMap<u64, PyNode> {
        self.inner
            .nodes_map()
            .iter()
            .map(|(k, v)| (*k, PyNode::from(v)))
            .collect()
    }

    /// Get all edges as a dict mapping edge_id -> Edge.
    fn edges_map(&self) -> HashMap<u64, PyEdge> {
        self.inner
            .edges_map()
            .iter()
            .map(|(k, v)| (*k, PyEdge::from(v)))
            .collect()
    }

    // -- Serialization ------------------------------------------------------

    /// Serialize the RoleGraph to a JSON string.
    fn to_json(&self) -> PyResult<String> {
        let serializable = self.inner.to_serializable();
        serializable
            .to_json()
            .map_err(|e| PyRuntimeError::new_err(format!("Serialization failed: {}", e)))
    }

    /// Serialize the RoleGraph to a pretty-printed JSON string.
    fn to_json_pretty(&self) -> PyResult<String> {
        let serializable = self.inner.to_serializable();
        serializable
            .to_json_pretty()
            .map_err(|e| PyRuntimeError::new_err(format!("Serialization failed: {}", e)))
    }

    /// Deserialize a RoleGraph from a JSON string.
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let serializable = ::terraphim_rolegraph::SerializableRoleGraph::from_json(json)
            .map_err(|e| PyValueError::new_err(format!("Failed to parse JSON: {}", e)))?;
        let inner = ::terraphim_rolegraph::RoleGraph::from_serializable_sync(serializable)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to restore RoleGraph: {}", e)))?;
        Ok(PyRoleGraph { inner })
    }

    // -- Properties ---------------------------------------------------------

    /// Get the role name.
    #[getter]
    fn role(&self) -> String {
        self.inner.role.original.clone()
    }

    // -- Dunder methods -----------------------------------------------------

    fn __repr__(&self) -> String {
        format!(
            "RoleGraph(role='{}', documents={})",
            self.inner.role.original,
            self.inner.document_count()
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }

    fn __len__(&self) -> usize {
        self.inner.document_count()
    }
}

// ---------------------------------------------------------------------------
// Free functions
// ---------------------------------------------------------------------------

/// Combine two node IDs into a unique edge ID using elegant pairing.
#[pyfunction]
fn magic_pair(x: u64, y: u64) -> u64 {
    ::terraphim_rolegraph::magic_pair(x, y)
}

/// Reverse an edge ID back into two node IDs.
#[pyfunction]
fn magic_unpair(z: u64) -> (u64, u64) {
    ::terraphim_rolegraph::magic_unpair(z)
}

/// Split text into paragraphs using unicode sentence boundary detection.
#[pyfunction]
fn split_paragraphs(text: &str) -> Vec<String> {
    ::terraphim_rolegraph::split_paragraphs(text)
        .into_iter()
        .map(|s| s.to_string())
        .collect()
}

// ---------------------------------------------------------------------------
// Module registration
// ---------------------------------------------------------------------------

/// Python module for Terraphim RoleGraph
///
/// This module provides knowledge graph operations for AI agents,
/// including document insertion, graph queries, path connectivity,
/// serialization, and statistics.
#[pymodule]
fn terraphim_rolegraph(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyRoleGraph>()?;
    m.add_class::<PyDocument>()?;
    m.add_class::<PyIndexedDocument>()?;
    m.add_class::<PyNode>()?;
    m.add_class::<PyEdge>()?;
    m.add_class::<PyGraphStats>()?;
    m.add_class::<PyLogicalOperator>()?;
    m.add_function(wrap_pyfunction!(magic_pair, m)?)?;
    m.add_function(wrap_pyfunction!(magic_unpair, m)?)?;
    m.add_function(wrap_pyfunction!(split_paragraphs, m)?)?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
