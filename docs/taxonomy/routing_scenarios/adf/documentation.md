# Documentation Routing

Documentation generation, README updates, changelog entries, API docs,
and technical writing. Lower priority since documentation is less time-sensitive.
Best served by models with good prose generation at low cost.

priority:: 40

synonyms:: documentation, readme, changelog, API docs, docstring, rustdoc,
synonyms:: documentation generator, technical writing, release notes, contributing guide,
synonyms:: architecture docs, user guide, mdbook

trigger:: documentation generation and technical writing tasks

route:: minimax, minimax-coding-plan/MiniMax-M2.5
action:: /home/alex/.bun/bin/opencode run -m {{ model }} --format json "{{ prompt }}"

route:: anthropic, claude-sonnet-4-6
action:: /home/alex/.local/bin/claude --model {{ model }} -p "{{ prompt }}" --max-turns 30

route:: zai, zai-coding-plan/glm-5-turbo
action:: /home/alex/.bun/bin/opencode run -m {{ model }} --format json "{{ prompt }}"

route:: openai, openai/gpt-5.4-mini
action:: /home/alex/.bun/bin/opencode run -m {{ model }} --format json "{{ prompt }}"
