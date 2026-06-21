---
planStatus:
  planId: design-gitea82-correction-event
  title: "Design: Phase 1 -- Expand learning capture (CorrectionEvent)"
  status: approved
  planType: implementation-design
  priority: high
  owner: alex
  tags: [terraphim-agent, learning-capture, correction-event, gitea-82]
  created: "2026-03-25"
  updated: "2026-03-25"
  progress: 0
  source: "plans/knowledge-suggestion-terraphim-agent.md"
  parentIssue: "terraphim/terraphim-ai#82 (Gitea)"
  parentEpic: "terraphim/terraphim-ai#81 (Gitea)"
---

# Implementation Design: Gitea #82 -- CorrectionEvent for Learning Capture

## Scope

Phase 1.1 and 1.2 only. Adds `CorrectionEvent` struct and `learn correction` CLI subcommand. Does NOT touch hooks (Phase 1.3-1.4, which overlap with GH#599) or Phase 2-5 code.

## File Changes

### 1. `crates/terraphim_agent/src/learnings/capture.rs`

#### 1.1 Add `CorrectionType` enum (after line 39, after `LearningSource`)

```rust
/// Type of user correction captured during an agent session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CorrectionType {
    /// "use bun instead of npm"
    ToolPreference,
    /// "we use Result<T> not unwrap()"
    CodePattern,
    /// "we call it X not Y"
    Naming,
    /// "always run tests before committing"
    WorkflowStep,
    /// "the endpoint is /api/v2 not /api/v1"
    FactCorrection,
    /// "use British English"
    StylePreference,
    /// Catchall
    Other(String),
}

impl std::fmt::Display for CorrectionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CorrectionType::ToolPreference => write!(f, "tool-preference"),
            CorrectionType::CodePattern => write!(f, "code-pattern"),
            CorrectionType::Naming => write!(f, "naming"),
            CorrectionType::WorkflowStep => write!(f, "workflow-step"),
            CorrectionType::FactCorrection => write!(f, "fact-correction"),
            CorrectionType::StylePreference => write!(f, "style-preference"),
            CorrectionType::Other(s) => write!(f, "other:{}", s),
        }
    }
}

impl std::str::FromStr for CorrectionType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "tool-preference" => Ok(CorrectionType::ToolPreference),
            "code-pattern" => Ok(CorrectionType::CodePattern),
            "naming" => Ok(CorrectionType::Naming),
            "workflow-step" => Ok(CorrectionType::WorkflowStep),
            "fact-correction" => Ok(CorrectionType::FactCorrection),
            "style-preference" => Ok(CorrectionType::StylePreference),
            other => {
                if let Some(suffix) = other.strip_prefix("other:") {
                    Ok(CorrectionType::Other(suffix.to_string()))
                } else {
                    Ok(CorrectionType::Other(other.to_string()))
                }
            }
        }
    }
}
```

#### 1.2 Add `CorrectionEvent` struct (after `CapturedLearning` impl block, ~line 134)

```rust
/// A user correction captured during an agent session.
/// Unlike CapturedLearning (which captures failed commands),
/// CorrectionEvent captures any user feedback: preferences,
/// naming conventions, workflow steps, fact corrections.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrectionEvent {
    /// Unique ID (UUID-timestamp, same format as CapturedLearning)
    pub id: String,
    /// Type of correction
    pub correction_type: CorrectionType,
    /// What the agent said/did originally
    pub original: String,
    /// What the user said instead
    pub corrected: String,
    /// Surrounding context (conversation snippet, file path, etc.)
    pub context_description: String,
    /// Source: project or global
    pub source: LearningSource,
    /// Context metadata
    pub context: LearningContext,
    /// Session ID for traceability
    pub session_id: Option<String>,
    /// Tags for categorisation
    pub tags: Vec<String>,
}

impl CorrectionEvent {
    /// Create a new correction event.
    pub fn new(
        correction_type: CorrectionType,
        original: String,
        corrected: String,
        context_description: String,
        source: LearningSource,
    ) -> Self {
        let id = format!("{}-{}", Uuid::new_v4().simple(), timestamp_millis());
        Self {
            id,
            correction_type,
            original,
            corrected,
            context_description,
            source,
            context: LearningContext::default(),
            session_id: None,
            tags: Vec::new(),
        }
    }

    /// Set session ID.
    pub fn with_session_id(mut self, session_id: String) -> Self {
        self.session_id = Some(session_id);
        self
    }

    /// Add tags.
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Convert to markdown format for storage.
    /// Uses same YAML frontmatter pattern as CapturedLearning.
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();

        // Frontmatter
        md.push_str("---\n");
        md.push_str(&format!("id: {}\n", self.id));
        md.push_str(&format!("type: correction\n"));
        md.push_str(&format!("correction_type: {}\n", self.correction_type));
        md.push_str(&format!("source: {:?}\n", self.source));
        md.push_str(&format!(
            "captured_at: {}\n",
            self.context.captured_at.to_rfc3339()
        ));
        md.push_str(&format!("working_dir: {}\n", self.context.working_dir));

        if let Some(ref hostname) = self.context.hostname {
            md.push_str(&format!("hostname: {}\n", hostname));
        }

        if let Some(ref session_id) = self.session_id {
            md.push_str(&format!("session_id: {}\n", session_id));
        }

        if !self.tags.is_empty() {
            md.push_str("tags:\n");
            for tag in &self.tags {
                md.push_str(&format!("  - {}\n", tag));
            }
        }

        md.push_str("---\n\n");

        // Body
        md.push_str("## Original\n\n");
        md.push_str(&format!("`{}`\n\n", self.original));

        md.push_str("## Corrected\n\n");
        md.push_str(&format!("`{}`\n\n", self.corrected));

        if !self.context_description.is_empty() {
            md.push_str("## Context\n\n");
            md.push_str(&self.context_description);
            md.push_str("\n\n");
        }

        md
    }

    /// Parse from markdown file content.
    pub fn from_markdown(content: &str) -> Option<Self> {
        let parts: Vec<&str> = content.splitn(3, "---").collect();
        if parts.len() < 3 {
            return None;
        }

        let frontmatter = parts[1].trim();
        let body = parts[2].trim();

        let mut id = String::new();
        let mut correction_type = CorrectionType::Other("unknown".to_string());
        let mut source = LearningSource::Project;
        let mut captured_at = Utc::now();
        let mut working_dir = String::new();
        let mut hostname = None;
        let mut session_id = None;
        let mut file_type = String::new();
        let tags = Vec::new();

        for line in frontmatter.lines() {
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim();
                let value = value.trim();
                match key {
                    "id" => id = value.to_string(),
                    "type" => file_type = value.to_string(),
                    "correction_type" => {
                        correction_type = value.parse().unwrap_or(
                            CorrectionType::Other("unknown".to_string())
                        );
                    }
                    "source" => {
                        source = if value == "Project" {
                            LearningSource::Project
                        } else {
                            LearningSource::Global
                        }
                    }
                    "captured_at" => {
                        captured_at = DateTime::parse_from_rfc3339(value)
                            .map(|d| d.with_timezone(&Utc))
                            .unwrap_or_else(|_| Utc::now())
                    }
                    "working_dir" => working_dir = value.to_string(),
                    "hostname" => hostname = Some(value.to_string()),
                    "session_id" => session_id = Some(value.to_string()),
                    _ => {}
                }
            }
        }

        // Must be a correction file
        if file_type != "correction" {
            return None;
        }

        // Extract original and corrected from body
        let original = extract_code_after_heading(body, "## Original");
        let corrected = extract_code_after_heading(body, "## Corrected");
        let context_description = extract_section_text(body, "## Context");

        Some(Self {
            id,
            correction_type,
            original: original.unwrap_or_default(),
            corrected: corrected.unwrap_or_default(),
            context_description: context_description.unwrap_or_default(),
            source,
            context: LearningContext {
                working_dir,
                captured_at,
                hostname,
                user: None,
            },
            session_id,
            tags,
        })
    }
}
```

#### 1.3 Add helper functions for markdown parsing

```rust
/// Extract inline code after a markdown heading.
fn extract_code_after_heading(body: &str, heading: &str) -> Option<String> {
    let idx = body.find(heading)?;
    let after = &body[idx + heading.len()..];
    // Find the first backtick-delimited code
    let start = after.find('`')? + 1;
    let rest = &after[start..];
    let end = rest.find('`')?;
    Some(rest[..end].to_string())
}

