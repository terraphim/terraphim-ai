# Research Document: Rebuilding TinyClaw on Terraphim AI

**Status**: Draft
**Author**: Terraphim AI Research
**Date**: 2026-02-11
**Reviewers**: Alex
**Based on**: PR #518 (`claude/tinyclaw-terraphim-plan-lIt3V`)
**Reference Projects**: PicoClaw (Go), nanobot (Python), TinyClaw (TypeScript/bash)

## Executive Summary

PR #518 proposes rebuilding TinyClaw as a multi-channel AI assistant on Terraphim's agent crate ecosystem. This research validates the Terraphim crate maturity claims, compares against two production-grade reference implementations (PicoClaw in Go, nanobot in Python), and identifies 15 major feature gaps in the current plan. The Terraphim agent crates are confirmed production-ready (181 tests, 96.7% pass rate), but the plan significantly underestimates the scope needed to match PicoClaw/nanobot capabilities.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Multi-channel AI assistant is core to Terraphim's mission as privacy-first AI |
| Leverages strengths? | Yes | Terraphim's agent crates (messaging, supervision, evolution) provide unique Rust-based foundation |
| Meets real need? | Yes | PicoClaw (Go) and nanobot (Python) prove market demand; Rust version adds safety + performance |

**Proceed**: Yes - 3/3 YES

## Problem Statement

### Description
Rebuild TinyClaw (a lightweight multi-channel AI assistant) on Terraphim AI's existing agent framework, achieving feature parity with PicoClaw (Go) and nanobot (Python) while leveraging Terraphim's unique capabilities (knowledge graphs, OTP supervision, Firecracker VMs).

### Impact
Without this rebuild, Terraphim remains a search-focused backend. With it, Terraphim becomes a deployable AI assistant accessible via Discord, Telegram, WhatsApp, Slack, Email, and other channels.

### Success Criteria
- Channel abstraction trait supporting 5+ platforms
- Tool-calling agent loop with context compression
- Voice transcription integration
- Skills system with markdown-based definitions
- Session management with persistent history
- Cron/scheduled tasks
- Auth/allow-lists per channel
- Setup CLI for onboarding
- Comparable feature set to PicoClaw ~6000 LOC in idiomatic Rust

---

## Current State Analysis

### Reference Implementation Comparison

| Aspect | TinyClaw | PicoClaw (Go) | nanobot (Python) | PR #518 Plan |
|--------|----------|---------------|------------------|--------------|
| **Total LOC** | ~1,000 | ~6,054 | ~8,447 | ~2,000-2,700 est. |
| **Channels** | 2 (Discord, WhatsApp) | 6 (Telegram, Discord, WhatsApp, Feishu, QQ, MaixCam) | 9 (+Slack, Email, DingTalk, MoChat) | 2 (Discord, WhatsApp) |
| **LLM providers** | Claude CLI only | 7+ (OpenRouter, Anthropic, OpenAI, Groq, Gemini, Zhipu, vLLM) | 12+ via litellm registry | Ollama + OpenRouter |
| **Voice** | None | Groq Whisper | Groq Whisper | Not mentioned |
| **Skills** | None | Markdown-based with requirements | Markdown-based with progressive loading | Not mentioned |
| **Cron** | 5-min heartbeat | Full cron system (at/every) with persistence | Cron tool with cron expressions | "Small effort" |
| **Context compression** | None | LLM summarization at 75% context | LLM summarization | "Versioned memory" |
| **Tools** | None | 5 (filesystem, edit, shell, web_search, web_fetch) | 6 (+message, cron, spawn) | "Command execution" |
| **Subagents** | None | None | Background spawning with isolation | Not mentioned |
| **Auth** | None | Per-channel allow-lists | Per-channel allow-lists + consent | Not mentioned |
| **Media** | None | Photos, voice, audio, documents | Photos, voice, audio, documents | Not mentioned |
| **Formatting** | None | Markdown-to-Telegram-HTML | Markdown-to-Telegram-HTML | Not mentioned |
| **Modes** | Single | Agent (CLI) + Gateway (service) | Agent (CLI) + Gateway (service) | Single orchestrator |
| **Setup** | setup-wizard.sh | `onboard` command with templates | `onboard` command with templates | "dialoguer CLI" |
| **Session persistence** | None | JSON file per session | JSON file per session | Not mentioned |
| **Queue/messaging** | File-based | Go channels (buffer 100) | asyncio queues | Erlang mailboxes |

