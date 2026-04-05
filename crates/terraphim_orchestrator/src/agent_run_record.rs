//! Structured agent run records with KG-boosted exit classification.
//!
//! Every agent run produces an `AgentRunRecord` with an `ExitClass` classified
//! via terraphim-automata Aho-Corasick matching on stderr/stdout patterns.
//!
//! The `ExitClassifier` builds a thesaurus from known exit patterns (see
//! `docs/src/kg/exit_classes.md`) and uses `find_matches()` to classify agent
//! output. When multiple exit classes match, the one with the highest match
//! count wins.
//!
//! # Architecture
//!
//! ```text
//! Agent exits (poll_agent_exits)
//!     |
//!     v
//! ExitClassifier::classify(exit_code, stdout, stderr)
//!     |-- build thesaurus (Concept per ExitClass, patterns as synonyms)
//!     |-- find_matches(combined_text, thesaurus)
//!     |-- count matches per ExitClass
//!     |-- pick highest count (or fallback to exit code)
//!     v
//! AgentRunRecord { exit_class, matched_patterns, confidence, ... }
//!     |
//!     +-> terraphim_persistence (Persistable)
//!     +-> quickwit LogDocument.extra
//!     +-> SharedLearningStore (learning generation)
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use terraphim_automata::matcher::find_matches;
use terraphim_types::{Concept, NormalizedTerm, NormalizedTermValue, Thesaurus};
use tracing::{debug, warn};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// ExitClass enum
// ---------------------------------------------------------------------------

/// Classified exit type for an agent run.
///
/// Determined by Aho-Corasick pattern matching on agent stdout/stderr,
/// using the exit class thesaurus from `docs/src/kg/exit_classes.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExitClass {
    /// Exit code 0 with meaningful output
    Success,
    /// Exit code 0 but no output (suspicious)
    EmptySuccess,
    /// Timed out, deadline exceeded, wall-clock kill
    Timeout,
    /// HTTP 429, rate limit, quota exceeded
    RateLimit,
    /// Compiler errors (error[E, unresolved import, etc.)
    CompilationError,
    /// Test failures (test result: FAILED, panicked at, etc.)
    TestFailure,
    /// LLM errors (model not found, context length, invalid API key)
    ModelError,
    /// Network failures (connection refused, DNS, ECONNRESET)
    NetworkError,
    /// OOM, disk full, no space left
    ResourceExhaustion,
    /// Permission denied, EACCES, 403
    PermissionDenied,
    /// SIGSEGV, SIGKILL, panic, stack overflow
    Crash,
    /// No patterns matched and non-zero exit
    Unknown,
}

impl fmt::Display for ExitClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExitClass::Success => write!(f, "success"),
            ExitClass::EmptySuccess => write!(f, "empty_success"),
            ExitClass::Timeout => write!(f, "timeout"),
            ExitClass::RateLimit => write!(f, "rate_limit"),
            ExitClass::CompilationError => write!(f, "compilation_error"),
            ExitClass::TestFailure => write!(f, "test_failure"),
            ExitClass::ModelError => write!(f, "model_error"),
            ExitClass::NetworkError => write!(f, "network_error"),
            ExitClass::ResourceExhaustion => write!(f, "resource_exhaustion"),
            ExitClass::PermissionDenied => write!(f, "permission_denied"),
            ExitClass::Crash => write!(f, "crash"),
            ExitClass::Unknown => write!(f, "unknown"),
        }
    }
}

impl ExitClass {
    /// Parse an ExitClass from its concept name in the thesaurus.
    fn from_concept_name(name: &str) -> Option<Self> {
        match name {
            "timeout" => Some(ExitClass::Timeout),
            "ratelimit" => Some(ExitClass::RateLimit),
            "compilationerror" => Some(ExitClass::CompilationError),
            "testfailure" => Some(ExitClass::TestFailure),
            "modelerror" => Some(ExitClass::ModelError),
            "networkerror" => Some(ExitClass::NetworkError),
            "resourceexhaustion" => Some(ExitClass::ResourceExhaustion),
            "permissiondenied" => Some(ExitClass::PermissionDenied),
            "crash" => Some(ExitClass::Crash),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// RunTrigger
// ---------------------------------------------------------------------------

/// What triggered an agent run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunTrigger {
    /// Scheduled via cron expression
    Cron,
    /// Triggered by @mention in Gitea issue/comment
    Mention,
    /// Triggered as part of a Flow DAG
    Flow,
    /// Manual trigger (CLI, webhook, etc.)
    Manual,
}

impl fmt::Display for RunTrigger {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RunTrigger::Cron => write!(f, "cron"),
            RunTrigger::Mention => write!(f, "mention"),
            RunTrigger::Flow => write!(f, "flow"),
            RunTrigger::Manual => write!(f, "manual"),
        }
    }
}

