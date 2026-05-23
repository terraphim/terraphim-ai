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

route:: pi-rust-openai-codex, gpt-5.5
action:: /home/alex/.local/bin/pi-rust --provider openai-codex --model {{ model }} -p "{{ prompt }}"

# Z.AI Coding Plan healthy via pi-rust; broken via opencode 1.14.48
# (opencode emits only step_start, no text). Investigation: 2026-05-23.
# Route through pi-rust until opencode integration is fixed upstream.
route:: pi-rust-zai, glm-5.1
is_free:: true
action:: /home/alex/.local/bin/pi-rust --provider zai-coding-plan --model {{ model }} -p "{{ prompt }}"
