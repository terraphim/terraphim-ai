# Changelog

All notable changes to `terraphim_types` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.0.0] - 2025-01-22

### Added

#### Core Types
- `RoleName`: Role identifier with case-insensitive lookup support
- `NormalizedTermValue`: Normalized string values (lowercase, trimmed)
- `NormalizedTerm`: Terms with unique IDs and optional URLs
- `Concept`: Abstract idea representation in knowledge graphs
- `Document`: Central content type with rich metadata
- `Edge`: Knowledge graph edges with document associations
- `Node`: Knowledge graph nodes representing concepts
- `Thesaurus`: Dictionary mapping terms to normalized concepts
- `Index`: Document collection for fast lookup
- `IndexedDocument`: Document references with graph embeddings

#### Search Types
- `SearchQuery`: Flexible search with single/multi-term support
- `LogicalOperator`: AND/OR operators for combining search terms
- `RelevanceFunction`: Scoring algorithms (TitleScorer, BM25, BM25F, BM25Plus, TerraphimGraph)
- `KnowledgeGraphInputType`: Input source types (Markdown, JSON)

#### Context Management
- `Conversation`: Multi-message conversation with global and message-specific context
- `ChatMessage`: Messages with role, content, and context items
- `ContextItem`: Contextual information for LLM with metadata
- `ContextType`: Context types (System, Document, SearchResult, KGTermDefinition, KGIndex, etc.)
- `ConversationId`, `MessageId`: Unique conversation and message identifiers
- `ConversationSummary`: Lightweight conversation overview
- `ContextHistory`: Tracking of context usage across conversations
- `ContextHistoryEntry`: Individual context usage records
- `ContextUsageType`: How context was added (Manual, Automatic, SearchResult, DocumentReference)
- `KGTermDefinition`: Knowledge graph term with synonyms and metadata
- `KGIndexInfo`: Knowledge graph index statistics

#### LLM Routing
- `Priority`: Priority levels (0-100) with helper methods
- `RoutingRule`: Pattern-based routing with priorities and metadata
- `RoutingDecision`: Final routing decision with confidence scores
- `RoutingScenario`: Routing scenarios (Default, Background, Think, LongContext, WebSearch, Image, Pattern, Priority, Custom)
- `PatternMatch`: Pattern match results with weighted scores

#### Multi-Agent Coordination
- `MultiAgentContext`: Session for coordinating multiple agents
- `AgentInfo`: Agent metadata (id, name, role, capabilities, model)
- `AgentCommunication`: Inter-agent messages with timestamps

### Features
- `typescript`: TypeScript type generation via `tsify` for WASM compatibility
- Full serde support for all types (Serialize/Deserialize)
- JsonSchema derive for API documentation
- WASM-compatible UUID generation with `js` feature for wasm32 targets

### Documentation
- Comprehensive module-level documentation with examples
- Rustdoc comments on all public types and methods
- Usage examples for common patterns:
  - Single and multi-term search queries
  - Document creation and indexing
  - Knowledge graph construction
  - Conversation management with context
  - LLM routing with priorities
  - Multi-agent coordination
- README with quick start guide
- Full API documentation

### Implementation Details
- Uses `ahash::AHashMap` for fast hashing
- Atomic ID generation for concepts
- Case-preserving role names with efficient lowercase comparison
- WASM-compatible random generation via `getrandom` with `wasm_js` feature
- Chrono for timestamp management (UTC)
- Thread-safe ID generation using atomic operations

[Unreleased]: https://github.com/terraphim/terraphim-ai/compare/v1.0.0...HEAD
[1.0.0]: https://github.com/terraphim/terraphim-ai/releases/tag/v1.0.0