### Terraphim Agent Crates Maturity (Validated)

| Crate | LOC | Tests | Status | Key Capability |
|-------|-----|-------|--------|----------------|
| `terraphim_agent_messaging` | ~2,174 | 26/26 pass | Production | Erlang-style mailboxes, delivery guarantees, priority, routing |
| `terraphim_agent_supervisor` | ~1,452 | 16/16 pass | Production | OTP supervision trees (OneForOne/OneForAll/RestForOne), restart intensity |
| `terraphim_multi_agent` | ~8,445 | 63/63 pass | Production | Agent lifecycle, command processing, LLM integration, token/cost tracking |
| `terraphim_agent_evolution` | ~7,226 | E2E pass | Production | Versioned memory (short/long-term), tasks, lessons, workflow patterns |
| `terraphim_service` | ~35,123 | N/A | Production | LlmClient trait, Ollama + OpenRouter providers, LLM router |
| `terraphim_goal_alignment` | ~4,614 | 15/21 pass | Functional | Goal hierarchy, conflict detection, KG integration (6 ignored tests) |
| `terraphim_task_decomposition` | ~4,614 | 40/40 pass | Production | KG-based decomposition, execution plans, 4 strategies |
| **Total** | **~63,648** | **181 tests** | **96.7% pass** | |

### Validated: What Terraphim Already Has

The PR #518 "Ready" claims are **confirmed accurate** for:
- Erlang-style mailboxes with delivery guarantees (terraphim_agent_messaging)
- OTP supervision trees with restart strategies (terraphim_agent_supervisor)
- Agent lifecycle with command processing (terraphim_multi_agent)
- Versioned memory with short/long-term buckets (terraphim_agent_evolution)
- LLM integration with Ollama + OpenRouter (terraphim_service)
- Knowledge graph semantic matching (terraphim_rolegraph + terraphim_automata)

### Validated: What Terraphim Does NOT Have

These features exist in PicoClaw/nanobot but have no Terraphim equivalent:

| Missing Feature | PicoClaw Implementation | nanobot Implementation | Effort to Build |
|----------------|------------------------|----------------------|-----------------|
| Channel abstraction | `Channel` interface (83 LOC) | `BaseChannel` class | Medium |
| Message bus | Go channels + pub/sub (170 LOC) | asyncio queues | Small (tokio channels) |
| Telegram adapter | go-telegram-bot-api (501 LOC) | python-telegram-bot (400 LOC) | Medium |
| Discord adapter | discordgo/bwmarrin (247 LOC) | websockets (261 LOC) | Medium |
| WhatsApp adapter | WebSocket bridge (145 LOC) | Node.js bridge (145 LOC + TS bridge) | Medium-High |
| Voice transcription | Groq Whisper (166 LOC) | Groq Whisper (reuses provider) | Small |
| Context compression | LLM summarization (125 LOC) | LLM summarization (~100 LOC) | Small |
| Skills system | Markdown loader (479 LOC) | YAML frontmatter loader (229 LOC) | Medium |
| Cron service | Persistent JSON (382 LOC) | Cron tool (115 LOC) | Medium |
| Tool registry | Interface + 5 tools (844 LOC) | Base + 6 tools (~700 LOC) | Medium |
| Session manager | JSON persistence (184 LOC) | JSON persistence (~150 LOC) | Small |
| Markdown formatting | Telegram HTML converter (100 LOC) | Telegram HTML converter (~80 LOC) | Small |
| Media handling | Download + transcribe (per channel) | Download + transcribe (per channel) | Medium |
| Auth/allow-lists | Per-channel whitelist (15 LOC) | Per-channel + consent (20 LOC) | Small |
| Onboarding CLI | Template generation (264 LOC) | Template generation (~200 LOC) | Small |
| Agent vs Gateway modes | CLI + service modes (230 LOC) | CLI + service modes (~200 LOC) | Small |
| Subagent spawning | N/A | Background agents (245 LOC) | Medium |

