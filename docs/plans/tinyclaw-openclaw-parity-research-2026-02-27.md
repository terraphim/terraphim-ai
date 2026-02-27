# Research Document: TinyClaw OpenClaw Parity via Terraphim Extensions

**Status**: Draft
**Author**: Terraphim AI
**Date**: 2026-02-27
**Reviewers**: [Pending]
**Related Issue**: #590

## Executive Summary

This research analyzes the gap between TinyClaw (a standalone multi-channel AI assistant in `crates/terraphim_tinyclaw/`) and OpenClaw-like capabilities. The goal is to extend TinyClaw to achieve practical parity with OpenClaw workflows while strictly adhering to the principle: **extend existing Terraphim functionality first, never duplicate**.

Key findings:
1. TinyClaw is completely standalone (zero Terraphim crate dependencies per #561)
2. Five critical capability gaps exist across sessions, web search, markdown commands, voice, and orchestration
3. Existing Terraphim crates (`terraphim-markdown-parser`, `terraphim_spawner`) can provide significant functionality without duplication
4. Foundation hardening must precede feature work to avoid compounding instability

---

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | **Yes** | Multi-channel AI assistant is core to Terraphim's product vision; team has invested heavily in TinyClaw |
| Leverages strengths? | **Yes** | Existing parser, spawner, and middleware crates provide reusable foundations |
| Meets real need? | **Yes** | Epic #590 explicitly tracks user demand for OpenClaw parity |

**Proceed**: **Yes** - All 3 questions answered YES

---

## Problem Statement

### Description

TinyClaw currently lacks several capabilities that users expect from a modern AI assistant comparable to OpenClaw:

1. **Session management**: `/reset` slash command doesn't actually clear session state
2. **Configuration wiring**: Config fields like `agent.max_session_messages` exist but are unused
3. **Web search**: `web_search` tool is a placeholder with no provider integration
4. **Markdown-defined workflows**: Skills are JSON-only; no markdown command support
5. **Voice transcription**: Tool exists but is feature-gated and non-functional
6. **Session tools**: No ability to list sessions, view history, or send messages across sessions
7. **Agent spawning**: No integration with `terraphim_spawner` for multi-agent workflows

### Impact

- Users cannot achieve practical OpenClaw-like workflows
- Feature drift between documentation and implementation causes confusion
- Safety gaps (skills bypass ExecutionGuard) create security risks
- Standalone architecture prevents leveraging Terraphim ecosystem improvements

### Success Criteria

1. `/reset` clears session messages and summary
2. All config fields are actively used by runtime
3. Skills use unified guard policy (no bypass path)
4. `web_search` returns real results from configured providers
5. Markdown-defined commands and skills load and execute
6. Voice transcription works when feature-enabled
7. Session tools expose list/history/send capabilities
8. Agent spawning integrates with `terraphim_spawner` (#560)

---

## Current State Analysis

### Codebase Structure

```
crates/terraphim_tinyclaw/
├── src/
│   ├── main.rs              # CLI entry (agent/gateway/skill modes)
│   ├── lib.rs               # Public exports
│   ├── config.rs            # Config with env expansion
│   ├── bus.rs               # MessageBus for channel communication
│   ├── session.rs           # SessionManager with JSONL persistence
│   ├── channel.rs           # Channel trait and manager
│   ├── channels/            # Telegram, Discord, CLI adapters
│   ├── agent/
│   │   ├── agent_loop.rs    # HybridLlmRouter + ToolCallingLoop
│   │   ├── proxy_client.rs  # Anthropic API format client
│   │   └── execution_guard.rs # Command safety checks
│   ├── tools/
│   │   ├── mod.rs           # Tool trait and registry
│   │   ├── filesystem.rs    # File operations
│   │   ├── shell.rs         # Shell execution (guarded)
│   │   ├── edit.rs          # Search/replace
│   │   ├── web.rs           # web_search (placeholder) + web_fetch
│   │   └── voice_transcribe.rs # Stub with voice feature gate
│   └── skills/
│       ├── mod.rs           # Skill types
│       ├── executor.rs      # Skill execution (bypasses guards)
│       ├── types.rs         # Skill JSON schema
│       └── monitor.rs       # Execution monitoring
├── examples/skills/         # JSON skill examples
└── Cargo.toml               # Features: telegram, discord, voice
```

### Key Components

| Component | Location | Purpose |
|-----------|----------|---------|
| SessionManager | `src/session.rs:152` | JSONL persistence, in-memory cache |
| ToolCallingLoop | `src/agent/agent_loop.rs:294` | Main agent loop with tool calling |
| HybridLlmRouter | `src/agent/agent_loop.rs:43` | 2-tier routing (proxy -> Ollama) |
| ExecutionGuard | `src/agent/execution_guard.rs` | Command safety evaluation |
| SkillExecutor | `src/skills/executor.rs:47` | JSON workflow execution |
| ToolRegistry | `src/tools/mod.rs:64` | Tool registration and dispatch |

### Data Flow

```
[Telegram/Discord/CLI] -> MessageBus -> ToolCallingLoop
                                              |
                    +-------------------------+-------------------------+
                    |                         |                         |
               SessionManager            ToolRegistry             HybridLlmRouter
                    |                         |                         |
            [JSONL storage]          [filesystem, shell,      [Proxy/Ollama]
                                      web, edit, voice]
```

### Integration Points

| Integration | Current State | Target State |
|-------------|---------------|--------------|
| terraphim-markdown-parser | **Not used** | Parse markdown commands/skills |
| terraphim_spawner | **Not used** | Agent spawning tool (#560) |
| terraphim_middleware | **Not used** | Web search providers |
| terraphim_config | **Not used** | Role-based configuration |

---

## Gap Analysis by Track

### Track 1: Foundation Hardening (#588)

| Issue | Current | Expected | Risk |
|-------|---------|----------|------|
| `/reset` command | Returns message but doesn't clear state | Clear messages + summary | Low |
| `max_session_messages` | Config field exists, unused | Trigger compression | Medium |
| Skill guard bypass | Skills execute shell directly | Route through ExecutionGuard | **High** |
| Tool config | `ToolsConfig` exists, unused | Wire to tool implementations | Low |
| Docs drift | Claims Matrix/voice support | Match actual feature state | Low |

**Code Evidence**:
- `agent_loop.rs:437`: `/reset` returns message without session clearing
- `config.rs:38`: `max_session_messages` defined, never read
- `skills/executor.rs:230`: Shell step uses `tokio::process::Command` directly, bypassing guard

### Track 2: Provider-Backed Web Search (#589)

| Issue | Current | Expected |
|-------|---------|----------|
| `web_search` | Placeholder returning static message | Real search via Brave/SearXNG/Google |
| `web_fetch` | Raw fetch only | Configurable mode (raw/readability) |
| Provider config | `WebToolsConfig` stub | Full provider selection + credentials |

**Code Evidence**:
- `tools/web.rs:24`: `WebSearchTool::search()` returns placeholder text
- `tools/web.rs:13`: Provider field exists but not configurable

### Track 3: Markdown Commands and Skills (#592)

| Issue | Current | Expected |
|-------|---------|----------|
| Skill format | JSON only | JSON + Markdown frontmatter |
| Command definitions | Hardcoded slash commands | Load from markdown files |
| Parser | Not integrated | Use `terraphim-markdown-parser` |

**Code Evidence**:
- `skills/types.rs`: JSON schema only
- `agent_loop.rs:428`: Slash commands hardcoded in `handle_slash_command()`
- `terraphim-markdown-parser/src/lib.rs`: Block ID parser available

### Track 4: Voice Transcription (#593)

| Issue | Current | Expected |
|-------|---------|----------|
| Feature gate | `voice` exists but empty | Real whisper integration |
| Download | Implemented | Keep |
| Conversion | Placeholder | WAV conversion (16kHz mono) |
| Transcription | Returns placeholder string | Whisper inference |
| Error handling | Generic | Provider unavailable, invalid audio, timeout |

**Code Evidence**:
- `tools/voice_transcribe.rs:108`: `transcribe()` returns placeholder
- `Cargo.toml:58`: `voice = []` (empty feature)
- `tools/voice_transcribe.rs:93`: `convert_to_wav()` is pass-through

### Track 5: Session Tools and Orchestration (#591)

| Issue | Current | Expected |
|-------|---------|----------|
| Session tools | None | `sessions_list`, `sessions_history`, `sessions_send` |
| Spawn tool | None | `agent_spawn` via terraphim_spawner |
| Cron | None | Split to follow-up issue |

**Code Evidence**:
- `tools/mod.rs:113`: No session tools in default registry
- Issue #560: Spawner integration already planned

---

## Constraints

### Technical Constraints

| Constraint | Description | Source |
|------------|-------------|--------|
| Zero Terraphim deps | TinyClaw is completely standalone | #561 finding |
| Feature gates | telegram, discord, voice must remain optional | Cargo.toml |
| Async runtime | Tokio required (already used) | `Cargo.toml:12` |
| Edition 2024 | Rust edition constraint | Workspace |

### Business Constraints

| Constraint | Description | Source |
|------------|-------------|--------|
| No breaking changes | Existing skills/configs must work | Stability |
| Documentation match | Docs must reflect shipped behavior | #588 acceptance |

### Non-Functional Requirements

| Requirement | Target | Current |
|-------------|--------|---------|
| Session list latency | < 100ms | N/A (not implemented) |
| Web search timeout | < 10s | N/A (placeholder) |
| Voice transcription | < 30s | N/A (stub) |
| Tool guard coverage | 100% | ~60% (skills bypass) |

---

## Vital Few (Essentialism)

### Essential Constraints (Max 3)

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| **Extend, don't duplicate** | Core epic principle; prevents ecosystem fragmentation | Issue #590 description |
| **Foundation before features** | Unstable base compounds bugs | #588 rationale |
| **Feature gates for heavy deps** | Voice transcription deps are large | Current `voice = []` design |

### Eliminated from Scope

Apply 5/25 rule to potential work:

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Matrix channel | Already disabled due to sqlite conflict; out of scope for parity |
| Custom LLM provider registry | Reuse existing Terraphim patterns instead |
| Built-in vector search | Use Terraphim's existing haystack/middleware |
| Custom markdown parser | Use `terraphim-markdown-parser` instead |
| Workflow DAG execution | Skills are sequential; complex DAG out of scope |

---

## Dependencies

### Internal Dependencies (Potential Reuse)

| Dependency | Location | What It Provides | Integration Risk |
|------------|----------|------------------|------------------|
| terraphim-markdown-parser | `crates/terraphim-markdown-parser/` | Block ID parsing, frontmatter extraction | Low - clean API |
| terraphim_spawner | `crates/terraphim_spawner/` | Agent spawning, health checks, pooling | Medium - new dependency |
| terraphim_middleware | `crates/terraphim_middleware/` | Web search providers (QueryRs, etc.) | Medium - may need adaptation |
| terraphim_config | `crates/terraphim_config/` | Role-based configuration | Medium - overlapping concerns |

### External Dependencies (Proposed)

| Dependency | Version | Purpose | Feature Gate |
|------------|---------|---------|--------------|
| whisper-rs | 0.11 | Voice transcription | `voice` |
| symphonia | 0.5 | Audio format conversion | `voice` |
| readability | 0.3 | HTML content extraction | default |

---

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| terraphim_spawner API mismatch | Medium | High | Coordinate with #560 implementation |
| Markdown parser needs extension | Medium | Medium | Contribute upstream or fork temporarily |
| Voice deps increase binary size | High | Medium | Feature gate + optional compilation |
| Session format compatibility | Low | High | Version field in JSONL |

### Open Questions

1. **Should markdown commands use the same parser as skills?** - Need to decide unified vs separate parsing
2. **How does OpenClaw handle session history queries?** - Need reference for #591 UX
3. **What's the exact provider API for web search?** - Brave vs SearXNG vs Google - need to pick primary
4. **Should voice transcription use local whisper or API?** - Local = heavy deps, API = privacy concerns

### Assumptions

1. **Assumption**: terraphim_spawner v1.8.0 API is stable - Basis: Issue #560 description
2. **Assumption**: terraphim-markdown-parser can be extended for frontmatter - Basis: Clean existing API
3. **Assumption**: Web search providers have compatible response formats - Basis: Need validation

---

## Research Findings

### Key Insights

1. **TinyClaw's standalone nature is intentional** - This allows clean addition of Terraphim dependencies rather than refactoring (#561)

2. **ExecutionGuard bypass is the highest risk** - Skills executing shell directly bypass all safety controls (confirmed in `skills/executor.rs:230`)

3. **Config is well-structured but unused** - `ToolsConfig`, `max_session_messages` exist but aren't wired to behavior

4. **Markdown parser is ready for reuse** - `terraphim-markdown-parser` has clean API for block extraction

5. **Voice feature gate is properly designed** - Just needs actual implementation behind the gate

### Relevant Prior Art

| Source | Relevance |
|--------|-----------|
| Issue #560 | terraphim_spawner integration plan |
| Issue #561 | Standalone architecture analysis |
| terraphim-markdown-parser | Block ID extraction patterns |
| OpenClaw (external) | Capability reference |

### Technical Spikes Needed

| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| terraphim_spawner integration test | Verify API compatibility | 2 hours |
| Markdown frontmatter parsing | Test parser extension | 2 hours |
| Brave Search API validation | Confirm response format | 1 hour |

---

## Recommendations

### Proceed/No-Proceed

**PROCEED** - The work is essential, leverages existing strengths, and meets validated need.

### Scope Recommendations

1. **Must Do (Foundation)**: #588 - Must complete before other tracks
2. **Should Do (Core Parity)**: #589, #592 - High user value
3. **Could Do (Extended)**: #593, #591 - Valuable but can sequence

### Sequencing Recommendation

```
Phase 1: Foundation (#588)
    |
Phase 2: Core Features (#589, #592 in parallel)
    |
Phase 3: Extended Features (#593, #591 in parallel)
```

### Risk Mitigation

1. **ExecutionGuard bypass**: Immediate fix in #588 before any other work
2. **Spawner API mismatch**: Prototype integration before committing to schedule
3. **Markdown parser gaps**: Fork temporarily if upstream changes needed

---

## Next Steps

If approved:
1. Create design document (Phase 2) for each track
2. Prioritize #588 as foundation
3. Coordinate with #560 for spawner alignment
4. Schedule implementation in sequenced phases

---

## Appendix

### Reference Materials

- Issue #590: Epic definition
- Issue #588-593: Sub-issue details
- Issue #560: Spawner integration
- Issue #561: Architecture analysis
- `crates/terraphim_tinyclaw/README.md`: Current capabilities

### Code Snippets

**ExecutionGuard bypass (skills/executor.rs:230)**:
```rust
SkillStep::Shell { command, working_dir } => {
    self.execute_shell_step(command, working_dir.as_deref(), &inputs)
        .await  // Direct shell execution, no guard check
}
```

**Unused config (config.rs:38)**:
```rust
#[serde(default = "default_max_session_messages")]
pub max_session_messages: usize,  // Never read by runtime
```

**Placeholder web search (tools/web.rs:24)**:
```rust
async fn search(&self, query: &str, num_results: usize) -> Result<String, ToolError> {
    // For now, return a placeholder implementation
    let results = format!("Search results for '{}' ...", query);
    Ok(results)
}
```