// ---------------------------------------------------------------------------
// AgentRunRecord
// ---------------------------------------------------------------------------

/// Structured record of a single agent run.
///
/// Produced by the reconciliation loop after an agent exits.
/// Persisted via `terraphim_persistence` and shipped to Quickwit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRunRecord {
    /// Unique identifier for this run
    pub run_id: Uuid,
    /// Name of the agent
    pub agent_name: String,
    /// When the run started
    pub started_at: DateTime<Utc>,
    /// When the run ended
    pub ended_at: DateTime<Utc>,
    /// Raw process exit code (None if killed by signal)
    pub exit_code: Option<i32>,
    /// Classified exit type (KG-boosted)
    pub exit_class: ExitClass,
    /// Model used for this run
    pub model_used: Option<String>,
    /// Whether a fallback model was used
    pub was_fallback: bool,
    /// Wall-clock duration in seconds
    pub wall_time_secs: f64,
    /// First 500 chars of stdout
    pub output_summary: String,
    /// First 500 chars of stderr
    pub error_summary: String,
    /// What triggered this run
    pub trigger: RunTrigger,
    /// Patterns matched during exit classification
    pub matched_patterns: Vec<String>,
    /// Classification confidence (0.0 - 1.0)
    pub confidence: f64,
}

impl AgentRunRecord {
    /// Truncate text to max_len characters.
    fn truncate(text: &str, max_len: usize) -> String {
        if text.len() <= max_len {
            text.to_string()
        } else {
            format!("{}...", &text[..max_len])
        }
    }

    /// Build the output summary from collected stdout lines.
    pub fn summarise_output(lines: &[String]) -> String {
        let combined = lines.join("\n");
        Self::truncate(&combined, 500)
    }

    /// Build the error summary from collected stderr lines.
    pub fn summarise_errors(lines: &[String]) -> String {
        let combined = lines.join("\n");
        Self::truncate(&combined, 500)
    }
}

// ---------------------------------------------------------------------------
// ExitClassifier
// ---------------------------------------------------------------------------

/// Classifies agent exits using Aho-Corasick pattern matching on stdout/stderr.
///
/// Builds a thesaurus where each `ExitClass` is a concept and known error
/// patterns are synonyms. Uses `terraphim_automata::find_matches()` to
/// identify patterns in agent output.
pub struct ExitClassifier {
    thesaurus: Thesaurus,
}

/// A pattern definition: (concept_name, patterns)
struct PatternDef {
    concept_name: &'static str,
    patterns: &'static [&'static str],
}

