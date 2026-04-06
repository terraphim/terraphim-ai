# Reasoning Routing

Strategic coordination, architecture decisions, product vision, and high-reasoning tasks.
Requires the strongest reasoning model available. Used for meta-coordination,
system design, and decisions that affect the entire project direction.

priority:: 80

synonyms:: meta-coordination, strategic planning, architecture review,
  product vision, system design, meta-coordinator, strategic decision,
  roadmap planning, technical strategy, cross-agent coordination,
  priority assessment, resource allocation, triage

trigger:: high-level strategic reasoning and cross-agent coordination decisions

route:: anthropic, claude-opus-4-6
action:: /home/alex/.local/bin/claude --model claude-opus-4-6 -p "{{ prompt }}" --max-turns 50

route:: anthropic, claude-haiku-4-5
action:: /home/alex/.local/bin/claude --model claude-haiku-4-5 -p "{{ prompt }}" --max-turns 30
