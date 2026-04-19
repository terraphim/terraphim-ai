# Implementation Plan: Learning & Knowledge Graph Evolution

**Status**: Draft
**Research Doc**: `.docs/research-issue-backlog-2026-04-17.md` (Cluster D)
**Author**: Terraphim AI Agent
**Date**: 2026-04-17
**Estimated Effort**: 6-8 weeks

## Overview

### Summary
This plan addresses the gaps in Terraphim's learning and knowledge graph infrastructure identified during disciplined research. The current implementation has 14 fully-built modules (learning capture, hooks, shared types, wiki sync, KG linter, session enrichment) but lacks cross-agent injection, Aider/Cline session connectors, and durable storage that aligns with Terraphim's knowledge-graph-native architecture.

### Approach
Instead of adding a separate SQLite database (which diverges from Terraphim's markdown-first KG pattern), we will:
1. Use local markdown files with YAML frontmatter as the learning store (consistent with `CapturedLearning::to_markdown()`)
2. Leverage `terraphim_persistable` for storage abstraction
3. Build cross-agent injection via polling the shared markdown KG
4. Add session connectors for Aider and Cline

### Scope

**In Scope:**
- Cross-agent learning injection (`injector.rs`)
- Aider session connector
- Cline session connector
- `learn inject` CLI command
- Markdown-based durable learning store via `terraphim_persistable`
- EIDOS research spike (design document only)

**Out of Scope:**
- Trust gate configuration (hardcoded auto-promotion retained)
- EIDOS implementation (deferred to post-research)
- New LLM providers
- Distributed consensus/Raft

**Avoid At All Cost:**
- Adding SQLite as a separate persistence layer (diverges from KG pattern)
- Rewriting learning capture (already mature)
- Building a custom distributed message queue for learnings
- Implementing EIDOS without completing the research spike first

## Architecture

### Component Diagram

```
+----------------------------------------------------------+
|                    terraphim-agent CLI                    |
|  learn capture → learn list → learn inject → learn query  |
+----------------------------------------------------------+
                          |
+----------------------------------------------------------+
|              Learning Injection (NEW: injector.rs)        |
|  - Poll markdown KG for L2/L3 learnings from other agents |
|  - Filter by agent context and relevance                  |
|  - Inject into local learning store                       |
+----------------------------------------------------------+
                          |
+----------------------------------------------------------+
|            Shared Learning Store (REDESIGNED)             |
|  - Local: markdown files with YAML frontmatter            |
|  - Shared: Gitea wiki (already implemented)               |
|  - Uses: terraphim_persistable (Persistable trait)        |
+----------------------------------------------------------+
                          |
+----------------------------------------------------------+
|               Session Connectors (NEW)                    |
|  - NativeClaudeConnector (existing)                       |
|  - AiderConnector (NEW)                                   |
|  - ClineConnector (NEW)                                   |
|  - terraphim-session-analyzer (existing, optional)        |
+----------------------------------------------------------+
```

### Data Flow

```
[Agent A Session] → [Learning Capture] → [Markdown File] → [Gitea Wiki Sync]
                                                           ↓
[Agent B] ← [Injector Poll] ← [Gitea Wiki / Local Markdown KG]
     ↓
[Agent B Learning Store] → [Procedure Extraction] → [Replay]
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Markdown files instead of SQLite | Aligns with Terraphim's KG-native pattern. Frontmatter gives structured metadata. Body gives rich text. Already used by `CapturedLearning`. | SQLite (diverges from pattern), JSON blobs (not human-readable), ReDB (overkill for text) |
| terraphim_persistable for storage | Reuses existing abstraction. Supports multiple backends (fs, s3, etc.). Async. | Direct fs::write (no abstraction), custom storage layer (reinventing) |
| Polling-based injection | Simple, stateless, works with Gitea wiki as the shared store. No need for message queue. | Pub-sub (needs infrastructure), RPC (couples agents), webhooks (needs server) |
| Hardcoded auto-promotion retained | Already works. Configurable trust gate is not in the vital few for this phase. | Full trust gate config (defers higher-value work) |

### Eliminated Options (Essentialism)

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| SQLite learning store | Diverges from Terraphim's markdown-first KG architecture | Creates a second persistence pattern to maintain |
| EIDOS implementation now | Needs research spike first. No code exists. | Risk of building the wrong thing |
| Trust gate configuration | Hardcoded thresholds work. Not highest priority. | Defers cross-agent injection which is higher value |
| Real-time push notifications | Adds infrastructure (webhooks, server) | Over-engineering for current scale |

### Simplicity Check

**What if this could be easy?**

The simplest design is: each learning is a markdown file with YAML frontmatter. Agents read each other's files. Gitea wiki syncs the shared ones. No database. No message queue. Just files.

**Senior Engineer Test**: A senior engineer would recognise this as "Unix philosophy applied to AI learning" -- text files, pipes, and simple tools.

**Nothing Speculative Checklist**:
- [x] No features the user didn't request
- [x] No abstractions "in case we need them later"
- [x] No flexibility "just in case"
- [x] No error handling for scenarios that cannot occur
- [x] No premature optimisation

## File Changes

### New Files

| File | Purpose |
|------|---------|
| `crates/terraphim_agent/src/shared_learning/injector.rs` | Cross-agent learning injection client |
| `crates/terraphim_agent/src/shared_learning/markdown_store.rs` | Markdown-based learning store implementing Persistable |
| `crates/terraphim_sessions/src/connector/aider.rs` | Aider session connector |
| `crates/terraphim_sessions/src/connector/cline.rs` | Cline session connector |
| `docs/eidos-research-2026-04-17.md` | EIDOS research spike document |

### Modified Files

| File | Changes |
|------|---------|
| `crates/terraphim_agent/src/shared_learning/mod.rs` | Add injector and markdown_store modules |
| `crates/terraphim_agent/src/shared_learning/store.rs` | Replace in-memory DeviceStorage with markdown_store backend |
| `crates/terraphim_agent/src/main.rs` | Add `learn inject` subcommand |
| `crates/terraphim_sessions/src/connector/mod.rs` | Register Aider and Cline connectors |
| `crates/terraphim_sessions/src/lib.rs` | Re-export new connectors |

### Deleted Files

None.

## API Design

### Public Types

```rust
/// Configuration for cross-agent learning injection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InjectorConfig {
    /// Directory containing shared markdown learnings
    pub shared_kg_path: PathBuf,
    /// How often to poll for new learnings (seconds)
    pub poll_interval_secs: u64,
    /// Minimum trust level to inject (default: L2)
    pub min_trust_level: TrustLevel,
    /// Agent IDs to exclude (self and known incompatible agents)
    pub excluded_agents: Vec<String>,
    /// Maximum learnings to inject per poll
    pub max_per_poll: usize,
}

