# Knowledge Graph

The Terraphim Knowledge Graph system provides semantic understanding and relationship mapping between concepts, enabling advanced document ranking and search capabilities.

## Architecture

### Core Components

#### Knowledge Graph Structure
```rust
pub struct RoleGraph {
    pub nodes: HashMap<u64, Node>,
    pub edges: HashMap<u64, Edge>,
    pub thesaurus: Thesaurus,
}
```

**Key Features**:
- Node-based concept representation
- Edge-based relationship mapping
- Thesaurus for synonym management
- Rank-based scoring system

#### Node Representation
```rust
pub struct Node {
    pub id: u64,
    pub rank: u64,
    pub connected_with: HashSet<u64>,
}
```

#### Edge Representation
```rust
pub struct Edge {
    pub id: u64,
    pub rank: u64,
    pub doc_hash: AHashMap<String, u64>,
}
```

## Graph Construction

### From Documents
```rust
use terraphim_rolegraph::RoleGraph;

let documents = load_documents_from_path("./docs/src/kg")?;
let graph = RoleGraph::from_documents(&documents)?;

println!("Graph contains {} nodes and {} edges",
    graph.nodes.len(), graph.edges.len());
```

### From Markdown Files
```rust
// Build graph from markdown files
let graph = RoleGraph::from_markdown_files("./docs/src/kg")?;

// Extract concepts and relationships
for (node_id, node) in &graph.nodes {
    println!("Node {}: rank {}", node_id, node.rank);
}
```

### From JSON Thesaurus
```rust
// Load existing thesaurus
let thesaurus = Thesaurus::from_file("./docs/src/kg/thesaurus.json")?;

// Build graph from thesaurus
let graph = RoleGraph::from_thesaurus(&thesaurus)?;
```

## Graph Traversal

### Node Traversal
```rust
// Find connected nodes
let connected_nodes = graph.get_connected_nodes(node_id)?;

// Get node rank
let rank = graph.get_node_rank(node_id)?;

// Find high-ranking nodes
let top_nodes = graph.get_top_ranked_nodes(10)?;
```

### Edge Traversal
```rust
// Find edges for a node
let edges = graph.get_node_edges(node_id)?;

// Get edge rank
let edge_rank = graph.get_edge_rank(edge_id)?;

// Find documents connected to edge
let documents = graph.get_edge_documents(edge_id)?;
```

### Path Finding
```rust
// Find shortest path between nodes
let path = graph.find_shortest_path(start_node, end_node)?;

// Find all paths between nodes
let paths = graph.find_all_paths(start_node, end_node)?;

// Calculate path weight
let weight = graph.calculate_path_weight(&path)?;
```

## Thesaurus Management

### Synonym Management
```rust
use terraphim_types::{Thesaurus, NormalizedTerm, NormalizedTermValue};

let mut thesaurus = Thesaurus::new("Default".to_string());

// Add normalized term
let term = NormalizedTerm::new(
    1,
    NormalizedTermValue::from("rust-programming".to_string())
);
thesaurus.insert(NormalizedTermValue::from("rust".to_string()), term);

// Add synonym
let synonym = NormalizedTerm::new(
    2,
    NormalizedTermValue::from("systems-programming".to_string())
);
thesaurus.insert(NormalizedTermValue::from("systems".to_string()), synonym);
```

### Term Lookup
```rust
// Find term by value
let term = thesaurus.get(&NormalizedTermValue::from("rust"))?;

// Get all terms
let all_terms: Vec<_> = thesaurus.keys().collect();

// Check if term exists
if thesaurus.contains_key(&NormalizedTermValue::from("rust")) {
    println!("Term 'rust' found in thesaurus");
}
```

## Integration with Scoring

### TerraphimGraph Relevance Function
```rust
// Use knowledge graph for document ranking
let role = Role {
    name: RoleName::from("Engineer"),
    relevance_function: RelevanceFunction::TerraphimGraph,
    terraphim_it: true,
    kg_path: Some("./docs/src/kg".to_string()),
    // ... other fields
};

// Search with graph-based ranking
let results = service.search(&search_query).await?;
```

### Graph-Based Document Ranking
```rust
// Rank documents using knowledge graph
let ranked_docs = graph.rank_documents(&documents, &query)?;

// Get graph-based scores
for doc in &ranked_docs {
    let graph_score = graph.calculate_document_score(doc, &query)?;
    println!("Document {}: graph score {}", doc.title, graph_score);
}
```

## Document Preprocessing

### KG Auto-Linking
```rust
// Apply knowledge graph preprocessing
if role.terraphim_it {
    let enhanced_doc = service.preprocess_document_content(doc, &role).await?;

    // Document now contains KG links
    println!("Enhanced document: {}", enhanced_doc.body);
}
```

