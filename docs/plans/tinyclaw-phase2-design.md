# Phase 2 Implementation Plan: TinyClaw Enhancements

**Status**: Draft
**Research Doc**: `docs/plans/tinyclaw-phase2-research.md`
**Author**: Terraphim AI Team
**Date**: 2026-02-12
**Estimated Effort**: 4-6 weeks

---

## Overview

### Summary
Extend TinyClaw with three major capabilities: WhatsApp integration via Matrix bridge, voice message transcription using Whisper, and a skills system for reusable workflows.

### Approach
Build incrementally on Phase 1 architecture:
1. **WhatsApp Channel** - New Channel trait implementation using Matrix SDK
2. **Voice Transcription** - New tool that downloads audio, transcribes with Whisper
3. **Skills System** - JSON-based workflow definitions with save/load/monitor

### Scope

**In Scope:**
- WhatsApp integration via Matrix bridge (mautrix-whatsapp)
- Voice transcription tool (Whisper base model)
- Skills system with JSON storage
- Audio format conversion (ogg/opus → wav)
- Skill CRUD operations (/skill save, /skill load, /skill list)

**Out of Scope:**
- Real-time voice streaming (batch processing only)
- WhatsApp Business API (approval process too long)
- Skill marketplace/cloud sync (local files only)
- Custom voice models (Whisper base only)

**Avoid At All Cost:**
- Native WhatsApp libraries (whatsapp-web-rs) - unreliable, frequent breakage
- Video transcription - scope creep, audio-only is sufficient
- Cloud-hosted skills - adds complexity, local files work

---

## Architecture

### Component Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                         TinyClaw Phase 2                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Channels                    Agent Loop          Tools           │
│  ┌─────────────┐            ┌──────────────┐   ┌──────────────┐ │
│  │ CLI         │            │              │   │ Filesystem   │ │
│  ├─────────────┤◄──────────►│   Hybrid     │   ├──────────────┤ │
│  │ Telegram    │            │   LLM        │   │ Edit         │ │
│  ├─────────────┤            │   Router     │   ├──────────────┤ │
│  │ Discord     │            │              │   │ Shell        │ │
│  ├─────────────┤            └──────────────┘   ├──────────────┤ │
│  │ WhatsApp ◄──┼──┐                           │ Web Search   │ │
│  │  (Matrix)   │  │                           ├──────────────┤ │
│  └─────────────┘  │                           │ Web Fetch    │ │
│                   │                           ├──────────────┤ │
│  Voice Messages   │                           │ Voice ◄──────┼─┘
│  ┌─────────────┐  │                           │ Transcribe   │
│  │ Download    │  │                           └──────────────┘
│  │ Convert     │  │
│  │ Whisper     │──┘
│  └─────────────┘
│
│  Skills
│  ┌─────────────┐
│  │ Load JSON   │
│  │ Execute     │
│  │ Monitor     │
│  └─────────────┘
│
└─────────────────────────────────────────────────────────────────┘
```

### Data Flow

**WhatsApp Message:**
```
WhatsApp → mautrix-whatsapp → Matrix Server → MatrixChannel → Bus → Agent → Response
```

**Voice Message:**
```
WhatsApp voice → Download → Convert to WAV → Whisper → Text → Agent
```

**Skill Execution:**
```
/skill run analyze-repo → Load JSON → Parse steps → Execute tools → Report results
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Matrix bridge for WhatsApp | Reliable, handles reconnection, rate limits | whatsapp-web-rs (breaks frequently) |
| Whisper base model | 74MB, 2x real-time, good quality | Large model (155MB, slower), Tiny (poor quality) |
| JSON for skills | Human-readable, standard, editable | YAML (slower), Binary (not readable) |
| Download-then-transcribe | Simpler than streaming, handles large files | Real-time streaming (complex, unnecessary) |