impl Default for InjectorConfig {
    fn default() -> Self {
        Self {
            shared_kg_path: dirs::data_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("terraphim")
                .join("shared-learnings"),
            poll_interval_secs: 300, // 5 minutes
            min_trust_level: TrustLevel::L2,
            excluded_agents: Vec::new(),
            max_per_poll: 10,
        }
    }
}

/// Result of a single injection poll
#[derive(Debug, Clone)]
pub struct InjectionResult {
    pub learnings_injected: usize,
    pub learnings_skipped: usize,
    pub errors: Vec<String>,
}

/// Markdown-based learning store that implements Persistable
#[derive(Debug, Serialize, Deserialize)]
pub struct MarkdownLearningStore {
    key: String,
    learnings_dir: PathBuf,
}
```

### Public Functions

```rust
/// Poll shared learning store for new learnings from other agents
///
/// # Arguments
/// * `config` - Injection configuration
/// * `local_store` - Local learning store to inject into
///
/// # Returns
/// Result of the injection poll
///
/// # Errors
/// Returns `LearningError::StorageError` if the shared store cannot be read
pub async fn poll_and_inject(
    config: &InjectorConfig,
    local_store: &mut SharedLearningStore,
) -> Result<InjectionResult, LearningError>;

/// Check if a learning should be injected based on context relevance
///
/// Uses BM25 similarity between the learning's context and the current
/// working directory / project context.
pub fn should_inject(
    learning: &SharedLearning,
    working_dir: &Path,
) -> bool;
```

### Session Connector Implementations

```rust
/// Aider session connector
#[derive(Debug, Default)]
pub struct AiderConnector;

#[async_trait]
impl SessionConnector for AiderConnector {
    fn source_id(&self) -> &str { "aider" }
    fn display_name(&self) -> &str { "Aider" }
    fn default_path(&self) -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(".aider"))
    }
    // ... detect() and import() implementations
}

/// Cline session connector
#[derive(Debug, Default)]
pub struct ClineConnector;

