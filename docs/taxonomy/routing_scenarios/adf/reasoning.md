# Reasoning Routing

Strategic coordination, architecture decisions, product vision, and high-reasoning tasks.
Requires the strongest reasoning model available. Used for meta-coordination,
system design, and decisions that affect the entire project direction.

priority:: 80

synonyms:: meta-coordination, strategic planning, architecture review,
synonyms:: product vision, system design, meta-coordinator, strategic decision,
synonyms:: roadmap planning, technical strategy, cross-agent coordination,
synonyms:: priority assessment, resource allocation, triage

trigger:: high-level strategic reasoning and cross-agent coordination decisions

route:: anthropic, claude-opus-4-6
action:: /home/alex/.local/bin/claude --model {{ model }} -p "{{ prompt }}" --max-turns 50

route:: anthropic, claude-haiku-4-5
action:: /home/alex/.local/bin/claude --model {{ model }} -p "{{ prompt }}" --max-turns 30

route:: zai, zai-coding-plan/glm-5
action:: /home/alex/.bun/bin/opencode run -m {{ model }} --format json "{{ prompt }}"

route:: openai, openai/gpt-5.4
action:: /home/alex/.bun/bin/opencode run -m {{ model }} --format json "{{ prompt }}"
