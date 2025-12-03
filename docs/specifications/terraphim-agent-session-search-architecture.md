# Terraphim Agent Session Search - Architecture Document

> **Version**: 1.0.0
> **Status**: Draft
> **Created**: 2025-12-03

## Overview

This document describes the technical architecture for the Session Search and Robot Mode features in `terraphim-agent`. The architecture extends existing Terraphim components while introducing new modules for session management and AI-friendly interfaces.

## System Context

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              External Systems                                │
├─────────────┬─────────────┬─────────────┬─────────────┬─────────────────────┤
│ Claude Code │   Cursor    │    Aider    │    Cline    │   Other Agents      │
│   (JSONL)   │  (SQLite)   │ (Markdown)  │   (JSON)    │                     │
└──────┬──────┴──────┬──────┴──────┬──────┴──────┬──────┴──────────┬──────────┘
       │             │             │             │                 │
       └─────────────┴──────┬──────┴─────────────┴─────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                         terraphim-agent                                      │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                      Session Connectors                              │    │
│  │  ┌───────────┐ ┌───────────┐ ┌───────────┐ ┌───────────┐           │    │
│  │  │  Claude   │ │  Cursor   │ │   Aider   │ │   Cline   │           │    │
│  │  │ Connector │ │ Connector │ │ Connector │ │ Connector │           │    │
│  │  └─────┬─────┘ └─────┬─────┘ └─────┬─────┘ └─────┬─────┘           │    │
│  └────────┼─────────────┼─────────────┼─────────────┼──────────────────┘    │
│           └─────────────┴──────┬──────┴─────────────┘                       │
│                                ▼                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                     Session Service                                  │    │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────────┐  │    │
│  │  │   Import    │  │   Index     │  │     Enrichment              │  │    │
│  │  │   Engine    │──│  (Tantivy)  │──│  (Knowledge Graph)          │  │    │
│  │  └─────────────┘  └─────────────┘  └─────────────────────────────┘  │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                │                                             │
│                                ▼                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                      Command Layer                                   │    │
│  │  ┌───────────────┐  ┌───────────────┐  ┌───────────────────────┐   │    │
│  │  │ Forgiving CLI │  │  Robot Mode   │  │   Self-Documentation  │   │    │
│  │  │    Parser     │  │   Formatter   │  │        API            │   │    │
│  │  └───────────────┘  └───────────────┘  └───────────────────────┘   │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                │                                             │
│                                ▼                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                    Existing Terraphim Core                           │    │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────────┐  │    │
│  │  │ terraphim_  │  │ terraphim_  │  │      terraphim_             │  │    │
│  │  │  automata   │  │  rolegraph  │  │        service              │  │    │
│  │  └─────────────┘  └─────────────┘  └─────────────────────────────┘  │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Module Architecture

### New Modules

```
crates/
├── terraphim_agent/                    # Existing - Enhanced
│   ├── src/
│   │   ├── main.rs                     # Entry point
│   │   ├── repl/
│   │   │   ├── mod.rs
│   │   │   ├── commands.rs             # Enhanced: Forgiving parser
│   │   │   ├── handler.rs              # Enhanced: Robot mode support
│   │   │   ├── aliases.rs              # NEW: Command aliases
│   │   │   └── robot_mode.rs           # NEW: Structured output
│   │   ├── robot/                      # NEW: Robot mode module
│   │   │   ├── mod.rs
│   │   │   ├── output.rs               # JSON/JSONL formatters
│   │   │   ├── schema.rs               # Response schemas
│   │   │   └── docs.rs                 # Self-documentation
│   │   ├── sessions/                   # NEW: Session management
│   │   │   ├── mod.rs
│   │   │   ├── commands.rs             # Session REPL commands
│   │   │   ├── service.rs              # Session service facade
│   │   │   └── display.rs              # Session output formatting
│   │   └── forgiving/                  # NEW: Forgiving CLI
│   │       ├── mod.rs
│   │       ├── parser.rs               # Edit-distance parser
│   │       └── suggestions.rs          # Command suggestions
│
├── terraphim_sessions/                 # NEW CRATE
│   ├── src/
│   │   ├── lib.rs
│   │   ├── connector/                  # Session connectors
│   │   │   ├── mod.rs
│   │   │   ├── trait.rs                # SessionConnector trait
│   │   │   ├── claude_code.rs          # Claude Code JSONL
│   │   │   ├── cursor.rs               # Cursor SQLite
│   │   │   ├── aider.rs                # Aider Markdown
│   │   │   └── registry.rs             # Connector registry
│   │   ├── index/                      # Tantivy index
│   │   │   ├── mod.rs
│   │   │   ├── schema.rs               # Index schema
│   │   │   ├── writer.rs               # Index writer
│   │   │   ├── reader.rs               # Index reader/search
│   │   │   └── tokenizer.rs            # Edge n-gram tokenizer
│   │   ├── enrichment/                 # KG enrichment
│   │   │   ├── mod.rs
│   │   │   ├── concept_extractor.rs
│   │   │   └── graph_builder.rs
│   │   ├── model/                      # Data models
│   │   │   ├── mod.rs
│   │   │   ├── session.rs
│   │   │   ├── message.rs
│   │   │   └── snippet.rs
│   │   └── service.rs                  # Main service
```

