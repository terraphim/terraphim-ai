# API Reference Snippets

Generated: 2026-06-04

Selected public types with proposed or existing doc comments.

---

## `terraphim_persistence`

### `DeviceStorage`

**Source:** `crates/terraphim_persistence/src/lib.rs:66`

```rust
/// Process-wide singleton managing all configured storage backends, ordered by
/// latency. `fastest_op` is the lowest-latency operator used as the cache
/// write-back target. Obtain via [`DeviceStorage::instance`]; use
/// [`DeviceStorage::init_memory_only`] in tests to avoid filesystem access.
pub struct DeviceStorage {
    pub ops: HashMap<String, (Operator, u128)>,
    pub fastest_op: Operator,
}
```

Key methods: `instance()`, `init_memory_only()`, `arc_instance()`, `arc_memory_only()`.

---

### `Persistable`

**Source:** `crates/terraphim_persistence/src/lib.rs:220`

```rust
/// A trait for persisting objects to and from the fastest configured storage
/// operator (file system, database, or cloud backend).
///
/// Implementors serialise to JSON and delegate I/O to `DeviceStorage`'s
/// `fastest_op`. Provides full CRUD: `save`, `load`, `get`, `delete`.
#[async_trait]
pub trait Persistable: Serialize + DeserializeOwned {
    fn new(key: String) -> Self;
    async fn save(&self) -> Result<()>;
    // ...
}
```

---

### `ConversationPersistence`

**Source:** `crates/terraphim_persistence/src/conversation.rs:10`

```rust
/// Async trait for saving, loading, and listing [`Conversation`] records.
///
/// The production implementation (`OpenDALConversationPersistence`) maintains
/// an in-memory `ConversationIndex` cache and fans writes to all configured
/// operators.
#[async_trait]
pub trait ConversationPersistence: Send + Sync {
    async fn save(&self, conversation: &Conversation) -> Result<()>;
    async fn load(&self, id: &ConversationId) -> Result<Conversation>;
    async fn delete(&self, id: &ConversationId) -> Result<()>;
    async fn list_ids(&self) -> Result<Vec<ConversationId>>;
    async fn exists(&self, id: &ConversationId) -> Result<bool>;
    async fn list_summaries(&self) -> Result<Vec<ConversationSummary>>;
}
```

---

## `haystack_core`

### `HaystackProvider`

**Source:** `crates/haystack_core/src/lib.rs:8`

Proposed doc comment (currently missing):

```rust
/// Abstraction over a data source (haystack) that can be indexed and searched.
///
/// Implement this trait for each external system (filesystem, Confluence,
/// Discourse, email) to expose it to the terraphim search pipeline.
pub trait HaystackProvider { ... }
```

---

## `terraphim_types` — Priority Gaps

The following core types in `lib.rs` are undocumented and referenced across
the workspace. One-line proposed comments:

| Type | Proposed doc |
|------|-------------|
| `RoleName` (line 171) | `/// Newtype wrapper around a role identifier string.` |
| `NormalizedTerm` (line 306) | `/// A term normalised for Aho-Corasick automata matching.` |
| `Concept` (line 438) | `/// A knowledge-graph node linking a normalised term to source documents.` |
| `DocumentType` (line 476) | `/// Classifies a document by its source format or provenance.` |
| `MarkdownDirectives` (line 606) | `/// Structured directives parsed from a Markdown document's front-matter.` |
| `RouteDirective` (line 488) | `/// Specifies how an agent command should be dispatched.` |