#[async_trait]
impl SessionConnector for ClineConnector {
    fn source_id(&self) -> &str { "cline" }
    fn display_name(&self) -> &str { "Cline" }
    fn default_path(&self) -> Option<PathBuf> {
        dirs::home_dir().map(|h| {
            h.join("Library/Application Support/Code/User/globalStorage/saoudrizwan.claude-dev")
        })
    }
    // ... detect() and import() implementations
}
```

### Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum InjectionError {
    #[error("shared learning store not found at {0}")]
    StoreNotFound(PathBuf),

    #[error("failed to parse learning from markdown: {0}")]
    ParseError(String),

    #[error("learning already exists in local store: {0}")]
    AlreadyExists(String),

    #[error("trust level too low: got {got:?}, need {need:?}")]
    TrustLevelTooLow { got: TrustLevel, need: TrustLevel },

    #[error("storage error: {0}")]
    Storage(#[from] terraphim_persistence::Error),
}
```

## Test Strategy

### Unit Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_injector_poll_empty_store` | `injector.rs` | Graceful handling of empty store |
| `test_injector_filters_by_trust_level` | `injector.rs` | Only L2/L3 learnings injected |
| `test_injector_excludes_self` | `injector.rs` | Agent doesn't inject its own learnings |
| `test_markdown_store_roundtrip` | `markdown_store.rs` | Save and load learning via Persistable |
| `test_should_inject_context_match` | `injector.rs` | BM25 relevance filtering |
| `test_aider_parse_session` | `aider.rs` | Parse Aider chat history |
| `test_cline_parse_session` | `cline.rs` | Parse Cline session files |

### Integration Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_inject_end_to_end` | `tests/injection.rs` | Full poll → filter → inject flow |
| `test_markdown_store_with_persistable` | `tests/persistence.rs` | Persistable trait integration |
| `test_aider_import_real_session` | `tests/connectors.rs` | Import real Aider session (if available) |

### Property Tests

```rust
proptest! {
    #[test]
    fn injector_never_duplicates(learnings: Vec<SharedLearning>) {
        // Injecting the same learning twice should not create duplicates
    }
}
```

## Implementation Steps

### Step 1: Markdown Store (Foundation)
**Files:** `crates/terraphim_agent/src/shared_learning/markdown_store.rs`
**Description:** Implement `MarkdownLearningStore` that uses `terraphim_persistable` to save/load learnings as markdown files with YAML frontmatter. Leverages existing `SharedLearning::to_markdown()` and `from_markdown()` methods.
**Tests:** Unit tests for roundtrip save/load
**Dependencies:** None
**Estimated:** 1 day

```rust
// Key code pattern
#[async_trait]
impl Persistable for MarkdownLearningStore {
    fn new(key: String) -> Self { ... }
    async fn save(&self) -> Result<()> { ... }
    async fn load(&mut self) -> Result<Self> { ... }
}
```

### Step 2: Aider Session Connector
**Files:** `crates/terraphim_sessions/src/connector/aider.rs`
**Description:** Parse Aider's `.aider.chat.history.md` and `.aider.input.history` files into `Session` objects.
**Tests:** Unit tests with synthetic Aider history
**Dependencies:** None
**Estimated:** 1 day

### Step 3: Cline Session Connector
**Files:** `crates/terraphim_sessions/src/connector/cline.rs`
**Description:** Parse Cline's VS Code extension storage into `Session` objects.
**Tests:** Unit tests with synthetic Cline session
**Dependencies:** None
**Estimated:** 1 day

### Step 4: Learning Injector
**Files:** `crates/terraphim_agent/src/shared_learning/injector.rs`
**Description:** Poll shared markdown KG, filter by trust level and context, inject into local store.
**Tests:** Unit tests for filtering, integration test for end-to-end
**Dependencies:** Steps 1, 2, 3
**Estimated:** 2 days

### Step 5: CLI Integration
**Files:** `crates/terraphim_agent/src/main.rs`
**Description:** Add `learn inject` subcommand. Wire `InjectorConfig` from CLI args or env vars.
**Tests:** CLI integration test
**Dependencies:** Step 4
**Estimated:** 0.5 day

### Step 6: Store Migration
**Files:** `crates/terraphim_agent/src/shared_learning/store.rs`
**Description:** Replace in-memory `DeviceStorage::arc_memory_only()` with `MarkdownLearningStore` backend.
**Tests:** Ensure all existing store tests pass with new backend
**Dependencies:** Step 1
**Estimated:** 1 day