## Component Details

### 1. Forgiving CLI Parser

**Location**: `crates/terraphim_agent/src/forgiving/`

**Purpose**: Parse commands with typo tolerance and flexibility.

```rust
/// Forgiving command parser with edit-distance correction
pub struct ForgivingParser {
    /// Known commands for matching
    known_commands: Vec<CommandSpec>,
    /// Aliases mapping
    aliases: HashMap<String, String>,
    /// Maximum edit distance for auto-correction
    max_auto_correct_distance: usize,
    /// Jaro-Winkler threshold for suggestions
    suggestion_threshold: f64,
}

impl ForgivingParser {
    /// Parse input with typo tolerance
    pub fn parse(&self, input: &str) -> ParseResult {
        // 1. Normalize input (trim, lowercase command)
        let normalized = self.normalize(input);

        // 2. Check for alias
        if let Some(expanded) = self.expand_alias(&normalized) {
            return self.parse_exact(&expanded);
        }

        // 3. Try exact match
        if let Ok(cmd) = self.parse_exact(&normalized) {
            return ParseResult::Exact(cmd);
        }

        // 4. Try fuzzy match
        let matches = self.fuzzy_match(&normalized);

        match matches.as_slice() {
            [] => ParseResult::Unknown(normalized),
            [(cmd, dist)] if *dist <= self.max_auto_correct_distance => {
                ParseResult::AutoCorrected {
                    original: normalized,
                    corrected: cmd.clone(),
                    distance: *dist,
                }
            }
            suggestions => ParseResult::Suggestions(suggestions.to_vec()),
        }
    }

    fn fuzzy_match(&self, input: &str) -> Vec<(String, usize)> {
        // Use Jaro-Winkler from terraphim_automata
        self.known_commands
            .iter()
            .filter_map(|cmd| {
                let similarity = jaro_winkler(&cmd.name, input);
                if similarity >= self.suggestion_threshold {
                    Some((cmd.name.clone(), edit_distance(&cmd.name, input)))
                } else {
                    None
                }
            })
            .sorted_by_key(|(_, dist)| *dist)
            .take(5)
            .collect()
    }
}

pub enum ParseResult {
    Exact(ReplCommand),
    AutoCorrected {
        original: String,
        corrected: ReplCommand,
        distance: usize,
    },
    Suggestions(Vec<(String, usize)>),
    Unknown(String),
}
```

### 2. Robot Mode Output

**Location**: `crates/terraphim_agent/src/robot/`

**Purpose**: Structured, machine-readable output for AI agents.

```rust
/// Robot mode output configuration
#[derive(Debug, Clone)]
pub struct RobotConfig {
    pub format: OutputFormat,
    pub max_tokens: Option<usize>,
    pub max_results: Option<usize>,
    pub max_content_length: Option<usize>,
    pub fields: FieldMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Json,
    Jsonl,
    Table,
    Minimal,
}

#[derive(Debug, Clone)]
pub enum FieldMode {
    Full,
    Summary,
    Minimal,
    Custom(Vec<String>),
}

/// Standard response envelope
#[derive(Debug, Serialize)]
pub struct RobotResponse<T: Serialize> {
    pub success: bool,
    pub meta: ResponseMeta,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<RobotError>,
}

#[derive(Debug, Serialize)]
pub struct ResponseMeta {
    pub command: String,
    pub elapsed_ms: u64,
    pub timestamp: DateTime<Utc>,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_corrected: Option<AutoCorrection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pagination: Option<Pagination>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_budget: Option<TokenBudget>,
}

#[derive(Debug, Serialize)]
pub struct RobotError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

/// Exit codes
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum ExitCode {
    Success = 0,
    ErrorGeneral = 1,
    ErrorUsage = 2,
    ErrorIndexMissing = 3,
    ErrorNotFound = 4,
    ErrorAuth = 5,
    ErrorNetwork = 6,
    ErrorTimeout = 7,
}
```