### Term Replacement
```rust
use terraphim_automata::matcher;

// Replace terms with KG links
let text = "Rust is a systems programming language";
let thesaurus = load_knowledge_graph_terms()?;

let enhanced_text = matcher::replace_matches(&text, &thesaurus, Format::Wiki)?;
// Result: "[[Rust]] is a [[systems-programming]] language"
```

## Graph Analysis

### Centrality Analysis
```rust
// Calculate node centrality
let centrality = graph.calculate_node_centrality(node_id)?;

// Find most central nodes
let central_nodes = graph.find_most_central_nodes(5)?;
```

### Community Detection
```rust
// Detect communities in the graph
let communities = graph.detect_communities()?;

// Get community for a node
let community = graph.get_node_community(node_id)?;
```

### Graph Metrics
```rust
// Calculate graph density
let density = graph.calculate_density()?;

// Calculate average node degree
let avg_degree = graph.calculate_average_degree()?;

// Calculate graph diameter
let diameter = graph.calculate_diameter()?;
```

## Performance Optimization

### Graph Caching
```rust
// Cache graph for performance
let cached_graph = RoleGraph::cached_from_path("./docs/src/kg")?;

// Use cached graph for searches
let results = cached_graph.search_documents(&query)?;
```

### Incremental Updates
```rust
// Update graph incrementally
graph.add_document(&new_document)?;

// Remove outdated nodes
graph.remove_node(outdated_node_id)?;

// Recalculate ranks
graph.recalculate_ranks()?;
```

## Configuration

### Graph Settings
```json
{
  "kg_path": "./docs/src/kg",
  "thesaurus_file": "thesaurus.json",
  "auto_linking": true,
  "min_rank_threshold": 5,
  "max_nodes": 10000,
  "cache_enabled": true
}
```

### Role Configuration
```json
{
  "name": "Engineer",
  "relevance_function": "terraphim-graph",
  "terraphim_it": true,
  "kg_path": "./docs/src/kg",
  "auto_linking": true
}
```

## Testing

### Graph Construction Tests
```rust
#[test]
fn test_graph_construction() {
    let documents = create_test_documents();
    let graph = RoleGraph::from_documents(&documents)?;

    assert!(graph.nodes.len() > 0);
    assert!(graph.edges.len() > 0);
    assert!(graph.thesaurus.len() > 0);
}
```

### Traversal Tests
```rust
#[test]
fn test_graph_traversal() {
    let graph = create_test_graph();

    // Test node traversal
    let connected = graph.get_connected_nodes(1)?;
    assert!(!connected.is_empty());

    // Test path finding
    let path = graph.find_shortest_path(1, 5)?;
    assert!(!path.is_empty());
}
```

### Integration Tests
```rust
#[tokio::test]
async fn test_kg_integration() {
    let config_state = create_test_config_state();
    let mut service = TerraphimService::new(config_state);

    let search_query = SearchQuery {
        search_term: NormalizedTermValue::from("rust"),
        skip: None,
        limit: Some(10),
        role: Some(RoleName::from("Engineer")),
    };

    let results = service.search(&search_query).await?;
    assert!(!results.is_empty());
}
```

## Best Practices

### Graph Construction
1. **Use meaningful terms**: Avoid generic terms like "the", "and", "or"
2. **Normalize terms**: Use consistent term formatting
3. **Set appropriate ranks**: Higher ranks for more important concepts
4. **Maintain relationships**: Ensure edges reflect meaningful connections

### Performance
1. **Cache graphs**: Use cached graphs for repeated searches
2. **Incremental updates**: Update graphs incrementally when possible
3. **Limit graph size**: Set reasonable limits for graph size
4. **Optimize queries**: Use efficient traversal algorithms

### Integration
1. **Test with real data**: Use actual document collections for testing
2. **Monitor performance**: Track graph construction and traversal times
3. **Validate results**: Ensure graph-based ranking improves search quality
4. **Update regularly**: Keep graphs current with document changes

## Future Enhancements

### Planned Features
1. **Dynamic graph updates**: Real-time graph modification
2. **Advanced algorithms**: PageRank, HITS, and other centrality measures
3. **Graph visualization**: Interactive graph exploration tools
4. **Machine learning**: ML-based relationship discovery
5. **Multi-language support**: Internationalization for graph terms

### Research Areas
1. **Graph embeddings**: Vector representations of graph nodes
2. **Semantic similarity**: Advanced similarity measures
3. **Graph neural networks**: Deep learning for graph analysis
4. **Temporal graphs**: Time-aware graph structures
5. **Heterogeneous graphs**: Multi-type node and edge support
