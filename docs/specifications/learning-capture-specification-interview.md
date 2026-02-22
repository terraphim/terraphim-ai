# Specification Interview Findings: Enhanced Learning Capture System

**Interview Date**: 2026-02-15
**Dimensions Covered**: Concurrency, Failure Modes, Edge Cases, User Experience, Scale, Security, Integration, Operational
**Convergence Status**: Complete

---

## Key Decisions from Interview

### Concurrency & Race Conditions

- **Unique filenames for concurrent writes**: Each capture writes to a unique timestamped file (e.g., `learning-20260215-143022-abc123.md`). This eliminates collision risk when multiple hooks fire simultaneously during parallel execution.
- **Parallel hook execution**: Learning capture runs alongside other PostToolUse hooks (notifications, logging). No serialization required.

### Failure Modes & Recovery

- **Capture failure handling**: If capture itself fails (disk full, permissions), log warning to stderr but continue. Learning may be lost but developer workflow is not blocked.
- **No retry mechanism**: Failed captures are not queued for retry. Simplicity over completeness.

### Edge Cases & Boundaries

- **Duplicate commands**: Store all failures separately. User can merge later if desired. Full history is preserved.
- **Binary output**: Truncate at first null byte. Store only the text portion of error output.
- **Chained commands**: Split `cmd1 && cmd2` into separate learning documents when `cmd2` fails. Each sub-command gets its own capture.
- **Scale limits**: No retention policy or storage limit. Keep everything; user manages if needed.
- **Command parsing**: Capture command as raw string. No structured parsing or shell tokenization.

### User Mental Models

- **Query behavior**: Use KG synonym expansion for queries. Searching "git push" finds related failures like "git push -f" via synonym graph.
- **Test command handling**: Configurable ignore patterns. Users can exclude `cargo test`, `npm test` failures from capture via config.
- **Multi-project workflow**: Hybrid approach - project-specific learnings (`.terraphim/learnings/`) with global fallback (`~/.terraphim/learnings/`) for common patterns.

### Scale & Performance

- **No limits**: Accept that learnings directory may grow large. Rely on filesystem and user management.
- **Unique filenames prevent lock contention**: No file locking needed; each capture is independent.

### Security & Privacy

- **Auto-redact sensitive patterns**: Detect and redact standard secret patterns before storing:
  - AWS access keys (`AKIA...`)
  - GCP service account keys
  - Azure keys
  - Generic API keys (`sk-...`, `xoxb-...`)
  - Passwords in connection strings
- **Strip environment variables**: Remove all `VAR=value` patterns from error output. Never store env var values.
- **Use standard detection library**: Implement using patterns from `detect-secrets` or similar.

### Integration Effects

- **Hook priority**: Parallel execution with other hooks.
- **Auto-suggest corrections**: When capturing, first try to match error against existing KG terms. If match found, suggest correction automatically.
- **No match fallback**: If no KG match, store failure only. User can add correction later via `terraphim-agent learn correct`.

### Operational Concerns

- **Debug visibility**: Add `--debug` flag to CLI commands for troubleshooting. No verbose logging by default.
- **No separate log file**: Capture status visible via hook output when debug mode enabled.

---

## Configuration Design

### Config File Structure

```toml
# .terraphim/learning-capture.toml

[learnings]
# Directory for project-specific learnings
project_dir = ".terraphim/learnings"

# Global fallback directory
global_dir = "~/.terraphim/learnings"

# Enable/disable capture
enabled = true

[learnings.ignore]
# Patterns to skip (glob-style)
commands = [
    "cargo test*",
    "npm test*",
    "pytest*",
    "yarn test*"
]

[learnings.redaction]
# Enable auto-redaction
enabled = true

# Patterns to redact (in addition to defaults)
custom_patterns = [
    "DATABASE_URL=.*",
    "API_KEY=.*"
]
```

### CLI Commands (Updated)

```bash
# Capture with debug output
terraphim-agent learn capture "git push -f" --error "rejected" --debug

# Query with synonym expansion (default)
terraphim-agent learn query "git push"

# Query exact match only
terraphim-agent learn query "git push" --exact

# Add correction to existing learning
terraphim-agent learn correct <learning-id> --correction "git push"

# List recent learnings with project/global indicator
terraphim-agent learn list --recent 10

# Show capture statistics
terraphim-agent learn stats

# Prune old learnings (manual)
terraphim-agent learn prune --older-than 90d
```