### Eliminated Options

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| whatsapp-web-rs direct | Meta blocks frequently, ToS violations | Sudden breakage, user frustration |
| Real-time voice streaming | Adds complexity, not needed for messaging | Delayed delivery, audio glitches |
| Skill cloud storage | Adds auth, sync complexity | Privacy concerns, offline failure |
| Whisper large model | 155MB download, 5x slower | Slow user experience, memory pressure |

### Simplicity Check

> "Minimum code that solves the problem. Nothing speculative."

**What if this could be easy?**
- WhatsApp: Matrix bridge handles complexity, we just implement Matrix channel
- Voice: Download file, run Whisper, return text - no streaming needed
- Skills: JSON files, simple sequential execution - no DAG or parallelism yet

**Senior Engineer Test**: ✅ This design would pass senior review - straightforward extensions of existing patterns.

**Nothing Speculative Checklist**:
- [x] No features the user didn't request (no video, no streaming)
- [x] No abstractions "in case we need them later" (skills are simple sequential)
- [x] No flexibility "just in case" (WhatsApp only via Matrix for now)
- [x] No error handling for scenarios that cannot occur (Whisper will always return something)
- [x] No premature optimization (base model is fast enough)

---

## File Changes

### New Files

| File | Purpose |
|------|---------|
| `src/channels/matrix.rs` | Matrix channel adapter for WhatsApp bridge |
| `src/tools/voice_transcribe.rs` | Audio download + Whisper transcription |
| `src/skills/mod.rs` | Skills system module |
| `src/skills/types.rs` | Skill JSON types |
| `src/skills/executor.rs` | Skill step execution |
| `src/skills/monitor.rs` | Skill monitoring/logging |

### Modified Files

| File | Changes |
|------|---------|
| `src/channels/mod.rs` | Add `pub mod matrix;` |
| `src/tools/mod.rs` | Add voice_transcribe tool to registry |
| `src/main.rs` | Add /skill slash commands |
| `src/agent/agent_loop.rs` | Handle skill execution, voice messages |
| `src/bus.rs` | Add media_url field to InboundMessage |
| `Cargo.toml` | Add matrix-sdk, whisper-rs, symphonia dependencies |

### Dependencies

#### New Dependencies
| Crate | Version | Justification |
|-------|---------|---------------|
| matrix-sdk | 0.7 | Matrix client for WhatsApp bridge |
| whisper-rs | 0.11 | OpenAI Whisper transcription |
| ort | 2.0 | ONNX runtime for Whisper |
| symphonia | 0.5 | Audio format conversion |
| tokio-util | 0.7 | Already present, codec support |

---

## API Design

### Matrix Channel

```rust
/// Matrix channel for WhatsApp bridge
pub struct MatrixChannel {
    config: MatrixConfig,
    client: Option<Client>,
    running: Arc<AtomicBool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MatrixConfig {
    pub homeserver_url: String,
    pub username: String,
    pub password: String,
    pub allow_from: Vec<String>,
}

#[async_trait]
impl Channel for MatrixChannel {
    fn name(&self) -> &str { "matrix" }

    async fn start(&self, bus: Arc<MessageBus>) -> anyhow::Result<()> {
        // Login to Matrix, sync room messages
        // Convert m.room.message to InboundMessage
    }

    async fn send(&self, msg: OutboundMessage) -> anyhow::Result<()> {
        // Send m.room.message to Matrix room
    }
}
```

### Voice Transcription Tool

```rust
/// Tool for transcribing voice messages
pub struct VoiceTranscribeTool {
    whisper: WhisperContext,
    temp_dir: PathBuf,
}

#[async_trait]
impl Tool for VoiceTranscribeTool {
    fn name(&self) -> &str { "voice_transcribe" }

    fn description(&self) -> &str {
        "Transcribe voice messages to text using Whisper"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "audio_url": {
                    "type": "string",
                    "description": "URL to audio file"
                },
                "language": {
                    "type": "string",
                    "description": "Language code (optional, auto-detect if not provided)"
                }
            },
            "required": ["audio_url"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String, ToolError> {
        // 1. Download audio from URL
        // 2. Convert to WAV 16kHz mono using symphonia
        // 3. Transcribe with Whisper
        // 4. Return text
    }
}
```

