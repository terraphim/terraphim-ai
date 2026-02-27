# Implementation Plan: TinyClaw OpenClaw Parity via Terraphim Extensions

**Status**: Draft
**Research Doc**: [tinyclaw-openclaw-parity-research-2026-02-27.md](./tinyclaw-openclaw-parity-research-2026-02-27.md)
**Author**: Terraphim AI
**Date**: 2026-02-27
**Related Issues**: #590 (epic), #588, #589, #591, #592, #593, #560

---

## Overview

### Summary

This plan implements OpenClaw parity for TinyClaw through five sequenced tracks. Each track extends existing Terraphim functionality rather than duplicating it, following the core epic principle.

### Approach

1. **Foundation First**: Fix #588 before any feature work to avoid compounding instability
2. **Extension Over Duplication**: Reuse `terraphim-markdown-parser`, `terraphim_spawner`
3. **Feature Gates**: Heavy dependencies (voice) remain optional
4. **Safety First**: Unified guard policy closes security gaps

### Scope

**In Scope:**
- Track 1 (#588): Foundation hardening (reset, config wiring, guardrails, docs)
- Track 2 (#589): Provider-backed web search and config-driven web tooling
- Track 3 (#592): Markdown-defined commands and skills
- Track 4 (#593): Voice transcription pipeline with graceful fallback
- Track 5 (#591): Session tools and orchestration runway (extends #560)

**Out of Scope:**
- Matrix channel (disabled due to sqlite conflict)
- Complex workflow DAGs (skills remain sequential)
- Built-in vector search (use haystack middleware separately)
- Custom LLM provider registry (use existing patterns)

**Avoid At All Cost** (from 5/25 analysis):
- Creating a parallel markdown parser when `terraphim-markdown-parser` exists
- Duplicating spawner functionality instead of using `terraphim_spawner`
- Adding features before foundation hardening (#588)
- Breaking existing skill JSON format (additive only)

---

## Architecture

### Component Diagram

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           TinyClaw Architecture                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │   Telegram   │  │   Discord    │  │     CLI      │  │   (Matrix)   │     │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘  └──────────────┘     │
│         │                  │                  │                              │
│         └──────────────────┼──────────────────┘                              │
│                            │                                                 │
│                    ┌───────▼────────┐                                        │
│                    │   MessageBus   │                                        │
│                    └───────┬────────┘                                        │
│                            │                                                 │
│              ┌─────────────┼─────────────┐                                   │
│              │             │             │                                   │
│     ┌────────▼─────┐ ┌────▼────┐ ┌──────▼──────┐                            │
│     │SessionManager│ │Tool     │ │ HybridLlm   │                            │
│     │  (JSONL)     │ │Registry │ │   Router    │                            │
│     └──────┬───────┘ └────┬────┘ └──────┬──────┘                            │
│            │              │             │                                    │
│            │    ┌─────────┼─────────┐   │                                    │
│            │    │         │         │   │                                    │
│            │    ▼         ▼         ▼   │                                    │
│            │ ┌────────┐ ┌──────┐ ┌──────────┐                                │
│            │ │Session │ │Web   │ │Markdown  │  <- NEW                       │
│            │ │Tools   │ │Search│ │Commands  │                                │
│            │ └────────┘ └──────┘ └──────────┘                                │
│            │    │         │         │                                        │
│            │    ▼         ▼         ▼                                        │
│            │ ┌────────┐ ┌──────────────┐ ┌──────────────┐                   │
│            │ │Agent   │ │  terraphim   │ │  terraphim   │  <- REUSED        │
│            │ │Spawn   │ │  _spawner    │ │-markdown-    │                   │
│            │ │        │ │              │ │  parser      │                   │
│            │ └────────┘ └──────────────┘ └──────────────┘                   │
│            │                                                                 │
│            ▼                                                                 │
│     ┌──────────────┐                                                        │
│     │   Unified    │  <- MODIFIED (closes guard bypass)                     │
│     │  Execution   │                                                        │
│     │   Guard      │                                                        │
│     └──────────────┘                                                        │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Data Flow (New Components)

```
# Markdown Command Flow
[Markdown File] -> terraphim-markdown-parser -> CommandDefinition
                                                    |
                                            [ToolCallingLoop]
                                                    |
                                            handle_slash_command()
                                                    |
                                              [Execution]

# Web Search Flow
[User Query] -> web_search tool -> WebSearchProvider (config-selected)
                                        |
                    +-------------------+-------------------+
                    |                   |                   |
                [Brave]            [SearXNG]          [Google]
                    |                   |                   |
                    +-------------------+-------------------+
                                        |
                                [Unified Response]
                                        |
                                   [LLM Context]

# Session Tool Flow
[User] -> sessions_list/history/send -> SessionManager
                                            |
                                    [Query/Filter]
                                            |
                                    [JSONL Files]
                                            |
                                    [Return Results]
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Use `terraphim-markdown-parser` for commands | Single parser, DRY | Custom TinyClaw-only parser |
| Feature gate voice deps | Binary size control | Always compile voice |
| Unified ExecutionGuard | Security: close bypass path | Separate guard for skills |
| Config-driven web provider | Runtime flexibility | Hardcoded provider |
| JSONL session format | Append-only, crash-safe | SQLite (conflict), single JSON |

### Eliminated Options

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| Custom markdown parser | Duplicates terraphim-markdown-parser | Maintenance burden, drift |
| Built-in whisper (no gate) | +50MB binary size | Deployment blocker |
| Separate skill guard | Complexity, inconsistency | Security holes |
| SQLite sessions | Dependency conflict with matrix-sdk | Build breakage |

### Simplicity Check

**What if this could be easy?**

The simplest design would:
1. Wire existing config fields directly (no schema changes)
2. Add markdown command loader as thin wrapper over existing parser
3. Reuse `terraphim_spawner` API without adaptation layer
4. Make voice a compile-time option (already done)

The current design achieves this simplicity for tracks 1, 3, 5. Tracks 2 and 4 require moderate complexity for provider abstraction and audio processing.

---

## Track Specifications

### Track 1: Foundation Hardening (#588)

#### Files Changed

| File | Change |
|------|--------|
| `src/session.rs` | Add `clear()` method, fix `/reset` |
| `src/agent/agent_loop.rs` | Wire `max_session_messages`, fix `/reset`, update slash handler |
| `src/skills/executor.rs` | Route shell steps through ExecutionGuard |
| `src/tools/shell.rs` | Export guard check for skills reuse |
| `src/config.rs` | Add validation for tool configs |
| `README.md` | Remove Matrix/voice claims until enabled |

#### API Changes

```rust
// session.rs - Add clear method
impl Session {
    /// Clear all messages and summary
    pub fn clear(&mut self) {
        self.messages.clear();
        self.summary = None;
        self.updated_at = Utc::now();
    }
}

// agent_loop.rs - Wire max_session_messages
impl ToolCallingLoop {
    fn check_compression_needed(&self, session: &Session) -> bool {
        session.messages.len() > self.config.agent.max_session_messages
    }
}

// skills/executor.rs - Use ExecutionGuard
async fn execute_shell_step(
    &self,
    command: &str,
    working_dir: Option<&str>,
    inputs: &HashMap<String, String>,
) -> Result<String, SkillError> {
    let substituted = self.substitute_template(command, inputs)?;

    // NEW: Check guard before execution
    let guard = ExecutionGuard::new();
    match guard.evaluate_shell(&substituted) {
        GuardDecision::Allow => { /* proceed */ }
        GuardDecision::Block { reason } => {
            return Err(SkillError::Blocked(reason));
        }
        GuardDecision::Warn { reason } => {
            log::warn!("Skill shell step with warning: {}", reason);
        }
    }

    // ... rest of execution
}
```

#### Test Strategy

| Test | Location | Purpose |
|------|----------|---------|
| `test_session_clear` | `session.rs` | Verify clear() removes messages + summary |
| `test_reset_command_clears` | `agent_loop.rs` | Integration: /reset -> clear() |
| `test_skill_shell_guarded` | `skills/executor.rs` | Verify dangerous commands blocked |
| `test_max_session_compression` | `agent_loop.rs` | Compression triggers at limit |

---

### Track 2: Provider-Backed Web Search (#589)

#### Files Changed

| File | Change |
|------|--------|
| `src/tools/web.rs` | Refactor to provider trait, implement real search |
| `src/tools/web_search/` | **NEW** Directory with provider implementations |
| `src/tools/web_search/mod.rs` | **NEW** Provider trait and dispatcher |
| `src/tools/web_search/brave.rs` | **NEW** Brave Search provider |
| `src/tools/web_search/searxng.rs` | **NEW** SearXNG provider |
| `src/tools/web_search/exa.rs` | **NEW** Exa AI Search provider |
| `src/tools/web_search/kimi_search.rs` | **NEW** Kimi Search provider |
| `src/config.rs` | Extend `WebToolsConfig` with provider credentials |

#### New Types

```rust
// tools/web_search/mod.rs
#[async_trait]
pub trait SearchProvider: Send + Sync {
    async fn search(&self, query: &str, num_results: usize) -> Result<SearchResults, SearchError>;
    fn name(&self) -> &str;
}

pub struct SearchResults {
    pub query: String,
    pub results: Vec<SearchResult>,
}

pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

pub struct ProviderDispatcher {
    providers: HashMap<String, Box<dyn SearchProvider>>,
    default: String,
}

// tools/web_search/brave.rs
pub struct BraveProvider {
    api_key: String,
    client: reqwest::Client,
}

#[async_trait]
impl SearchProvider for BraveProvider {
    async fn search(&self, query: &str, num_results: usize) -> Result<SearchResults, SearchError> {
        let response = self.client
            .get("https://api.search.brave.com/res/v1/web/search")
            .header("X-Subscription-Token", &self.api_key)
            .query(&[("q", query), ("count", &num_results.to_string())])
            .send()
            .await?;

        // Parse and return standardized results
        parse_brave_response(response).await
    }

    fn name(&self) -> &str { "brave" }
}
```

#### Config Extension

```toml
[tools.web]
search_provider = "brave"  # or "searxng", "exa", "kimi_search"
fetch_mode = "readability"  # or "raw"

[tools.web.brave]
api_key = BRAVE_API_KEY_ENV

[tools.web.searxng]
base_url = "https://search.example.com"

[tools.web.exa]
api_key = EXA_API_KEY_ENV

[tools.web.kimi_search]
api_key = KIMI_API_KEY_ENV
base_url = "https://search.moonshot.cn"
```

#### Test Strategy

| Test | Location | Purpose |
|------|----------|---------|
| `test_brave_provider_search` | `web_search/brave.rs` | Mock API response parsing |
| `test_provider_dispatcher` | `web_search/mod.rs` | Route to correct provider |
| `test_web_fetch_readability` | `web.rs` | Extract main content |
| `test_search_integration` | `tests/` | Live test with real API (ignored) |

---

### Track 3: Markdown Commands and Skills (#592)

#### Files Changed

| File | Change |
|------|--------|
| `src/commands/mod.rs` | **NEW** Markdown command types and loader |
| `src/commands/loader.rs` | **NEW** Load commands from directory |
| `src/commands/executor.rs` | **NEW** Execute markdown-defined commands |
| `src/skills/markdown.rs` | **NEW** Markdown skill support |
| `src/agent/agent_loop.rs` | Integrate markdown commands into slash handler |
| `Cargo.toml` | Add `terraphim-markdown-parser` dependency |

#### New Types

```rust
// commands/mod.rs
pub struct MarkdownCommand {
    pub name: String,
    pub description: String,
    pub arguments: Vec<CommandArgument>,
    pub steps: Vec<CommandStep>,
    pub source_path: PathBuf,
}

pub struct CommandArgument {
    pub name: String,
    pub description: String,
    pub required: bool,
    pub default: Option<String>,
}

pub enum CommandStep {
    Tool { tool: String, args: serde_json::Value },
    Llm { prompt: String },
    Respond { template: String },
}

pub struct CommandRegistry {
    commands: HashMap<String, MarkdownCommand>,
}

// commands/loader.rs
pub fn load_commands_from_dir(dir: &Path) -> Result<Vec<MarkdownCommand>, CommandError> {
    // Use terraphim-markdown-parser to extract frontmatter and content
    for entry in fs::read_dir(dir)? {
        let content = fs::read_to_string(entry.path())?;
        let command = parse_command_markdown(&content, entry.path())?;
        commands.push(command);
    }
    Ok(commands)
}

fn parse_command_markdown(content: &str, path: PathBuf) -> Result<MarkdownCommand, CommandError> {
    // Parse frontmatter (YAML or TOML)
    let (frontmatter, body) = split_frontmatter(content)?;
    let metadata: CommandMetadata = parse_frontmatter(frontmatter)?;

    // Parse steps from markdown content
    let steps = parse_command_steps(body)?;

    Ok(MarkdownCommand {
        name: metadata.name,
        description: metadata.description,
        arguments: metadata.arguments,
        steps,
        source_path: path,
    })
}
```

#### Markdown Format

```markdown
---
name: analyze-repo
description: Analyze a repository for issues
arguments:
  - name: path
    description: Path to repository
    required: true
  - name: focus
    description: Analysis focus
    required: false
    default: general
---

# Analyze Repository

## Step 1: List files
```tool:shell
command: find {path} -type f -name "*.rs" | head -20
```

## Step 2: Analyze with LLM
```tool:llm
prompt: |
  Analyze this Rust codebase for {focus} issues:

  {previous_output}
```

## Step 3: Respond
```respond
template: |
  ## Analysis Results

  {previous_output}
```
```

#### Test Strategy

| Test | Location | Purpose |
|------|----------|---------|
| `test_parse_command_frontmatter` | `commands/loader.rs` | Extract metadata |
| `test_parse_command_steps` | `commands/loader.rs` | Parse tool/llm/respond blocks |
| `test_execute_markdown_command` | `commands/executor.rs` | End-to-end execution |
| `test_json_skill_still_works` | `skills/executor.rs` | Backward compatibility |

---

### Track 4: Voice Transcription (#593)

#### Files Changed

| File | Change |
|------|--------|
| `src/tools/voice_transcribe.rs` | Implement real transcription |
| `Cargo.toml` | Add `whisper-rs`, `symphonia` deps behind `voice` feature |
| `src/config.rs` | Add voice configuration section |

#### Implementation

```rust
// tools/voice_transcribe.rs (voice feature enabled)
#[cfg(feature = "voice")]
pub struct VoiceTranscribeTool {
    temp_dir: PathBuf,
    whisper: WhisperContext,
}

#[cfg(feature = "voice")]
#[async_trait]
impl Tool for VoiceTranscribeTool {
    async fn execute(&self, args: serde_json::Value) -> Result<String, ToolError> {
        let audio_url = extract_url(&args)?;

        // 1. Download
        let audio_path = self.download_audio(audio_url).await?;

        // 2. Convert to WAV (16kHz mono)
        let wav_path = self.convert_to_wav(&audio_path).await
            .map_err(|e| ToolError::ExecutionFailed {
                tool: "voice_transcribe".to_string(),
                message: format!("Audio conversion failed: {}", e),
            })?;

        // 3. Transcribe with Whisper
        let text = self.transcribe(&wav_path).await
            .map_err(|e| ToolError::ExecutionFailed {
                tool: "voice_transcribe".to_string(),
                message: format!("Transcription failed: {}", e),
            })?;

        // 4. Cleanup
        let _ = tokio::fs::remove_file(&audio_path).await;
        let _ = tokio::fs::remove_file(&wav_path).await;

        Ok(text)
    }
}

#[cfg(feature = "voice")]
impl VoiceTranscribeTool {
    async fn convert_to_wav(&self, input: &Path) -> Result<PathBuf, TranscribeError> {
        use symphonia::default::get_probe;
        use symphonia::core::codecs::DecoderOptions;

        // Decode any format to raw samples
        let probe = get_probe();
        let source = File::open(input)?;
        let mss = MediaSourceStream::new(Box::new(source), Default::default());

        // Probe and decode
        let result = probe.format(
            &Default::default(),
            mss,
            &Default::default(),
            &Default::default(),
        )?;

        // Resample to 16kHz mono if needed
        // Write to WAV file

        Ok(wav_path)
    }

    async fn transcribe(&self, wav_path: &Path) -> Result<String, TranscribeError> {
        use whisper_rs::{FullParams, SamplingStrategy};

        let params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        let pcm_data = read_wav_16khz_mono(wav_path)?;

        self.whisper.full(params, &pcm_data)?;

        // Extract text from segments
        let num_segments = self.whisper.full_n_segments()?;
        let mut text = String::new();
        for i in 0..num_segments {
            text.push_str(&self.whisper.full_get_segment_text(i)?);
            text.push(' ');
        }

        Ok(text.trim().to_string())
    }
}

// Stub implementation when voice feature disabled
#[cfg(not(feature = "voice"))]
#[async_trait]
impl Tool for VoiceTranscribeTool {
    async fn execute(&self, _args: serde_json::Value) -> Result<String, ToolError> {
        Ok("[Voice transcription requires 'voice' feature enabled at compile time]".to_string())
    }
}
```

#### Error Handling

| Error Condition | User Message |
|-----------------|--------------|
| Feature not compiled | "Voice transcription not enabled. Recompile with --features voice" |
| Download failed | "Could not download audio. Check URL and network." |
| Invalid audio | "Audio file could not be decoded. Supported: OGG, MP3, WAV" |
| Transcription timeout | "Transcription timed out. Try a shorter audio clip." |
| Model not found | "Voice model not found. Run: tinyclaw download-model" |

#### Test Strategy

| Test | Location | Purpose |
|------|----------|---------|
| `test_voice_feature_disabled` | `voice_transcribe.rs` | Graceful fallback |
| `test_voice_download_audio` | `voice_transcribe.rs` | Mock HTTP download |
| `test_voice_invalid_url` | `voice_transcribe.rs` | Error handling |
| `test_voice_transcribe_live` | `tests/` | Live test (ignored, requires feature) |

---

### Track 5: Session Tools and Orchestration (#591)

#### Files Changed

| File | Change |
|------|--------|
| `src/tools/session_tools.rs` | **NEW** Session tool implementations |
| `src/tools/mod.rs` | Register session tools |
| `src/tools/agent_spawn.rs` | **NEW** Agent spawn tool (extends #560) |

#### New Types

```rust
// tools/session_tools.rs
pub struct SessionListTool;
pub struct SessionHistoryTool;
pub struct SessionSendTool;

#[async_trait]
impl Tool for SessionListTool {
    fn name(&self) -> &str { "sessions_list" }

    fn description(&self) -> &str {
        "List all active sessions with metadata (message count, last activity)"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "filter": {
                    "type": "string",
                    "description": "Optional filter by channel (telegram, discord, cli)",
                    "enum": ["telegram", "discord", "cli"]
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum sessions to return",
                    "default": 50
                }
            }
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<String, ToolError> {
        let filter = args["filter"].as_str();
        let limit = args["limit"].as_u64().unwrap_or(50) as usize;

        let sessions = self.list_sessions(filter, limit).await?;

        // Format as readable table
        let output = format_sessions_table(&sessions);
        Ok(output)
    }
}

#[async_trait]
impl Tool for SessionHistoryTool {
    fn name(&self) -> &str { "sessions_history" }

    fn description(&self) -> &str {
        "Get message history for a specific session"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "session_key": {
                    "type": "string",
                    "description": "Session key (format: channel:chat_id)"
                },
                "limit": {
                    "type": "integer",
                    "description": "Number of recent messages",
                    "default": 20
                }
            },
            "required": ["session_key"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<String, ToolError> {
        let session_key = args["session_key"].as_str()
            .ok_or_else(|| ToolError::InvalidArguments {
                tool: "sessions_history".to_string(),
                message: "Missing session_key".to_string(),
            })?;

        let limit = args["limit"].as_u64().unwrap_or(20) as usize;

        // Load session and format history
        let history = self.get_history(session_key, limit).await?;
        Ok(history)
    }
}

// tools/agent_spawn.rs (extends #560)
pub struct AgentSpawnTool {
    spawner: Arc<terraphim_spawner::AgentSpawner>,
}

#[async_trait]
impl Tool for AgentSpawnTool {
    fn name(&self) -> &str { "agent_spawn" }

    fn description(&self) -> &str {
        "Spawn an external AI agent (opencode, claude-code, codex) to handle a task"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "agent_type": {
                    "type": "string",
                    "description": "Type of agent to spawn",
                    "enum": ["opencode", "claude-code", "codex"]
                },
                "task": {
                    "type": "string",
                    "description": "Task description for the agent"
                },
                "working_dir": {
                    "type": "string",
                    "description": "Working directory for the agent"
                },
                "timeout_secs": {
                    "type": "integer",
                    "description": "Timeout in seconds",
                    "default": 300
                }
            },
            "required": ["agent_type", "task"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<String, ToolError> {
        let agent_type = args["agent_type"].as_str()
            .ok_or_else(|| ToolError::InvalidArguments {
                tool: "agent_spawn".to_string(),
                message: "Missing agent_type".to_string(),
            })?;

        let task = args["task"].as_str().unwrap_or("");
        let working_dir = args["working_dir"].as_str();
        let timeout_secs = args["timeout_secs"].as_u64().unwrap_or(300);

        // Spawn via terraphim_spawner
        let handle = self.spawner.spawn(agent_type, task, working_dir).await
            .map_err(|e| ToolError::ExecutionFailed {
                tool: "agent_spawn".to_string(),
                message: format!("Failed to spawn agent: {}", e),
            })?;

        // Wait for completion or timeout
        let result = tokio::time::timeout(
            Duration::from_secs(timeout_secs),
            handle.wait_for_output()
        ).await
        .map_err(|_| ToolError::Timeout {
            tool: "agent_spawn".to_string(),
            seconds: timeout_secs,
        })?;

        match result {
            Ok(output) => Ok(format!("Agent completed:\n{}", output)),
            Err(e) => Err(ToolError::ExecutionFailed {
                tool: "agent_spawn".to_string(),
                message: format!("Agent failed: {}", e),
            }),
        }
    }
}
```

#### Test Strategy

| Test | Location | Purpose |
|------|----------|---------|
| `test_session_list_tool` | `session_tools.rs` | List sessions |
| `test_session_history_tool` | `session_tools.rs` | Get history |
| `test_session_send_tool` | `session_tools.rs` | Cross-session message |
| `test_agent_spawn_mock` | `agent_spawn.rs` | Mock spawner integration |
| `test_agent_spawn_live` | `tests/` | Live test (ignored, requires setup) |

---

## Implementation Sequencing

### Phase 1: Foundation (Track 1 - #588)

**Goal**: Stable base for feature work

**Steps**:
1. **Session clear method** (2 hours)
   - Add `Session::clear()`
   - Test: messages and summary cleared

2. **Fix /reset command** (1 hour)
   - Call `session.clear()` in handler
   - Test: integration with session manager

3. **Wire max_session_messages** (2 hours)
   - Read config value in agent_loop
   - Test: compression triggers at limit

4. **Close guard bypass** (4 hours)
   - Export ExecutionGuard checks
   - Integrate into skill shell execution
   - Test: dangerous commands blocked in skills

5. **Docs alignment** (1 hour)
   - Remove Matrix/voice claims
   - Update feature status table

**Phase 1 Exit Criteria**:
- [ ] `/reset` clears session messages and summary
- [ ] `agent.max_session_messages` triggers compression
- [ ] Skill shell steps go through ExecutionGuard
- [ ] Docs match shipped behavior
- [ ] All tests pass

### Phase 2: Core Features (Tracks 2 & 3 - #589, #592)

**Goal**: Primary OpenClaw parity features

**Track 2 Steps**:
1. Search provider trait (2 hours)
2. Brave provider implementation (4 hours)
3. SearXNG provider (2 hours)
4. Provider dispatcher (2 hours)
5. Web fetch improvements (2 hours)

**Track 3 Steps**:
1. Add terraphim-markdown-parser dependency (1 hour)
2. Markdown command types (2 hours)
3. Command loader (4 hours)
4. Command executor (4 hours)
5. Slash command integration (2 hours)
6. Markdown skill support (4 hours)

**Phase 2 Exit Criteria**:
- [ ] `web_search` returns real results
- [ ] Provider configurable in TOML
- [ ] Markdown commands load and execute
- [ ] JSON skills still work (backward compat)
- [ ] Integration tests pass

### Phase 3: Extended Features (Tracks 4 & 5 - #593, #591)

**Goal**: Voice and orchestration capabilities

**Track 4 Steps**:
1. Add voice deps (whisper-rs, symphonia) (1 hour)
2. Implement audio conversion (4 hours)
3. Implement whisper transcription (4 hours)
4. Error handling polish (2 hours)

**Track 5 Steps**:
1. Session list tool (2 hours)
2. Session history tool (2 hours)
3. Session send tool (2 hours)
4. Agent spawn tool (extends #560) (4 hours)
5. Tool registration (1 hour)

**Phase 3 Exit Criteria**:
- [ ] Voice transcription works (with feature)
- [ ] Graceful fallback without feature
- [ ] Session tools functional
- [ ] Agent spawn integrates with terraphim_spawner
- [ ] All tests pass

---

## Test Strategy

### Unit Tests

| Test | Location | Phase |
|------|----------|-------|
| `test_session_clear` | `session.rs` | 1 |
| `test_skill_guard_integration` | `skills/executor.rs` | 1 |
| `test_brave_provider` | `web_search/brave.rs` | 2 |
| `test_command_loader` | `commands/loader.rs` | 2 |
| `test_voice_conversion` | `voice_transcribe.rs` | 3 |
| `test_session_tools` | `session_tools.rs` | 3 |

### Integration Tests

| Test | Location | Phase |
|------|----------|-------|
| `test_reset_end_to_end` | `tests/agent_loop.rs` | 1 |
| `test_web_search_live` | `tests/web_search.rs` | 2 |
| `test_markdown_command_e2e` | `tests/commands.rs` | 2 |
| `test_voice_pipeline` | `tests/voice.rs` | 3 |
| `test_session_orchestration` | `tests/orchestration.rs` | 3 |

### Regression Tests

| Test | Purpose |
|------|---------|
| `test_json_skills_still_work` | Backward compatibility |
| `test_config_loading` | Config format unchanged |
| `test_existing_tools` | No tool behavior regression |

---

## Dependencies

### New Dependencies

| Crate | Version | Purpose | Feature |
|-------|---------|---------|---------|
| terraphim-markdown-parser | workspace | Parse markdown commands | default |
| terraphim_spawner | workspace | Agent spawning | default |
| readability | 0.3 | HTML content extraction | default |
| whisper-rs | 0.11 | Voice transcription | voice |
| symphonia | 0.5 | Audio format conversion | voice |

### Dependency Updates

| File | Change |
|------|--------|
| `Cargo.toml` | Add deps above |

---

## Rollback Plan

If issues discovered:

1. **Per-track rollback**: Each track is independent; disable by reverting track-specific commits
2. **Feature gate disable**: Voice can be disabled by not using `--features voice`
3. **Config fallback**: New config fields have sensible defaults; removing them doesn't break

**Feature flags**:
- `voice`: Disable if transcription issues
- `telegram`/`discord`: Channel-specific, already gated

---

## Migration

### Config Migration

New config sections are additive. Old configs work without changes.

```toml
# New optional sections
[tools.web]
search_provider = "brave"

[tools.web.brave]
api_key = API_KEY_PLACEHOLDER

[commands]
load_path = "~/.config/terraphim/commands/"
```

### Session Format

JSONL format unchanged. New fields optional.

---

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Coordinate with #560 | Pending | Shared |
| Choose primary web search provider | Pending | Product |
| Voice model distribution | Pending | DevOps |

---

## Approval Checklist

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Performance targets agreed
- [ ] Human approval received
- [ ] Sequencing approved (Foundation -> Core -> Extended)

---

## Appendix

### Track/Issue Mapping

| Track | Issue | Name | Priority |
|-------|-------|------|----------|
| 1 | #588 | Foundation Hardening | **P0 (First)** |
| 2 | #589 | Provider Web Search | P1 |
| 3 | #592 | Markdown Commands | P1 |
| 4 | #593 | Voice Transcription | P2 |
| 5 | #591 | Session Tools + Spawn | P2 |

### File Change Summary

**New Files (7)**:
- `src/tools/web_search/mod.rs`
- `src/tools/web_search/brave.rs`
- `src/tools/web_search/searxng.rs`
- `src/commands/mod.rs`
- `src/commands/loader.rs`
- `src/commands/executor.rs`
- `src/tools/session_tools.rs`
- `src/tools/agent_spawn.rs`
- `src/skills/markdown.rs`

**Modified Files (8)**:
- `src/session.rs`
- `src/agent/agent_loop.rs`
- `src/skills/executor.rs`
- `src/tools/shell.rs`
- `src/tools/web.rs`
- `src/tools/mod.rs`
- `src/config.rs`
- `Cargo.toml`
- `README.md`