---

## Redaction Patterns (Standard Set)

```rust
const SECRET_PATTERNS: &[(&str, &str)] = &[
    // AWS
    (r"AWS_ACCESS_KEY_ID=\S+", "AWS_ACCESS_KEY_ID=[REDACTED]"),
    (r"AWS_SECRET_ACCESS_KEY=\S+", "AWS_SECRET_ACCESS_KEY=[REDACTED]"),
    (r"AKIA[A-Z0-9]{16}", "AKIA[REDACTED]"),
    
    // GCP
    (r"GOOGLE_APPLICATION_CREDENTIALS=\S+", "GOOGLE_APPLICATION_CREDENTIALS=[REDACTED]"),
    
    // Azure
    (r"AZURE_CLIENT_SECRET=\S+", "AZURE_CLIENT_SECRET=[REDACTED]"),
    
    // Generic API keys
    (r"API_KEY=\S+", "API_KEY=[REDACTED]"),
    (r"SECRET_KEY=\S+", "SECRET_KEY=[REDACTED]"),
    (r"PASSWORD=\S+", "PASSWORD=[REDACTED]"),
    (r"TOKEN=\S+", "TOKEN=[REDACTED]"),
    
    // Connection strings
    (r"postgresql://[^@]+:[^@]+@", "postgresql://[REDACTED]@"),
    (r"mysql://[^@]+:[^@]+@", "mysql://[REDACTED]@"),
    (r"redis://[^@]+:[^@]+@", "redis://[REDACTED]@"),
    
    // Generic env vars (strip all)
    (r"[A-Z_]+=\S+", "[ENV_REDACTED]"),
];
```

---

## Chained Command Handling

When a command like `cmd1 && cmd2 || cmd3` fails:

1. Parse the command to identify sub-commands
2. Determine which sub-command failed (based on exit code and position)
3. Create separate learning for the failing sub-command
4. Include full original command as context

Example:
```
Original: docker compose up -d && npm run migrate
Failed: npm run migrate (exit code 1)

Learning 1:
  command: npm run migrate
  full_chain: docker compose up -d && npm run migrate
  failed_at: position 2
  error: [output from npm run migrate]
```

---

## Existing Infrastructure Leverage

### Reusable Components from terraphim-automata

| Component | Location | How We Use It |
|-----------|----------|---------------|
| `replace_matches()` | `terraphim_automata::matcher` | Secret redaction via KG-style thesaurus |
| `find_matches()` | `terraphim_automata::matcher` | Auto-suggest corrections by matching failed command |
| `Logseq` builder | `terraphim_automata::builder` | Already builds Thesaurus from `synonyms::` markdown |
| `UrlProtector` | `terraphim_automata::url_protector` | Pattern - adapt for `SecretRedactor` |
| `LinkType::PlainText` | `terraphim_automata::matcher` | Redaction output format |

### Reusable Components from terraphim-hooks

| Component | Location | How We Use It |
|-----------|----------|---------------|
| `ReplacementService` | `terraphim_hooks::replacement` | Wrap automata for hook use |
| `HookResult` | `terraphim_hooks::replacement` | Structured result with changed flag |
| `replace_fail_open()` | `terraphim_hooks::replacement` | Fail-open semantics for capture |
| `discover_binary()` | `terraphim_hooks::discovery` | Binary discovery utilities |

### Reusable Components from terraphim-rolegraph

| Component | Location | How We Use It |
|-----------|----------|---------------|
| `RoleGraph` | `terraphim_rolegraph` | Document indexing + co-occurrence graph |
| `query_graph()` | `terraphim_rolegraph` | KG synonym expansion already built-in |
| `insert_document()` | `terraphim_rolegraph` | Index learning documents |

### Key Insight: Secret Redaction = Pattern Replacement

Instead of building new redaction logic, we **reuse `replace_matches()`** with a secret-patterns thesaurus:

```rust
// Reuse existing automata for redaction
fn redact_secrets(text: &str) -> String {
    let secret_thesaurus = build_secret_thesaurus();  // Standard patterns
    let result = terraphim_automata::replace_matches(
        text, 
        secret_thesaurus, 
        LinkType::PlainText  // Output: "[REDACTED]"
    );
    String::from_utf8(result.unwrap()).unwrap()
}
```

