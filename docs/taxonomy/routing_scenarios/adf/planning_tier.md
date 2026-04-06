# Planning Tier

Strategic reasoning, architecture design, research, and high-level decisions.
Uses the strongest reasoning models. Any agent escalates here when task
requires planning, not just meta-coordinator.

Maps to ZDP phases: disciplined-research, disciplined-design.

priority:: 80

synonyms:: strategic planning, architecture design, system design, research
synonyms:: discovery, requirements analysis, feasibility study, risk assessment
synonyms:: product vision, roadmap planning, technical strategy, design decision
synonyms:: create a plan, design the, architect, specification, blueprint
synonyms:: meta-coordination, cross-agent coordination, triage, resource allocation
synonyms:: disciplined-research, disciplined-design

trigger:: tasks requiring deep reasoning, architecture decisions, or strategic planning

route:: anthropic, claude-opus-4-6
action:: /home/alex/.local/bin/claude --model {{ model }} -p "{{ prompt }}" --max-turns 50

route:: openai, openai/gpt-5.4
action:: /home/alex/.bun/bin/opencode run -m {{ model }} --format json "{{ prompt }}"

route:: zai, zai-coding-plan/glm-5
action:: /home/alex/.bun/bin/opencode run -m {{ model }} --format json "{{ prompt }}"
