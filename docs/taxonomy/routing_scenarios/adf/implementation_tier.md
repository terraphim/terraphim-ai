# Implementation Tier

Code implementation, code review, bug fixes, testing, security auditing,
merge review, documentation, and log analysis. Mid-range models balancing
speed, quality, and cost for the workhorse development tasks.

Maps to ZDP phase: disciplined-implementation.

priority:: 50

synonyms:: implement, build, code, fix, refactor, feature, PR, coding task
synonyms:: bug fix, patch, enhancement, migration, scaffold, cargo build
synonyms:: code review, spec validation, quality assessment, design review
synonyms:: merge, PR review, approve, verdict, merge coordinator
synonyms:: test, QA, regression, integration test, cargo test, test failure
synonyms:: security audit, vulnerability scan, CVE, cargo audit
synonyms:: log analysis, error pattern, incident, observability
synonyms:: product development, feature prioritisation, user story
synonyms:: documentation, readme, changelog, API docs, technical writing
synonyms:: disciplined-implementation

trigger:: code writing, review, testing, and mid-complexity development tasks

route:: anthropic, sonnet
action:: /home/alex/.local/bin/claude --model {{ model }} -p "{{ prompt }}" --max-turns 50

route:: kimi, kimi-for-coding/k2p5
action:: /home/alex/.bun/bin/opencode run -m {{ model }} --format json "{{ prompt }}"

route:: openai, openai/gpt-5.3-codex
action:: /home/alex/.bun/bin/opencode run -m {{ model }} --format json "{{ prompt }}"

route:: zai, zai-coding-plan/glm-5-turbo
action:: /home/alex/.bun/bin/opencode run -m {{ model }} --format json "{{ prompt }}"