The `UrlProtector` pattern (mask → transform → restore) can be adapted for secrets.

## Implementation Updates

### Updated Types (Minimal New Code)

```rust
/// Captured learning from a failed command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapturedLearning {
    /// Unique ID (timestamp-based)
    pub id: String,
    /// Original command that failed
    pub command: String,
    /// For chained commands, the specific failing sub-command
    pub failing_subcommand: Option<String>,
    /// Full command chain (if chained)
    pub full_chain: Option<String>,
    /// Error output (redacted via automata)
    pub error_output: String,
    /// Suggested correction (if auto-suggested from KG)
    pub correction: Option<String>,
    /// Source: "project" or "global"
    pub source: LearningSource,
    /// Context
    pub context: LearningContext,
    /// Tags for categorization
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LearningSource {
    Project,
    Global,
}

/// Minimal config - most defaults work
#[derive(Debug, Clone)]
pub struct LearningCaptureConfig {
    /// Project-specific learnings directory
    pub project_dir: PathBuf,
    /// Global fallback directory
    pub global_dir: PathBuf,
    /// Enable capture
    pub enabled: bool,
    /// Patterns to ignore (glob)
    pub ignore_patterns: Vec<String>,
}
```

### Functions (Mostly Wrappers Around Existing Code)

```rust
// NEW: Build secret redaction thesaurus for use with replace_matches()
fn build_secret_thesaurus() -> Thesaurus {
    let mut thesaurus = Thesaurus::new("secrets");
    // AWS keys
    let aws = NormalizedTerm::new(1, NormalizedTermValue::from("[REDACTED]"));
    thesaurus.insert(NormalizedTermValue::from("AKIA[A-Z0-9]{16}"), aws);
    // ... more patterns
    thesaurus
}

// WRAPPER: Uses terraphim_automata::replace_matches
pub fn redact_secrets(text: &str) -> String;

// WRAPPER: Uses terraphim_automata::find_matches
pub fn auto_suggest_correction(
    command: &str,
    rolegraph: &RoleGraph,
) -> Option<String> {
    // Use find_matches to check if command matches existing KG terms
    let matches = terraphim_automata::find_matches(command, rolegraph.thesaurus(), true)?;
    matches.first().map(|m| m.normalized_term.display())
}

// NEW: Parse chained commands
pub fn parse_chained_command(command: &str) -> Vec<String>;

// SIMPLE: Glob matching via glob crate
pub fn should_ignore(command: &str, patterns: &[String]) -> bool;

// SIMPLE: Check project dir, fallback to global
pub fn determine_storage_location(config: &LearningCaptureConfig) -> PathBuf;
```

---

## Deferred Items

| Item | Reason |
|------|--------|
| Automatic KG term creation from repeated failures | Requires more user research on workflow |
| Web UI for learning management | Out of scope - CLI-first approach |
| Learning export/import | Can be added later if needed |
| Team learning sharing | Requires infrastructure not yet planned |

---

## Interview Summary

The specification interview revealed several critical design decisions that significantly impact implementation:

1. **Security-first approach**: Auto-redaction of secrets and environment variables is mandatory, not optional. This affects the capture pipeline - redaction must happen before any storage.

2. **Hybrid storage model**: Project-specific learnings with global fallback means the system must support two storage locations and merge queries across both. This affects the RoleGraph initialization.

3. **Chained command splitting**: Rather than storing raw commands, the system must parse and split chained commands to capture the actual failing sub-command. This adds complexity but improves retrieval quality.

4. **Parallel hook execution**: No locking required, but capture must be fast (<50ms) to not delay other hooks. Unique filenames eliminate race conditions.

5. **Auto-suggest before store**: The capture flow should first query existing KG for corrections, only store as "uncorrected learning" if no match found. This changes the capture pipeline order.

These decisions add moderate complexity but significantly improve the user experience and security posture of the system.

---

## Revised Implementation Steps (Leveraging Existing Code)

### Effort Reduction Summary

