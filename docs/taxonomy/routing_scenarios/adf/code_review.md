# Code Review Routing

Architecture review, spec validation, quality assessment, and deep code analysis.
Requires strong reasoning to evaluate design decisions, identify subtle bugs,
and assess architectural coherence across multiple crates.

priority:: 70

synonyms:: code review, architecture review, spec validation, quality assessment,
synonyms:: quality coordinator, design review, PR review quality, code quality,
synonyms:: architectural analysis, spec-validator, compliance review

trigger:: thorough code review requiring architectural reasoning and quality judgement

route:: anthropic, claude-opus-4-6
action:: /home/alex/.local/bin/claude --model {{ model }} -p "{{ prompt }}" --max-turns 50

route:: kimi, kimi-for-coding/k2p5
action:: /home/alex/.bun/bin/opencode run -m {{ model }} --format json "{{ prompt }}"

route:: zai, zai-coding-plan/glm-5
action:: /home/alex/.bun/bin/opencode run -m {{ model }} --format json "{{ prompt }}"

route:: openai, openai/gpt-5.4
action:: /home/alex/.bun/bin/opencode run -m {{ model }} --format json "{{ prompt }}"
