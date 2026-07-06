# Recognised Agent

Fleet automation accounts authorised to open auto-merge-eligible PRs. A
Gitea PR author login matching one of these `synonyms::` entries (or
starting with the `adf-` prefix, enforced separately in code) satisfies the
`require_agent_author` auto-merge gate without a human re-reviewing PR
authorship. All other quality gates (confidence, P0/P1 counts, diff size,
acceptance criteria) still apply.

To onboard a new fleet automation account, add its exact Gitea login to the
`synonyms::` line below and restart (or wait for) the orchestrator's next
allowlist reload — no code change or rebuild required.

synonyms:: claude-code, root, implementation-swarm