### Step 7: EIDOS Research Spike
**Files:** `docs/eidos-research-2026-04-17.md`
**Description:** Research and design EIDOS architecture. Document confidence scoring, episodic reasoning, and KG entry gating.
**Tests:** N/A (research document)
**Dependencies:** None
**Estimated:** 2 days

### Step 8: Documentation and Examples
**Files:** `docs/`, `examples/`
**Description:** Document the injection flow, connector setup, and markdown store format.
**Tests:** Doc tests
**Dependencies:** All above
**Estimated:** 0.5 day

## Rollback Plan

If issues discovered:
1. Revert `store.rs` to use `DeviceStorage::arc_memory_only()` (one-line change)
2. Disable `learn inject` subcommand via feature flag
3. Remove Aider/Cline connectors from registry (comment out)

Feature flags:
- `markdown-store` - Enables markdown-based persistence
- `cross-agent-injection` - Enables injector and `learn inject`
- `aider-connector` - Enables Aider session import
- `cline-connector` - Enables Cline session import

## Dependencies

### New Dependencies

| Crate | Version | Justification |
|-------|---------|---------------|
| None | - | Reuses existing `terraphim_persistable`, `opendal`, `serde_yaml` |

### Dependency Updates

| Crate | From | To | Reason |
|-------|------|-----|--------|
| None | - | - | No updates needed |

## Performance Considerations

### Expected Performance

| Metric | Target | Measurement |
|--------|--------|-------------|
| Injection poll latency | < 100ms for 100 learnings | Benchmark |
| Markdown save latency | < 10ms per learning | Benchmark |
| Session import (Aider) | < 1s per 1000 messages | Benchmark |
| Memory footprint | No increase (was in-memory, now disk) | Profiling |

### Benchmarks to Add

```rust
#[bench]
fn bench_injector_poll_100_learnings(b: &mut Bencher) {
    let store = create_test_store(100);
    b.iter(|| poll_and_inject(&config, &mut local_store));
}
```

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Aider session file format confirmation | Pending | User to verify `.aider.chat.history.md` format |
| Cline session file format confirmation | Pending | User to verify Cline VS Code extension storage path |
| Gitea wiki path for shared learnings | Pending | Ops to confirm `TERRAPHIM_SHARED_KG_PATH` env var |

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Performance targets agreed
- [ ] Human approval received

---

## Appendix: Storage Design Detail

### Markdown File Format

Each learning becomes a file at `{learnings_dir}/{agent_id}/{learning_id}.md`:

```markdown
---
id: "550e8400-e29b-41d4-a716-446655440000"
agent_id: "terraphim-agent-pa"
agent_name: "Personal Assistant"
captured_at: "2026-04-17T10:30:00Z"
trust_level: "L2"
source: "AutoExtract"
importance: 0.85
quality:
  applied_count: 5
  effective_count: 4
  agent_count: 3
  success_rate: 0.8
entities:
  - "CommandRegistry"
  - "MarkdownCommandParser"
---

# Command Registry Duplicate Detection

## Problem
When registering commands from markdown files, duplicate command names cause `CommandRegistryError::DuplicateCommand`.

## Correction
Use `HashMap::entry().or_insert()` pattern to handle duplicates gracefully by merging command definitions.

## Context
Working directory: /home/alex/projects/terraphim/terraphim-ai
Project: terraphim-ai
```

### Persistable Integration

```rust
#[async_trait]
impl Persistable for MarkdownLearningStore {
    fn new(key: String) -> Self {
        let learnings_dir = std::env::var("TERRAPHIM_LEARNINGS_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                dirs::data_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join("terraphim")
                    .join("learnings")
            });
        Self { key, learnings_dir }
    }

    async fn save(&self) -> Result<()> {
        let path = self.learnings_dir.join(&self.key);
        let content = self.to_markdown()?;
        tokio::fs::write(&path, content).await?;
        Ok(())
    }

    async fn load(&mut self) -> Result<Self> {
        let path = self.learnings_dir.join(&self.key);
        let content = tokio::fs::read_to_string(&path).await?;
        let learning = SharedLearning::from_markdown(&content)?;
        // ... populate self from learning
        Ok(self)
    }
}
```

This aligns with Terraphim's existing `CapturedLearning::to_markdown()` and `from_markdown()` methods, and uses the filesystem as the operator via `terraphim_persistable`.