/// Exit class pattern definitions matching `docs/src/kg/exit_classes.md`.
const EXIT_CLASS_PATTERNS: &[PatternDef] = &[
    PatternDef {
        concept_name: "timeout",
        patterns: &[
            "timed out",
            "deadline exceeded",
            "wall-clock kill",
            "context deadline exceeded",
            "operation timed out",
            "execution expired",
        ],
    },
    PatternDef {
        concept_name: "ratelimit",
        patterns: &[
            "429",
            "rate limit",
            "too many requests",
            "quota exceeded",
            "rate_limit_exceeded",
            "throttled",
        ],
    },
    PatternDef {
        concept_name: "compilationerror",
        patterns: &[
            "error[E",
            "cannot find",
            "unresolved import",
            "cargo build failed",
            "failed to compile",
            "aborting due to",
            "could not compile",
        ],
    },
    PatternDef {
        concept_name: "testfailure",
        patterns: &[
            "test result: FAILED",
            "failures:",
            "panicked at",
            "assertion failed",
            "thread 'main' panicked",
            "cargo test failed",
        ],
    },
    PatternDef {
        concept_name: "modelerror",
        patterns: &[
            "model not found",
            "context length exceeded",
            "invalid api key",
            "invalid_api_key",
            "model_not_found",
            "insufficient_quota",
            "content_policy_violation",
        ],
    },
    PatternDef {
        concept_name: "networkerror",
        patterns: &[
            "connection refused",
            "dns resolution",
            "ECONNRESET",
            "ssl handshake",
            "network unreachable",
            "connection reset",
            "ENOTFOUND",
            "ETIMEDOUT",
        ],
    },
    PatternDef {
        concept_name: "resourceexhaustion",
        patterns: &[
            "out of memory",
            "OOM",
            "no space left",
            "disk full",
            "cannot allocate memory",
            "memory allocation failed",
        ],
    },
    PatternDef {
        concept_name: "permissiondenied",
        patterns: &[
            "permission denied",
            "EACCES",
            "403 Forbidden",
            "access denied",
            "insufficient permissions",
            "not authorized",
        ],
    },
    PatternDef {
        concept_name: "crash",
        patterns: &[
            "SIGSEGV",
            "SIGKILL",
            "stack overflow",
            "SIGABRT",
            "segmentation fault",
            "bus error",
            "SIGBUS",
        ],
    },
];

impl ExitClassifier {
    /// Create a new ExitClassifier with the built-in exit class thesaurus.
    pub fn new() -> Self {
        Self {
            thesaurus: Self::build_thesaurus(),
        }
    }

    /// Build a thesaurus from the exit class pattern definitions.
    ///
    /// Each exit class is a Concept, and its known stderr/stdout patterns
    /// are inserted as synonyms mapping to that concept.
    fn build_thesaurus() -> Thesaurus {
        let mut thesaurus = Thesaurus::new("exit_classes".to_string());

        for def in EXIT_CLASS_PATTERNS {
            let concept = Concept::from(def.concept_name.to_string());
            let nterm = NormalizedTerm::new(concept.id, concept.value.clone());

            // Insert the concept itself
            thesaurus.insert(concept.value.clone(), nterm.clone());

            // Insert each pattern as a synonym
            for pattern in def.patterns {
                thesaurus.insert(NormalizedTermValue::new(pattern.to_string()), nterm.clone());
            }
        }

        thesaurus
    }

    /// Classify an agent exit based on exit code and captured output.
    ///
    /// Uses Aho-Corasick matching from `terraphim_automata::find_matches()`
    /// against the exit class thesaurus. When multiple classes match,
    /// the one with the highest match count wins.
    pub fn classify(
        &self,
        exit_code: Option<i32>,
        stdout_lines: &[String],
        stderr_lines: &[String],
    ) -> ExitClassification {
        // Combine stdout and stderr for pattern matching
        let combined = format!("{}\n{}", stdout_lines.join("\n"), stderr_lines.join("\n"));

        // Handle success cases first
        if exit_code == Some(0) {
            // Check if output is empty (suspicious)
            let has_output = stdout_lines.iter().any(|l| !l.trim().is_empty());
            if !has_output {
                return ExitClassification {
                    exit_class: ExitClass::EmptySuccess,
                    matched_patterns: vec![],
                    confidence: 0.8,
                };
            }

            // Even for exit code 0, check for error patterns (some tools
            // return 0 but print errors to stderr)
            let classification = self.match_patterns(&combined);
            if classification.exit_class != ExitClass::Unknown {
                // Downgrade confidence since exit code was 0
                return ExitClassification {
                    confidence: classification.confidence * 0.5,
                    ..classification
                };
            }

            return ExitClassification {
                exit_class: ExitClass::Success,
                matched_patterns: vec![],
                confidence: 1.0,
            };
        }

        // Non-zero exit: classify by pattern matching
        let classification = self.match_patterns(&combined);
        if classification.exit_class != ExitClass::Unknown {
            return classification;
        }

        // No patterns matched, non-zero exit
        ExitClassification {
            exit_class: ExitClass::Unknown,
            matched_patterns: vec![],
            confidence: 0.0,
        }
    }

