# Merge Review Routing

PR merge coordination, verdict collection, approval gating, and merge execution.
The merge coordinator collects verdicts from specialist reviewers and makes
the final merge/reject decision. Needs reliable, fast execution.

priority:: 65

synonyms:: merge, PR review, approve, verdict, merge coordinator,
  merge gate, approval, pull request merge, review verdict,
  merge decision, PR approval, review chain, go no-go

trigger:: pull request merge coordination and approval verdict collection

route:: kimi, kimi-for-coding/k2p5
action:: opencode -m kimi-for-coding/k2p5 -p "{{ prompt }}"

route:: anthropic, claude-sonnet-4-6
action:: /home/alex/.local/bin/claude --model claude-sonnet-4-6 -p "{{ prompt }}" --max-turns 30