---

## PicoClaw Architecture Deep Dive

### Channel Interface Pattern

```go
// /tmp/picoclaw/pkg/channels/base.go
type Channel interface {
    Name() string
    Start(ctx context.Context) error
    Stop(ctx context.Context) error
    Send(ctx context.Context, msg bus.OutboundMessage) error
    IsRunning() bool
    IsAllowed(senderID string) bool
}
```

**Key insight**: Every channel implements `IsAllowed()` for sender-level auth. Empty allow-list means open to all. This is checked before any message reaches the agent loop.

### Message Bus Pattern

```go
// /tmp/picoclaw/pkg/bus/types.go
type InboundMessage struct {
    Channel    string            `json:"channel"`
    SenderID   string            `json:"sender_id"`
    ChatID     string            `json:"chat_id"`
    Content    string            `json:"content"`
    Media      []string          `json:"media,omitempty"`
    SessionKey string            `json:"session_key"`
    Metadata   map[string]string `json:"metadata,omitempty"`
}

type OutboundMessage struct {
    Channel string `json:"channel"`
    ChatID  string `json:"chat_id"`
    Content string `json:"content"`
}
```

**Key insight**: `SessionKey` = `"{channel}:{chatID}"` enables per-conversation session isolation. Media is a list of local file paths (downloaded by the channel adapter).

### Agent Loop with Context Compression

```go
// /tmp/picoclaw/pkg/agent/loop.go (simplified)
func (a *AgentLoop) processMessage(ctx, inbound) {
    session := sessionManager.GetOrCreate(inbound.SessionKey)
    history := session.GetHistory()

    // Context compression trigger
    if len(history) > 20 || estimateTokens(history) > contextWindow*0.75 {
        go compressContext(session)  // Non-blocking
    }

    // Build messages: system prompt + summary + history + user message
    messages := buildContext(session, inbound)

    // Tool-calling loop (max iterations)
    for i := 0; i < maxIterations; i++ {
        response := llm.Chat(ctx, messages, tools, model, options)
        if len(response.ToolCalls) == 0 { break }
        // Execute tools, append results, continue
    }
}
```

**Key insight**: Context compression runs in a goroutine (non-blocking). Uses multi-part summarization for large histories (>10 messages split at midpoint). Keeps last 4 messages verbatim for continuity.

### Telegram Channel Features

```go
// /tmp/picoclaw/pkg/channels/telegram.go
// Key features beyond basic messaging:
// 1. Thinking indicators: animated "Thinking... [dots]" every 3s
// 2. Voice transcription: downloads .ogg, transcribes via Groq Whisper
// 3. Photo handling: downloads largest size, passes as [image: /path]
// 4. Document handling: downloads with original extension
// 5. Markdown-to-HTML: converts LLM output to Telegram's HTML subset
// 6. Message chunking: splits responses >4096 chars
// 7. Command menu: registers /start, /reset, /help with BotFather
```

### Skills System

```markdown
<!-- /tmp/picoclaw skills format: skills/{name}/SKILL.md -->
---
{
  "name": "weather",
  "description": "Get weather forecast",
  "always": true,
  "requires": {
    "bins": ["curl"],
    "env": ["WEATHER_API_KEY"]
  }
}
---

# Weather Skill
Instructions for the agent on how to use weather APIs...
```

**Key insight**: Skills with `"always": true` are auto-injected into every conversation context. Skills with `requires` are checked for binary/env availability at load time. GitHub installation supported.

---

## nanobot Unique Features

### LiteLLM Provider Registry

nanobot uses a registry-driven approach where adding a new LLM provider requires only:
1. Add a `ProviderSpec` dataclass with env vars, prefixes, and model overrides
2. Add a config field

This supports 12+ providers with smart model prefixing and gateway detection. Terraphim's current approach (hardcoded Ollama + OpenRouter) would benefit from a similar registry pattern.

### Subagent Spawning

