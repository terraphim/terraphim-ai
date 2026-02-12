# Phase 2 Research Document: TinyClaw Enhancements

**Status**: Draft
**Author**: Terraphim AI Team
**Date**: 2026-02-12
**Related**: Phase 1 Implementation Complete (Issue #519, PR #518)
**Previous Design**: `docs/plans/tinyclaw-terraphim-design.md`

---

## Executive Summary

Phase 1 of TinyClaw delivered a production-ready multi-channel AI assistant with Telegram, Discord, and CLI support. Phase 2 will extend capabilities with WhatsApp integration, voice transcription, persistent skills, and advanced orchestration features.

**Key Finding**: WhatsApp integration is high-value but complex (requires 3rd-party bridges). Voice transcription adds accessibility. Skills system enables reusable workflows. All three align with the original TinyClaw vision of a ubiquitous AI assistant.

---

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | ✅ YES | Extends successful Phase 1, high user demand for WhatsApp |
| Leverages strengths? | ✅ YES | Builds on existing channel architecture, uses terraphim_multi_agent |
| Meets real need? | ✅ YES | WhatsApp = 2+ billion users, voice = accessibility requirement |

**Proceed**: ✅ YES - All 3 questions answered affirmatively

---

## Problem Statement

### Current State (Phase 1)
TinyClaw supports:
- ✅ Telegram (via teloxide)
- ✅ Discord (via serenity)
- ✅ CLI (interactive terminal)
- ✅ 5 tools (filesystem, edit, shell, web_search, web_fetch)
- ✅ Session persistence
- ✅ Tool-calling agent loop

### Gaps Identified
1. **Missing WhatsApp** - 2+ billion active users, dominant in many markets
2. **Text-only interaction** - No voice support for accessibility or hands-free use
3. **Ephemeral workflows** - Cannot save/load reusable skill sequences
4. **Single-agent limitation** - No subagent spawning for parallel tasks

### Success Criteria
1. WhatsApp messages route through TinyClaw agent loop
2. Voice messages transcribed and processed as text
3. Skills can be created, saved, loaded, and shared
4. Subagents can be spawned for parallel task execution

---

## Current State Analysis

### Existing Channel Architecture
```
Channel trait (src/channel.rs)
├── CliChannel (src/channels/cli.rs)
├── TelegramChannel (src/channels/telegram.rs)
└── DiscordChannel (src/channels/discord.rs)

Pattern: Each adapter implements Channel trait
         start() spawns listener
         send() formats for platform
         is_allowed() checks whitelist
```

### Extension Points Identified
| Component | Extension Point | Location |
|-----------|-----------------|----------|
| Channels | `Channel` trait implementation | `src/channels/` |
| Tools | `Tool` trait in registry | `src/tools/mod.rs` |
| Session | `SessionManager` persistence | `src/session.rs` |
| Messages | `InboundMessage/OutboundMessage` | `src/bus.rs` |

### Code Locations
| Feature | Location | Notes |
|---------|----------|-------|
| Channel trait | `src/channel.rs:9` | Async trait, Send + Sync |
| Message bus | `src/bus.rs:91` | tokio mpsc channels |
| Tool registry | `src/tools/mod.rs:76` | HashMap of Box<dyn Tool> |
| Agent loop | `src/agent/agent_loop.rs:147` | Hybrid router, tool execution |

---

## Constraints

### Technical Constraints

#### WhatsApp Integration
- **No official Rust SDK** - Must use unofficial libraries or bridges
- **WhatsApp Business API** - Requires Meta approval, phone number registration
- **Rate limiting** - Aggressive limits on message sending
- **Web vs Mobile** - Desktop requires phone connection

#### Voice Transcription
- **Model size** - Whisper models 39MB-155MB (base to large)
- **Inference latency** - Real-time requires optimization
- **Language support** - 99 languages, but quality varies
- **Audio format** - Must handle multiple formats (opus, ogg, mp3, wav)

#### Skills System
- **Storage format** - JSON serialization needed
- **Versioning** - Skills may need schema migration
- **Security** - Skill definitions could contain malicious commands
- **Sharing** - Import/export functionality required

### Business Constraints
- **Timeline**: 4-6 weeks for Phase 2 MVP
- **Resources**: 1-2 developers
- **Dependencies**: Must not break Phase 1 functionality
- **Compliance**: WhatsApp Business API has strict usage policies

### Non-Functional Requirements

| Requirement | Target | Current |
|-------------|--------|---------|
| Voice transcription latency | < 5s for 30s audio | N/A |
| WhatsApp message delivery | < 2s | N/A |
| Skill load time | < 100ms | N/A |
| Memory per voice model | < 200MB (base model) | N/A |

---

## Vital Few (Essentialism)

### Essential Constraints (Max 3)

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| WhatsApp via Matrix bridge | Meta approval for official API takes weeks/months | Research shows unofficial libraries face blocking |
| Whisper base model | Large models too slow for real-time, tiny too inaccurate | Benchmarks show base is sweet spot |
| Skills as JSON files | Must be human-readable for debugging | CLI users expect editable configs |

### Eliminated from Scope (5/25 Rule)

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Native WhatsApp library (whatsapp-web-rs) | Unreliable, frequent breaking changes by Meta |
| Real-time voice streaming | Too complex for Phase 2, batch transcription sufficient |
| Skill marketplace/cloud sync | Scope creep, local files satisfy MVP |
| Video transcription | Audio-only sufficient for Phase 2 |
| Custom voice models | Whisper base model is state-of-art for general use |

---

## Dependencies

### Internal Dependencies
| Dependency | Impact | Risk |
|------------|--------|------|
| terraphim_multi_agent | LLM integration for skills | Low - already integrated |
| terraphim_config | Config loading for skills | Low - proven stable |
| MessageBus | Voice messages need media support | Medium - extend struct |

### External Dependencies

#### WhatsApp Options
| Option | Pros | Cons | Risk |
|--------|------|------|------|
| **Matrix bridge (recommended)** | Reliable, established, Rust SDK | Requires Matrix server | Low |
| whatsapp-web-rs | Direct, no bridge needed | Frequent breakage, ToS issues | High |
| WhatsApp Business API | Official, supported | Approval required, expensive | Medium |

#### Voice Transcription
| Crate | Model | Size | Latency | Risk |
|-------|-------|------|---------|------|
| **whisper-rs** | OpenAI Whisper | 39MB-155MB | 2-5s | Low |
| rust-whisper | Same | Same | Same | Medium (less mature) |

#### Skills Storage
| Format | Pros | Cons | Risk |
|--------|------|------|------|
| **JSON** | Human-readable, standard | Verbose | Low |
| YAML | More readable | Slower parsing | Low |
| TOML | Good for configs | Less tooling | Low |

---

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| WhatsApp blocks bridge | Medium | High | Fallback to Matrix, monitor rate limits |
| Whisper model too slow | Low | Medium | Use base model, quantize if needed |
| Skill format changes | Medium | Medium | Version field in JSON, migration path |
| Audio format compatibility | Medium | Medium | Convert to WAV before transcription |

### Open Questions

1. **WhatsApp Business vs Personal** - Do we need both or is bridge sufficient?
   - *Investigation*: Check user requirements
   - *Owner*: Product team

2. **Voice message size limits** - WhatsApp has 100MB limit, but what's practical?
   - *Investigation*: Test with various audio lengths
   - *Owner*: Dev team

3. **Skill sharing mechanism** - Git-based or file-based?
   - *Investigation*: Prototype both approaches
   - *Owner*: Dev team

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| Matrix bridge is acceptable | Users can set up Matrix server | User adoption blocked | No - needs validation |
| Whisper base model quality sufficient | OpenAI benchmarks | User complaints about accuracy | No - needs testing |
| Skills are primarily user-created | TinyClaw is power-user tool | Skills system unused | Partial - based on TinyClaw user base |

### Multiple Interpretations Considered

| Interpretation | Implications | Why Chosen/Rejected |
|----------------|--------------|---------------------|
| **Voice as first-class citizen** | Every message could be voice | Rejected - too expensive, text primary |
| Voice as accessibility feature | Only when explicitly requested | **Chosen** - aligns with use case |
| WhatsApp as primary channel | Optimize for WhatsApp first | Rejected - multi-channel parity |
| WhatsApp as equal channel | Same features as Telegram/Discord | **Chosen** - consistent UX |

---

## Research Findings

### WhatsApp Integration Analysis

**Matrix Bridge Approach** (Recommended):
- Uses `matrix-sdk` Rust crate
- WhatsApp bridge: `mautrix-whatsapp`
- Architecture: TinyClaw → Matrix → WhatsApp bridge → WhatsApp
- Pros: Reliable, handles reconnection, respects rate limits
- Cons: Requires Matrix homeserver setup

**Alternative - whatsapp-web-rs**:
- Direct WebSocket connection to WhatsApp Web
- Pros: No middleman, lower latency
- Cons: Breaks frequently with WhatsApp updates, ToS violations

### Voice Transcription Analysis

**Whisper Base Model** (Recommended):
- Size: 74MB (downloadable on first use)
- Language: 99 languages supported
- Speed: ~2x real-time on modern CPU
- Quality: Very good for general transcription
- Integration: `whisper-rs` crate with `ort` backend

**Audio Pipeline**:
```
WhatsApp/Discord voice message (ogg/opus)
    ↓
Download to temp file
    ↓
Convert to WAV (16kHz, mono) using `symphonia`
    ↓
Transcribe with Whisper
    ↓
Text → Agent loop
```

### Skills System Analysis

**Skill Definition**:
```json
{
  "name": "analyze-repo",
  "version": "1.0.0",
  "description": "Analyze a git repository",
  "steps": [
    {"tool": "shell", "args": {"command": "git clone {repo_url}"}},
    {"tool": "filesystem", "args": {"operation": "list_directory", "path": "."}}
  ]
}
```

**Storage Location**: `~/.config/terraphim/skills/`

---

## Recommendations

### Proceed/No-Proceed
✅ **PROCEED** - All essential questions answered YES, risks manageable

### Scope Recommendations

**Phase 2 MVP** (4-6 weeks):
1. WhatsApp via Matrix bridge
2. Voice transcription (Whisper base)
3. Skills system (JSON-based, local storage)

**Phase 2+** (Future):
- Subagent spawning (requires more research on terraphim_multi_agent)
- Evaluation framework (metrics collection)
- WhatsApp Business API (if approved)

### Risk Mitigation Recommendations

1. **WhatsApp blocking**: Implement health checks for Matrix bridge, fallback to "bridge unavailable" message
2. **Voice quality**: Allow users to retry with different audio, log accuracy metrics
3. **Skill compatibility**: Version field mandatory, migration tool for format changes

---

## Next Steps

If approved:
1. Create Phase 2 Design Document (specify file changes, APIs, test strategy)
2. Conduct specification interview for edge cases
3. Implement in 3 steps: WhatsApp → Voice → Skills
4. Validation and deployment

---

## Appendix

### Reference Materials
- Original TinyClaw design: `docs/plans/tinyclaw-terraphim-design.md`
- Matrix SDK: https://github.com/matrix-org/matrix-rust-sdk
- Whisper Rust: https://github.com/tazz4843/whisper-rs
- mautrix-whatsapp: https://github.com/mautrix/whatsapp

### Code Snippets

**WhatsApp Message Structure** (Matrix bridge):
```json
{
  "type": "m.room.message",
  "content": {
    "msgtype": "m.text",
    "body": "Hello from WhatsApp"
  },
  "sender": "@whatsapp_12345:matrix.example.com"
}
```

**Voice Message** (WhatsApp):
```json
{
  "msgtype": "m.audio",
  "url": "mxc://example.com/audio-file",
  "info": {
    "mimetype": "audio/ogg",
    "size": 12345
  }
}
```

---

**Research Complete**: Phase 2 scope defined, ready for design document creation.
