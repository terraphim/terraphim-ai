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

route:: kimi, kimi-for-coding/k2p6
action:: /home/alex/.bun/bin/opencode run -m {{ model }} --format json "{{ prompt }}"

# Z.AI Coding Plan healthy via pi-rust; broken via opencode 1.14.48
# (opencode emits only step_start, no text). Investigation: 2026-05-23.
# Route through pi-rust until opencode integration is fixed upstream.
route:: zai-coding-plan, glm-5.1
is_free:: true
action:: /home/alex/.local/bin/pi-rust --provider zai-coding-plan --model {{ model }} -p "{{ prompt }}"