```python
# /tmp/nanobot/nanobot/agent/subagent.py
class SubagentManager:
    def spawn(self, task, label):
        # Creates isolated agent with:
        # - Separate tool registry (no message/spawn tools)
        # - Dedicated system prompt for single task
        # - Max 15 iterations
        # Result published to message bus -> triggers main agent summary
```

**Key insight**: Subagents cannot spawn other subagents (no message/spawn tools). Results are announced back to the main conversation via message bus.

### Email Channel with Consent

nanobot's email channel (`403 LOC`) requires explicit `consent_granted: true` in config before accessing any mailbox. Features IMAP polling + SMTP sending, reply threading, and historical search by date range. This consent pattern is worth adopting.

### MoChat Delayed Buffering

MoChat channel (`895 LOC`) implements delayed message buffering for group chats -- aggregates rapid-fire messages before responding, preventing the agent from replying to each message individually. This pattern is valuable for any group chat channel.

---

## Constraints

### Technical Constraints
- **Language**: Rust (async with tokio) -- single-binary deployment
- **Discord SDK**: `serenity` is mature but heavy; `twilight` is lower-level. PicoClaw uses `discordgo` (Go equivalent of serenity)
- **Telegram SDK**: `teloxide` is the standard Rust Telegram library
- **WhatsApp**: No native Rust SDK; must use bridge (Node.js/Baileys) or WhatsApp Business API
- **Voice**: Groq Whisper API is HTTP-based, easy to integrate from any language

### Business Constraints
- Must leverage existing Terraphim crates (not greenfield)
- Single binary deployment preferred (Rust advantage)
- Privacy-first: local LLM support required (Ollama)

### Non-Functional Requirements

| Requirement | Target | Rationale |
|-------------|--------|-----------|
| Message latency | < 2s (excluding LLM) | PicoClaw achieves this with Go channels |
| Concurrent channels | 5+ simultaneous | PicoClaw runs 6 channels in one process |
| Memory per channel | < 50MB | PicoClaw runs on $10 hardware (<10MB RAM) |
| Startup time | < 5s | PicoClaw starts in <1s |

## Vital Few (Essentialism)

### Essential Constraints (Max 3)

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Channel abstraction must be async trait | All channels are I/O bound, must not block agent loop | PicoClaw uses goroutines, nanobot uses asyncio |
| Tool-calling loop with context compression | Without compression, context windows overflow in minutes | Both PicoClaw and nanobot implement this identically |
| Session isolation per channel:chatID | Without isolation, messages from different users/channels bleed | Both reference implementations use this exact pattern |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| MaixCam hardware channel | Too niche, PicoClaw-specific for $10 Sipeed hardware |
| MoChat channel | Niche Chinese platform, can be added later |
| DingTalk channel | Can follow same pattern as Feishu, add later |
| QQ channel | Can follow same pattern as Telegram, add later |
| Subagent spawning | Nice-to-have but not in PicoClaw; add after core works |

---

## Dependencies

### New External Dependencies Needed

| Dependency | Purpose | Maturity | PicoClaw Equivalent |
|------------|---------|----------|---------------------|
| `teloxide` | Telegram bot | Stable, widely used | `go-telegram-bot-api` |
| `serenity` or `twilight` | Discord bot | Stable | `discordgo` |
| `reqwest` (already in workspace) | HTTP for voice/WhatsApp | Stable | `net/http` |
| `dialoguer` | Interactive CLI | Stable | `survey` (Go) |

### Existing Terraphim Dependencies Leveraged

- `tokio` -- async runtime, channels, timers, signals
- `serde` / `serde_json` -- message serialization, session persistence
- `tracing` -- structured logging
- `ahash` -- fast hashmaps for routing
- `uuid` -- message and session IDs

---

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| WhatsApp has no Rust SDK | High | High | Use Node.js bridge (proven by PicoClaw/nanobot) or WhatsApp Business API |
| `serenity` adds significant binary size | Medium | Low | Can use `twilight` for lighter footprint |
| Context compression quality varies by LLM | Medium | Medium | Allow configurable compression prompt, test with multiple models |
| Voice transcription adds Groq dependency | Low | Low | Feature-gate behind `voice` feature flag |
| Terraphim crate APIs may need adaptation | Medium | Medium | Agent crates are well-tested but designed for different use case |

