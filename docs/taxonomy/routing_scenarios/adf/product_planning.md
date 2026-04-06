# Product Planning Routing

Product development, roadmap planning, feature prioritisation, user story creation,
and product ownership tasks. Needs balanced reasoning and good writing for
creating clear, actionable product artefacts.

priority:: 60

synonyms:: product, roadmap, feature prioritisation, user story, product owner,
  product development, backlog, sprint planning, product requirements,
  feature request, product vision, user need, market fit

trigger:: product planning and feature prioritisation for development roadmap

route:: anthropic, claude-sonnet-4-6
action:: /home/alex/.local/bin/claude --model claude-sonnet-4-6 -p "{{ prompt }}" --max-turns 50

route:: kimi, kimi-for-coding/k2p5
action:: opencode -m kimi-for-coding/k2p5 -p "{{ prompt }}"
