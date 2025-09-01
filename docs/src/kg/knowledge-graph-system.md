# Terraphim Knowledge Graph System

## Overview

The Terraphim Knowledge Graph (KG) system provides semantic search capabilities by building thesauri from markdown files and using graph-based ranking algorithms. The system converts synonym relationships into graph structures that dramatically improve search relevance and discoverability.

## Architecture Components

### Core Components

1. **Logseq Builder** - Extracts synonyms from markdown files using `synonyms::` syntax
2. **Thesaurus** - Maps synonyms to normalized concept terms with unique IDs
3. **RoleGraph** - Graph structure with nodes, edges, and documents for ranking
4. **TerraphimGraph Relevance Function** - Graph-based scoring algorithm
5. **Knowledge Graph Local** - Local markdown file processing for KG construction

## Knowledge Graph Construction

### Source Files

Knowledge graphs are built from markdown files in `docs/src/kg/`:

```
docs/src/kg/
├── terraphim-graph.md    # Graph architecture concepts
├── service.md           # Service definitions
├── haystack.md          # Haystack integration
├── bug-reporting.md     # Bug reporting terminology and structured analysis
├── issue-tracking.md    # Domain-specific issue tracking terminology
└── [additional KG files]
```

### Synonym Syntax

Markdown files use the `synonyms::` syntax to define concept relationships:

```markdown
# Terraphim-graph

## Terraphim Graph scorer

Terraphim Graph (scorer) is using unique graph embeddings.

synonyms:: graph embeddings, graph, knowledge graph based embeddings

Now we will have a concept "Terraphim Graph Scorer" with synonyms.
```

### Thesaurus Construction

The Logseq builder processes markdown files to create thesaurus mappings:

```rust
let logseq_builder = Logseq::default();
let thesaurus = logseq_builder
    .build("Terraphim Engineer".to_string(), kg_path)
    .await?;
```

**Example thesaurus output:**
```
'terraphim-graph' -> 'terraphim-graph' (ID: 3)
'graph embeddings' -> 'terraphim-graph' (ID: 3)
'graph' -> 'terraphim-graph' (ID: 3)
'knowledge graph based embeddings' -> 'terraphim-graph' (ID: 3)
'haystack' -> 'haystack' (ID: 1)
'service' -> 'service' (ID: 2)
```

## Graph Structure

### RoleGraph Components

The RoleGraph converts thesaurus data into searchable graph structures:

```rust
pub struct RoleGraph {
    pub role: RoleName,
    nodes: AHashMap<u64, Node>,           // Concept nodes
    edges: AHashMap<u64, Edge>,           // Connections between concepts
    documents: AHashMap<String, IndexedDocument>, // Indexed content
    pub thesaurus: Thesaurus,             // Synonym mappings
    pub ac: AhoCorasick,                  // Fast pattern matching
}
```

### Node Structure

Each node represents a concept with connections:

```rust
pub struct Node {
    pub id: u64,                    // Unique concept ID
    pub rank: u64,                  // Importance score
    pub connected_with: HashSet<u64>, // Edge IDs connecting this node
}
```

### Edge Structure

Edges connect concepts and track document associations:

```rust
pub struct Edge {
    pub id: u64,                           // Unique edge ID
    pub rank: u64,                         // Connection strength
    pub doc_hash: HashMap<String, u64>,    // Documents referencing this edge
}
```

## Search and Ranking Algorithm

### TerraphimGraph Relevance Function

The TerraphimGraph relevance function uses graph structure for ranking:

1. **Pattern Matching** - Find synonym matches in query text using Aho-Corasick
2. **Node Discovery** - Map matched terms to concept nodes via thesaurus
3. **Edge Traversal** - Follow connections between related concepts
4. **Rank Calculation** - Combine node rank + edge rank + document rank
5. **Result Aggregation** - Sort by total rank and return top results

### Ranking Formula

```rust
let total_rank = node.rank + edge.rank + document_rank;
```

The ranking rewards:
- **Concept Importance** (node.rank) - How central the concept is
- **Connection Strength** (edge.rank) - How strongly concepts are related
- **Document Relevance** (document_rank) - How relevant the document is

### Query Processing

```rust
pub fn query_graph(
    &self,
    query_string: &str,
    offset: Option<usize>,
    limit: Option<usize>,
) -> Result<Vec<(String, IndexedDocument)>> {
    // 1. Find matching node IDs using Aho-Corasick
    let node_ids = self.find_matching_node_ids(query_string);

    // 2. Traverse graph structure for each matched node
    for node_id in node_ids {
        let node = self.nodes.get(&node_id)?;

        // 3. Follow edges to find connected documents
        for edge_id in &node.connected_with {
            let edge = self.edges.get(edge_id)?;

            // 4. Calculate combined ranking
            for (document_id, document_rank) in &edge.doc_hash {
                let total_rank = node.rank + edge.rank + document_rank;
                // Aggregate results...
            }
        }
    }

    // 5. Sort by rank and return top results
    ranked_documents.sort_by_key(|(_, doc)| std::cmp::Reverse(doc.rank));
    Ok(documents)
}
```

