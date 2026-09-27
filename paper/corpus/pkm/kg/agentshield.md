---
title: AgentShield
synonyms:
  - ecc-agentshield
  - agent-shield
tags: security, claude-code, scanner, ecc
---

# AgentShield

Security scanner shipped with Everything Claude Code (ECC). 1,282 tests covering Claude Code configuration surface:

- `CLAUDE.md`: hardcoded secrets, injection vectors
- `settings.json`: misconfigured permissions
- MCP server configs: 25+ known CVEs
- Hooks: injection analysis
- Agents: prompt injection, privilege escalation
- Skills: supply chain verification

Run: `npx ecc-agentshield scan` (no install), `: fix` (auto-fix safe issues), `: opus : stream` (three Opus 4.6 agents in Attacker/Defender/Auditor red-team pipeline).

Output: letter grade plus severity buckets (Critical/High/Medium/Low) with file:line and fix hint.

Candidate for adoption in CTO Executive System CI (gates against `.claude/`, MCP configs, terraphim-skills installs). See `plans/ecc-agentshield-leverage.md`.
