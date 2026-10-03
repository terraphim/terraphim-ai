# CAAM - Coding Agent Account Manager

## Overview

**caam** (Coding Agent Account Manager) is a Go-based CLI tool by Dicklesworthstone that enables sub-100ms credential rotation for AI coding assistants (Claude Code, Codex, Gemini CLI).

Repository: https://github.com/Dicklesworthstone/coding_agent_account_manager

## Problem It Solves

Fixed-cost AI subscriptions (Claude Max $200/mo, GPT Pro $200/mo, Gemini Ultra $275/mo) have usage limits. When hit mid-flow, the official OAuth switch takes 30-60 seconds:

```
/login → browser opens → sign out → sign in → authorise → wait → return
```

caam reduces this to ~50ms by swapping auth files directly.

## How It Works

OAuth tokens are bearer tokens stored in local files. caam manages these through a vault system:

```
~/.local/share/caam/vault/
├── claude/
│   ├── alice@gmail.com/
│   │   ├── .claude.json        # Backed up auth
│   │   ├── auth.json           # From ~/.config/claude-code/
│   │   └── meta.json           # Timestamp, original paths
│   └── bob@gmail.com/
├── codex/
│   └── work@company.com/
└── gemini/
    └── personal@gmail.com/
```

**Core mechanism**: `cp` with intelligence. Back up auth files → restore instantly when switching.

## Supported Tools

| Tool | Auth Location | Subscription |
|------|--------------|-------------|
| Claude Code | `~/.claude.json` + `~/.config/claude-code/auth.json` | Claude Max ($200/mo) |
| Codex CLI | `~/.codex/auth.json` | GPT Pro ($200/mo) |
| Gemini CLI | `~/.gemini/settings.json` | Gemini Ultra ($275/mo) |

## Rotation Algorithms

### Smart (Default)
Multi-factor scoring considers:
- **Cooldown state** - Profiles in cooldown are excluded
- **Health status** - Prefers healthy profiles (token validity >1h)
- **Recency** - Avoids profiles used in last 30 minutes
- **Plan type** - Slight preference for higher-tier plans
- **Random jitter** - Breaks ties unpredictably

### Round Robin
Sequential rotation through profiles, skipping cooldowns. Predictable, even distribution.

### Random
Pure random selection among non-cooldown profiles.

## Key Commands

```bash
# Backup current auth
caam backup claude alice@gmail.com

# Switch instantly (<100ms)
caam activate claude bob@gmail.com

# Auto-select best profile
caam activate claude --auto

# Preview rotation choice
caam next claude

# Mark profile as rate-limited (60min cooldown)
caam cooldown set claude

# Wrap CLI with automatic failover
caam run claude -- "your prompt"

# Check status across all tools
caam status
```

## Cooldown Tracking

When an account hits a rate limit:
1. Mark it with `caam cooldown set`
2. Rotation algorithms skip it automatically
3. Uses database-backed tracking with configurable windows
4. Penalty system with exponential decay (20% reduction every 5 minutes)

## Profile Detection

`caam status` uses **SHA-256 content hashing**:
1. Hash current auth files
2. Compare against all vault profiles
3. Match = active profile

This detects profiles even after manual switches, reboots, etc.

## Two Operating Modes

### Vault Profiles (Simple)
- Swap auth files in place
- One account active at a time per tool
- Instant switching

### Isolated Profiles (Advanced)
- Full directory isolation with pseudo-HOME
- Run multiple accounts simultaneously
- Each gets own `$HOME` and `$CODEX_HOME`

## Health Scoring

Visual indicators show profile state:

| Icon | Status | Meaning |
|------|--------|---------|
| 🟢 | Healthy | Token valid >1h, no recent errors |
| 🟡 | Warning | Token expiring <1h, or minor issues |
| 🔴 | Critical | Token expired, repeated errors |
| ⚪ | Unknown | No health data |

## Integration with Terraphim

### Potential Integration Points

1. **Agent Account Management**
   - Terraphim agents could use caam to manage multiple AI subscriptions
   - Automatic failover when hitting usage limits
   - Transparent to the agent's workflow

2. **Profile Rotation in Multi-Agent Setups**
   - Use smart rotation to distribute work across accounts
   - Project-profile associations for contextual account selection
   - Prevent agents from fighting over the same account

3. **Session Continuity**
   - When Terraphim restores sessions via continuity loop
   - Restore the correct AI account context
   - Maintain account state across agent restarts

4. **Handoff Integration**
   - Include active caam profile in handoff YAML
   - Next agent continues with same account
   - `memory/handoffs/PENDING.yaml` could track account context

## Related Tools in Flywheel Ecosystem

- **ntm** - Multi-agent tmux orchestration (has rotation features)
- **coding_agent_account_manager** - Standalone credential manager
- **mcp_agent_mail** - Agent messaging (could notify about account switches)

## Configuration

```yaml
# ~/.caam/config.yaml
stealth:
  rotation:
    enabled: true
    algorithm: smart  # smart | round_robin | random
  cooldown:
    enabled: true
    default_duration: 60m

default_provider: claude
```

## References

- GitHub: https://github.com/Dicklesworthstone/coding_agent_account_manager
- Flywheel: https://agent-flywheel.com/
- Discord: https://discord.gg/gnCHsYDR25
- Install: `brew install dicklesworthstone/tap/caam`
