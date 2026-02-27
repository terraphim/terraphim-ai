# TinyClaw Architecture

Multi-channel AI assistant architecture with intelligent LLM routing.

**Version:** 1.0
**Last Updated:** 2026-02-26

---

## Table of Contents

1. [Overview](#overview)
2. [System Architecture](#system-architecture)
3. [Components](#components)
4. [Data Flow](#data-flow)
5. [LLM Routing](#llm-routing)
6. [OAuth Integration](#oauth-integration)
7. [Channel Adapters](#channel-adapters)
8. [Session Management](#session-management)
9. [Tool System](#tool-system)
10. [Configuration](#configuration)

---

## Overview

TinyClaw is a multi-channel AI assistant that connects to Telegram, Discord, and other messaging platforms. It routes user messages through an intelligent agent loop that leverages tool-calling capabilities and context management.

### Key Features

- **Multi-channel support**: Telegram, Discord, CLI
- **Hybrid LLM routing**: Combines local (Ollama) and cloud (Codex, Claude, Gemini) LLM providers
- **Tool-calling agent loop**: Autonomous task execution with confidence scoring
- **Session persistence**: JSONL-based conversation history
- **OAuth integration**: Automatic token management from Codex CLI

---

## System Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           TinyClaw Gateway                               │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐   │
│  │   Telegram   │  │   Discord   │  │    CLI      │  │   Matrix    │   │
│  │  Channel     │  │  Channel    │  │   Channel   │  │ (disabled)  │   │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘   │
│         │                 │                 │                 │          │
│         └─────────────────┴────────┬────────┴─────────────────┘          │
│                                    │                                     │
│                          ┌─────────▼─────────┐                          │
│                          │   Message Bus     │                          │
│                          │  (Async Channels) │                          │
│                          └─────────┬─────────┘                          │
│                                    │                                     │
│                          ┌─────────▼─────────┐                          │
│                          │  Agent Loop       │                          │
│                          │  (Tool Calling)   │                          │
│                          └─────────┬─────────┘                          │
│                                    │                                     │
│         ┌──────────────────────────┼──────────────────────────┐        │
│         │                          │                          │        │
│  ┌──────▼──────┐          ┌───────▼───────┐          ┌──────▼──────┐  │
│  │   Proxy     │          │   Direct LLM   │          │   Tools     │  │
│  │   Client    │          │    Client      │          │  Registry   │  │
│  └──────┬──────┘          └───────┬───────┘          └─────────────┘  │
│         │                          │                                     │
└─────────│──────────────────────────│─────────────────────────────────────┘
          │                          │
          ▼                          ▼
┌─────────────────────┐     ┌─────────────────────┐
│  terraphim-llm-    │     │     Ollama          │
│  proxy              │     │  (localhost:11434)  │
│  (localhost:3456)   │     └─────────────────────┘
│         │
│         ├───────────────────┬───────────────────┐
│         │                   │                   │
│         ▼                   ▼                   ▼
│  ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  │   Codex     │     │   Claude    │     │   Gemini    │
│  │  (OAuth)   │     │  (OAuth)    │     │  (OAuth)    │
│  └─────────────┘     └─────────────┘     └─────────────┘
└─────────────────────────────────────────────────────────────────┘
```

---

## Components

### 1. Channel Adapters

| Channel | Protocol | Features |
|---------|----------|----------|
| Telegram | Polling/Webhook | Markdown, stickers, callbacks |
| Discord | WebSocket | Embeds, components |
| CLI | Stdin/Stdout | Interactive mode |
| Matrix | Client-Server API | Disabled in channel wiring (sqlite dependency conflict) |

**Location:** `src/channels/`

### 2. Message Bus

Async channel-based pub/sub system for inter-component communication.

- **Inbound queue**: Messages from channels to agent
- **Outbound queue**: Responses from agent to channels
- **Event bus**: System events (typing, errors)

**Location:** `src/bus.rs`

### 3. Agent Loop

Tool-calling agent with:
- Context management
- Prompt sanitization
- Confidence scoring
- Tool execution

**Location:** `src/agent/agent_loop.rs`

### 4. LLM Clients

| Client | Use Case | Provider |
|--------|----------|----------|
| ProxyClient | Tool calls, complex reasoning | terraphim-llm-proxy |
| DirectLlmClient | Simple QA, compression | Ollama, OpenAI, Anthropic |

**Location:** `src/agent/`

### 5. Tool Registry

Registered tools for agent execution:
- `filesystem`: File operations
- `shell`: Command execution
- `edit`: Code editing
- `web_search`: Internet search
- `web_fetch`: URL content retrieval
- `voice_transcribe`: Audio transcription stub (placeholder response)

**Location:** `src/tools/`

---

## Data Flow

### 1. Inbound Message Flow

```
Telegram User
     │
     ▼
┌────────────────┐
│ Telegram       │  1. Receive update
│ Channel       │  2. Parse message
└────┬───────────┘  3. Validate sender
     │
     ▼
┌────────────────┐
│ Message Bus    │  4. Create InboundMessage
│ (inbound)     │  5. Publish to agent
└────┬───────────┘
     │
     ▼
┌────────────────┐
│ Agent Loop     │  6. Load/create session
│                │  7. Add to context
└────┬───────────┘
     │
     ▼
```

### 2. LLM Request Flow

```
Agent Loop
     │
     ▼
┌────────────────┐
│ HybridLlmRouter│  1. Check proxy availability
│                │  2. Determine routing
└────┬───────────┘
     │
     ├──► Proxy (tool calls)
     │         │
     │         ▼
     │    ┌────────────────────┐
     │    │ terraphim-llm-    │
     │    │ proxy             │
     │    └─────────┬──────────┘
     │              │
     │    ┌─────────▼─────────┐
     │    │ OAuth/Token Mgmt  │
     │    └─────────┬──────────┘
     │              │
     │    ┌─────────▼─────────┐
     │    │ Provider Router  │
     │    │ (Codex/Claude/    │
     │    │  Gemini/etc)      │
     │    └─────────┬──────────┘
     │              │
     └──────────────┘
     │
     ▼
Direct (simple QA)
     │
     ▼
┌────────────────┐
│ DirectLlmClient│
│ (Ollama)       │
└────────────────┘
```

### 3. Tool Execution Flow

```
LLM Response
(contains tool_call)
     │
     ▼
┌────────────────┐
│ Tool Registry │  1. Parse tool_call
│                │  2. Lookup tool
└────┬───────────┘
     │
     ▼
┌────────────────┐
│ Tool Executor │  3. Validate params
│                │  4. Execute tool
└────┬───────────┘  5. Capture output
     │              6. Format result
     ▼
┌────────────────┐
│ Context        │  7. Add result to
│ Manager        │     conversation
└────┬───────────┘
     │
     ▼
┌────────────────┐
│ LLM (loop)    │  8. Continue or respond
└────────────────┘
```

---

## LLM Routing

### Hybrid Architecture

TinyClaw uses a hybrid LLM routing strategy:

```
┌─────────────────────────────────────────────────────────────┐
│                     Request Type                            │
└────────────────────────────┬────────────────────────────────┘
                             │
         ┌───────────────────┴───────────────────┐
         │                                       │
         ▼                                       ▼
┌─────────────────────┐               ┌─────────────────────┐
│   Tool-Calling     │               │   Simple QA/        │
│   Complex Tasks     │               │   Compression       │
└─────────┬───────────┘               └─────────┬───────────┘
          │                                     │
          ▼                                     ▼
┌─────────────────────┐               ┌─────────────────────┐
│ Proxy Client       │               │ Direct LLM Client  │
│ (terraphim-llm-   │               │ (Ollama)            │
│  proxy)            │               │                     │
│                    │               │                     │
│ - 6-phase routing │               │ - Fast             │
│ - Tool format conv│               │ - Cheap            │
│ - 9+ providers    │               │ - Local            │
└─────────┬─────────┘               └─────────┬─────────────┘
          │                                   │
          ▼                                   ▼
┌─────────────────────────────────────────────────────────────┐
│  terraphim-llm-proxy                                      │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐    │
│  │   Codex      │  │   Claude     │  │   Gemini     │    │
│  │  (OAuth)     │  │  (OAuth)    │  │  (OAuth)    │    │
│  └──────────────┘  └──────────────┘  └──────────────┘    │
└─────────────────────────────────────────────────────────────┘
```

### Proxy Routing Scenarios

| Scenario | Default Route | Description |
|----------|--------------|-------------|
| `default` | `openai-codex,gpt-5.2-codex` | Standard requests |
| `think` | `openai-codex,gpt-5.2` | Complex reasoning |
| `background` | `openai-codex,gpt-5.2-codex` | Low-priority tasks |
| `long_context` | `openai-codex,gpt-5.2` | Large inputs (>60K tokens) |
| `web_search` | `openai-codex,gpt-5.2` | With web search tool |

---

## OAuth Integration

### Codex CLI OAuth Flow

```
┌──────────────────────────────────────────────────────────────────┐
│                      Startup Sequence                            │
└────────────────────────────────┬─────────────────────────────────┘
                                 │
                                 ▼
┌──────────────────────────────────────────────────────────────────┐
│  terraphim-llm-proxy                                        │
│                                                               │
│  1. Check ~/.codex/auth.json                                │
│  2. Parse tokens (support new nested format)                │
│  3. Import to internal token store                          │
│  4. Ready for API calls                                     │
└────────────────────────────────┬─────────────────────────────────┘
                                 │
                                 ▼
┌──────────────────────────────────────────────────────────────────┐
│  Token Storage                                               │
│                                                               │
│  ~/.terraphim-llm-proxy/tokens/openai/                       │
│  └── google-oauth2|113845183208630617447.json                │
│      {                                                       │
│        "access_token": "eyJ...",                            │
│        "refresh_token": "rt_...",                           │
│        "expires_at": "2026-03-08T10:10:16Z",               │
│        "provider": "openai",                               │
│        "account_id": "google-oauth2|..."                   │
│      }                                                       │
└───────────────────────────────────────────────────────────────┘
```

### Supported OAuth Providers

| Provider | Token Source | Features |
|----------|--------------|----------|
| OpenAI Codex | `~/.codex/auth.json` | Auto-import on startup |
| Claude (Anthropic) | Browser OAuth | Manual flow |
| Gemini (Google) | Browser OAuth | Manual flow |
| GitHub Copilot | Device flow | Automatic |

---

## Channel Adapters

### Telegram Channel

```
┌─────────────────────────────────────────────────────────────────┐
│                      Telegram Bot                              │
│                                                                 │
│  ┌─────────────┐      ┌─────────────┐      ┌─────────────┐   │
│  │  Polling   │      │   Webhook   │      │  Callbacks  │   │
│  │  (default) │      │ (production)│      │  (buttons)  │   │
│  └──────┬──────┘      └──────┬──────┘      └──────┬──────┘   │
│         │                    │                    │          │
│         └────────────────────┼────────────────────┘          │
│                                │                               │
│                                ▼                               │
│                     ┌─────────────────────┐                   │
│                     │  Message Parser     │                   │
│                     │  - Text            │                   │
│                     │  - Commands (/xxx) │                   │
│                     │  - Stickers        │                   │
│                     │  - Photos           │                   │
│                     └─────────────────────┘                   │
└────────────────────────────────────────────────────────────────┘
```

### Message Transformation

| Telegram Format | Internal Format | Notes |
|-----------------|-----------------|-------|
| `Markdown` | `Markdown` | Telegram-specific HTML |
| `/command` | `Message(role=user)` | `/help` implemented; `/reset` returns confirmation only; `/role ...` returns not implemented |
| `CallbackQuery` | `Message(role=callback)` | Button presses |

---

## Session Management

### Session Storage

```
/tmp/tinyclaw/sessions/
├── 2026/
│   └── 02/
│       └── 26/
│           ├── telegram_287867183_001.jsonl
│           ├── telegram_287867183_002.jsonl
│           └── discord_123456789.jsonl
```

### Session Format (JSONL)

```jsonl
{"timestamp":"2026-02-26T10:00:00Z","role":"user","content":"Hello"}
{"timestamp":"2026-02-26T10:00:01Z","role":"assistant","content":"Hi! How can I help?"}
{"timestamp":"2026-02-26T10:00:05Z","role":"tool","name":"web_search","input":{"query":"..."},"output":"..."}
```

### Session Lifecycle

```
1. New Message
       │
       ▼
2. Create/Load Session
   (session_key: "telegram:chat_id")
       │
       ▼
3. Load History from JSONL
       │
       ▼
4. Add to Context
   (with compression if needed)
       │
       ▼
5. Process Message
       │
       ▼
6. Save to JSONL
```

---

## Tool System

### Available Tools

| Tool | Capabilities | Confidence Guard |
|------|--------------|------------------|
| `filesystem` | Read, write, list files | High |
| `shell` | Execute commands | Medium (allowlist) |
| `edit` | Modify code | High |
| `web_search` | Internet search | Low |
| `web_fetch` | Get URL content | Medium |
| `voice_transcribe` | Audio to text (stub; placeholder output) | High |

### Tool Execution Flow

```
LLM Response (tool_call)
     │
     ▼
┌─────────────────────────┐
│ Tool Registry          │
│ - Validate name        │
│ - Parse parameters     │
└───────────┬─────────────┘
            │
            ▼
┌─────────────────────────┐
│ Confidence Checker      │
│ - Pattern matching     │
│ - Dangerous command    │
│   detection            │
└───────────┬─────────────┘
            │
     ┌──────┴──────┐
     │              │
     ▼              ▼
┌─────────┐   ┌─────────┐
│ Blocked │   │ Execute │
│         │   │         │
└─────────┘   └────┬────┘
                   │
                   ▼
            ┌─────────────┐
            │ Tool        │
            │ Executor    │
            └──────┬──────┘
                   │
                   ▼
            ┌─────────────┐
            │ Result      │
            │ Formatter   │
            └─────────────┘
```

---

## Configuration

### Configuration Hierarchy

```
1. Default values (hardcoded)
       │
       ▼
2. Config file (~/.config/terraphim/tinyclaw.toml)
       │
       ▼
3. Environment variables (${VAR})
       │
       ▼
4. Command-line flags
```

### Key Configuration Sections

```toml
[agent]
workspace = "/tmp/tinyclaw"

[llm.proxy]
base_url = "http://localhost:3456"

[llm.direct]
provider = "ollama"
model = "llama3.2:3b"
base_url = "http://localhost:11434"

[channels.telegram]
token = "${TELEGRAM_BOT_TOKEN}"
allow_from = ["123456789"]

[skills]
skills_dir = "~/.config/terraphim/skills"
```

---

## Deployment

### Development Mode

```bash
# Terminal 1: Start proxy
cd terraphim-llm-proxy
cargo run --release -- --config config.toml

# Terminal 2: Start TinyClaw
cd terraphim-ai
cargo run -p terraphim_tinyclaw --release -- gateway
```

### Production Mode (Docker)

```yaml
services:
  proxy:
    image: terraphim/llm-proxy:latest
    ports:
      - "3456:3456"
    volumes:
      - ./proxy.toml:/etc/terraphim/proxy.toml
      - ~/.codex:/root/.codex:ro

  tinyclaw:
    image: terraphim/tinyclaw:latest
    ports:
      - "8080:8080"
    environment:
      - TELEGRAM_BOT_TOKEN=${TELEGRAM_BOT_TOKEN}
    volumes:
      - ./tinyclaw.toml:/etc/terraphim/tinyclaw.toml
    depends_on:
      - proxy
```

### systemd Service

```ini
[Unit]
Description=TinyClaw Gateway
After=network.target

[Service]
Type=simple
Environment="TELEGRAM_BOT_TOKEN=xxx"
ExecStart=/usr/local/bin/terraphim-tinyclaw gateway
Restart=always

[Install]
WantedBy=multi-user.target
```

---

## Monitoring

### Health Check

```bash
curl http://localhost:3456/health
```

Response:
```json
{
  "status": "healthy",
  "version": "0.1.6",
  "timestamp": "2026-02-26T10:00:00Z",
  "checks": {
    "database": "healthy",
    "providers": "healthy",
    "sessions": "healthy",
    "metrics": "healthy"
  }
}
```

### Metrics

```bash
curl http://localhost:3456/metrics
```

---

## Related Documentation

- [Codex Quickstart](./docs/TINYCLAW_CODEX_QUICKSTART.md)
- [Telegram Setup](./docs/TELEGRAM_QUICKSTART.md)
- [Skills System](./examples/skills/README.md)
- [terraphim-llm-proxy](../terraphim-llm-proxy/docs/OAUTH_GUIDE.md)
