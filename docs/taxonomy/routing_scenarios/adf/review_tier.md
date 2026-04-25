# Review Tier

Verification, validation, compliance checking, and plan review.
Fast, cheap models that check work rather than create it. Used by all
verification agents and for any task that reviews existing output.

Maps to ZDP phases: disciplined-verification, disciplined-validation.

priority:: 40

synonyms:: review plan, check results, verify, validate, compliance check
synonyms:: acceptance test, UAT, quality gate, GO NO-GO, pass fail
synonyms:: check test results, review output, evaluate, assess
synonyms:: drift detection, drift check, compliance audit
synonyms:: documentation review, changelog review, release notes review
synonyms:: disciplined-verification, disciplined-validation

trigger:: verification, validation, and review tasks that check existing work

route:: anthropic, haiku
action:: /home/alex/.local/bin/claude --model {{ model }} -p "{{ prompt }}" --max-turns 30

route:: kimi, kimi-for-coding/k2p5
action:: /home/alex/.bun/bin/opencode run -m {{ model }} --format json "{{ prompt }}"

route:: zai, zai-coding-plan/glm-5-turbo
action:: /home/alex/.bun/bin/opencode run -m {{ model }} --format json "{{ prompt }}"

route:: openai, openai/gpt-5.4-mini
action:: /home/alex/.bun/bin/opencode run -m {{ model }} --format json "{{ prompt }}"

route:: minimax, minimax-coding-plan/MiniMax-M2.5
action:: /home/alex/.bun/bin/opencode run -m {{ model }} --format json "{{ prompt }}"
