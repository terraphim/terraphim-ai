---
tracker:
  kind: gitea
  owner: terraphim
  repo: test-project
  api_key: $GITEA_TOKEN
  active_states:
    - Todo
    - In Progress
  terminal_states:
    - Done
    - Closed
    - Cancelled

polling:
  interval_ms: 30000

workspace:
  root: /tmp/symphony_workspaces

hooks:
  after_create: "git init && git checkout -b symphony-workspace"
  before_run: "git status"
  timeout_ms: 30000

agent:
  max_concurrent_agents: 5
  max_turns: 10
  max_retry_backoff_ms: 300000

codex:
  command: "codex app-server"
  turn_timeout_ms: 3600000
  read_timeout_ms: 5000
  stall_timeout_ms: 300000
---
You are working on issue {{ issue.identifier }}: {{ issue.title }}.

{% if issue.description %}
## Issue Description

{{ issue.description }}
{% endif %}

## Instructions

1. Read the issue carefully and understand what needs to be done.
2. Examine the relevant code in this workspace.
3. Implement the required changes following the project's coding standards.
4. Write tests to verify your changes.
5. Commit your changes with a clear message referencing {{ issue.identifier }}.

{% if attempt %}
This is retry attempt {{ attempt }}. Review any previous work and continue from where you left off.
{% endif %}