## Performance Characteristics

### Search Performance

Based on comprehensive testing:

- **Initial KG State**: 10 terms, 3 nodes, 5 edges
- **Query Response**: Consistent rank 34 for "terraphim-graph"
- **Search Speed**: Fast pattern matching with Aho-Corasick
- **Memory Efficiency**: Compact graph representation

### Ranking Improvement

Adding synonyms creates dramatic ranking improvements:

| Metric | Before | After | Improvement |
|--------|--------|-------|-------------|
| Thesaurus Terms | 10 | 16 | +60% |
| Graph Nodes | 3 | 4 | +33% |
| Graph Edges | 5 | 8 | +60% |
| "terraphim-graph" Rank | 28 | 117 | **+318%** |

## Role Configuration

### Terraphim Engineer Role

The Terraphim Engineer role uses local KG with TerraphimGraph relevance:

```json
{
  "name": "Terraphim Engineer",
  "relevance_function": "terraphim-graph",
  "kg": {
    "knowledge_graph_local": {
      "input_type": "markdown",
      "path": "docs/src/kg"
    }
  }
}
```

### Local vs Remote Thesaurus

**Local KG (Recommended):**
- Built from `docs/src/kg` markdown files
- 10-16 terms from local content
- Domain-specific, highly relevant
- Fast building (~10 seconds)

**Remote Thesaurus:**
- Downloaded from external URL
- 1,725+ terms from general content
- May miss local domain terms
- Network dependency

## Implementation Examples

### Building Knowledge Graph

```rust
use terraphim_middleware::thesaurus::{Logseq, ThesaurusBuilder};
use terraphim_rolegraph::RoleGraph;
use terraphim_types::RoleName;

// 1. Build thesaurus from local KG files
let logseq_builder = Logseq::default();
let thesaurus = logseq_builder
    .build("Terraphim Engineer".to_string(), kg_path)
    .await?;

// 2. Create rolegraph with thesaurus
let role_name = RoleName::new("Terraphim Engineer");
let mut rolegraph = RoleGraph::new(role_name, thesaurus).await?;

// 3. Index documents into rolegraph
rolegraph.insert_document(&document.id, document);

// 4. Search with graph-based ranking
let results = rolegraph.query_graph("terraphim-graph", Some(0), Some(10))?;
```

### Adding New Knowledge

```rust
// Create new KG file with synonyms
let new_kg_content = r#"
# Graph Analysis

## Advanced Graph Processing

Graph Analysis provides deep insights into data relationships.

synonyms:: data analysis, network analysis, graph processing,
          relationship mapping, connectivity analysis,
          terraphim-graph, graph embeddings

This enhances graph-based system capabilities.
"#;

// Write to KG directory
fs::write(&kg_path.join("graph-analysis.md"), new_kg_content).await?;

// Rebuild thesaurus to include new terms
let expanded_thesaurus = logseq_builder
    .build("Terraphim Engineer".to_string(), &kg_path)
    .await?;
```

### Measuring Graph Growth

```rust
// Measure initial state
let initial_nodes = rolegraph.nodes_map().len();
let initial_edges = rolegraph.edges_map().len();
let initial_terms = thesaurus.len();

// ... add new content and rebuild ...

// Measure growth
let node_growth = expanded_nodes - initial_nodes;
let edge_growth = expanded_edges - initial_edges;
let term_growth = expanded_terms - initial_terms;

println!("Growth: +{} terms, +{} nodes, +{} edges",
    term_growth, node_growth, edge_growth);
```

## Best Practices

### Content Strategy

1. **Domain-Specific Terms** - Use terminology relevant to your domain
2. **Synonym Research** - Include terms users actually search for
3. **Concept Mapping** - Group related terms under common concepts
4. **Strategic Placement** - Add important synonyms to boost key terms

### Performance Optimization

1. **Local KG Preferred** - Use local markdown files for domain relevance
2. **Measured Growth** - Track thesaurus and graph expansion metrics
3. **Test-Driven** - Validate ranking improvements with tests
4. **Incremental Building** - Add synonyms gradually and measure impact

### Testing and Validation

1. **Isolated Testing** - Use temporary directories for safe testing
2. **Baseline Measurement** - Record initial state before changes
3. **Impact Validation** - Verify ranking improvements after additions
4. **Regression Testing** - Ensure changes don't break existing functionality