    /// Run Aho-Corasick matching and group by exit class.
    fn match_patterns(&self, text: &str) -> ExitClassification {
        let matches = match find_matches(text, self.thesaurus.clone(), false) {
            Ok(m) => m,
            Err(e) => {
                warn!(error = %e, "exit class pattern matching failed");
                return ExitClassification {
                    exit_class: ExitClass::Unknown,
                    matched_patterns: vec![],
                    confidence: 0.0,
                };
            }
        };

        if matches.is_empty() {
            return ExitClassification {
                exit_class: ExitClass::Unknown,
                matched_patterns: vec![],
                confidence: 0.0,
            };
        }

        // Group matches by exit class concept
        let mut class_counts: HashMap<String, (usize, Vec<String>)> = HashMap::new();
        for m in &matches {
            let concept_name = m.normalized_term.value.as_str().to_string();
            let entry = class_counts
                .entry(concept_name)
                .or_insert_with(|| (0, Vec::new()));
            entry.0 += 1;
            let pattern = m.term.clone();
            if !entry.1.contains(&pattern) {
                entry.1.push(pattern);
            }
        }

        debug!(
            matched_classes = ?class_counts.keys().collect::<Vec<_>>(),
            total_matches = matches.len(),
            "exit class pattern matches"
        );

        // Pick the exit class with the most matches
        let (best_concept, (count, matched_patterns)) = class_counts
            .into_iter()
            .max_by_key(|(_, (count, _))| *count)
            .expect("non-empty matches guaranteed above");

        let exit_class = ExitClass::from_concept_name(&best_concept).unwrap_or(ExitClass::Unknown);

        // Confidence: ratio of dominant class matches to total matches
        let confidence = if matches.is_empty() {
            0.0
        } else {
            (count as f64) / (matches.len() as f64)
        };

        ExitClassification {
            exit_class,
            matched_patterns,
            confidence,
        }
    }
}

impl Default for ExitClassifier {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of exit classification.
#[derive(Debug, Clone)]
pub struct ExitClassification {
    /// The classified exit type
    pub exit_class: ExitClass,
    /// Patterns that were matched
    pub matched_patterns: Vec<String>,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f64,
}

// ---------------------------------------------------------------------------
// Persistence
// ---------------------------------------------------------------------------

/// Persistence trait for agent run records.
///
/// Follows the same pattern as `LearningPersistence` in `learning.rs`.
#[async_trait::async_trait]
pub trait RunRecordPersistence: Send + Sync {
    /// Store a run record.
    async fn insert(&self, record: &AgentRunRecord) -> Result<(), RunRecordError>;

    /// Query records by agent name.
    async fn query_by_agent(&self, agent_name: &str)
        -> Result<Vec<AgentRunRecord>, RunRecordError>;

    /// Query records by exit class.
    async fn query_by_exit_class(
        &self,
        exit_class: ExitClass,
    ) -> Result<Vec<AgentRunRecord>, RunRecordError>;

    /// Count records by exit class in a time range.
    async fn count_by_class_since(
        &self,
        since: DateTime<Utc>,
    ) -> Result<HashMap<ExitClass, usize>, RunRecordError>;
}

/// Errors for run record persistence.
#[derive(Debug, thiserror::Error)]
pub enum RunRecordError {
    #[error("storage error: {0}")]
    Storage(String),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// In-memory implementation for testing.
#[derive(Default)]
pub struct InMemoryRunRecordStore {
    records: std::sync::Mutex<Vec<AgentRunRecord>>,
}

#[async_trait::async_trait]
impl RunRecordPersistence for InMemoryRunRecordStore {
    async fn insert(&self, record: &AgentRunRecord) -> Result<(), RunRecordError> {
        let mut records = self
            .records
            .lock()
            .map_err(|e| RunRecordError::Storage(e.to_string()))?;
        records.push(record.clone());
        Ok(())
    }

    async fn query_by_agent(
        &self,
        agent_name: &str,
    ) -> Result<Vec<AgentRunRecord>, RunRecordError> {
        let records = self
            .records
            .lock()
            .map_err(|e| RunRecordError::Storage(e.to_string()))?;
        Ok(records
            .iter()
            .filter(|r| r.agent_name == agent_name)
            .cloned()
            .collect())
    }