### 3. Self-Documentation API

**Location**: `crates/terraphim_agent/src/robot/docs.rs`

```rust
/// Self-documentation for AI agents
pub struct SelfDocumentation {
    commands: Vec<CommandDoc>,
}

#[derive(Debug, Serialize)]
pub struct CommandDoc {
    pub name: String,
    pub aliases: Vec<String>,
    pub description: String,
    pub arguments: Vec<ArgumentDoc>,
    pub flags: Vec<FlagDoc>,
    pub examples: Vec<ExampleDoc>,
    pub response_schema: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct ArgumentDoc {
    pub name: String,
    #[serde(rename = "type")]
    pub arg_type: String,
    pub required: bool,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FlagDoc {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short: Option<String>,
    #[serde(rename = "type")]
    pub flag_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    pub description: String,
}

#[derive(Debug, Serialize)]
pub struct ExampleDoc {
    pub description: String,
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
}

impl SelfDocumentation {
    /// Generate capabilities summary
    pub fn capabilities(&self) -> Capabilities {
        Capabilities {
            name: "terraphim-agent".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            description: "Privacy-first AI assistant with knowledge graph search".into(),
            features: self.detect_features(),
            commands: self.commands.iter().map(|c| c.name.clone()).collect(),
            supported_formats: vec!["json", "jsonl", "table", "minimal"],
        }
    }

    /// Generate schema for specific command
    pub fn schema(&self, command: &str) -> Option<&CommandDoc> {
        self.commands.iter().find(|c| c.name == command)
    }

    /// Generate all schemas
    pub fn all_schemas(&self) -> &[CommandDoc] {
        &self.commands
    }
}
```

### 4. Session Connectors

**Location**: `crates/terraphim_sessions/src/connector/`

```rust
/// Trait for session source connectors
#[async_trait]
pub trait SessionConnector: Send + Sync {
    /// Unique identifier for this source
    fn source_id(&self) -> &str;

    /// Human-readable name
    fn display_name(&self) -> &str;

    /// Check if source is available on this system
    async fn detect(&self) -> ConnectorStatus;

    /// Get default path for this source
    fn default_path(&self) -> Option<PathBuf>;

    /// Import sessions from source
    async fn import(&self, options: ImportOptions) -> Result<ImportResult, ConnectorError>;

    /// Watch for new sessions (optional, for real-time indexing)
    fn supports_watch(&self) -> bool { false }

    /// Start watching for changes
    async fn watch(&self) -> Result<mpsc::Receiver<SessionEvent>, ConnectorError> {
        Err(ConnectorError::WatchNotSupported)
    }
}

#[derive(Debug)]
pub struct ImportOptions {
    /// Custom path override
    pub path: Option<PathBuf>,
    /// Only import sessions after this date
    pub since: Option<DateTime<Utc>>,
    /// Only import sessions before this date
    pub until: Option<DateTime<Utc>>,
    /// Maximum sessions to import
    pub limit: Option<usize>,
    /// Skip sessions already imported
    pub incremental: bool,
}

#[derive(Debug)]
pub struct ImportResult {
    pub sessions_imported: usize,
    pub sessions_skipped: usize,
    pub errors: Vec<ImportError>,
    pub duration: Duration,
}

/// Claude Code connector implementation
pub struct ClaudeCodeConnector {
    base_path: PathBuf,
}

#[async_trait]
impl SessionConnector for ClaudeCodeConnector {
    fn source_id(&self) -> &str { "claude-code" }
    fn display_name(&self) -> &str { "Claude Code" }

    async fn detect(&self) -> ConnectorStatus {
        let path = self.default_path().unwrap();
        if path.exists() {
            ConnectorStatus::Available { path, sessions_estimate: None }
        } else {
            ConnectorStatus::NotFound
        }
    }

    fn default_path(&self) -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(".claude"))
    }

    async fn import(&self, options: ImportOptions) -> Result<ImportResult, ConnectorError> {
        let path = options.path.unwrap_or_else(|| self.default_path().unwrap());

        // Parse JSONL files from ~/.claude/projects/*/
        let sessions = self.parse_jsonl_files(&path, &options).await?;

        Ok(ImportResult {
            sessions_imported: sessions.len(),
            sessions_skipped: 0,
            errors: vec![],
            duration: Duration::from_secs(0),
        })
    }
}
```

