# terraphim_types - Core Type Definitions

## Overview

`terraphim_types` provides fundamental data structures used throughout the Terraphim ecosystem. This crate contains no business logic - it defines the domain models, types, and shared structures that other crates build upon.

## Domain Model

### Core Concepts

#### Role
Represents a user profile or persona with specific knowledge domains, search preferences, and configuration.

```rust
pub struct Role {
    pub shortname: Option<String>,
    pub name: RoleName,
    pub relevance_function: RelevanceFunction,
    pub terraphim_it: bool,
    pub theme: String,
    pub kg: Option<KnowledgeGraph>,
    pub haystacks: Vec<Haystack>,
    pub llm_enabled: bool,
    pub llm_api_key: Option<String>,
    pub llm_model: Option<String>,
    pub llm_auto_summarize: bool,
    pub llm_chat_enabled: bool,
    pub llm_chat_system_prompt: Option<String>,
    pub llm_chat_model: Option<String>,
    pub llm_context_window: Option<u64>,
    pub extra: AHashMap<String, Value>,
    pub llm_router_enabled: bool,
    pub llm_router_config: Option<LlmRouterConfig>,
}
```

**Key Responsibilities:**
- Define user knowledge domains
- Configure search relevance functions
- Manage LLM integration settings
- Specify data sources (haystacks)

#### Document
The central unit of content in Terraphim. Documents come from various sources and are indexed for semantic search.

```rust
pub struct Document {
    pub id: String,
    pub url: String,
    pub title: String,
    pub body: String,
    pub description: Option<String>,
    pub summarization: Option<String>,
    pub stub: Option<String>,
    pub tags: Option<Vec<String>>,
    pub rank: Option<u64>,
    pub source_haystack: Option<String>,
    pub doc_type: DocumentType,
    pub synonyms: Option<Vec<String>>,
    pub route: Option<RouteDirective>,
    pub priority: Option<u8>,
}
```

**Key Responsibilities:**
- Store content and metadata
- Track source and classification
- Maintain search rankings
- Link to knowledge graph concepts

#### Thesaurus
Mapping from normalised terms to concepts, supporting synonyms and URLs.

```rust
pub struct Thesaurus {
    pub name: String,
    pub terms: AHashMap<NormalisedTermValue, NormalisedTerm>,
}
```

**Key Responsibilities:**
- Normalise terminology
- Store concept mappings
- Provide synonym support
- Link concepts to external resources

#### Node
Concept entity in the knowledge graph.

```rust
pub struct Node {
    pub id: u64,
    pub name: String,
    pub description: String,
    pub url: Option<String>,
}
```

**Key Responsibilities:**
- Represent abstract concepts
- Store metadata and descriptions
- Link to external resources

#### Edge
Relationship between two nodes in the knowledge graph.

```rust
pub struct Edge {
    pub id: u64,
    pub from_node_id: u64,
    pub to_node_id: u64,
    pub relationship: String,
}
```

**Key Responsibilities:**
- Define concept relationships
- Enable graph traversal
- Support relationship types

## Data Models

### Normalised Types

#### NormalisedTermValue
A string that has been normalised to lowercase and trimmed.

```rust
pub struct NormalisedTermValue(String);

impl NormalisedTermValue {
    pub fn new(term: String) -> Self;
    pub fn as_str(&self) -> &str;
}
```

**Use Cases:**
- Case-insensitive term matching
- Consistent key generation
- Normalised storage

#### NormalisedTerm
Higher-level term with unique identifier and display values.

```rust
pub struct NormalisedTerm {
    pub id: u64,
    pub value: NormalisedTermValue,
    pub display_value: Option<String>,
    pub url: Option<String>,
}
```

**Use Cases:**
- Unique concept identification
- Preserving original case for display
- Linking concepts to URLs

#### RoleName
Role name with case-insensitive lookup support.

```rust
pub struct RoleName {
    pub original: String,
    pub lowercase: String,
}
```

**Use Cases:**
- User profile identification
- Case-insensitive comparisons
- Preserving display names

### Search Types

#### SearchQuery
Structure for search requests with terms and operators.

```rust
pub struct SearchQuery {
    pub search_term: NormalisedTermValue,
    pub search_terms: Option<Vec<NormalisedTermValue>>,
    pub operator: Option<LogicalOperator>,
    pub skip: Option<u64>,
    pub limit: Option<u64>,
    pub role: Option<RoleName>,
    pub layer: Layer,
    pub include_pinned: bool,
}
```