    async fn query_by_exit_class(
        &self,
        exit_class: ExitClass,
    ) -> Result<Vec<AgentRunRecord>, RunRecordError> {
        let records = self
            .records
            .lock()
            .map_err(|e| RunRecordError::Storage(e.to_string()))?;
        Ok(records
            .iter()
            .filter(|r| r.exit_class == exit_class)
            .cloned()
            .collect())
    }

    async fn count_by_class_since(
        &self,
        since: DateTime<Utc>,
    ) -> Result<HashMap<ExitClass, usize>, RunRecordError> {
        let records = self
            .records
            .lock()
            .map_err(|e| RunRecordError::Storage(e.to_string()))?;
        let mut counts: HashMap<ExitClass, usize> = HashMap::new();
        for record in records.iter().filter(|r| r.ended_at >= since) {
            *counts.entry(record.exit_class).or_insert(0) += 1;
        }
        Ok(counts)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn classifier() -> ExitClassifier {
        ExitClassifier::new()
    }

    #[test]
    fn classify_success_with_output() {
        let c = classifier();
        let result = c.classify(Some(0), &["review complete, 3 findings".to_string()], &[]);
        assert_eq!(result.exit_class, ExitClass::Success);
        assert!(result.confidence > 0.9);
    }

    #[test]
    fn classify_empty_success() {
        let c = classifier();
        let result = c.classify(Some(0), &[], &[]);
        assert_eq!(result.exit_class, ExitClass::EmptySuccess);
    }

    #[test]
    fn classify_timeout() {
        let c = classifier();
        let result = c.classify(
            Some(1),
            &[],
            &["error: operation timed out after 300s".to_string()],
        );
        assert_eq!(result.exit_class, ExitClass::Timeout);
        assert!(result.confidence > 0.0);
        assert!(result
            .matched_patterns
            .iter()
            .any(|p| p.contains("timed out")));
    }

    #[test]
    fn classify_rate_limit() {
        let c = classifier();
        let result = c.classify(
            Some(1),
            &[],
            &[
                "HTTP 429 Too Many Requests".to_string(),
                "rate limit exceeded, retrying in 60s".to_string(),
            ],
        );
        assert_eq!(result.exit_class, ExitClass::RateLimit);
        assert!(result.matched_patterns.len() >= 2);
    }

    #[test]
    fn classify_compilation_error() {
        let c = classifier();
        let result = c.classify(
            Some(101),
            &[],
            &[
                "error[E0433]: failed to resolve: use of undeclared crate or module".to_string(),
                "error[E0412]: cannot find type `FooBar`".to_string(),
                "error: aborting due to 2 previous errors".to_string(),
            ],
        );
        assert_eq!(result.exit_class, ExitClass::CompilationError);
    }

    #[test]
    fn classify_test_failure() {
        let c = classifier();
        let result = c.classify(
            Some(101),
            &[
                "running 5 tests".to_string(),
                "test result: FAILED. 3 passed; 2 failed; 0 ignored".to_string(),
            ],
            &["thread 'main' panicked at 'assertion failed'".to_string()],
        );
        assert_eq!(result.exit_class, ExitClass::TestFailure);
    }

    #[test]
    fn classify_model_error() {
        let c = classifier();
        let result = c.classify(
            Some(1),
            &[],
            &["Error: model not found: gpt-5-turbo".to_string()],
        );
        assert_eq!(result.exit_class, ExitClass::ModelError);
    }

    #[test]
    fn classify_network_error() {
        let c = classifier();
        let result = c.classify(
            Some(1),
            &[],
            &["Error: connection refused (os error 111)".to_string()],
        );
        assert_eq!(result.exit_class, ExitClass::NetworkError);
    }

    #[test]
    fn classify_resource_exhaustion() {
        let c = classifier();
        let result = c.classify(
            Some(137),
            &[],
            &["fatal: out of memory, malloc failed".to_string()],
        );
        assert_eq!(result.exit_class, ExitClass::ResourceExhaustion);
    }

    #[test]
    fn classify_permission_denied() {
        let c = classifier();
        let result = c.classify(
            Some(1),
            &[],
            &["Error: permission denied (os error 13)".to_string()],
        );
        assert_eq!(result.exit_class, ExitClass::PermissionDenied);
    }

    #[test]
    fn classify_crash() {
        let c = classifier();
        let result = c.classify(
            Some(139),
            &[],
            &["fatal runtime error: stack overflow".to_string()],
        );
        assert_eq!(result.exit_class, ExitClass::Crash);
    }

    #[test]
    fn classify_unknown_exit() {
        let c = classifier();
        let result = c.classify(
            Some(42),
            &["some generic output".to_string()],
            &["some generic error".to_string()],
        );
        assert_eq!(result.exit_class, ExitClass::Unknown);
        assert_eq!(result.confidence, 0.0);
    }

    #[test]
    fn classify_mixed_patterns_picks_dominant() {
        let c = classifier();
        // stderr has 1 timeout pattern and 3 compilation error patterns
        let result = c.classify(
            Some(1),
            &[],
            &[
                "error: operation timed out".to_string(),
                "error[E0433]: cannot find module".to_string(),
                "error[E0412]: cannot find type".to_string(),
                "error: aborting due to 2 previous errors".to_string(),
            ],
        );
        // CompilationError should win because it has more matches
        assert_eq!(result.exit_class, ExitClass::CompilationError);
    }

    #[test]
    fn exit_class_display_roundtrip() {
        for class in [
            ExitClass::Success,
            ExitClass::EmptySuccess,
            ExitClass::Timeout,
            ExitClass::RateLimit,
            ExitClass::CompilationError,
            ExitClass::TestFailure,
            ExitClass::ModelError,
            ExitClass::NetworkError,
            ExitClass::ResourceExhaustion,
            ExitClass::PermissionDenied,
            ExitClass::Crash,
            ExitClass::Unknown,
        ] {
            let display = class.to_string();
            assert!(
                !display.is_empty(),
                "ExitClass::Display should not be empty"
            );
        }
    }

    #[test]
    fn exit_class_serialization() {
        let class = ExitClass::CompilationError;
        let json = serde_json::to_string(&class).unwrap();
        assert_eq!(json, r#""compilation_error""#);
        let deserialized: ExitClass = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, class);
    }

    #[test]
    fn agent_run_record_serialization() {
        let record = AgentRunRecord {
            run_id: Uuid::nil(),
            agent_name: "test-agent".to_string(),
            started_at: Utc::now(),
            ended_at: Utc::now(),
            exit_code: Some(1),
            exit_class: ExitClass::Timeout,
            model_used: Some("kimi-k2.5".to_string()),
            was_fallback: false,
            wall_time_secs: 42.5,
            output_summary: "some output".to_string(),
            error_summary: "timed out".to_string(),
            trigger: RunTrigger::Cron,
            matched_patterns: vec!["timed out".to_string()],
            confidence: 0.95,
        };
        let json = serde_json::to_string(&record).unwrap();
        let deserialized: AgentRunRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.exit_class, ExitClass::Timeout);
        assert_eq!(deserialized.agent_name, "test-agent");
    }