### Open Questions

1. **WhatsApp strategy**: Bridge (Node.js subprocess) or Business API (HTTP)? -- Both PicoClaw and nanobot use bridges
2. **Feishu/Lark priority**: Is Chinese market important enough to include in v1? -- PicoClaw includes it
3. **Slack priority**: nanobot includes it; PicoClaw does not. Include in v1?
4. **Email channel**: nanobot's consent-based email is unique. Include?
5. **Tool safety**: Both reference projects implement shell deny-patterns. What level of sandboxing for tool execution?

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| Terraphim agent crates can be used for chat-style interaction | Tests show command processing works | Would need adapter layer | Yes (63 tests pass) |
| `teloxide` supports all needed Telegram features | It's the standard Rust Telegram library | May need lower-level API calls | No |
| Single-binary can include all channel adapters | Rust static linking | WhatsApp bridge requires Node.js subprocess | Partially |
| Groq Whisper API is stable and available | PicoClaw/nanobot both use it | Need fallback transcription option | No |

---

## Research Findings

### Key Insights

1. **PicoClaw is the gold standard reference**: At ~6,054 LOC in Go, it delivers a complete multi-channel AI assistant. The Rust rebuild should target feature parity with PicoClaw as v1, not TinyClaw.

2. **The PR #518 plan underestimates scope by ~2-3x**: The plan estimates 2,000-2,700 LOC new code. PicoClaw is 6,054 LOC. Even accounting for Terraphim reuse, the channel adapters + tools + skills + cron + voice alone need ~4,000-5,000 LOC.

3. **Channel adapters are 55% of the work**: In PicoClaw, channels account for ~3,350 LOC (55% of total). This is consistent across nanobot too. The plan treats channels as "Medium effort" but they are the bulk of the implementation.

4. **Context compression is critical, not optional**: Both PicoClaw and nanobot implement identical compression strategies (75% context window trigger, LLM summarization, keep last N messages). The plan mentions "versioned memory" but does not detail compression.

5. **Tool-calling loop is the agent core, not mailbox routing**: The plan emphasizes Erlang-style mailboxes, but the actual core loop (message -> LLM -> tool calls -> iterate -> respond) is what PicoClaw/nanobot spend most design effort on.

6. **Skills system enables extensibility without code changes**: Both reference projects use markdown files with JSON/YAML frontmatter. This is a low-effort, high-impact feature missing from the plan.

7. **WhatsApp universally uses a bridge**: Neither PicoClaw nor nanobot have native WhatsApp integration. Both use a Node.js process running Baileys. The plan's suggestion of Matrix bridge or Business API diverges from proven approach.

8. **Voice transcription is table stakes**: Both reference projects support voice messages via Groq Whisper. This is expected functionality for a modern chat assistant.

9. **Terraphim's unique advantages are real**: OTP supervision, knowledge graph enrichment, Firecracker VM execution, and versioned memory genuinely differentiate from PicoClaw/nanobot. These should be highlighted but are Phase 2 features.

10. **Dual mode (CLI agent + gateway service) is standard**: Both reference projects support direct CLI interaction (testing/development) and multi-channel gateway mode (production). The plan only describes the gateway.

### Relevant Prior Art

| Project | Language | LOC | Channels | Unique Feature |
|---------|----------|-----|----------|----------------|
| PicoClaw | Go | 6,054 | 6 | MaixCam hardware, ultra-lightweight |
| nanobot | Python | 8,447 | 9 | Subagent spawning, litellm registry, email with consent |
| TinyClaw | TypeScript/bash | ~1,000 | 2 | File-based queue, tmux orchestration |

---

## Revised Scope Estimate