| Original Step | Revised Approach | Savings |
|---------------|------------------|---------|
| Secret redaction logic | Reuse `replace_matches()` with secret thesaurus | ~4 hours |
| Auto-suggest corrections | Reuse `find_matches()` against existing KG | ~2 hours |
| KG term builder | Reuse `Logseq` builder (already exists) | ~2 hours |
| Hook infrastructure | Reuse `ReplacementService` and `HookResult` | ~2 hours |
| Query with expansion | RoleGraph already does KG synonym expansion | ~3 hours |

**Total Savings: ~13 hours** (from ~16 hours to ~3-4 hours of new code)

### Step 1: Types and Config (1 hour)
**Files:** `crates/terraphim_agent/src/learnings/mod.rs`
**Description:** Define `CapturedLearning`, `LearningSource`, `LearningCaptureConfig`
**Dependencies:** None

```rust
// Minimal types - no complex logic
pub struct CapturedLearning { ... }
pub struct LearningCaptureConfig { ... }
```

### Step 2: Secret Redaction via Automata (1 hour)
**Files:** `crates/terraphim_agent/src/learnings/redaction.rs`
**Description:** Build secret thesaurus, wrap `replace_matches()`
**Dependencies:** Step 1, `terraphim_automata`

```rust
// Wrapper around existing automata
pub fn redact_secrets(text: &str) -> String {
    let thesaurus = build_secret_thesaurus();
    let result = terraphim_automata::replace_matches(text, thesaurus, LinkType::PlainText)?;
    String::from_utf8(result).unwrap()
}
```

### Step 3: Capture Logic (1.5 hours)
**Files:** `crates/terraphim_agent/src/learnings/capture.rs`
**Description:** Implement `capture_failed_command()` using existing patterns
**Dependencies:** Step 1, Step 2, `terraphim_rolegraph`

```rust
pub fn capture_failed_command(
    command: &str,
    error_output: &str,
    exit_code: i32,
    config: &LearningCaptureConfig,
) -> Result<PathBuf, LearningError> {
    // 1. Redact secrets (uses automata)
    let redacted = redact_secrets(error_output);
    
    // 2. Check for auto-suggestion (uses find_matches)
    let correction = auto_suggest_correction(command, &rolegraph);
    
    // 3. Write markdown file (standard file I/O)
    let learning = CapturedLearning { ... };
    let path = storage_path.join(format!("{}.md", learning.id));
    fs::write(&path, learning.to_markdown())?;
    Ok(path)
}
```

### Step 4: CLI Integration (1 hour)
**Files:** `crates/terraphim_agent/src/main.rs`
**Description:** Add `learn` subcommand with `capture`, `list`, `query`
**Dependencies:** Step 3

```rust
Commands::Learn { subcommand } => match subcommand {
    LearnCommands::Capture { command, error } => {
        let path = capture_failed_command(&command, &error, 1, &config)?;
        println!("Captured: {}", path.display());
    }
    LearnCommands::Query { pattern } => {
        // Uses RoleGraph.query_graph() - already supports KG expansion
        let results = rolegraph.query_graph(&pattern, None, Some(10))?;
        for (id, doc) in results {
            println!("{}: {} (rank: {})", id, doc.title, doc.rank);
        }
    }
    // ...
}
```

### Step 5: Hook Integration (0.5 hours)
**Files:** Hook script using existing CLI
**Description:** PostToolUse hook that calls CLI
**Dependencies:** Step 4

```bash
#!/bin/bash
# Minimal hook - just calls CLI
if [ "$EXIT_CODE" -ne 0 ]; then
    terraphim-agent learn capture "$COMMAND" --error "$STDERR" --exit-code "$EXIT_CODE"
fi
```

### Step 6: Documentation (1 hour)
**Files:** `docs/src/kg/learnings-system.md`, `skills/learning-capture/skill.md`
**Dependencies:** Step 4, Step 5

**Total Estimated: 6 hours** (vs original 16 hours)

---

## Files Changed Summary

| File | Status | New Lines |
|------|--------|-----------|
| `crates/terraphim_agent/src/learnings/mod.rs` | NEW | ~50 |
| `crates/terraphim_agent/src/learnings/redaction.rs` | NEW | ~80 |
| `crates/terraphim_agent/src/learnings/capture.rs` | NEW | ~100 |
| `crates/terraphim_agent/src/main.rs` | MODIFIED | +30 |
| `docs/src/kg/learnings-system.md` | NEW | ~100 |

**Total New Code: ~360 lines** (vs original estimate of ~800+ lines)