    #[test]
    fn summarise_truncates_long_output() {
        let lines: Vec<String> = (0..100).map(|i| format!("line {}", i)).collect();
        let summary = AgentRunRecord::summarise_output(&lines);
        assert!(summary.len() <= 504); // 500 + "..."
    }

    #[tokio::test]
    async fn in_memory_store_insert_and_query() {
        let store = InMemoryRunRecordStore::default();
        let record = AgentRunRecord {
            run_id: Uuid::new_v4(),
            agent_name: "test-agent".to_string(),
            started_at: Utc::now(),
            ended_at: Utc::now(),
            exit_code: Some(1),
            exit_class: ExitClass::Timeout,
            model_used: None,
            was_fallback: false,
            wall_time_secs: 10.0,
            output_summary: String::new(),
            error_summary: "timed out".to_string(),
            trigger: RunTrigger::Cron,
            matched_patterns: vec!["timed out".to_string()],
            confidence: 0.9,
        };

        store.insert(&record).await.unwrap();

        let by_agent = store.query_by_agent("test-agent").await.unwrap();
        assert_eq!(by_agent.len(), 1);
        assert_eq!(by_agent[0].exit_class, ExitClass::Timeout);

        let by_class = store.query_by_exit_class(ExitClass::Timeout).await.unwrap();
        assert_eq!(by_class.len(), 1);

        let by_class_empty = store.query_by_exit_class(ExitClass::Crash).await.unwrap();
        assert!(by_class_empty.is_empty());
    }
}