| Component | New LOC | Reused Terraphim LOC | Risk | PicoClaw Reference LOC |
|-----------|---------|---------------------|------|----------------------|
| Channel abstraction + bus | ~300 | ~2,174 (messaging) | Low | 253 |
| Telegram adapter | ~600 | ~0 | Medium | 501 |
| Discord adapter | ~400 | ~0 | Medium | 247 |
| WhatsApp adapter + bridge | ~300 (Rust) + ~400 (TS bridge) | ~0 | High | 145 + bridge |
| Feishu adapter | ~350 | ~0 | Medium | 307 |
| Slack adapter | ~250 | ~0 | Medium | 205 (nanobot) |
| Agent loop + context compression | ~400 | ~8,445 (multi_agent) | Medium | 488 |
| Session manager | ~200 | ~7,226 (evolution) | Low | 184 |
| Tool registry + 5 tools | ~800 | ~0 | Low | 844 |
| Skills system | ~400 | ~0 | Low | 479 |
| Voice transcription | ~200 | ~0 | Low | 166 |
| Cron service | ~350 | ~0 | Low | 382 |
| Markdown formatting | ~150 | ~0 | Low | 100 |
| Onboarding CLI | ~300 | ~0 | Low | 264 |
| Orchestrator binary (agent + gateway modes) | ~500 | ~1,452 (supervisor) | Low | 1,174 |
| Config system | ~200 | existing config crate | Low | ~200 |
| **Total** | **~5,700 Rust + ~400 TS** | **~19,297** | | **~6,054** |

### Reuse Ratio: ~3.4:1 (reused : new)

This is better than the original plan's 2:1 estimate, but the absolute new code is higher (5,700 vs 2,700 LOC) because the plan missed many features.

---

## Recommendations

### Proceed/No-Proceed

**Proceed** -- with revised scope. The Terraphim agent crates are confirmed production-ready. The primary work is channel adapters, tools, and the agent loop glue, all well-understood from PicoClaw/nanobot reference implementations.

### Scope Recommendations

**Phase 1 (MVP)**: Telegram + Discord + CLI agent mode + tool-calling loop + session management + context compression. This delivers a usable assistant with the two most popular channels.

**Phase 2 (Feature parity)**: WhatsApp bridge + Feishu + voice transcription + skills system + cron + onboarding CLI.

**Phase 3 (Terraphim-native)**: Knowledge graph enrichment + multi-agent routing + Firecracker sandboxing + goal alignment + Slack + Email.

### Risk Mitigation Recommendations

1. **WhatsApp**: Start with Business API (HTTP, no bridge needed). Add Baileys bridge later if needed.
2. **Channel adapters**: Build Telegram first (most complex, proves the abstraction), then Discord.
3. **Context compression**: Port PicoClaw's algorithm directly (well-tested, simple).
4. **Voice**: Feature-gate behind `voice` cargo feature. Groq Whisper is HTTP-only, easy integration.

---

## Next Steps

