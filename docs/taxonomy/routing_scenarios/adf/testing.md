# Testing Routing

Test execution, QA, regression testing, integration testing, and browser-based testing.
Needs reliable models that can run test suites, interpret failures, and suggest fixes.

priority:: 55

synonyms:: test, QA, regression, integration test, browser test, test guardian,
synonyms:: cargo test, test failure, test suite, unit test, end-to-end, e2e test,
synonyms:: browser-qa, test coverage, test fix, flaky test

trigger:: test execution, failure analysis, and quality assurance tasks

route:: kimi, kimi-for-coding/k2p5
action:: /home/alex/.bun/bin/opencode run -m {{ model }} --format json "{{ prompt }}"

route:: anthropic, claude-sonnet-4-6
action:: /home/alex/.local/bin/claude --model {{ model }} -p "{{ prompt }}" --max-turns 50

route:: zai, zai-coding-plan/glm-5-turbo
action:: /home/alex/.bun/bin/opencode run -m {{ model }} --format json "{{ prompt }}"

route:: openai, openai/gpt-5.3-codex
action:: /home/alex/.bun/bin/opencode run -m {{ model }} --format json "{{ prompt }}"
