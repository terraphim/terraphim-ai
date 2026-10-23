## Model Weather Report — 2026-06-23T09:30:02.422798713Z

**4 tier(s) | 33 model(s) | live probe (15s timeout per model)**

### THINKING : Planning Tier

> deep reasoning -- strongest models for architecture, planning, decisions
> priority: 80

| Status | Provider | Model | CLI | Latency | Cost |
|--------|----------|-------|-----|---------|------|
| FAIR | anthropic | `opus` | claude | 11824ms | paid |
| FAIR | kimi-for-coding | `kimi-k2.6` | pi-rust | 6171ms | paid |
| FAIR | kimi | `kimi-for-coding/k2p6` | opencode | 14244ms | paid |
| FAIR | kimi-for-coding | `kimi-k2-thinking` | pi-rust | 5489ms | paid |
| FAIR | openai | `openai/gpt-5.4` | opencode | 10306ms | paid |
| FAIR | openai | `opencode/gpt-5.5` | opencode | 4502ms | paid |
| UNKNOWN | openai-codex | `gpt-5.5` | pi-rust | - | paid |
| STORMY | zai-coding-plan | `glm-5.2` | pi-rust | 15002ms | FREE |
| | | | | | *timeout after 15s* |
| STORMY | zai-coding-plan | `glm-5.1` | pi-rust | 15001ms | FREE |
| | | | | | *timeout after 15s* |

### THINKING : Decision Tier

> deep reasoning -- strongest models for architecture, planning, decisions
> priority: 65

| Status | Provider | Model | CLI | Latency | Cost |
|--------|----------|-------|-----|---------|------|
| UNKNOWN | openai-codex | `gpt-5.5` | pi-rust | - | paid |
| FAIR | openai | `opencode/gpt-5.5` | opencode | 4502ms | paid |
| FAIR | kimi-for-coding | `kimi-k2.6` | pi-rust | 6171ms | paid |
| FAIR | kimi | `kimi-for-coding/k2p6` | opencode | 14244ms | paid |
| STORMY | zai-coding-plan | `glm-5.2` | pi-rust | 15002ms | FREE |
| | | | | | *timeout after 15s* |
| STORMY | zai-coding-plan | `glm-5.1` | pi-rust | 15001ms | FREE |
| | | | | | *timeout after 15s* |

### WORKHORSE : Implementation Tier

> balanced -- mid-range models for implementation and review
> priority: 50

| Status | Provider | Model | CLI | Latency | Cost |
|--------|----------|-------|-----|---------|------|
| STORMY | zai-coding-plan | `zai-coding-plan/glm-5.2` | pi-rust | 15002ms | FREE |
| | | | | | *timeout after 15s* |
| STORMY | zai-coding-plan | `zai-coding-plan/glm-5.1` | pi-rust | 15001ms | FREE |
| | | | | | *timeout after 15s* |
| OFFLINE | anthropic | `sonnet` | claude | 2484ms | paid |
| | | | | | *exit exit status: 1: * |
| FAIR | kimi-for-coding | `kimi-k2.5` | pi-rust | 5107ms | paid |
| FAIR | kimi | `kimi-for-coding/k2p5` | opencode | 12761ms | paid |
| FAIR | openai | `openai/gpt-5.3-codex` | opencode | 3521ms | paid |
| STORMY | minimax | `minimax-coding-plan/MiniMax-M3` | opencode | 15001ms | paid |
| | | | | | *timeout after 15s* |
| STORMY | minimax-coding-plan | `MiniMax-M3` | pi-rust | 15000ms | paid |
| | | | | | *timeout after 15s* |
| FAIR | minimax | `minimax-coding-plan/MiniMax-M2.7-highspeed` | opencode | 10201ms | paid |
| FAIR | minimax-coding-plan | `MiniMax-M2.7-highspeed` | pi-rust | 3400ms | paid |

### FAST & CHEAP : Review Tier

> fast and cheap -- verification, validation, lightweight checks
> priority: 40

| Status | Provider | Model | CLI | Latency | Cost |
|--------|----------|-------|-----|---------|------|
| STORMY | zai-coding-plan | `zai-coding-plan/glm-5.2` | pi-rust | 15002ms | FREE |
| | | | | | *timeout after 15s* |
| STORMY | zai-coding-plan | `zai-coding-plan/glm-5.1` | pi-rust | 15001ms | FREE |
| | | | | | *timeout after 15s* |
| FAIR | anthropic | `haiku` | claude | 12457ms | paid |
| FAIR | kimi-for-coding | `kimi-k2.5` | pi-rust | 5107ms | paid |
| FAIR | kimi | `kimi-for-coding/k2p5` | opencode | 12761ms | paid |
| FAIR | openai | `openai/gpt-5.4-mini` | opencode | 9780ms | paid |
| STORMY | minimax | `minimax-coding-plan/MiniMax-M3` | opencode | 15001ms | paid |
| | | | | | *timeout after 15s* |
| FAIR | minimax | `minimax-coding-plan/MiniMax-M2.5` | opencode | 9690ms | FREE |

**Summary:** **19 available** (0 sunny, 19 fair, 0 cloudy) | 11 stormy | 1 offline | 2 unknown
