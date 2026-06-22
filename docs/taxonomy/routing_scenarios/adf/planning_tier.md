# Planning Tier

Strategic reasoning, architecture design, research, and high-level decisions.
Uses the strongest reasoning models. Any agent escalates here when task
requires planning, not just meta-coordinator.

Maps to ZDP phases: disciplined-research, disciplined-design.

priority:: 80

synonyms:: strategic planning, architecture design, system design
synonyms:: create a plan, design new architecture, roadmap planning
synonyms:: product vision, technical strategy, feasibility study
synonyms:: meta-coordination, cross-agent coordination, resource allocation
synonyms:: disciplined-research, disciplined-design

trigger:: tasks requiring deep reasoning, architecture decisions, or strategic planning

route:: anthropic, opus
action:: /home/alex/.local/bin/claude --model {{ model }} -p "{{ prompt }}" --max-turns 50

route:: kimi, kimi-for-coding/k2p6
action:: /home/alex/.bun/bin/opencode run -m {{ model }} --format json "{{ prompt }}"

route:: openai, openai/gpt-5.4
action:: /home/alex/.bun/bin/opencode run -m {{ model }} --format json "{{ prompt }}"

route:: openai, opencode/gpt-5.5
action:: /home/alex/.bun/bin/opencode run -m {{ model }} --format json "{{ prompt }}"

route:: openai-codex, gpt-5.5
action:: /home/alex/.local/bin/pi-rust --provider openai-codex --model {{ model }} -p "{{ prompt }}"

# Z.AI Coding Plan: GLM-5.2 for deep reasoning (free via subscription).
route:: zai-coding-plan, glm-5.2
is_free:: true
action:: /home/alex/.local/bin/pi-rust --provider zai-coding-plan --model {{ model }} -p "{{ prompt }}"

# GLM-5.1 as fallback.
route:: zai-coding-plan, glm-5.1
is_free:: true
action:: /home/alex/.local/bin/pi-rust --provider zai-coding-plan --model {{ model }} -p "{{ prompt }}"