## Troubleshooting

### Common Issues

**No Search Results:**
- Check if thesaurus contains expected terms
- Verify role uses TerraphimGraph relevance function
- Ensure KG path points to correct directory

**Low Search Rankings:**
- Add more relevant synonyms to target concepts
- Check synonym syntax in markdown files
- Verify graph structure has sufficient connections

**Build Failures:**
- Validate markdown file syntax
- Check file permissions in KG directory
- Ensure Logseq builder has access to files

### Debug Information

```rust
// Print thesaurus contents
for (term, normalized_term) in &thesaurus {
    println!("'{}' -> '{}' (ID: {})",
        term.as_str(),
        normalized_term.value.as_str(),
        normalized_term.id);
}

// Check graph structure
println!("Nodes: {}, Edges: {}, Documents: {}",
    rolegraph.nodes_map().len(),
    rolegraph.edges_map().len(),
    rolegraph.get_all_documents().count());

// Test search functionality
let results = rolegraph.query_graph("test-term", Some(0), Some(5))?;
println!("Search results: {} found", results.len());
```

## Bug Reporting and Issue Tracking Enhancement (2025-01-31)

### Domain-Specific Knowledge Graph Files

The Terraphim KG system has been enhanced with comprehensive bug reporting and issue tracking terminology:

**bug-reporting.md** - Core bug reporting concepts:
- **Steps to Reproduce** - Comprehensive synonyms for reproduction procedures
- **Expected Behaviour** - Terminology for intended system behavior
- **Actual Behaviour** - Variations for describing observed problems
- **Impact Analysis** - Business and operational impact terminology
- **Bug Classification** - Issue categorization and severity terms
- **Quality Assurance** - QA processes and testing terminology

**issue-tracking.md** - Domain-specific terminology:
- **Payroll System Issues** - Salary calculation and compensation problems
- **Data Consistency Problems** - Synchronization and integrity issues
- **HR System Integration** - Human resources system connectivity
- **System Integration Failures** - Cross-system communication problems
- **Performance Degradation** - System slowdown and bottleneck terminology
- **User Experience Issues** - UI/UX problem descriptions

### MCP Integration Testing

Comprehensive test suite validates bug reporting functionality:

**test_bug_report_extraction.rs** - Core functionality testing:
- Extracts **2,615 paragraphs** from comprehensive bug reports
- Extracts **165 paragraphs** from short content scenarios
- Tests all four bug report sections systematically
- Validates connectivity analysis across related terms

**test_kg_term_verification.rs** - Knowledge graph validation:
- **Payroll terms**: 3 suggestions (provider, service, middleware)
- **Data consistency terms**: 9 suggestions (data analysis, network analysis, etc.)
- **Quality assurance terms**: 9 suggestions (connectivity analysis, graph processing, etc.)

### Performance Improvements

The enhanced knowledge graph demonstrates significant improvements in structured document analysis:

- **Semantic Understanding**: Enhanced ability to process structured bug reports using semantic understanding rather than keyword matching
- **Domain Coverage**: Comprehensive terminology coverage for technical documentation and issue tracking
- **Extraction Performance**: Robust paragraph extraction across different content types and sizes
- **Term Recognition**: Effective autocomplete functionality with expanded terminology

## Future Enhancements

### Planned Features

1. **Dynamic KG Updates** - Hot-reload KG changes without restart
2. **Graph Visualization** - Visual representation of concept relationships
3. **Advanced Ranking** - Machine learning-enhanced relevance scoring
4. **Multi-Language Support** - Synonym support for multiple languages
5. **Performance Optimization** - Caching and incremental updates
6. **Domain Expansion** - Additional specialized terminology for specific industries and use cases

### Integration Opportunities

1. **External Ontologies** - Import from RDF/OWL knowledge bases
2. **Collaborative Editing** - Multi-user KG development workflows
3. **Analytics Dashboard** - Search analytics and KG health monitoring
4. **API Extensions** - RESTful APIs for KG management

## Conclusion

The Terraphim Knowledge Graph system provides powerful semantic search capabilities through graph-based ranking. By converting synonym relationships into graph structures, the system dramatically improves search relevance and provides a framework for continuous improvement through strategic content additions.

**Key Benefits:**
- 🔍 **Semantic Search** - Find content by meaning, not just keywords
- 📈 **Ranking Improvement** - Up to 318% ranking boost from synonyms
- 🎯 **Domain Relevance** - Local KG ensures domain-specific accuracy
- 🔧 **Easy Expansion** - Simple markdown syntax for adding knowledge
- 📊 **Measurable Impact** - Comprehensive testing framework for validation

The knowledge graph system forms the foundation for intelligent, context-aware search in the Terraphim AI platform.