/// Extract plain text section after a heading (up to next heading or EOF).
fn extract_section_text(body: &str, heading: &str) -> Option<String> {
    let idx = body.find(heading)?;
    let after = &body[idx + heading.len()..].trim_start();
    // Find next heading or end
    let end = after.find("\n## ").unwrap_or(after.len());
    let text = after[..end].trim().to_string();
    if text.is_empty() { None } else { Some(text) }
}
```

#### 1.4 Add `capture_correction` function (after `capture_failed_command`)

```rust
/// Capture a user correction as a learning document.
///
/// # Arguments
///
/// * `correction_type` - Type of correction
/// * `original` - What the agent said/did
/// * `corrected` - What the user said instead
/// * `context_description` - Surrounding context
/// * `config` - Learning capture configuration
///
/// # Returns
///
/// Path to the saved correction file.
pub fn capture_correction(
    correction_type: CorrectionType,
    original: &str,
    corrected: &str,
    context_description: &str,
    config: &LearningCaptureConfig,
) -> Result<PathBuf, LearningError> {
    if !config.enabled {
        return Err(LearningError::Ignored("Capture disabled".to_string()));
    }

    // Redact secrets from all text fields
    let redacted_original = redact_secrets(original);
    let redacted_corrected = redact_secrets(corrected);
    let redacted_context = redact_secrets(context_description);

    let storage_dir = config.storage_location();
    fs::create_dir_all(&storage_dir)
        .map_err(|e| LearningError::StorageError(format!("Cannot create storage dir: {}", e)))?;

    let source = if storage_dir == config.project_dir {
        LearningSource::Project
    } else {
        LearningSource::Global
    };

    let correction = CorrectionEvent::new(
        correction_type.clone(),
        redacted_original,
        redacted_corrected,
        redacted_context,
        source,
    )
    .with_tags(vec![
        "correction".to_string(),
        format!("type:{}", correction_type),
    ]);

    let filename = format!("correction-{}.md", correction.id);
    let filepath = storage_dir.join(&filename);
    fs::write(&filepath, correction.to_markdown())?;

    log::info!("Captured correction: {}", filepath.display());
    Ok(filepath)
}
```

#### 1.5 Update `list_learnings` to include corrections

```rust
/// Unified learning entry for display (learning or correction).
#[derive(Debug, Clone)]
pub enum LearningEntry {
    Learning(CapturedLearning),
    Correction(CorrectionEvent),
}