### 5. Session Index (Tantivy)

**Location**: `crates/terraphim_sessions/src/index/`

```rust
use tantivy::{
    schema::{Schema, Field, TEXT, STORED, STRING, FAST, INDEXED},
    Index, IndexWriter, IndexReader,
    collector::TopDocs,
    query::QueryParser,
    tokenizer::{TextAnalyzer, SimpleTokenizer, LowerCaser, Stemmer, Language},
};

/// Session search index using Tantivy
pub struct SessionIndex {
    index: Index,
    reader: IndexReader,
    schema: SessionSchema,
    query_parser: QueryParser,
}

pub struct SessionSchema {
    // Identifiers
    pub session_id: Field,
    pub message_id: Field,
    pub source: Field,

    // Searchable content (TEXT = tokenized + indexed)
    pub content: Field,
    pub code_content: Field,

    // Filterable (STRING = not tokenized, FAST = column store)
    pub timestamp: Field,
    pub role: Field,
    pub language: Field,
    pub project_path: Field,

    // Knowledge graph (TEXT for search, STORED for retrieval)
    pub concepts: Field,
}

impl SessionIndex {
    pub fn new(index_path: &Path) -> Result<Self, IndexError> {
        let schema = Self::build_schema();
        let index = Index::create_in_dir(index_path, schema.schema.clone())?;

        // Register custom tokenizers
        Self::register_tokenizers(&index);

        let reader = index.reader()?;
        let query_parser = QueryParser::for_index(
            &index,
            vec![schema.content, schema.code_content, schema.concepts],
        );

        Ok(Self { index, reader, schema, query_parser })
    }

    fn build_schema() -> SessionSchema {
        let mut builder = Schema::builder();

        SessionSchema {
            session_id: builder.add_text_field("session_id", STRING | STORED),
            message_id: builder.add_text_field("message_id", STRING | STORED),
            source: builder.add_text_field("source", STRING | STORED | FAST),
            content: builder.add_text_field("content", TEXT | STORED),
            code_content: builder.add_text_field("code_content", TEXT | STORED),
            timestamp: builder.add_i64_field("timestamp", INDEXED | STORED | FAST),
            role: builder.add_text_field("role", STRING | FAST),
            language: builder.add_text_field("language", STRING | FAST),
            project_path: builder.add_text_field("project_path", STRING | STORED),
            concepts: builder.add_text_field("concepts", TEXT | STORED),
        }
    }

    fn register_tokenizers(index: &Index) {
        // Edge n-gram tokenizer for code patterns
        let code_tokenizer = TextAnalyzer::builder(EdgeNgramTokenizer::new(2, 15))
            .filter(LowerCaser)
            .build();

        index.tokenizers().register("code", code_tokenizer);

        // Standard tokenizer with stemming for natural language
        let text_tokenizer = TextAnalyzer::builder(SimpleTokenizer::default())
            .filter(LowerCaser)
            .filter(Stemmer::new(Language::English))
            .build();

        index.tokenizers().register("text", text_tokenizer);
    }

    /// Search sessions with query
    pub fn search(&self, query: &str, options: SearchOptions) -> Result<SearchResults, IndexError> {
        let searcher = self.reader.searcher();
        let query = self.query_parser.parse_query(query)?;

        let top_docs = searcher.search(
            &query,
            &TopDocs::with_limit(options.limit.unwrap_or(10)),
        )?;

        let results = top_docs
            .into_iter()
            .map(|(score, doc_address)| {
                let doc = searcher.doc(doc_address)?;
                self.doc_to_search_result(doc, score)
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(SearchResults {
            results,
            total_hits: top_docs.len(),
            elapsed: Duration::from_millis(0), // TODO: measure
        })
    }
}
```

### 6. Knowledge Graph Enrichment

**Location**: `crates/terraphim_sessions/src/enrichment/`

