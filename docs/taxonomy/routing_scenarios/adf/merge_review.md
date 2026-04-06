# Merge Review Routing

PR merge coordination, verdict collection, approval gating, and merge execution.
The merge coordinator collects verdicts from specialist reviewers and makes
the final merge/reject decision. Needs reliable, fast execution.

priority:: 65

synonyms:: merge, PR review, approve, verdict, merge coordinator,
synonyms:: merge gate, approval, pull request merge, review verdict,
synonyms:: merge decision, PR approval, review chain, go no-go

trigger:: pull request merge coordination and approval verdict collection

route:: kimi, kimi-for-coding/k2p5
action:: /home/alex/.bun/bin/opencode run -m {{ model }} --format json "{{ prompt }}"

route:: anthropic, claude-sonnet-4-6
action:: /home/alex/.local/bin/claude --model {{ model }} -p "{{ prompt }}" --max-turns 30

route:: zai, zai-coding-plan/glm-5-turbo
action:: /home/alex/.bun/bin/opencode run -m {{ model }} --format json "{{ prompt }}"

route:: openai, openai/gpt-5.3-codex
action:: /home/alex/.bun/bin/opencode run -m {{ model }} --format json "{{ prompt }}"