**Use Cases:**
- Single-term search
- Multi-term boolean search
- Role-scoped queries
- Pagination and layering

#### LogicalOperator
Boolean operators for combining search terms.

```rust
pub enum LogicalOperator {
    And,
    Or,
    Not,
}
```

**Use Cases:**
- Combining search criteria
- Excluding terms
- Complex query building

#### RelevanceFunction
Algorithm for ranking search results.

```rust
pub enum RelevanceFunction {
    TitleScorer,
    BM25,
    BM25F,
    BM25Plus,
    TerraphimGraph,
}
```

**Use Cases:**
- Title matching (`TitleScorer`)
- Statistical ranking (`BM25`, `BM25F`, `BM25Plus`)
- Knowledge graph-based ranking (`TerraphimGraph`)

### Document Types

#### DocumentType
Classification of document types.

```rust
pub enum DocumentType {
    KgEntry,
    Document,
    ConfigDocument,
}
```

**Use Cases:**
- Knowledge graph entries
- Regular documents
- Configuration documents

#### IndexedDocument
Document with search indexes and concept links.

```rust
pub struct IndexedDocument {
    pub document: Document,
    pub index: Index,
    pub connected_node_ids: Vec<u64>,
}
```

**Use Cases:**
- Search-ready documents
- Knowledge graph integration
- Optimised retrieval

### LLM Types

#### Conversation
Chat context with messages and metadata.

```rust
pub struct Conversation {
    pub id: String,
    pub messages: Vec<ChatMessage>,
    pub context_items: Vec<ContextItem>,
    pub role: RoleName,
}
```

**Use Cases:**
- Managing chat history
- Context window management
- Role-specific conversations

#### ChatMessage
Individual message in a conversation.

```rust
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    pub timestamp: Option<i64>,
}
```

**Use Cases:**
- Storing user/assistant messages
- Timestamp tracking
- Conversation flow

#### ContextItem
Fragment of context for LLM requests.

```rust
pub struct ContextItem {
    pub content: String,
    pub source: String,
    pub relevance: f64,
}
```

**Use Cases:**
- Building LLM context
- Ranking context fragments
- Source attribution

### Routing Types

#### RoutingRule
Rule-based LLM provider selection.

```rust
pub struct RoutingRule {
    pub capability: String,
    pub provider: String,
    pub model: String,
    pub priority: Priority,
}
```

**Use Cases:**
- Capability-based routing
- Provider selection
- Model specification

#### RoutingDecision
Result of routing logic.

```rust
pub struct RoutingDecision {
    pub provider: String,
    pub model: String,
    pub reasoning: String,
}
```

**Use Cases:**
- Routing execution results
- Audit trail
- Debug information

#### Priority
Priority levels for routing decisions.

```rust
pub enum Priority {
    High,
    Medium,
    Low,
}
```

**Use Cases:**
- Rule ordering
- Fallback prioritisation
- Resource allocation

### Multi-Agent Types

#### MultiAgentContext
Shared context for coordinated agents.

```rust
pub struct MultiAgentContext {
    pub agents: Vec<AgentInfo>,
    pub shared_state: AHashMap<String, Value>,
    pub tasks: Vec<Task>,
}
```

**Use Cases:**
- Agent coordination
- State sharing
- Task distribution

#### AgentInfo
Information about an agent.

```rust
pub struct AgentInfo {
    pub id: String,
    pub name: String,
    pub capabilities: Vec<String>,
    pub status: AgentStatus,
}
```

**Use Cases:**
- Agent discovery
- Capability matching
- Status tracking

### Dynamic Ontology Types

#### SchemaSignal
Signal indicating schema structure.

```rust
pub struct SchemaSignal {
    pub entities: Vec<String>,
    pub relationships: Vec<String>,
}
```

**Use Cases:**
- Schema discovery
- Ontology learning
- Structure detection

#### ExtractedEntity
Entity extracted from content.

```rust
pub struct ExtractedEntity {
    pub text: String,
    pub type: String,
    pub confidence: f64,
}
```

**Use Cases:**
- Entity recognition
- Confidence scoring
- Type classification

#### CoverageSignal
Signal indicating coverage level.

```rust
pub struct CoverageSignal {
    pub entities: Vec<String>,
    pub coverage: f64,
}
```

**Use Cases:**
- Coverage measurement
- Quality assessment
- Progress tracking

#### GroundingMetadata
Metadata for grounding operations.

```rust
pub struct GroundingMetadata {
    pub sources: Vec<String>,
    pub confidence: f64,
    pub timestamp: i64,
}
```

