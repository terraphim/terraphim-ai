# Decision Tier

Analytical decisions over execution data: log triage, retrospective,
quality evaluation, conflict resolution between agent verdicts, post-merge
gate assessment. Lower priority than strategic planning but above
implementation -- requires reasoning, not just code.

Maps to ZDP phases: disciplined-verification and disciplined-validation
(retrospective and quality dimensions).

priority:: 65

synonyms:: analyse logs, log triage, root cause analysis, incident review
synonyms:: post-merge gate, merge verdict, conflict resolution
synonyms:: nightwatch retrospective, quality evaluation, KLS rating
synonyms:: spec validation gap analysis, decision audit
synonyms:: pattern detection, anomaly review, fleet health

trigger:: analytical decisions over execution data with session continuity

route:: openai-codex, gpt-5.5
action:: /home/alex/.local/bin/pi-rust --provider openai-codex --model {{ model }} -p "{{ prompt }}"

route:: openai, opencode/gpt-5.5
action:: /home/alex/.bun/bin/opencode run -m {{ model }} --format json "{{ prompt }}"

# Kimi K2.6 via pi-rust (faster, more reliable than opencode).
route:: kimi-for-coding, kimi-k2.6
action:: /home/alex/.local/bin/pi-rust --provider kimi-for-coding --model {{ model }} -p "{{ prompt }}"

# Kimi K2.6 via opencode (fallback).
route:: kimi, kimi-for-coding/k2p6
action:: /home/alex/.bun/bin/opencode run -m {{ model }} --format json "{{ prompt }}"

# Z.AI Coding Plan: GLM-5.2 for analytical decisions (free via subscription).
route:: zai-coding-plan, glm-5.2
is_free:: true
action:: /home/alex/.local/bin/pi-rust --provider zai-coding-plan --model {{ model }} -p "{{ prompt }}"

# GLM-5.1 as fallback.
route:: zai-coding-plan, glm-5.1
is_free:: true
action:: /home/alex/.local/bin/pi-rust --provider zai-coding-plan --model {{ model }} -p "{{ prompt }}"
