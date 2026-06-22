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

# Z.AI Coding Plan: GLM-5.2 is the latest model (free via subscription).
# Route through pi-rust; opencode integration broken for zai-coding-plan.
route:: zai-coding-plan, zai-coding-plan/glm-5.2
is_free:: true
action:: /home/alex/.local/bin/pi-rust --provider zai-coding-plan --model {{ model }} -p "{{ prompt }}"

# GLM-5.1 as fallback when 5.2 is unavailable.
route:: zai-coding-plan, zai-coding-plan/glm-5.1
is_free:: true
action:: /home/alex/.local/bin/pi-rust --provider zai-coding-plan --model {{ model }} -p "{{ prompt }}"

route:: anthropic, sonnet
action:: /home/alex/.local/bin/claude --model {{ model }} -p "{{ prompt }}" --max-turns 50

route:: kimi, kimi-for-coding/k2p5
action:: /home/alex/.bun/bin/opencode run -m {{ model }} --format json "{{ prompt }}"

route:: openai, openai/gpt-5.3-codex
action:: /home/alex/.bun/bin/opencode run -m {{ model }} --format json "{{ prompt }}"

# MiniMax-M3 via opencode (subscription plan).
route:: minimax, minimax-coding-plan/MiniMax-M3
action:: /home/alex/.bun/bin/opencode run -m {{ model }} --format json "{{ prompt }}"

# MiniMax-M3 via pi-rust (subscription plan).
route:: minimax-coding-plan, MiniMax-M3
action:: /home/alex/.local/bin/pi-rust --provider minimax-coding-plan --model {{ model }} -p "{{ prompt }}"

# MiniMax-M2.7 as fallback.
route:: minimax, minimax-coding-plan/MiniMax-M2.7-highspeed
action:: /home/alex/.bun/bin/opencode run -m {{ model }} --format json "{{ prompt }}"

route:: minimax-coding-plan, MiniMax-M2.7-highspeed
action:: /home/alex/.local/bin/pi-rust --provider minimax-coding-plan --model {{ model }} -p "{{ prompt }}"
