# Summary: terraphim_types/src/lib.rs

## Purpose

Core type definitions for the Terraphim AI system. Provides fundamental data structures for knowledge graphs, documents, search, conversations, LLM routing, multi-agent coordination, and dynamic ontology.

## Key Types

### Knowledge Graph
- `Thesaurus`: Dictionary with synonyms mapping to concepts
- `Concept`: Higher-level normalized term with ID
- `Node`: Concept with connections (connected_with HashSet)
- `Edge`: Directed relationship with doc_hash
- `NormalizedTerm`: ID + value + display_value + url + action + priority + trigger + pinned
- `NormalizedTermValue`: Case-insensitive string wrapper

### Document Management
- `Document`: id, url, title, body, description, summarization, tags, rank, doc_type, route, priority, quality_score
- `IndexedDocument`: id, matched_edges, rank, tags, nodes, quality_score
- `Index`: HashMap<String, Document> wrapper
- `DocumentType`: KgEntry, Document, ConfigDocument

### Search Operations
- `SearchQuery`: search_term, search_terms, operator, skip, limit, role, layer, include_pinned, min_quality
- `LogicalOperator`: And, Or
- `Layer`: One (title+tags), Two (+summary), Three (full content)
- `RelevanceFunction`: TerraphimGraph, TitleScorer, BM25, BM25F, BM25Plus

### Conversation Context
- `ConversationId`, `MessageId`: Unique identifiers
- `ContextType`: System, UserInput, Document, SearchResult, External, KGTermDefinition, KGIndex
- `ContextItem`: id, context_type, title, summary, content, metadata, created_at, relevance_score

### LLM & Routing
- `RouteDirective`: provider, model, action template, is_free
- `MarkdownDirectives`: doc_type, synonyms, route, routes, priority, trigger, pinned, heading
- `RoutingDecision`, `Priority`

### Multi-Agent
- `MultiAgentContext`, `AgentInfo`

### Quality & Review
- `QualityScore`: knowledge, logic, structure, last_evaluated
- `ReviewFinding`: FindingCategory, FindingSeverity

### Personas
- `PersonaDefinition`, `CharacteristicDef`, `SfiaSkillDef`

### LLM Usage
- `LlmUsage`, `LlmResult`, `ModelPricing`

### Other
- `RoleName`: Case-insensitive role identification
- `KnowledgeGraphInputType`: Markdown, Json
- `QualityScore`: K/L/S composite scoring

## Feature Flags

- `typescript`: TypeScript type generation via tsify
- `medical`: SNOMED CT/UMLS extraction
- `hgnc`: HGNC gene normalization
- `kg-integration`: Shared learning types