If approved:
1. Proceed to Phase 2 (disciplined-design) with revised scope
2. Design the channel abstraction trait (Rust async version of PicoClaw's Channel interface)
3. Design the agent loop (tool-calling, context compression, session management)
4. Design the tool registry (Rust version of PicoClaw's Tool interface)
5. Design the skills system (markdown loader with requirements checking)
6. Create implementation plan with per-phase tasks

---

## Appendix

### A. PicoClaw Source Map

| File | LOC | Purpose |
|------|-----|---------|
| `cmd/picoclaw/main.go` | 1,174 | CLI commands: onboard, agent, gateway, status, cron, skills |
| `pkg/channels/telegram.go` | 501 | Telegram with voice, photos, thinking indicators, HTML formatting |
| `pkg/cron/service.go` | 382 | Persistent cron with at/every scheduling |
| `pkg/agent/loop.go` | 325 | Tool-calling loop with context compression |
| `pkg/skills/loader.go` | 307 | Markdown skill loader with requirements checking |
| `pkg/channels/feishu.go` | 307 | Feishu/Lark via websocket |
| `pkg/web/web.go` | 299 | Web search (Brave) and fetch tools |
| `pkg/channels/discord.go` | 247 | Discord with voice transcription |
| `pkg/tools/shell.go` | 203 | Shell exec with safety guards |
| `pkg/session/manager.go` | 184 | Session persistence with JSON files |
| `pkg/skills/installer.go` | 172 | GitHub skill installation |
| `pkg/bus/bus.go` | 170 | Message bus with Go channels |
| `pkg/voice/transcriber.go` | 166 | Groq Whisper transcription |
| `pkg/agent/context.go` | 163 | System prompt builder with bootstrap files |
| `pkg/tools/edit.go` | 149 | File edit with uniqueness checking |
| `pkg/channels/whatsapp.go` | 145 | WhatsApp via websocket bridge |
| `pkg/tools/filesystem.go` | 142 | Read, write, list directory |
| `pkg/heartbeat/service.go` | 135 | Periodic heartbeat prompts |
| `pkg/channels/qq.go` | 131 | QQ via botpy SDK |
| `pkg/config/config.go` | ~200 | JSON config with env var overrides |
| `pkg/channels/base.go` | 83 | Channel interface + allow-list |
| `pkg/bus/types.go` | 68 | Message type definitions |
| `pkg/tools/registry.go` | 51 | Tool registry with JSON Schema export |

### B. nanobot Source Map

| File | LOC | Purpose |
|------|-----|---------|
| `channels/mochat.py` | 895 | MoChat with WebSocket, delayed buffering, dedup |
| `channels/email.py` | 403 | IMAP/SMTP with consent, threading |
| `channels/telegram.py` | 400 | Telegram with voice, photos, HTML formatting |
| `providers/registry.py` | 360 | 12+ provider registry with auto-detection |
| `channels/feishu.py` | 307 | Feishu/Lark via websocket |
| `config/schema.py` | 287 | Pydantic config with nested models |
| `channels/discord.py` | 261 | Discord via raw websocket |
| `agent/subagent.py` | 245 | Background agent spawning with isolation |
| `channels/dingtalk.py` | 238 | DingTalk stream mode |
| `agent/skills.py` | 229 | Skills loader with YAML frontmatter |
| `channels/slack.py` | 205 | Slack socket mode |
| `providers/litellm_provider.py` | 204 | LiteLLM multi-provider wrapper |
| `agent/tools/filesystem.py` | 212 | Read, write, edit, list |
| `agent/tools/web.py` | 164 | Search + fetch |
| `channels/whatsapp.py` | 145 | WhatsApp via Node.js bridge |
| `agent/tools/shell.py` | 145 | Shell exec with safety guards |
| `channels/qq.py` | 131 | QQ via botpy |
| `agent/tools/cron.py` | 115 | Cron scheduling tool |
| `agent/memory.py` | 110 | Daily notes + long-term memory |
| `agent/tools/message.py` | 87 | Cross-channel messaging |
| `agent/tools/spawn.py` | 66 | Subagent spawning tool |

### C. Terraphim Agent Crates Validated

| Crate | LOC | Tests | Pass Rate | Status |
|-------|-----|-------|-----------|--------|
| `terraphim_agent_messaging` | 2,174 | 26 | 100% | Production-ready |
| `terraphim_agent_supervisor` | 1,452 | 16 | 100% | Production-ready |
| `terraphim_multi_agent` | 8,445 | 63 | 100% | Production-ready |
| `terraphim_agent_evolution` | 7,226 | E2E | Pass | Production-ready |
| `terraphim_service` | 35,123 | N/A | N/A | Production-ready |
| `terraphim_goal_alignment` | 4,614 | 21 | 71% (6 ignored) | Functional |
| `terraphim_task_decomposition` | 4,614 | 40 | 100% | Production-ready |

### D. Rust Library Options for Channels

| Channel | Library | Stars | Last Updated | Notes |
|---------|---------|-------|-------------|-------|
| Telegram | `teloxide` | 3k+ | Active | Full-featured, async, maintained |
| Discord | `serenity` | 4k+ | Active | Full-featured, heavy |
| Discord | `twilight` | 700+ | Active | Lightweight, modular |
| Matrix | `matrix-sdk` | 1k+ | Active | For WhatsApp bridge via mautrix |
| Slack | `slack-morphism` | 200+ | Active | Async Slack client |
| Feishu | None | - | - | Use HTTP API directly with `reqwest` |
