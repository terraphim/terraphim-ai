# terraphim_types

[![Crates.io](https://img.shields.io/crates/v/terraphim_types.svg)](https://crates.io/crates/terraphim_types)
[![Documentation](https://docs.rs/terraphim_types/badge.svg)](https://docs.rs/terraphim_types)
[![License](https://img.shields.io/crates/l/terraphim_types.svg)](https://github.com/terraphim/terraphim-ai/blob/main/LICENSE-Apache-2.0)

Core type definitions for the Terraphim AI system.

## Overview

`terraphim_types` provides the fundamental data structures used throughout the Terraphim ecosystem for knowledge graph management, document indexing, search operations, and LLM-powered conversations.

## Features

- **Knowledge Graph Types**: Build and query semantic knowledge graphs
- **Document Management**: Index and search documents from multiple sources
- **Search Operations**: Flexible queries with logical operators (AND/OR)
- **Conversation Context**: Manage LLM conversations with rich context
- **LLM Routing**: Priority-based routing to different AI providers
- **Multi-Agent Coordination**: Coordinate multiple AI agents
- **WASM Support**: TypeScript type generation for browser integration

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
terraphim_types = "1.0.0"
```

For TypeScript/WASM support:

```toml
[dependencies]
terraphim_types = { version = "1.0.0", features = ["typescript"] }
```

## Quick Start

### Creating a Search Query

```rust
use terraphim_types::{SearchQuery, NormalizedTermValue, LogicalOperator, RoleName};

// Simple single-term query
let query = SearchQuery {
    search_term: NormalizedTermValue::from("rust async"),
    search_terms: None,
    operator: None,
    skip: None,
    limit: Some(10),
    role: Some(RoleName::new("engineer")),
};

// Multi-term AND query
let multi_query = SearchQuery::with_terms_and_operator(
    NormalizedTermValue::from("async"),
    vec![NormalizedTermValue::from("tokio"), NormalizedTermValue::from("runtime")],
    LogicalOperator::And,
    Some(RoleName::new("engineer")),
);

println!("Query has {} terms", multi_query.get_all_terms().len()); // 3
```

### Working with Documents

```rust
use terraphim_types::Document;

let document = Document {
    id: "rust-book-ch1".to_string(),
    url: "https://doc.rust-lang.org/book/ch01-00-getting-started.html".to_string(),
    title: "Getting Started".to_string(),
    body: "Let's start your Rust journey...".to_string(),
    description: Some("Introduction to Rust programming".to_string()),
    summarization: None,
    stub: None,
    tags: Some(vec!["rust".to_string(), "tutorial".to_string()]),
    rank: Some(95),
    source_haystack: Some("rust-docs".to_string()),
};

println!("Document: {} (rank: {})", document.title, document.rank.unwrap_or(0));
```

### Building a Knowledge Graph

```rust
use terraphim_types::{Thesaurus, NormalizedTermValue, NormalizedTerm};

let mut thesaurus = Thesaurus::new("programming".to_string());

// Add normalized terms
thesaurus.insert(
    NormalizedTermValue::from("rust"),
    NormalizedTerm {
        id: 1,
        value: NormalizedTermValue::from("rust programming language"),
        url: Some("https://rust-lang.org".to_string()),
    }
);

thesaurus.insert(
    NormalizedTermValue::from("async"),
    NormalizedTerm {
        id: 2,
        value: NormalizedTermValue::from("asynchronous programming"),
        url: Some("https://rust-lang.github.io/async-book/".to_string()),
    }
);

println!("Thesaurus has {} terms", thesaurus.len());
```

### Managing Conversations

```rust
use terraphim_types::{Conversation, ChatMessage, RoleName, ContextItem, Document};

// Create a new conversation
let mut conversation = Conversation::new(
    "Discussing Rust async".to_string(),
    RoleName::new("engineer"),
);

// Add a user message
let mut user_msg = ChatMessage::user("Explain async/await in Rust".to_string());

// Add context from a document
let doc = Document {
    id: "async-book".to_string(),
    title: "Async Programming in Rust".to_string(),
    body: "Async/await syntax makes it easier to write asynchronous code...".to_string(),
    url: "https://rust-lang.github.io/async-book/".to_string(),
    description: Some("Guide to async Rust".to_string()),
    summarization: None,
    stub: None,
    tags: Some(vec!["rust".to_string(), "async".to_string()]),
    rank: None,
    source_haystack: None,
};

user_msg.add_context(ContextItem::from_document(&doc));
conversation.add_message(user_msg);

// Add assistant response
let assistant_msg = ChatMessage::assistant(
    "Async/await in Rust provides...".to_string(),
    Some("claude-3-sonnet".to_string()),
);
conversation.add_message(assistant_msg);

println!("Conversation has {} messages", conversation.messages.len());
```

### LLM Routing with Priorities

```rust
use terraphim_types::{RoutingRule, RoutingDecision, RoutingScenario, Priority};

// Create a high-priority routing rule for code tasks
let code_rule = RoutingRule::new(
    "code-gen".to_string(),
    "Code Generation".to_string(),
    r"(code|implement|function|class)".to_string(),
    Priority::HIGH,
    "anthropic".to_string(),
    "claude-3-opus".to_string(),
)
.with_description("Route coding tasks to most capable model".to_string())
.with_tag("coding".to_string());

// Create a routing decision
let decision = RoutingDecision::with_rule(
    "anthropic".to_string(),
    "claude-3-opus".to_string(),
    RoutingScenario::Pattern("code generation".to_string()),
    Priority::HIGH,
    0.95,
    code_rule.id.clone(),
    "Matched code generation pattern".to_string(),
);

println!("Routing to {} (confidence: {})", decision.provider, decision.confidence);
```

## Type Categories

### Knowledge Graph Types

- **`NormalizedTermValue`**: Normalized, lowercase string values
- **`NormalizedTerm`**: Terms with unique IDs and URLs
- **`Concept`**: Abstract ideas in the knowledge graph
- **`Node`**: Graph nodes representing concepts
- **`Edge`**: Connections between nodes
- **`Thesaurus`**: Dictionary mapping terms to normalized concepts

### Document Types

- **`Document`**: Primary content unit with metadata
- **`Index`**: Collection of indexed documents
- **`IndexedDocument`**: Document reference with graph embeddings

### Search Types

- **`SearchQuery`**: Flexible search with logical operators
- **`LogicalOperator`**: AND/OR operators for multi-term queries
- **`RelevanceFunction`**: Scoring algorithms (TitleScorer, BM25, TerraphimGraph)
- **`KnowledgeGraphInputType`**: Input source types (Markdown, JSON)

### Context Management Types

- **`Conversation`**: Multi-message conversation with context
- **`ChatMessage`**: Single message in a conversation
- **`ContextItem`**: Contextual information for LLM
- **`ContextType`**: Types of context (Document, SearchResult, KGTermDefinition, etc.)
- **`ConversationId`**, **`MessageId`**: Unique identifiers

### Routing Types

- **`Priority`**: Priority levels (0-100) for routing decisions
- **`RoutingRule`**: Pattern-based routing rules
- **`RoutingDecision`**: Final routing decision
- **`RoutingScenario`**: Routing scenarios (Think, LongContext, WebSearch, etc.)
- **`PatternMatch`**: Pattern match results with scores

### Multi-Agent Types

- **`MultiAgentContext`**: Coordination between multiple agents
- **`AgentInfo`**: Information about an AI agent
- **`AgentCommunication`**: Messages between agents

## Features

### TypeScript Support

Enable TypeScript type generation for WASM compatibility:

```toml
[dependencies]
terraphim_types = { version = "1.0.0", features = ["typescript"] }
```

This enables `#[derive(Tsify)]` on types, generating TypeScript definitions automatically.

## Examples

See the [examples directory](../../examples/) in the main repository for more comprehensive examples:

- **Knowledge graph construction**
- **Multi-term search queries**
- **Context-aware conversations**
- **LLM routing strategies**
- **Multi-agent coordination**

## Documentation

Full API documentation is available on [docs.rs](https://docs.rs/terraphim_types).

## Minimum Supported Rust Version (MSRV)

This crate requires Rust 1.70 or later.

## License

Licensed under Apache-2.0. See [LICENSE](../../LICENSE-Apache-2.0) for details.

## Contributing

Contributions are welcome! Please see the [main repository](https://github.com/terraphim/terraphim-ai) for contribution guidelines.

## Related Crates

- **[terraphim_automata](../terraphim_automata)**: Text matching and autocomplete engine
- **[terraphim_rolegraph](../terraphim_rolegraph)**: Knowledge graph implementation
- **[terraphim_service](../terraphim_service)**: Main service layer
- **[terraphim_server](../../terraphim_server)**: HTTP API server

## Support

- **Discord**: https://discord.gg/VPJXB6BGuY
- **Discourse**: https://terraphim.discourse.group
- **Issues**: https://github.com/terraphim/terraphim-ai/issues
