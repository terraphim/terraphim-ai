---
tracker:
  kind: gitea
  endpoint: https://git.terraphim.cloud
  api_key: $GITEA_TOKEN
  owner: terraphim
  repo: pagerank-viewer

polling:
  interval_ms: 30000

agent:
  runner: claude-code
  max_concurrent_agents: 2
  max_turns: 10
  claude_flags: "--dangerously-skip-permissions --allowedTools Bash,Read,Write,Edit,Glob,Grep"

workspace:
  root: ~/symphony_workspaces

hooks:
  after_create: "git clone https://git.terraphim.cloud/terraphim/pagerank-viewer.git ."
  before_run: "git fetch origin && git checkout main && git pull"
  after_run: "git add -A && git commit -m 'symphony: auto-commit for {{ issue.identifier }}' || true"
  timeout_ms: 60000

codex:
  turn_timeout_ms: 3600000
  stall_timeout_ms: 600000
---
You are working on issue {{ issue.identifier }}: {{ issue.title }}.

{% if issue.description %}
## Issue Description

{{ issue.description }}
{% endif %}

## Instructions

1. Read the issue carefully.
2. Examine the relevant code in this workspace.
3. Implement the required changes following project standards.
4. Write tests to verify your changes.
5. Commit with a message referencing {{ issue.identifier }}.

{% if attempt %}
This is retry attempt {{ attempt }}. Review previous work and continue.
{% endif %}
