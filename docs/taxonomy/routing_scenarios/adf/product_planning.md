# Product Planning Routing

Product development, roadmap planning, feature prioritisation, user story creation,
and product ownership tasks. Needs balanced reasoning and good writing for
creating clear, actionable product artefacts.

priority:: 60

synonyms:: product, roadmap, feature prioritisation, user story, product owner,
synonyms:: product development, backlog, sprint planning, product requirements,
synonyms:: feature request, product vision, user need, market fit

trigger:: product planning and feature prioritisation for development roadmap

route:: anthropic, claude-sonnet-4-6
action:: /home/alex/.local/bin/claude --model {{ model }} -p "{{ prompt }}" --max-turns 50

route:: kimi, kimi-for-coding/k2p5
action:: /home/alex/.bun/bin/opencode run -m {{ model }} --format json "{{ prompt }}"

route:: zai, zai-coding-plan/glm-5
action:: /home/alex/.bun/bin/opencode run -m {{ model }} --format json "{{ prompt }}"

route:: openai, openai/gpt-5.4
action:: /home/alex/.bun/bin/opencode run -m {{ model }} --format json "{{ prompt }}"