**Use Cases:**
- Source attribution
- Confidence tracking
- Temporal grounding

## Specialised Modules

### Medical Types (feature: "medical")

#### HgncGene
HGNC gene normalisation data.

```rust
pub struct HgncGene {
    pub hgnc_id: String,
    pub symbol: String,
    pub name: String,
    pub alias_symbols: Vec<String>,
}
```

**Use Cases:**
- Gene normalisation
- Symbol lookup
- Alias expansion

#### HgncNormalizer
Normaliser for HGNC genes.

```rust
pub struct HgncNormalizer {
    pub genes: AHashMap<String, HgncGene>,
}
```

**Use Cases:**
- Consistent gene naming
- Symbol resolution
- Alias matching

### Persona Types

#### PersonaDefinition
Agent persona with characteristics and skills.

```rust
pub struct PersonaDefinition {
    pub name: String,
    pub characteristics: Vec<CharacteristicDef>,
    pub skills: Vec<SfiaSkillDef>,
}
```

**Use Cases:**
- Agent behaviour definition
- Skill specification
- Personality modelling

#### CharacteristicDef
Behavioural characteristic.

```rust
pub struct CharacteristicDef {
    pub name: String,
    pub description: String,
    pub weight: f64,
}
```

**Use Cases:**
- Behaviour shaping
- Weighted characteristics
- Personality traits

#### SfiaSkillDef
SFIA skill definition.

```rust
pub struct SfiaSkillDef {
    pub name: String,
    pub category: String,
    pub proficiency: f64,
}
```

**Use Cases:**
- Skill specification
- Proficiency tracking
- Category organisation

## Implementation Patterns

### Type Safety

- Use `Option<T>` for optional fields
- Use `Result<T, E>` for fallible operations
- Use `Arc<T>` for shared immutable data
- Use `AHashMap<K, V>` for high-performance maps

### Serialisation

- Implement `Serialize` and `Deserialize`
- Use `serde` with sensible defaults
- Support JSON interchange
- Optional TypeScript generation (`tsify` feature)

### Validation

- Builder patterns for complex construction
- Constructor methods with validation
- `new()` and `with_*()` pattern
- Sensible defaults for all fields

## Relationships

### Core Relationships

```
Role 1..* Haystack
Role 1..1 KnowledgeGraph
Document 1..* Tag
Node 1..* Edge
Document 1..* IndexedDocument
Thesaurus 1..* NormalisedTerm
```

### Search Relationships

```
SearchQuery 1..1 NormalisedTermValue
SearchQuery 0..1 LogicalOperator
SearchQuery 0..1 Role
IndexedDocument 1..1 Document
IndexedDocument 0..* Node
```

### LLM Relationships

```
Conversation 1..* ChatMessage
Conversation 1..1 Role
Conversation 1..* ContextItem
RoutingRule 0..* RoutingDecision
RoutingRule 1..1 Priority
```

## Future Extensions

### Planned Additions

- Additional document types
- Enhanced relevance functions
- Richer agent information
- More sophisticated routing rules
- Extended ontology types

### Compatibility

- Maintain backward compatibility
- Versioned schema evolution
- Migration utilities
- Deprecation warnings

## Best Practices

### Type Usage

- Prefer explicit types over dynamic values
- Use `Option<T>` for nullable fields
- Document invariants in comments
- Provide builder methods for complex types

### Serialisation

- Use sensible serde defaults
- Handle missing fields gracefully
- Provide human-readable JSON
- Support feature-gated optional fields

### Error Handling

- Use `thiserror` for error types
- Provide context in error messages
- Categorise errors for handling
- Support conversion between error types

## Testing

### Test Coverage

- Unit tests for all types
- Serialisation/deserialisation tests
- Edge case handling
- Integration with dependent crates

### Test Patterns

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalized_term_value() {
        let value = NormalisedTermValue::new("  Test  ".to_string());
        assert_eq!(value.as_str(), "test");
    }

    #[test]
    fn test_role_name_case_insensitive() {
        let role1 = RoleName::new("DataScientist");
        let role2 = RoleName::new("datascientist");
        assert_eq!(role1.as_lowercase(), role2.as_lowercase());
    }
}
```

## References

- [Serde documentation](https://serde.rs/)
- [Schemars for JSON Schema](https://github.com/GREsau/schemars)
- [TSify for TypeScript](https://github.com/madonohashi/tsify)
- [ahash for fast hash maps](https://github.com/tkaitch/ahash)