impl LearningEntry {
    pub fn captured_at(&self) -> DateTime<Utc> {
        match self {
            LearningEntry::Learning(l) => l.context.captured_at,
            LearningEntry::Correction(c) => c.context.captured_at,
        }
    }

    pub fn source(&self) -> &LearningSource {
        match self {
            LearningEntry::Learning(l) => &l.source,
            LearningEntry::Correction(c) => &c.source,
        }
    }

    pub fn id(&self) -> &str {
        match self {
            LearningEntry::Learning(l) => &l.id,
            LearningEntry::Correction(c) => &c.id,
        }
    }

    /// Summary line for display.
    pub fn summary(&self) -> String {
        match self {
            LearningEntry::Learning(l) => {
                format!("[cmd] {} (exit: {})", l.command, l.exit_code)
            }
            LearningEntry::Correction(c) => {
                format!("[{}] {} -> {}", c.correction_type, c.original, c.corrected)
            }
        }
    }

    /// Correction text if any.
    pub fn correction_text(&self) -> Option<&str> {
        match self {
            LearningEntry::Learning(l) => l.correction.as_deref(),
            LearningEntry::Correction(c) => Some(&c.corrected),
        }
    }
}
```

Add new function:

```rust
/// List all entries (learnings + corrections) from storage.
pub fn list_all_entries(
    storage_dir: &PathBuf,
    limit: usize,
) -> Result<Vec<LearningEntry>, LearningError> {
    let mut entries = Vec::new();

    if !storage_dir.exists() {
        return Ok(entries);
    }

    for entry in fs::read_dir(storage_dir)?.flatten() {
        let path = entry.path();
        if path.extension().map(|e| e == "md").unwrap_or(false) {
            if let Ok(content) = fs::read_to_string(&path) {
                let filename = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                if filename.starts_with("correction-") {
                    if let Some(correction) = CorrectionEvent::from_markdown(&content) {
                        entries.push(LearningEntry::Correction(correction));
                    }
                } else if let Some(learning) = CapturedLearning::from_markdown(&content) {
                    entries.push(LearningEntry::Learning(learning));
                }
            }
        }
    }

    entries.sort_by(|a, b| b.captured_at().cmp(&a.captured_at()));
    if entries.len() > limit {
        entries.truncate(limit);
    }

    Ok(entries)
}