### Skills System

```rust
/// Skill definition from JSON
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Skill {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: Option<String>,
    pub steps: Vec<SkillStep>,
}

/// Individual step in a skill
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum SkillStep {
    #[serde(rename = "tool")]
    Tool {
        tool: String,
        args: Value,
    },
    #[serde(rename = "llm")]
    Llm {
        prompt: String,
    },
}

/// Skill execution handle
pub struct SkillExecutor {
    skills_dir: PathBuf,
    current_execution: Option<ExecutionHandle>,
}

impl SkillExecutor {
    /// Load skill from file
    pub fn load(&self, name: &str) -> anyhow::Result<Skill>;

    /// Save skill to file
    pub fn save(&self, skill: &Skill) -> anyhow::Result<()>;

    /// List available skills
    pub fn list(&self) -> anyhow::Result<Vec<String>>;

    /// Execute skill with given inputs
    pub async fn execute(&mut self, skill: &Skill, inputs: HashMap<String, String>) -> anyhow::Result<SkillResult>;

    /// Cancel current execution
    pub fn cancel(&mut self) -> anyhow::Result<()>;
}
```

### Enhanced InboundMessage

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboundMessage {
    pub channel: String,
    pub sender_id: String,
    pub chat_id: String,
    pub content: String,
    pub media: Vec<String>,
    pub media_url: Option<String>, // NEW: URL to download voice/file
    pub media_type: Option<MediaType>, // NEW: voice, image, file
    pub metadata: HashMap<String, String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MediaType {
    Voice,
    Image,
    File,
    Video,
}
```

---

## Test Strategy

### Unit Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_matrix_channel_new` | `channels/matrix.rs` | Channel creation |
| `test_matrix_login` | `channels/matrix.rs` | Matrix authentication |
| `test_voice_transcribe_mock` | `tools/voice_transcribe.rs` | Tool execution |
| `test_skill_load` | `skills/executor.rs` | JSON parsing |
| `test_skill_execute_steps` | `skills/executor.rs` | Step execution |
| `test_skill_cancel` | `skills/monitor.rs` | Cancellation |

### Integration Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_matrix_whatsapp_flow` | `tests/matrix_integration.rs` | Full Matrix→WhatsApp flow |
| `test_voice_message_pipeline` | `tests/voice_integration.rs` | Download→Transcribe→Text |
| `test_skill_end_to_end` | `tests/skill_integration.rs` | Save→Load→Execute |

### Property Tests

```rust
// Skill JSON should always parse if valid
proptest! {
    #[test]
    fn skill_json_roundtrip(skill: Skill) {
        let json = serde_json::to_string(&skill).unwrap();
        let parsed: Skill = serde_json::from_str(&json).unwrap();
        assert_eq!(skill.name, parsed.name);
        assert_eq!(skill.steps.len(), parsed.steps.len());
    }
}
```

---

## Implementation Steps

### Step 1: WhatsApp via Matrix (Week 1-2)

**Files:** `src/channels/matrix.rs`, `src/channels/mod.rs`, `Cargo.toml`

**Description:** Implement Matrix channel adapter that connects to mautrix-whatsapp bridge

**Tasks:**
1. Add matrix-sdk dependency
2. Implement MatrixChannel with login/sync
3. Handle m.room.message events
4. Support text and media messages
5. Map Matrix users to allow_from whitelist

**Tests:**
- Unit: Channel creation, login
- Integration: Message flow (mock Matrix server)

**Dependencies:** None
**Estimated:** 8 hours

### Step 2: Voice Transcription Tool (Week 2-3)

**Files:** `src/tools/voice_transcribe.rs`, `src/tools/mod.rs`

**Description:** Create tool that downloads voice messages and transcribes with Whisper

**Tasks:**
1. Add whisper-rs, ort, symphonia dependencies
2. Implement audio download (reqwest)
3. Implement format conversion (ogg→wav)
4. Integrate Whisper base model
5. Add voice_transcribe to ToolRegistry

**Tests:**
- Unit: Mock Whisper, test pipeline
- Integration: Real audio file transcription

**Dependencies:** Step 1 (for Matrix voice messages)
**Estimated:** 10 hours

### Step 3: Skills System Core (Week 3-4)

**Files:** `src/skills/mod.rs`, `src/skills/types.rs`, `src/skills/executor.rs`

**Description:** JSON-based skill definitions with sequential step execution

**Tasks:**
1. Define Skill and SkillStep types
2. Implement JSON serialization
3. Create SkillExecutor with load/save/list
4. Implement step execution (tool calls, LLM prompts)
5. Add skills directory (~/.config/terraphim/skills/)

**Tests:**
- Unit: Load/save, step execution
- Integration: End-to-end skill execution

**Dependencies:** None (independent)
**Estimated:** 12 hours

### Step 4: Skills Slash Commands (Week 4)

**Files:** `src/main.rs`, `src/agent/agent_loop.rs`

**Description:** CLI commands for skill management

**Tasks:**
1. Add /skill save <name> - Save current conversation as skill
2. Add /skill load <name> - Load and execute skill
3. Add /skill list - Show available skills
4. Add /skill cancel - Cancel running skill
5. Handle skill execution in agent loop

**Tests:**
- Unit: Slash command parsing
- Integration: Full command flow

**Dependencies:** Step 3
**Estimated:** 6 hours

### Step 5: Skills Monitoring (Week 5)

**Files:** `src/skills/monitor.rs`

**Description:** Monitor skill execution, logging, progress reporting

**Tasks:**
1. Add execution logging
2. Implement progress reporting (step X of Y)
3. Add timeout handling
4. Create execution report on completion

**Tests:**
- Unit: Monitor functionality
- Integration: Long-running skill monitoring

**Dependencies:** Step 4
**Estimated:** 4 hours

### Step 6: Integration & Polish (Week 5-6)

**Files:** All modified files

**Description:** End-to-end testing, documentation, examples

**Tasks:**
1. Write integration tests for all features
2. Create example skills (analyze-repo, research-topic)
3. Update documentation
4. Performance testing
5. Code review and cleanup

**Tests:**
- Full test suite
- UBS static analysis
- Benchmarks

**Dependencies:** All previous steps
**Estimated:** 10 hours

---

## Rollback Plan

If issues discovered:

1. **WhatsApp not working**: Disable Matrix feature flag, keep Telegram/Discord
2. **Voice too slow**: Disable voice_transcribe tool, text-only mode
3. **Skills buggy**: Disable /skill commands, core agent loop unaffected

**Feature Flags:**
- `matrix` - Enable Matrix/WhatsApp support
- `voice` - Enable voice transcription
- `skills` - Enable skill system

---

## Performance Considerations

### Expected Performance

| Metric | Target | Measurement |
|--------|--------|-------------|
| Matrix message latency | < 2s | Message round-trip |
| Voice transcription (30s) | < 5s | Download + convert + Whisper |
| Skill load time | < 100ms | File I/O |
| Memory per voice model | < 200MB | Whisper base + ort |

### Benchmarks to Add

```rust
#[bench]
fn bench_voice_transcribe_30s(b: &mut Bencher) {
    let audio = load_test_audio("30s_voice.ogg");
    let tool = VoiceTranscribeTool::new();
    b.iter(|| tool.transcribe(&audio));
}

#[bench]
fn bench_skill_execute_10_steps(b: &mut Bencher) {
    let skill = create_test_skill(10);
    let executor = SkillExecutor::new();
    b.iter(|| executor.execute(&skill, HashMap::new()));
}
```

---

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Matrix server setup docs | Pending | Docs team |
| Whisper model download strategy | Pending | Dev team |
| Skill examples (3 minimum) | Pending | Dev team |

---

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Performance targets agreed
- [ ] Human approval received

---

**Design Complete**: Ready for specification interview and implementation.