```rust
use terraphim_automata::{AutocompleteIndex, load_thesaurus};
use terraphim_rolegraph::RoleGraph;

/// Enriches sessions with knowledge graph concepts
pub struct SessionEnricher {
    /// Automata index for concept detection
    automata: Arc<AutocompleteIndex>,
    /// Role graph for relationship building
    rolegraph: Arc<RwLock<RoleGraph>>,
}

impl SessionEnricher {
    /// Enrich a session with concepts
    pub async fn enrich(&self, session: &mut Session) -> EnrichmentResult {
        let mut concepts = HashSet::new();
        let mut concept_matches = Vec::new();

        for message in &mut session.messages {
            // Extract concepts from message content
            let matches = self.automata.find_matches(&message.content);

            for matched in matches {
                concepts.insert(matched.term.clone());
                concept_matches.push(ConceptMatch {
                    concept: matched.term.clone(),
                    message_id: message.id,
                    position: matched.position,
                    confidence: matched.score,
                });
            }

            // Store concepts in message
            message.concepts = concepts.iter().cloned().collect();

            // Also check code snippets
            for snippet in &message.snippets {
                let code_matches = self.automata.find_matches(&snippet.content);
                for matched in code_matches {
                    concepts.insert(matched.term.clone());
                }
            }
        }

        // Find concept connections
        let connections = self.find_concept_connections(&concepts).await;

        EnrichmentResult {
            session_id: session.id,
            concepts: concepts.into_iter().collect(),
            concept_matches,
            connections,
            dominant_topics: self.identify_dominant_topics(&concept_matches),
        }
    }

    /// Find connections between concepts via knowledge graph
    async fn find_concept_connections(
        &self,
        concepts: &HashSet<String>,
    ) -> Vec<(String, String)> {
        let rolegraph = self.rolegraph.read().await;
        let concept_list: Vec<_> = concepts.iter().collect();
        let mut connections = Vec::new();

        // Check pairwise connections
        for i in 0..concept_list.len() {
            for j in (i + 1)..concept_list.len() {
                if rolegraph.are_connected(concept_list[i], concept_list[j]) {
                    connections.push((
                        concept_list[i].clone(),
                        concept_list[j].clone(),
                    ));
                }
            }
        }

        connections
    }
}
```

## Data Flow

### Import Flow

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Source    │────▶│  Connector  │────▶│   Parser    │────▶│  Session    │
│   Files     │     │  (detect)   │     │ (normalize) │     │   Model     │
└─────────────┘     └─────────────┘     └─────────────┘     └──────┬──────┘
                                                                    │
                    ┌─────────────┐     ┌─────────────┐            │
                    │   Index     │◀────│  Enricher   │◀───────────┘
                    │  (Tantivy)  │     │ (concepts)  │
                    └─────────────┘     └─────────────┘
```

### Search Flow

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Query     │────▶│  Forgiving  │────▶│   Query     │
│   Input     │     │   Parser    │     │  Expansion  │
└─────────────┘     └─────────────┘     └──────┬──────┘
                                               │
                    ┌─────────────┐            │
                    │   Tantivy   │◀───────────┘
                    │   Search    │
                    └──────┬──────┘
                           │
┌─────────────┐     ┌──────▼──────┐     ┌─────────────┐
│   Robot     │◀────│   Result    │◀────│  Concept    │
│   Output    │     │  Formatter  │     │  Expansion  │
└─────────────┘     └─────────────┘     └─────────────┘
```

## Integration with Existing Components

### terraphim_automata Integration

```rust
// Use existing fuzzy search for forgiving CLI
use terraphim_automata::fuzzy_autocomplete_search_jaro_winkler;

// Use existing concept extraction for enrichment
use terraphim_automata::{
    AutocompleteIndex,
    find_matches,
    extract_paragraphs_from_automata,
};
```

### terraphim_service Integration

```rust
// Sessions integrate with existing search
impl TuiService {
    pub async fn search_with_sessions(
        &self,
        query: &str,
        options: SearchOptions,
    ) -> SearchResults {
        // Search documents
        let doc_results = self.search(query).await?;

        // Search sessions
        let session_results = self.session_index.search(query, options)?;

        // Merge and rank
        self.merge_results(doc_results, session_results)
    }
}
```

### terraphim_config Integration