/// Query all entries (learnings + corrections) by pattern.
pub fn query_all_entries(
    storage_dir: &PathBuf,
    pattern: &str,
    exact: bool,
) -> Result<Vec<LearningEntry>, LearningError> {
    let all = list_all_entries(storage_dir, usize::MAX)?;
    let pattern_lower = pattern.to_lowercase();

    let filtered: Vec<_> = all
        .into_iter()
        .filter(|entry| {
            let text = match entry {
                LearningEntry::Learning(l) => {
                    format!("{} {}", l.command, l.error_output)
                }
                LearningEntry::Correction(c) => {
                    format!("{} {} {}", c.original, c.corrected, c.context_description)
                }
            };
            if exact {
                text.contains(pattern)
            } else {
                text.to_lowercase().contains(&pattern_lower)
            }
        })
        .collect();

    Ok(filtered)
}
```

### 2. `crates/terraphim_agent/src/learnings/mod.rs`

#### 2.1 Update public exports (after line 33)

Add to exports:
```rust
pub use capture::{
    CorrectionEvent, CorrectionType, LearningEntry,
    capture_correction, list_all_entries, query_all_entries,
};
```

### 3. `crates/terraphim_agent/src/main.rs`

#### 3.1 Add `Correction` variant to `LearnSub` enum (after `Correct` variant, ~line 761)

```rust
    /// Record a user correction (tool preference, naming, workflow, etc.)
    Correction {
        /// What the agent said/did originally
        #[arg(long)]
        original: String,
        /// What the user said instead
        #[arg(long)]
        corrected: String,
        /// Type of correction
        #[arg(long, default_value = "other")]
        correction_type: String,
        /// Context description
        #[arg(long, default_value = "")]
        context: String,
        /// Session ID for traceability
        #[arg(long)]
        session_id: Option<String>,
    },
```

#### 3.2 Add match arm in `run_learn_command` (after `LearnSub::Correct` arm, ~line 2104)

```rust
        LearnSub::Correction {
            original,
            corrected,
            correction_type,
            context,
            session_id,
        } => {
            use learnings::{CorrectionType, capture_correction};
            let ct: CorrectionType = correction_type.parse()
                .unwrap_or(CorrectionType::Other(correction_type.clone()));
            match capture_correction(ct, &original, &corrected, &context, &config) {
                Ok(path) => {
                    println!("Captured correction: {}", path.display());
                    Ok(())
                }
                Err(e) => {
                    eprintln!("Failed to capture correction: {}", e);
                    Err(e.into())
                }
            }
        }
```

#### 3.3 Update `LearnSub::List` arm to use `list_all_entries`

Replace the existing List arm (lines 2021-2054) to call `list_all_entries` instead of `list_learnings`, and display both types using `LearningEntry::summary()`.

#### 3.4 Update `LearnSub::Query` arm to use `query_all_entries`

Replace the existing Query arm (lines 2056-2091) similarly.

## Test Cases

### Unit Tests (in `capture.rs`)

1. **`test_correction_event_to_markdown`**: Create CorrectionEvent, verify markdown output contains all fields
2. **`test_correction_event_roundtrip`**: Create -> to_markdown -> from_markdown, verify fields match
3. **`test_capture_correction`**: Use TempDir, call capture_correction, verify file exists with correct content
4. **`test_correction_secret_redaction`**: Pass AWS key in original/corrected, verify it's redacted in stored file
5. **`test_list_all_entries_mixed`**: Store 2 learnings + 2 corrections, call list_all_entries, verify all 4 returned sorted by date
6. **`test_query_all_entries_finds_corrections`**: Store corrections, query by correction text, verify found
7. **`test_correction_type_roundtrip`**: Display + FromStr for each CorrectionType variant
8. **`test_learning_entry_summary`**: Verify summary format for both Learning and Correction variants

### Integration Test

9. **CLI test**: Run `terraphim-agent learn correction --original "npm install" --corrected "bun add" --correction-type tool-preference`, then `terraphim-agent learn list` and verify the correction appears in output

## Backward Compatibility

- Existing `list_learnings` and `query_learnings` functions remain unchanged (no breaking changes)
- New `list_all_entries` and `query_all_entries` are additive
- The CLI `learn list` and `learn query` switch to the new unified functions, but output format includes a type indicator (`[cmd]` vs `[tool-preference]`) so users can distinguish
- Existing learning files (prefixed `learning-`) continue to parse correctly
- New correction files (prefixed `correction-`) are ignored by old code that only calls `CapturedLearning::from_markdown`

## Dependencies

No new crate dependencies. Uses existing: `uuid`, `chrono`, `serde`, `thiserror`, `glob`, `tempfile` (dev).

## Acceptance Criteria

1. `cargo test -p terraphim_agent` passes with all new tests
2. `cargo clippy -p terraphim_agent` reports no warnings on new code
3. `terraphim-agent learn correction --original X --corrected Y` stores a file in learnings/
4. `terraphim-agent learn list` shows both learnings and corrections
5. `terraphim-agent learn query "bun"` finds corrections containing "bun"
6. Secret redaction works on correction text
7. All existing learning tests continue to pass unchanged