```rust
// Session configuration in role config
#[derive(Debug, Deserialize)]
pub struct SessionConfig {
    /// Enable session indexing
    pub enabled: bool,
    /// Session sources to index
    pub sources: Vec<String>,
    /// Index storage path
    pub index_path: PathBuf,
    /// Auto-import on startup
    pub auto_import: bool,
}
```

## Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("Connector error: {0}")]
    Connector(#[from] ConnectorError),

    #[error("Index error: {0}")]
    Index(#[from] IndexError),

    #[error("Enrichment error: {0}")]
    Enrichment(#[from] EnrichmentError),

    #[error("Parse error: {0}")]
    Parse(#[from] ParseError),
}

#[derive(Debug, thiserror::Error)]
pub enum ConnectorError {
    #[error("Source not found: {path}")]
    NotFound { path: PathBuf },

    #[error("Permission denied: {path}")]
    PermissionDenied { path: PathBuf },

    #[error("Invalid format: {message}")]
    InvalidFormat { message: String },

    #[error("Watch not supported for this connector")]
    WatchNotSupported,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
```

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_forgiving_parser_exact_match() {
        let parser = ForgivingParser::default();
        let result = parser.parse("/search query");
        assert!(matches!(result, ParseResult::Exact(_)));
    }

    #[test]
    fn test_forgiving_parser_typo_correction() {
        let parser = ForgivingParser::default();
        let result = parser.parse("/serach query");
        assert!(matches!(result, ParseResult::AutoCorrected { .. }));
    }

    #[tokio::test]
    async fn test_claude_code_connector_detect() {
        let connector = ClaudeCodeConnector::new();
        let status = connector.detect().await;
        // Status depends on environment
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_session_import_and_search() {
    let temp_dir = tempdir().unwrap();
    let index = SessionIndex::new(temp_dir.path()).unwrap();

    // Create test session
    let session = Session {
        id: Uuid::new_v4(),
        source: "test".into(),
        messages: vec![Message {
            content: "How do I handle async errors in Rust?".into(),
            ..Default::default()
        }],
        ..Default::default()
    };

    // Index session
    index.add_session(&session).unwrap();
    index.commit().unwrap();

    // Search
    let results = index.search("async errors Rust", Default::default()).unwrap();
    assert_eq!(results.results.len(), 1);
}
```

## Performance Considerations

### Index Performance

- **Batch writes**: Commit after every 1000 documents
- **Reader reload**: Use `reader.reload()` for real-time search
- **Segment merging**: Configure merge policy for read-heavy workload

### Memory Management

- **Streaming import**: Process files in chunks, not all at once
- **Index caching**: Keep hot segments in memory
- **Result pagination**: Default limit of 10, max of 100

### Startup Optimization

- **Lazy loading**: Don't load index until first search
- **Background indexing**: Import new sessions async
- **Warm-up queries**: Pre-warm common searches

## Security Considerations

### Data Privacy

- All data stored locally
- No network calls for session data
- File permissions respected

### Secret Detection

```rust
lazy_static! {
    static ref SECRET_PATTERNS: Vec<Regex> = vec![
        Regex::new(r"(?i)(api[_-]?key|secret|password|token)\s*[:=]\s*['\"]?[\w-]+").unwrap(),
        Regex::new(r"sk-[a-zA-Z0-9]{32,}").unwrap(),  // OpenAI
        Regex::new(r"ghp_[a-zA-Z0-9]{36}").unwrap(),  // GitHub
    ];
}

fn redact_secrets(content: &str) -> String {
    let mut result = content.to_string();
    for pattern in SECRET_PATTERNS.iter() {
        result = pattern.replace_all(&result, "[REDACTED]").to_string();
    }
    result
}
```

## Future Extensions

### Phase 2+ Considerations

1. **Semantic Search**: Add embedding support alongside BM25
2. **Cross-Machine Sync**: Optional encrypted sync
3. **Session Replay**: Interactive session playback
4. **Analytics Dashboard**: TUI-based analytics view

### Plugin Architecture

```rust
/// Plugin trait for custom connectors
pub trait ConnectorPlugin: SessionConnector {
    fn metadata(&self) -> PluginMetadata;
    fn initialize(&mut self, config: &Config) -> Result<()>;
}

/// Dynamic connector loading
pub struct ConnectorRegistry {
    builtin: Vec<Box<dyn SessionConnector>>,
    plugins: Vec<Box<dyn ConnectorPlugin>>,
}
```
